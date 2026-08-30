//! Deterministic whole-editor semantic and visual trial runner.

use super::widgets::AuthorAction;
use super::{Driver, ExternalFormat, UiAction};
use crate::EditorEvent;
use masonry::accesskit::{Action, ActionData, ActionRequest, TreeId};
use masonry::core::{Widget, WidgetId};
use masonry::theme::default_property_set;
use masonry::widgets::{ButtonPress, SizedBox};
use masonry_testing::{TestHarness, TestHarnessParams};
use masonry_winit::app::WindowId;
use nuif_codec::{
    CanonicalText, Decoder, Encoder, MAX_INPUT_BYTES, canonical_hash,
    read_bounded as read_bounded_stream,
};
use nuif_core::{Document, EntityId};
use nuif_protocol::apply_patch;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const USAGE: &str =
    "usage: nuif-editor-automation --document <nuif> --scenario <json> --artifact-dir <dir>";
const MAX_SCENARIO_BYTES: usize = 2 * 1024 * 1024;
const MAX_ACTIONS: usize = 10_000;
const MIN_WINDOW_WIDTH: u32 = 900;
const MIN_WINDOW_HEIGHT: u32 = 600;
const MAX_WINDOW_EDGE: u32 = 4096;

#[derive(Debug)]
struct Options {
    document: PathBuf,
    scenario: PathBuf,
    artifact_directory: PathBuf,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WindowSize {
    width: u32,
    height: u32,
}

impl Default for WindowSize {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 800,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    schema_version: u32,
    #[serde(default)]
    window: WindowSize,
    actions: Vec<TrialAction>,
    #[serde(default)]
    assertions: Assertions,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "action")]
enum TrialAction {
    Select {
        author_id: EntityId,
    },
    SetValue {
        author_id: EntityId,
        label: String,
        value: String,
    },
    Undo,
    Redo,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Assertions {
    selection: Option<Vec<EntityId>>,
    entity_count: Option<usize>,
    operation_count: Option<usize>,
    #[serde(default)]
    values: Vec<ValueAssertion>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ValueAssertion {
    author_id: EntityId,
    label: String,
    value: String,
}

#[derive(Debug, Serialize)]
struct SemanticNode {
    author_id: EntityId,
    label: String,
    role: String,
    value: Option<String>,
    widget_id: u64,
}

#[derive(Debug, Serialize)]
struct ArtifactPaths {
    screenshot: String,
    file_menu_screenshot: String,
    document: String,
    semantics: String,
}

#[derive(Debug, Serialize)]
struct Report {
    schema_version: u32,
    status: &'static str,
    platform: &'static str,
    architecture: &'static str,
    window: [u32; 2],
    canonical_hash: String,
    replay_hash: String,
    shell_rgba_sha256: String,
    file_menu_rgba_sha256: String,
    document_rgba_sha256: String,
    selection: Vec<EntityId>,
    entities: usize,
    operations: usize,
    semantic_nodes: usize,
    file_menu_routes: Vec<String>,
    actions: Vec<TrialAction>,
    artifacts: ArtifactPaths,
}

/// Runs a deterministic native-editor trial without opening a platform window.
///
/// # Errors
///
/// Returns an error for invalid or excessive input, missing semantic widgets,
/// failed accessibility dispatch, failed assertions, or artifact I/O failures.
pub fn run() -> Result<(), String> {
    let options = parse_options()?;
    let scenario = load_scenario(&options.scenario)?;
    validate_scenario(&scenario)?;
    let document = load_document(&options.document)?;
    let replay_base = document.clone();
    let window_id = WindowId::next();
    let mut driver = Driver::new(window_id, document, Some(options.document.clone()));

    for action in &scenario.actions {
        execute_action(&mut driver, scenario.window, action)?;
    }
    verify_assertions(&driver, &scenario.assertions)?;

    let mut replayed = replay_base;
    for patch in driver.editor.operation_log() {
        apply_patch(&mut replayed, patch).map_err(|error| error.to_string())?;
    }
    let observed_hash =
        canonical_hash(driver.editor.document()).map_err(|error| error.to_string())?;
    let replay_hash = canonical_hash(&replayed).map_err(|error| error.to_string())?;
    if observed_hash != replay_hash {
        return Err(format!(
            "native editor/replay mismatch: editor={observed_hash}, replay={replay_hash}"
        ));
    }

    let mut harness = build_harness(&mut driver, scenario.window);
    let screenshot = harness.render();
    let semantics = collect_semantics(&harness, &driver)?;
    let mut menu_toggle_harness = build_harness(&mut driver, scenario.window);
    let _ = menu_toggle_harness.redraw();
    press_command(
        &mut driver,
        &mut menu_toggle_harness,
        UiAction::ToggleFileMenu,
    )?;
    let mut file_menu_harness = build_harness(&mut driver, scenario.window);
    let file_menu_routes = verify_file_menu_routes(&driver)?;
    let file_menu_screenshot = file_menu_harness.render();
    let event = driver
        .editor
        .execute(crate::EditorCommand::Snapshot {
            width: super::VIEWPORT_WIDTH,
            height: super::VIEWPORT_HEIGHT,
        })
        .map_err(|error| error.to_string())?;
    let EditorEvent::Snapshot { snapshot } = event else {
        return Err("snapshot command returned an unexpected event".to_owned());
    };

    fs::create_dir_all(&options.artifact_directory).map_err(|error| error.to_string())?;
    let screenshot_path = options.artifact_directory.join("editor-shell.png");
    let file_menu_screenshot_path = options.artifact_directory.join("editor-file-menu.png");
    let document_path = options.artifact_directory.join("output.nuif");
    let semantics_path = options.artifact_directory.join("semantics.json");
    let report_path = options.artifact_directory.join("report.json");
    screenshot
        .save(&screenshot_path)
        .map_err(|error| error.to_string())?;
    file_menu_screenshot
        .save(&file_menu_screenshot_path)
        .map_err(|error| error.to_string())?;
    let canonical_document = CanonicalText
        .encode(driver.editor.document())
        .map_err(|error| error.to_string())?;
    fs::write(&document_path, canonical_document).map_err(|error| error.to_string())?;
    write_json(&semantics_path, &semantics)?;

    let report = Report {
        schema_version: 1,
        status: "passed",
        platform: env::consts::OS,
        architecture: env::consts::ARCH,
        window: [scenario.window.width, scenario.window.height],
        canonical_hash: observed_hash,
        replay_hash,
        shell_rgba_sha256: format!("{:x}", Sha256::digest(screenshot.as_raw())),
        file_menu_rgba_sha256: format!("{:x}", Sha256::digest(file_menu_screenshot.as_raw())),
        document_rgba_sha256: snapshot.raster.rgba_sha256.clone(),
        selection: driver.editor.selection().to_vec(),
        entities: driver.editor.document().entities.len(),
        operations: driver.editor.operation_log().len(),
        semantic_nodes: semantics.len(),
        file_menu_routes,
        actions: scenario.actions,
        artifacts: ArtifactPaths {
            screenshot: screenshot_path.display().to_string(),
            file_menu_screenshot: file_menu_screenshot_path.display().to_string(),
            document: document_path.display().to_string(),
            semantics: semantics_path.display().to_string(),
        },
    };
    write_json(&report_path, &report)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn parse_options() -> Result<Options, String> {
    let mut args = env::args().skip(1);
    let mut document = None;
    let mut scenario = None;
    let mut artifact_directory = None;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--document" => document = Some(required_argument(&mut args, "--document")?.into()),
            "--scenario" => scenario = Some(required_argument(&mut args, "--scenario")?.into()),
            "--artifact-dir" => {
                artifact_directory = Some(required_argument(&mut args, "--artifact-dir")?.into());
            }
            "--help" | "-h" => return Err(USAGE.to_owned()),
            unknown => return Err(format!("unknown argument {unknown:?}; {USAGE}")),
        }
    }
    Ok(Options {
        document: document.ok_or_else(|| format!("--document is required; {USAGE}"))?,
        scenario: scenario.ok_or_else(|| format!("--scenario is required; {USAGE}"))?,
        artifact_directory: artifact_directory
            .ok_or_else(|| format!("--artifact-dir is required; {USAGE}"))?,
    })
}

fn required_argument(
    args: &mut impl Iterator<Item = String>,
    name: &str,
) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{name} requires a value; {USAGE}"))
}

fn load_document(path: &Path) -> Result<Document, String> {
    let mut file = fs::File::open(path).map_err(|error| error.to_string())?;
    let bytes =
        read_bounded_stream(&mut file, MAX_INPUT_BYTES).map_err(|error| error.to_string())?;
    CanonicalText
        .decode(&bytes)
        .map_err(|error| error.to_string())
}

fn load_scenario(path: &Path) -> Result<Scenario, String> {
    let mut file = fs::File::open(path).map_err(|error| error.to_string())?;
    let bytes =
        read_bounded_stream(&mut file, MAX_SCENARIO_BYTES).map_err(|error| error.to_string())?;
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}

fn validate_scenario(scenario: &Scenario) -> Result<(), String> {
    if scenario.schema_version != 1 {
        return Err(format!(
            "unsupported scenario schema version {}",
            scenario.schema_version
        ));
    }
    if scenario.actions.len() > MAX_ACTIONS {
        return Err(format!(
            "scenario contains {} actions; limit is {MAX_ACTIONS}",
            scenario.actions.len()
        ));
    }
    if !(MIN_WINDOW_WIDTH..=MAX_WINDOW_EDGE).contains(&scenario.window.width)
        || !(MIN_WINDOW_HEIGHT..=MAX_WINDOW_EDGE).contains(&scenario.window.height)
    {
        return Err(format!(
            "window must be between {MIN_WINDOW_WIDTH}×{MIN_WINDOW_HEIGHT} and {MAX_WINDOW_EDGE}×{MAX_WINDOW_EDGE}"
        ));
    }
    Ok(())
}

fn build_harness(driver: &mut Driver, size: WindowSize) -> TestHarness<SizedBox> {
    let root = SizedBox::new(driver.build_view()).prepare();
    driver.root_widget_id = Some(root.id());
    TestHarness::create_with(
        default_property_set(),
        root,
        TestHarnessParams::default().with_size((size.width, size.height)),
    )
}

fn execute_action(
    driver: &mut Driver,
    window: WindowSize,
    action: &TrialAction,
) -> Result<(), String> {
    let mut harness = build_harness(driver, window);
    let _ = harness.redraw();
    match action {
        TrialAction::Select { author_id } => {
            let widget_id = driver
                .entity_widgets
                .get(author_id)
                .copied()
                .ok_or_else(|| format!("no layer widget has author_id {author_id}"))?;
            process_click(&mut harness, widget_id);
            let (emitted, source) = harness.pop_action::<AuthorAction>().ok_or_else(|| {
                format!("layer widget {author_id} emitted no accessibility action")
            })?;
            require_source(widget_id, source)?;
            driver.handle_author_action(emitted);
        }
        TrialAction::SetValue {
            author_id,
            label,
            value,
        } => {
            let semantic_label = match label.as_str() {
                "name" | "width" | "height" | "x" | "y" | "gap" | "padding_top"
                | "padding_right" | "padding_bottom" | "padding_left" | "fill" | "text"
                | "font_size" | "line_height" => label.as_str(),
                _ => return Err(format!("unsupported native control label {label:?}")),
            };
            let widget_id = driver
                .control_widgets
                .get(&(*author_id, semantic_label))
                .copied()
                .ok_or_else(|| {
                    format!(
                        "no visible {semantic_label:?} control has author_id {author_id}; select the entity first"
                    )
                })?;
            harness.process_access_event(ActionRequest {
                action: Action::SetValue,
                target_tree: TreeId::ROOT,
                target_node: widget_id.to_raw().into(),
                data: Some(ActionData::Value(value.clone().into())),
            });
            let (emitted, source) = harness.pop_action::<AuthorAction>().ok_or_else(|| {
                format!("control {semantic_label:?} emitted no accessibility action")
            })?;
            require_source(widget_id, source)?;
            driver.handle_author_action(emitted);
        }
        TrialAction::Undo => press_command(driver, &mut harness, UiAction::Undo)?,
        TrialAction::Redo => press_command(driver, &mut harness, UiAction::Redo)?,
    }
    if harness.pop_action_erased().is_some() {
        return Err("native widget emitted an unexpected extra action".to_owned());
    }
    Ok(())
}

fn press_command(
    driver: &mut Driver,
    harness: &mut TestHarness<SizedBox>,
    command: UiAction,
) -> Result<(), String> {
    let widget_id = driver
        .actions
        .iter()
        .find_map(|(widget, action)| (*action == command).then_some(*widget))
        .ok_or_else(|| format!("native command button {command:?} is absent"))?;
    process_click(harness, widget_id);
    let (_, source) = harness
        .pop_action::<ButtonPress>()
        .ok_or_else(|| format!("native command button {command:?} emitted no action"))?;
    require_source(widget_id, source)?;
    if !driver.handle_ui_action(command) {
        return Err(format!("native command {command:?} declined execution"));
    }
    Ok(())
}

fn require_source(expected: WidgetId, observed: WidgetId) -> Result<(), String> {
    if expected == observed {
        Ok(())
    } else {
        Err(format!(
            "accessibility action source mismatch: expected {}, observed {}",
            expected.to_raw(),
            observed.to_raw()
        ))
    }
}

fn process_click(harness: &mut TestHarness<SizedBox>, widget_id: WidgetId) {
    harness.process_access_event(ActionRequest {
        action: Action::Click,
        target_tree: TreeId::ROOT,
        target_node: widget_id.to_raw().into(),
        data: None,
    });
}

fn verify_file_menu_routes(driver: &Driver) -> Result<Vec<String>, String> {
    let mut routes = Vec::new();
    for (name, action) in [
        ("new", UiAction::New),
        ("import_nuif", UiAction::ImportNative),
        ("save", UiAction::Save),
        ("save_as", UiAction::SaveAs),
        ("export_png", UiAction::ExportSnapshot),
    ] {
        require_visible_action(driver, action, name)?;
        routes.push(name.to_owned());
    }
    for (format, profile) in [
        (ExternalFormat::Svg, "svg"),
        (ExternalFormat::HtmlCss, "html_css"),
        (ExternalFormat::Dtcg, "dtcg"),
        (ExternalFormat::Penpot, "penpot"),
    ] {
        let import = format!("import_{profile}");
        require_visible_action(driver, UiAction::ImportExternal(format), &import)?;
        routes.push(import);
        let export = format!("export_{profile}");
        require_visible_action(driver, UiAction::ExportExternal(format), &export)?;
        routes.push(export);
    }
    Ok(routes)
}

fn require_visible_action(driver: &Driver, action: UiAction, name: &str) -> Result<(), String> {
    if driver.actions.values().any(|observed| *observed == action) {
        Ok(())
    } else {
        Err(format!("file menu route {name:?} is absent"))
    }
}

fn verify_assertions(driver: &Driver, assertions: &Assertions) -> Result<(), String> {
    if let Some(expected) = &assertions.selection
        && driver.editor.selection() != expected
    {
        return Err(format!(
            "selection mismatch: expected {expected:?}, observed {:?}",
            driver.editor.selection()
        ));
    }
    if let Some(expected) = assertions.entity_count
        && driver.editor.document().entities.len() != expected
    {
        return Err(format!(
            "entity-count mismatch: expected {expected}, observed {}",
            driver.editor.document().entities.len()
        ));
    }
    if let Some(expected) = assertions.operation_count
        && driver.editor.operation_log().len() != expected
    {
        return Err(format!(
            "operation-count mismatch: expected {expected}, observed {}",
            driver.editor.operation_log().len()
        ));
    }
    let tree = driver.editor.accessibility_tree();
    for expected in &assertions.values {
        let observed = tree
            .iter()
            .find(|node| node.author_id == Some(expected.author_id) && node.label == expected.label)
            .and_then(|node| node.value.as_deref());
        if observed != Some(expected.value.as_str()) {
            return Err(format!(
                "value mismatch for {} {}: expected {:?}, observed {observed:?}",
                expected.author_id, expected.label, expected.value
            ));
        }
    }
    Ok(())
}

fn collect_semantics(
    harness: &TestHarness<SizedBox>,
    driver: &Driver,
) -> Result<Vec<SemanticNode>, String> {
    let mut nodes = Vec::new();
    for (author_id, widget_id) in &driver.entity_widgets {
        let node = harness
            .access_node(*widget_id)
            .ok_or_else(|| format!("accessibility node for entity {author_id} is absent"))?;
        nodes.push(SemanticNode {
            author_id: *author_id,
            label: node.label().unwrap_or_default().clone(),
            role: format!("{:?}", node.role()),
            value: node.value(),
            widget_id: widget_id.to_raw(),
        });
    }
    for ((author_id, semantic_label), widget_id) in &driver.control_widgets {
        let node = harness.access_node(*widget_id).ok_or_else(|| {
            format!("accessibility node for {author_id} {semantic_label} is absent")
        })?;
        nodes.push(SemanticNode {
            author_id: *author_id,
            label: node.label().unwrap_or_else(|| (*semantic_label).to_owned()),
            role: format!("{:?}", node.role()),
            value: node.value(),
            widget_id: widget_id.to_raw(),
        });
    }
    nodes.sort_by(|left, right| {
        (left.author_id, &left.label, left.widget_id).cmp(&(
            right.author_id,
            &right.label,
            right.widget_id,
        ))
    });
    Ok(nodes)
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    fs::write(path, bytes).map_err(|error| error.to_string())
}
