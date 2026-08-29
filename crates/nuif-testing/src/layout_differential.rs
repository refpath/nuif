//! Foreign-reference layout trials against Taffy and a pinned headless Chrome.

use crate::responsive_card_fixture;
use nuif_core::{
    Align, Document, Edges, Entity, EntityId, EntityKind, FlowDirection, LayoutFamily, LayoutStyle,
    SizeIntent,
};
use nuif_layout::{EvaluationContext, Rect as NuifRect, evaluate};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use taffy::prelude::{
    AlignItems, AvailableSpace, Dimension, Display, FlexDirection, NodeId, Position,
    Rect as TaffyRect, Size as TaffySize, Style, TaffyTree, auto, length, percent,
};

pub const TAFFY_VERSION: &str = "0.14.0";
pub const PINNED_CHROME_VERSION: &str = "152.0.7977.64";
pub const MAX_FOREIGN_TOLERANCE_PX: f64 = 0.1;
const DEFAULT_SEED: u64 = 0x4e55_4946_4c41_594f;

#[derive(Clone, Debug)]
pub struct DifferentialConfig {
    pub seed: u64,
    pub generated_cases: u32,
    pub output: PathBuf,
    pub chrome: Option<PathBuf>,
    pub enforce_browser_version: bool,
}

impl Default for DifferentialConfig {
    fn default() -> Self {
        Self {
            seed: DEFAULT_SEED,
            generated_cases: 12,
            output: PathBuf::from("target/layout-differential-report.json"),
            chrome: None,
            enforce_browser_version: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DifferentialReport {
    pub schema_version: u32,
    pub seed: u64,
    pub engines: DifferentialEngines,
    pub cases: Vec<DifferentialCaseReport>,
    pub summary: DifferentialSummary,
}

impl DifferentialReport {
    #[must_use]
    pub fn passed(&self) -> bool {
        self.summary.unclassified_divergences == 0
            && self.summary.blocking_divergences == 0
            && (!self.engines.browser.version_enforced || self.engines.browser.version_matches_pin)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DifferentialEngines {
    pub reference: EngineVersion,
    pub taffy: EngineVersion,
    pub browser: BrowserVersion,
    pub rust_toolchain: String,
    pub source_revision: Option<String>,
    pub dirty: Option<bool>,
    pub operating_system: String,
    pub architecture: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineVersion {
    pub name: String,
    pub version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserVersion {
    pub name: String,
    pub expected: String,
    pub observed: String,
    pub executable: String,
    pub version_matches_pin: bool,
    pub version_enforced: bool,
    pub launch_arguments: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DifferentialSummary {
    pub cases: usize,
    pub comparisons: usize,
    pub compared_components: usize,
    pub classified_divergences: usize,
    pub unclassified_divergences: usize,
    pub blocking_divergences: usize,
    pub schema_losses: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DifferentialCaseReport {
    pub name: String,
    pub source: String,
    pub viewport: (u32, u32),
    pub tolerance: FixtureTolerance,
    pub boxes: EngineBoxes,
    pub comparisons: Vec<EngineComparison>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureTolerance {
    pub assertion_px: f64,
    pub observed_taffy_browser_max_px: f64,
    pub rationale: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineBoxes {
    pub reference: BTreeMap<EntityId, NuifRect>,
    pub taffy: BTreeMap<EntityId, NuifRect>,
    pub browser: BTreeMap<EntityId, NuifRect>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineComparison {
    pub left: String,
    pub right: String,
    pub compared_components: usize,
    pub max_delta_px: f64,
    pub divergences: Vec<LayoutDivergence>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutDivergence {
    pub entity: EntityId,
    pub component: RectComponent,
    pub left_value: f64,
    pub right_value: f64,
    pub absolute_delta_px: f64,
    pub classification: DivergenceClassification,
    pub explanation: String,
    pub blocking: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RectComponent {
    X,
    Y,
    Width,
    Height,
    Presence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DivergenceClassification {
    SchemaLoss,
    ReferenceEvaluatorDefect,
    TaffyDifference,
    BrowserTargetDifference,
    Unclassified,
}

struct DifferentialCase {
    name: String,
    source: String,
    viewport: (u32, u32),
    document: Document,
}

#[derive(Clone)]
struct ResolvedStyle {
    width: SizeIntent,
    height: SizeIntent,
    direction: FlowDirection,
    gap: f64,
}

#[derive(Clone, Copy)]
struct ParentStyle {
    family: LayoutFamily,
    direction: FlowDirection,
}

/// Runs every differential fixture and writes its complete machine report.
///
/// # Errors
///
/// Returns an error when the browser cannot be located or executed, an oracle
/// cannot evaluate a fixture, or the report cannot be serialized or written.
pub fn run_and_write(config: &DifferentialConfig) -> Result<DifferentialReport, String> {
    let chrome = locate_chrome(config.chrome.as_deref())?;
    let observed_version = browser_version(&chrome)?;
    let version_matches_pin = observed_version == PINNED_CHROME_VERSION;
    let cases = differential_cases(config.seed, config.generated_cases);
    let mut reports = Vec::with_capacity(cases.len());
    let mut summary = DifferentialSummary::default();
    for case in cases {
        let report = run_case(&case, &chrome)?;
        accumulate_summary(&mut summary, &report);
        reports.push(report);
    }
    summary.cases = reports.len();
    let report = DifferentialReport {
        schema_version: 1,
        seed: config.seed,
        engines: DifferentialEngines {
            reference: EngineVersion {
                name: "nuif-reference-layout".to_owned(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
            },
            taffy: EngineVersion {
                name: "taffy".to_owned(),
                version: TAFFY_VERSION.to_owned(),
            },
            browser: BrowserVersion {
                name: "Chrome for Testing".to_owned(),
                expected: PINNED_CHROME_VERSION.to_owned(),
                observed: observed_version,
                executable: chrome.display().to_string(),
                version_matches_pin,
                version_enforced: config.enforce_browser_version,
                launch_arguments: chrome_arguments(),
            },
            rust_toolchain: command_text("rustc", &["-Vv"]).unwrap_or_else(|| "unknown".to_owned()),
            source_revision: command_text("git", &["rev-parse", "HEAD"]),
            dirty: command_text("git", &["status", "--porcelain"]).map(|value| !value.is_empty()),
            operating_system: env::consts::OS.to_owned(),
            architecture: env::consts::ARCH.to_owned(),
        },
        cases: reports,
        summary,
    };
    if let Some(parent) = config.output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let encoded = serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?;
    fs::write(&config.output, encoded).map_err(|error| error.to_string())?;
    Ok(report)
}

fn run_case(case: &DifferentialCase, chrome: &Path) -> Result<DifferentialCaseReport, String> {
    let context =
        EvaluationContext::viewport(f64::from(case.viewport.0), f64::from(case.viewport.1));
    let reference = evaluate(&case.document, &context).boxes;
    let taffy = evaluate_taffy(&case.document, &context)?;
    let browser = evaluate_browser(&case.document, &context, chrome)?;
    let foreign_max = max_box_delta(&taffy, &browser);
    let tolerance = calibrated_tolerance(foreign_max);
    let comparisons = vec![
        compare_engines(
            "reference",
            &reference,
            "taffy",
            &taffy,
            &case.document,
            tolerance,
        ),
        compare_engines(
            "reference",
            &reference,
            "browser",
            &browser,
            &case.document,
            tolerance,
        ),
        compare_engines(
            "taffy",
            &taffy,
            "browser",
            &browser,
            &case.document,
            tolerance,
        ),
    ];
    Ok(DifferentialCaseReport {
        name: case.name.clone(),
        source: case.source.clone(),
        viewport: case.viewport,
        tolerance: FixtureTolerance {
            assertion_px: tolerance,
            observed_taffy_browser_max_px: foreign_max,
            rationale: "fixture-local ceiling of the measured Taffy/browser maximum to 0.01 px; exact foreign agreement stays exact, and the safety ceiling is 0.1 px".to_owned(),
        },
        boxes: EngineBoxes {
            reference,
            taffy,
            browser,
        },
        comparisons,
    })
}

fn accumulate_summary(summary: &mut DifferentialSummary, report: &DifferentialCaseReport) {
    summary.comparisons += report.comparisons.len();
    for comparison in &report.comparisons {
        summary.compared_components += comparison.compared_components;
        for divergence in &comparison.divergences {
            if divergence.classification == DivergenceClassification::Unclassified {
                summary.unclassified_divergences += 1;
            } else {
                summary.classified_divergences += 1;
            }
            if divergence.blocking {
                summary.blocking_divergences += 1;
            }
            if divergence.classification == DivergenceClassification::SchemaLoss {
                summary.schema_losses += 1;
            }
        }
    }
}

fn compare_engines(
    left_name: &str,
    left: &BTreeMap<EntityId, NuifRect>,
    right_name: &str,
    right: &BTreeMap<EntityId, NuifRect>,
    document: &Document,
    tolerance: f64,
) -> EngineComparison {
    let ids: BTreeSet<_> = left.keys().chain(right.keys()).copied().collect();
    let mut divergences = Vec::new();
    let mut max_delta = 0.0_f64;
    let mut compared_components = 0;
    for id in ids {
        let (Some(left_rect), Some(right_rect)) = (left.get(&id), right.get(&id)) else {
            let (classification, explanation, blocking) =
                classify(document, id, RectComponent::Presence, left_name, right_name);
            divergences.push(LayoutDivergence {
                entity: id,
                component: RectComponent::Presence,
                left_value: f64::from(left.contains_key(&id)),
                right_value: f64::from(right.contains_key(&id)),
                absolute_delta_px: 1.0,
                classification,
                explanation,
                blocking,
            });
            continue;
        };
        for (component, left_value, right_value) in rect_components(left_rect, right_rect) {
            compared_components += 1;
            let delta = (left_value - right_value).abs();
            max_delta = max_delta.max(delta);
            if delta > tolerance {
                let (classification, explanation, blocking) =
                    classify(document, id, component, left_name, right_name);
                divergences.push(LayoutDivergence {
                    entity: id,
                    component,
                    left_value,
                    right_value,
                    absolute_delta_px: delta,
                    classification,
                    explanation,
                    blocking,
                });
            }
        }
    }
    EngineComparison {
        left: left_name.to_owned(),
        right: right_name.to_owned(),
        compared_components,
        max_delta_px: max_delta,
        divergences,
    }
}

fn calibrated_tolerance(observed_foreign_max: f64) -> f64 {
    ((observed_foreign_max * 100.0).ceil() / 100.0).min(MAX_FOREIGN_TOLERANCE_PX)
}

fn rect_components(left: &NuifRect, right: &NuifRect) -> [(RectComponent, f64, f64); 4] {
    [
        (RectComponent::X, left.x, right.x),
        (RectComponent::Y, left.y, right.y),
        (RectComponent::Width, left.width, right.width),
        (RectComponent::Height, left.height, right.height),
    ]
}

fn classify(
    document: &Document,
    entity: EntityId,
    _component: RectComponent,
    left: &str,
    right: &str,
) -> (DivergenceClassification, String, bool) {
    if has_grid_ancestor(document, entity) {
        return (
            DivergenceClassification::SchemaLoss,
            "profile 0 names the grid family but has no authored track or placement fields; the reference stack fallback cannot preserve CSS Grid auto-placement".to_owned(),
            false,
        );
    }
    match (left, right) {
        ("reference", _) | (_, "reference") => (
            DivergenceClassification::ReferenceEvaluatorDefect,
            "the CSS-compatible subset differs from a foreign layout engine and requires a reference-evaluator correction or a narrower declared equivalence".to_owned(),
            true,
        ),
        ("taffy", "browser") | ("browser", "taffy") => (
            DivergenceClassification::TaffyDifference,
            "Taffy and the pinned browser disagree outside the fixture tolerance".to_owned(),
            true,
        ),
        _ => (
            DivergenceClassification::Unclassified,
            "the engine pair is not covered by the differential classifier".to_owned(),
            true,
        ),
    }
}

fn has_grid_ancestor(document: &Document, entity: EntityId) -> bool {
    let parents = parent_map(document);
    let mut current = Some(entity);
    while let Some(id) = current {
        if document
            .entities
            .get(&id)
            .is_some_and(|item| item.authored.layout.family == LayoutFamily::Grid)
        {
            return true;
        }
        current = parents.get(&id).copied();
    }
    false
}

fn parent_map(document: &Document) -> BTreeMap<EntityId, EntityId> {
    let mut parents = BTreeMap::new();
    for (parent, entity) in &document.entities {
        for child in &entity.children {
            parents.insert(*child, *parent);
        }
    }
    parents
}

fn max_box_delta(left: &BTreeMap<EntityId, NuifRect>, right: &BTreeMap<EntityId, NuifRect>) -> f64 {
    left.iter()
        .filter_map(|(id, left_rect)| right.get(id).map(|right_rect| (left_rect, right_rect)))
        .flat_map(|(left_rect, right_rect)| rect_components(left_rect, right_rect))
        .map(|(_, left_value, right_value)| (left_value - right_value).abs())
        .fold(0.0_f64, f64::max)
}

fn evaluate_taffy(
    document: &Document,
    context: &EvaluationContext,
) -> Result<BTreeMap<EntityId, NuifRect>, String> {
    let mut tree = TaffyTree::<()>::new();
    tree.disable_rounding();
    let mut nodes = BTreeMap::new();
    let mut roots = Vec::new();
    for root in &document.roots {
        roots.push(build_taffy_node(
            document, *root, None, context, &mut tree, &mut nodes,
        )?);
    }
    for root in &roots {
        tree.compute_layout(
            *root,
            TaffySize {
                width: AvailableSpace::Definite(to_f32(context.viewport.width)),
                height: AvailableSpace::Definite(to_f32(context.viewport.height)),
            },
        )
        .map_err(|error| error.to_string())?;
    }
    let reverse: Vec<_> = nodes.iter().map(|(id, node)| (*node, *id)).collect();
    let mut boxes = BTreeMap::new();
    for root in roots {
        collect_taffy_boxes(&tree, root, &reverse, 0.0, 0.0, &mut boxes)?;
    }
    Ok(boxes)
}

fn build_taffy_node(
    document: &Document,
    id: EntityId,
    parent: Option<ParentStyle>,
    context: &EvaluationContext,
    tree: &mut TaffyTree<()>,
    nodes: &mut BTreeMap<EntityId, NodeId>,
) -> Result<NodeId, String> {
    let entity = document
        .entities
        .get(&id)
        .ok_or_else(|| format!("missing entity {id}"))?;
    let resolved = resolve_style(entity, context);
    let own = ParentStyle {
        family: entity.authored.layout.family,
        direction: resolved.direction,
    };
    let mut children = Vec::with_capacity(entity.children.len());
    for child in &entity.children {
        children.push(build_taffy_node(
            document,
            *child,
            Some(own),
            context,
            tree,
            nodes,
        )?);
    }
    let style = taffy_style(entity, &resolved, parent, context);
    let node = tree
        .new_with_children(style, &children)
        .map_err(|error| error.to_string())?;
    nodes.insert(id, node);
    Ok(node)
}

fn taffy_style(
    entity: &Entity,
    resolved: &ResolvedStyle,
    parent: Option<ParentStyle>,
    context: &EvaluationContext,
) -> Style {
    let is_root = parent.is_none();
    let mut style = Style {
        display: match entity.authored.layout.family {
            LayoutFamily::Grid => Display::Grid,
            _ => Display::Flex,
        },
        flex_direction: match resolved.direction {
            FlowDirection::Row => FlexDirection::Row,
            FlowDirection::Column => FlexDirection::Column,
        },
        align_items: Some(match entity.authored.layout.align {
            Align::Start => AlignItems::START,
            Align::Center => AlignItems::CENTER,
            Align::End => AlignItems::END,
            Align::Stretch => AlignItems::STRETCH,
        }),
        gap: TaffySize::length(resolved.gap.max(0.0)),
        padding: TaffyRect {
            left: length(entity.authored.layout.padding.left),
            right: length(entity.authored.layout.padding.right),
            top: length(entity.authored.layout.padding.top),
            bottom: length(entity.authored.layout.padding.bottom),
        },
        min_size: TaffySize::length(0.0),
        ..Style::default()
    };
    style.size = if is_root {
        TaffySize {
            width: length(context.viewport.width),
            height: length(context.viewport.height),
        }
    } else {
        TaffySize {
            width: dimension(&resolved.width, intrinsic_width(entity)),
            height: dimension(&resolved.height, intrinsic_height(entity)),
        }
    };
    if let Some(parent) = parent {
        if matches!(
            parent.family,
            LayoutFamily::Freeform | LayoutFamily::Constraint
        ) {
            style.position = Position::Absolute;
            style.inset.left = length(entity.authored.position.x);
            style.inset.top = length(entity.authored.position.y);
        } else if parent.family != LayoutFamily::Grid {
            let main = match parent.direction {
                FlowDirection::Row => &resolved.width,
                FlowDirection::Column => &resolved.height,
            };
            if matches!(main, SizeIntent::Fill) {
                style.flex_grow = 1.0;
                style.flex_shrink = 1.0;
                style.flex_basis = length(0.0);
            } else {
                style.flex_shrink = 0.0;
            }
            let cross = match parent.direction {
                FlowDirection::Row => &resolved.height,
                FlowDirection::Column => &resolved.width,
            };
            if matches!(cross, SizeIntent::Fill) {
                style.align_self = Some(taffy::style::AlignSelf::STRETCH);
            }
        }
    }
    style
}

fn dimension(intent: &SizeIntent, intrinsic: f64) -> Dimension {
    match intent {
        SizeIntent::Auto | SizeIntent::Fill => auto(),
        SizeIntent::Fixed(value) => length(*value),
        SizeIntent::Intrinsic | SizeIntent::MinContent | SizeIntent::MaxContent => {
            length(intrinsic)
        }
        SizeIntent::Percentage(value) => percent(*value / 100.0),
        SizeIntent::FitContent(limit) => length(intrinsic.min(*limit)),
    }
}

fn collect_taffy_boxes(
    tree: &TaffyTree<()>,
    node: NodeId,
    ids: &[(NodeId, EntityId)],
    parent_x: f64,
    parent_y: f64,
    boxes: &mut BTreeMap<EntityId, NuifRect>,
) -> Result<(), String> {
    let layout = tree.layout(node).map_err(|error| error.to_string())?;
    let x = parent_x + f64::from(layout.location.x);
    let y = parent_y + f64::from(layout.location.y);
    if let Some((_, id)) = ids.iter().find(|(candidate, _)| *candidate == node) {
        boxes.insert(
            *id,
            NuifRect {
                x,
                y,
                width: f64::from(layout.size.width),
                height: f64::from(layout.size.height),
            },
        );
    }
    for child in tree.children(node).map_err(|error| error.to_string())? {
        collect_taffy_boxes(tree, child, ids, x, y, boxes)?;
    }
    Ok(())
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "Taffy uses f32 geometry; finite profile-0 viewport values are intentionally lowered"
)]
fn to_f32(value: f64) -> f32 {
    value as f32
}

fn evaluate_browser(
    document: &Document,
    context: &EvaluationContext,
    chrome: &Path,
) -> Result<BTreeMap<EntityId, NuifRect>, String> {
    let directory = env::temp_dir().join(format!(
        "nuif-layout-differential-{}-{}",
        std::process::id(),
        context.fingerprint().replace([':', '@'], "-")
    ));
    if directory.exists() {
        return Err(format!(
            "temporary browser directory already exists: {}",
            directory.display()
        ));
    }
    fs::create_dir(&directory).map_err(|error| error.to_string())?;
    let page = directory.join("fixture.html");
    fs::write(&page, browser_html(document, context)?).map_err(|error| error.to_string())?;
    let url = file_url(&page)?;
    let output = Command::new(chrome)
        .args(chrome_arguments())
        .arg(url)
        .output()
        .map_err(|error| error.to_string());
    let cleanup = fs::remove_dir_all(&directory);
    let output = output?;
    cleanup.map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "Chrome exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    parse_browser_output(&String::from_utf8(output.stdout).map_err(|error| error.to_string())?)
}

fn chrome_arguments() -> Vec<String> {
    [
        "--headless=new",
        "--disable-gpu",
        "--no-sandbox",
        "--hide-scrollbars",
        "--force-device-scale-factor=1",
        "--dump-dom",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn browser_html(document: &Document, context: &EvaluationContext) -> Result<String, String> {
    let mut html = String::from(
        "<!doctype html><html><head><meta charset=\"utf-8\"><style>html,body{margin:0;padding:0;}*{box-sizing:border-box;min-width:0;min-height:0;}body{overflow:hidden;}</style></head><body>",
    );
    for root in &document.roots {
        write_browser_entity(document, *root, None, context, &mut html)?;
    }
    html.push_str(
        "<script>(()=>{const boxes={};for(const el of document.querySelectorAll('[data-nuif-id]')){const r=el.getBoundingClientRect();boxes[el.dataset.nuifId]={x:r.x,y:r.y,width:r.width,height:r.height};}document.body.replaceChildren(document.createTextNode(JSON.stringify(boxes)));})();</script></body></html>",
    );
    Ok(html)
}

fn write_browser_entity(
    document: &Document,
    id: EntityId,
    parent: Option<ParentStyle>,
    context: &EvaluationContext,
    html: &mut String,
) -> Result<(), String> {
    let entity = document
        .entities
        .get(&id)
        .ok_or_else(|| format!("missing entity {id}"))?;
    let resolved = resolve_style(entity, context);
    let own = ParentStyle {
        family: entity.authored.layout.family,
        direction: resolved.direction,
    };
    write!(html, "<div data-nuif-id=\"{id}\" style=\"").map_err(|error| error.to_string())?;
    write_browser_style(entity, &resolved, parent, context, html)?;
    html.push_str("\">");
    for child in &entity.children {
        write_browser_entity(document, *child, Some(own), context, html)?;
    }
    html.push_str("</div>");
    Ok(())
}

fn write_browser_style(
    entity: &Entity,
    resolved: &ResolvedStyle,
    parent: Option<ParentStyle>,
    context: &EvaluationContext,
    css: &mut String,
) -> Result<(), String> {
    match entity.authored.layout.family {
        LayoutFamily::Grid => css.push_str("display:grid;"),
        _ => css.push_str("display:flex;"),
    }
    css.push_str("position:relative;");
    write!(
        css,
        "flex-direction:{};gap:{}px;padding:{}px {}px {}px {}px;align-items:{};",
        if resolved.direction == FlowDirection::Row {
            "row"
        } else {
            "column"
        },
        resolved.gap.max(0.0),
        entity.authored.layout.padding.top,
        entity.authored.layout.padding.right,
        entity.authored.layout.padding.bottom,
        entity.authored.layout.padding.left,
        match entity.authored.layout.align {
            Align::Start => "flex-start",
            Align::Center => "center",
            Align::End => "flex-end",
            Align::Stretch => "stretch",
        },
    )
    .map_err(|error| error.to_string())?;
    if parent.is_none() {
        write!(
            css,
            "width:{}px;height:{}px;flex:none;",
            context.viewport.width, context.viewport.height
        )
        .map_err(|error| error.to_string())?;
        return Ok(());
    }
    write_css_dimension(css, "width", &resolved.width, intrinsic_width(entity))?;
    write_css_dimension(css, "height", &resolved.height, intrinsic_height(entity))?;
    if let Some(parent) = parent {
        if matches!(
            parent.family,
            LayoutFamily::Freeform | LayoutFamily::Constraint
        ) {
            write!(
                css,
                "position:absolute;left:{}px;top:{}px;",
                entity.authored.position.x, entity.authored.position.y
            )
            .map_err(|error| error.to_string())?;
        } else if parent.family != LayoutFamily::Grid {
            let main = match parent.direction {
                FlowDirection::Row => &resolved.width,
                FlowDirection::Column => &resolved.height,
            };
            if matches!(main, SizeIntent::Fill) {
                css.push_str("flex:1 1 0px;");
            } else {
                css.push_str("flex:0 0 auto;");
            }
            let cross = match parent.direction {
                FlowDirection::Row => &resolved.height,
                FlowDirection::Column => &resolved.width,
            };
            if matches!(cross, SizeIntent::Fill) {
                css.push_str("align-self:stretch;");
            }
        }
    }
    Ok(())
}

fn write_css_dimension(
    css: &mut String,
    property: &str,
    intent: &SizeIntent,
    intrinsic: f64,
) -> Result<(), String> {
    match intent {
        SizeIntent::Auto | SizeIntent::Fill => write!(css, "{property}:auto;"),
        SizeIntent::Fixed(value) => write!(css, "{property}:{value}px;"),
        SizeIntent::Intrinsic | SizeIntent::MinContent | SizeIntent::MaxContent => {
            write!(css, "{property}:{intrinsic}px;")
        }
        SizeIntent::Percentage(value) => write!(css, "{property}:{value}%;"),
        SizeIntent::FitContent(limit) => {
            write!(css, "{property}:{}px;", intrinsic.min(*limit))
        }
    }
    .map_err(|error| error.to_string())
}

fn parse_browser_output(output: &str) -> Result<BTreeMap<EntityId, NuifRect>, String> {
    let start = output
        .find("<body>")
        .ok_or_else(|| "Chrome output has no body".to_owned())?
        + "<body>".len();
    let end = output[start..]
        .find("</body>")
        .map(|offset| start + offset)
        .ok_or_else(|| "Chrome output has no closing body".to_owned())?;
    serde_json::from_str(&output[start..end]).map_err(|error| error.to_string())
}

fn file_url(path: &Path) -> Result<String, String> {
    let canonical = path.canonicalize().map_err(|error| error.to_string())?;
    let value = canonical
        .to_str()
        .ok_or_else(|| format!("path is not valid UTF-8: {}", canonical.display()))?;
    Ok(format!(
        "file://{}",
        value
            .replace('%', "%25")
            .replace(' ', "%20")
            .replace('#', "%23")
    ))
}

fn resolve_style(entity: &Entity, context: &EvaluationContext) -> ResolvedStyle {
    let mut resolved = ResolvedStyle {
        width: entity.authored.width.clone(),
        height: entity.authored.height.clone(),
        direction: entity.authored.layout.direction,
        gap: entity.authored.layout.gap,
    };
    for rule in &entity.authored.responsive {
        let matches_width = rule
            .when
            .min_width
            .is_none_or(|minimum| context.viewport.width >= minimum)
            && rule
                .when
                .max_width
                .is_none_or(|maximum| context.viewport.width <= maximum);
        let matches_theme = rule.when.theme.as_ref().is_none_or(|theme| {
            context
                .theme
                .as_ref()
                .is_some_and(|current| current == theme)
        });
        if matches_width && matches_theme {
            if let Some(value) = &rule.width {
                resolved.width = value.clone();
            }
            if let Some(value) = &rule.height {
                resolved.height = value.clone();
            }
            if let Some(value) = rule.direction {
                resolved.direction = value;
            }
            if let Some(value) = rule.gap {
                resolved.gap = value;
            }
        }
    }
    resolved
}

fn intrinsic_width(entity: &Entity) -> f64 {
    entity.authored.text.as_ref().map_or(0.0, |text| {
        let longest = text
            .content
            .lines()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0);
        f64::from(u32::try_from(longest).unwrap_or(u32::MAX)) * text.size * 0.6
    })
}

fn intrinsic_height(entity: &Entity) -> f64 {
    entity.authored.text.as_ref().map_or(0.0, |text| {
        f64::from(u32::try_from(text.content.lines().count().max(1)).unwrap_or(u32::MAX))
            * text.line_height
    })
}

fn differential_cases(seed: u64, generated: u32) -> Vec<DifferentialCase> {
    let mut cases = Vec::new();
    for viewport in [(360, 640), (768, 768), (1440, 900)] {
        cases.push(DifferentialCase {
            name: format!("v0-responsive-card-{}x{}", viewport.0, viewport.1),
            source: "conformance/fixtures/v0-responsive-card".to_owned(),
            viewport,
            document: responsive_card_fixture(),
        });
    }
    let mut rng = DifferentialRng::new(seed);
    for index in 0..generated {
        let family = match index % 3 {
            0 => LayoutFamily::Stack,
            1 => LayoutFamily::Flex,
            _ => LayoutFamily::Grid,
        };
        let viewport = (320 + rng.bounded(1000), 320 + rng.bounded(580));
        cases.push(DifferentialCase {
            name: format!("generated-{index:03}-{family:?}").to_lowercase(),
            source: format!("seed:{seed}:case:{index}"),
            viewport,
            document: generated_case(&mut rng, index, family, viewport),
        });
    }
    cases
}

fn generated_case(
    rng: &mut DifferentialRng,
    index: u32,
    family: LayoutFamily,
    _viewport: (u32, u32),
) -> Document {
    let document_id = EntityId::new(0x1000 + u128::from(index) * 0x100);
    let root_id = EntityId::new(document_id.0 + 1);
    let mut document = Document::empty(document_id);
    let direction = if rng.bounded(2) == 0 {
        FlowDirection::Row
    } else {
        FlowDirection::Column
    };
    let padding = f64::from(rng.bounded(17));
    let gap = f64::from(rng.bounded(13));
    let mut root = Entity::new(root_id, EntityKind::Container);
    root.authored.width = SizeIntent::Fill;
    root.authored.height = SizeIntent::Fill;
    root.authored.layout = LayoutStyle {
        family,
        direction,
        gap,
        padding: Edges {
            top: padding,
            right: padding,
            bottom: padding,
            left: padding,
        },
        align: match rng.bounded(4) {
            0 => Align::Start,
            1 => Align::Center,
            2 => Align::End,
            _ => Align::Stretch,
        },
    };
    let child_count = 2 + rng.bounded(4);
    for child_index in 0..child_count {
        let child_id = EntityId::new(root_id.0 + 1 + u128::from(child_index));
        let mut child = Entity::new(child_id, EntityKind::Shape(nuif_core::ShapeKind::Rectangle));
        let fixed_main = SizeIntent::Fixed(f64::from(24 + rng.bounded(96)));
        let fixed_cross = SizeIntent::Fixed(f64::from(16 + rng.bounded(80)));
        let main = if family != LayoutFamily::Grid && rng.bounded(3) == 0 {
            SizeIntent::Fill
        } else {
            fixed_main
        };
        let cross = if rng.bounded(3) == 0 {
            SizeIntent::Fill
        } else {
            fixed_cross
        };
        match direction {
            FlowDirection::Row => {
                child.authored.width = main;
                child.authored.height = cross;
            }
            FlowDirection::Column => {
                child.authored.width = cross;
                child.authored.height = main;
            }
        }
        root.children.push(child_id);
        document.entities.insert(child_id, child);
    }
    document.roots.push(root_id);
    document.entities.insert(root_id, root);
    document
}

struct DifferentialRng(u64);

impl DifferentialRng {
    const fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        u32::try_from(self.0 >> 32).unwrap_or(u32::MAX)
    }

    fn bounded(&mut self, exclusive: u32) -> u32 {
        self.next() % exclusive
    }
}

fn locate_chrome(explicit: Option<&Path>) -> Result<PathBuf, String> {
    let mut candidates = Vec::new();
    if let Some(path) = explicit {
        candidates.push(path.to_path_buf());
    }
    if let Some(path) = env::var_os("NUIF_CHROME") {
        candidates.push(PathBuf::from(path));
    }
    candidates.extend([
        PathBuf::from("target/chrome-for-testing/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing"),
        PathBuf::from("target/chrome-for-testing/chrome-mac-x64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing"),
        PathBuf::from("target/chrome-for-testing/chrome-linux64/chrome"),
        PathBuf::from("/Applications/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing"),
        PathBuf::from("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"),
        PathBuf::from("/usr/bin/google-chrome"),
        PathBuf::from("/usr/bin/chromium"),
    ]);
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(|| {
            "Chrome not found; run tools/browser/install-chrome-for-testing.sh or set NUIF_CHROME"
                .to_owned()
        })
}

fn browser_version(chrome: &Path) -> Result<String, String> {
    let output = Command::new(chrome)
        .arg("--version")
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(format!("{} --version failed", chrome.display()));
    }
    let text = String::from_utf8(output.stdout).map_err(|error| error.to_string())?;
    text.split_whitespace()
        .find(|part| {
            part.chars()
                .next()
                .is_some_and(|first| first.is_ascii_digit())
        })
        .map(str::to_owned)
        .ok_or_else(|| format!("could not parse Chrome version from {text:?}"))
}

fn command_text(program: &str, arguments: &[&str]) -> Option<String> {
    let output = Command::new(program).args(arguments).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_cases_are_valid_and_deterministic() {
        let first = differential_cases(7, 6);
        let second = differential_cases(7, 6);
        assert_eq!(first.len(), 9);
        assert!(first.iter().all(|case| {
            nuif_core::validate(&case.document)
                .iter()
                .all(|diagnostic| diagnostic.severity != nuif_core::Severity::Error)
        }));
        assert_eq!(
            first.iter().map(|case| &case.name).collect::<Vec<_>>(),
            second.iter().map(|case| &case.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn taffy_matches_reference_for_a_fixed_flex_case() {
        let mut rng = DifferentialRng::new(1);
        let document = generated_case(&mut rng, 0, LayoutFamily::Flex, (640, 480));
        let context = EvaluationContext::viewport(640.0, 480.0);
        let reference = evaluate(&document, &context).boxes;
        let foreign = evaluate_taffy(&document, &context).unwrap();
        assert_eq!(
            reference.keys().collect::<Vec<_>>(),
            foreign.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn browser_output_parser_reads_entity_keyed_boxes() {
        let id = EntityId::new(1);
        let encoded = format!(
            "<html><body>{{\"{id}\":{{\"x\":1.0,\"y\":2.0,\"width\":3.0,\"height\":4.0}}}}</body></html>"
        );
        let boxes = parse_browser_output(&encoded).unwrap();
        assert!((boxes[&id].width - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn tolerance_is_fixture_local_and_safety_capped() {
        assert!((calibrated_tolerance(0.012_51) - 0.02).abs() < f64::EPSILON);
        assert!(calibrated_tolerance(0.0).abs() < f64::EPSILON);
        assert!((calibrated_tolerance(9.0) - MAX_FOREIGN_TOLERANCE_PX).abs() < f64::EPSILON);
    }

    #[test]
    fn browser_lock_matches_the_compiled_pin() {
        let lock: serde_json::Value = serde_json::from_str(include_str!(
            "../../../conformance/browser-oracle.lock.json"
        ))
        .unwrap();
        assert_eq!(lock["version"], PINNED_CHROME_VERSION);
    }
}
