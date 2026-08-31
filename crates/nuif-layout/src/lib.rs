#![doc = "Deterministic authored-to-resolved layout evaluation for NUIF."]

use nuif_core::{
    Align, AssetId, AssetKind, Diagnostic, Document, Entity, EntityId, EntityKind, Fidelity,
    FlowDirection, GridArea, GridTrack, LayoutFamily, Severity, SizeIntent, TextFontBinding,
    resolve_grid_placements, resolve_text_font_binding, validate,
};
use nuif_text::{
    PINNED_FONT_SHA256, ShapeRequest, ShapedRun, TextDirection, hard_lines, shape_hard_lines,
    shape_hard_lines_resource,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::convert::Infallible;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationContext {
    pub viewport: Size,
    pub scale_factor: f64,
    pub locale: String,
    pub writing_direction: WritingDirection,
    pub theme: Option<String>,
    #[serde(default)]
    pub font_hashes: BTreeSet<String>,
    /// Exact local font bytes keyed by SHA-256. Resource bytes are runtime
    /// inputs and never serialized as part of the evaluation context; the
    /// fingerprint already commits to the corresponding `font_hashes`.
    #[serde(skip)]
    pub font_resources: BTreeMap<String, Arc<[u8]>>,
    #[serde(default)]
    pub capabilities: BTreeSet<String>,
}

impl EvaluationContext {
    #[must_use]
    pub fn viewport(width: f64, height: f64) -> Self {
        Self {
            viewport: Size { width, height },
            scale_factor: 1.0,
            locale: "en".to_owned(),
            writing_direction: WritingDirection::LeftToRight,
            theme: None,
            font_hashes: BTreeSet::new(),
            font_resources: BTreeMap::new(),
            capabilities: BTreeSet::new(),
        }
    }

    #[must_use]
    pub fn fingerprint(&self) -> String {
        format!(
            "{}x{}@{}:{}:{:?}:{:?}:{:?}:{:?}",
            self.viewport.width,
            self.viewport.height,
            self.scale_factor,
            self.locale,
            self.writing_direction,
            self.theme,
            self.font_hashes,
            self.capabilities
        )
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WritingDirection {
    #[default]
    LeftToRight,
    RightToLeft,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutSnapshot {
    pub context_fingerprint: String,
    pub boxes: BTreeMap<EntityId, Rect>,
    pub diagnostics: Vec<Diagnostic>,
}

pub trait LayoutEvaluator {
    type Error;

    /// Evaluates authored layout for a fully specified evaluation context.
    ///
    /// # Errors
    ///
    /// Returns an implementation-defined error when layout cannot be
    /// evaluated because of invalid input, unsupported required semantics,
    /// resource limits, or evaluator failure.
    fn evaluate(
        &self,
        document: &Document,
        context: &EvaluationContext,
    ) -> Result<LayoutSnapshot, Self::Error>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ReferenceLayout;

impl LayoutEvaluator for ReferenceLayout {
    type Error = Infallible;

    fn evaluate(
        &self,
        document: &Document,
        context: &EvaluationContext,
    ) -> Result<LayoutSnapshot, Self::Error> {
        Ok(evaluate(document, context))
    }
}

#[must_use]
pub fn evaluate(document: &Document, context: &EvaluationContext) -> LayoutSnapshot {
    let mut snapshot = LayoutSnapshot {
        context_fingerprint: context.fingerprint(),
        boxes: BTreeMap::new(),
        diagnostics: Vec::new(),
    };
    let validation = validate(document);
    if validation
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        snapshot.diagnostics = validation;
        return snapshot;
    }
    let root_bounds = Rect {
        x: 0.0,
        y: 0.0,
        width: context.viewport.width.max(0.0),
        height: context.viewport.height.max(0.0),
    };
    for root in &document.roots {
        layout_entity(document, *root, root_bounds, context, &mut snapshot, true);
    }
    snapshot
}

fn layout_entity(
    document: &Document,
    id: EntityId,
    available: Rect,
    context: &EvaluationContext,
    snapshot: &mut LayoutSnapshot,
    is_root: bool,
) {
    let Some(entity) = document.entities.get(&id) else {
        return;
    };
    let resolved = resolved_authored(entity, context);
    let intrinsic = intrinsic_size(document, entity, context);
    let width = if is_root && matches!(resolved.width, SizeIntentRef::Auto | SizeIntentRef::Fill) {
        available.width
    } else {
        resolve_axis(&resolved.width, available.width, intrinsic.width)
    };
    let height = if is_root && matches!(resolved.height, SizeIntentRef::Auto | SizeIntentRef::Fill)
    {
        available.height
    } else {
        resolve_axis(&resolved.height, available.height, intrinsic.height)
    };
    let rect = Rect {
        x: if is_root {
            available.x
        } else {
            available.x + resolved.x
        },
        y: if is_root {
            available.y
        } else {
            available.y + resolved.y
        },
        width: width.max(0.0),
        height: height.max(0.0),
    };
    snapshot.boxes.insert(id, rect);
    record_entity_diagnostics(document, entity, context, snapshot);
    layout_children(document, entity, rect, context, snapshot, resolved);
}

fn record_entity_diagnostics(
    document: &Document,
    entity: &Entity,
    context: &EvaluationContext,
    snapshot: &mut LayoutSnapshot,
) {
    let id = entity.id;
    if let EntityKind::Text = entity.kind
        && let Some(text) = &entity.authored.text
    {
        let binding = resolve_text_font_binding(document, text);
        let (code, message, pointer, fidelity) = match binding {
            TextFontBinding::Substituted {
                asset,
                replacement_sha256,
                ..
            } if context.font_hashes.contains(replacement_sha256) => (
                "TEXT_FONT_SUBSTITUTED",
                format!(
                    "font {} uses declared substitute asset {asset} ({replacement_sha256})",
                    text.font
                ),
                format!("/entities/{id}/authored/text/font_asset"),
                Fidelity::Approximated {
                    reason: "layout metrics use the declared replacement font".to_owned(),
                },
            ),
            TextFontBinding::Substituted {
                asset,
                replacement_sha256,
                ..
            } => (
                "TEXT_FONT_SUBSTITUTE_NOT_PINNED",
                format!(
                    "declared substitute font asset {asset} ({replacement_sha256}) is absent from the evaluation context"
                ),
                format!("/entities/{id}/authored/text/font_asset"),
                Fidelity::Unsupported {
                    reason: "substitute font metrics are unavailable".to_owned(),
                },
            ),
            TextFontBinding::Unavailable { asset, .. } => (
                "TEXT_FONT_UNAVAILABLE",
                format!("font {} is unavailable through asset {asset}", text.font),
                format!("/entities/{id}/authored/text/font_asset"),
                Fidelity::Unsupported {
                    reason: "font resource is intentionally unavailable".to_owned(),
                },
            ),
            TextFontBinding::Unbound { requested_sha256 }
            | TextFontBinding::Exact {
                sha256: requested_sha256,
                ..
            } if !requested_sha256.is_empty()
                && !context.font_hashes.contains(requested_sha256) =>
            {
                (
                    "TEXT_FONT_NOT_PINNED",
                    format!("font {} is absent from the evaluation context", text.font),
                    format!("/entities/{id}/authored/text/font_sha256"),
                    Fidelity::Approximated {
                        reason: "font metrics use the deterministic fallback estimate".to_owned(),
                    },
                )
            }
            TextFontBinding::Invalid { .. }
            | TextFontBinding::Unbound { .. }
            | TextFontBinding::Exact { .. } => return,
        };
        snapshot.diagnostics.push(Diagnostic {
            code: code.to_owned(),
            severity: Severity::Warning,
            message,
            entity: Some(id),
            pointer: Some(pointer),
            fidelity: Some(fidelity),
        });
    }
    if let EntityKind::Unknown(unknown) = &entity.kind {
        let fallback = document
            .extension_declarations
            .fallback_kind
            .get(&unknown.namespace)
            .map_or("container", String::as_str);
        snapshot.diagnostics.push(Diagnostic {
            code: "LAYOUT_UNKNOWN_KIND_FALLBACK".to_owned(),
            severity: Severity::Information,
            message: format!(
                "unknown kind {}:{} uses declared {fallback} fallback layout",
                unknown.namespace, unknown.kind,
            ),
            entity: Some(id),
            pointer: Some(format!("/entities/{id}/kind")),
            fidelity: Some(Fidelity::PreservedUnrenderable {
                namespace: unknown.namespace.clone(),
            }),
        });
    }
}

#[derive(Clone, Copy)]
struct ResolvedAuthored {
    width: SizeIntentRef,
    height: SizeIntentRef,
    x: f64,
    y: f64,
    direction: FlowDirection,
    gap: f64,
}

#[derive(Clone, Copy)]
enum SizeIntentRef {
    Auto,
    Fixed(f64),
    Fill,
    Intrinsic,
    Percentage(f64),
    MinContent,
    MaxContent,
    FitContent(f64),
}

impl SizeIntentRef {
    fn from_intent(intent: &SizeIntent) -> Self {
        match intent {
            SizeIntent::Auto => Self::Auto,
            SizeIntent::Fixed(value) => Self::Fixed(*value),
            SizeIntent::Fill => Self::Fill,
            SizeIntent::Intrinsic => Self::Intrinsic,
            SizeIntent::Percentage(value) => Self::Percentage(*value),
            SizeIntent::MinContent => Self::MinContent,
            SizeIntent::MaxContent => Self::MaxContent,
            SizeIntent::FitContent(value) => Self::FitContent(*value),
        }
    }
}

fn resolved_authored(entity: &Entity, context: &EvaluationContext) -> ResolvedAuthored {
    let mut width = SizeIntentRef::from_intent(&entity.authored.width);
    let mut height = SizeIntentRef::from_intent(&entity.authored.height);
    let mut direction = entity.authored.layout.direction;
    let mut gap = entity.authored.layout.gap;
    for rule in &entity.authored.responsive {
        let width_matches = rule
            .when
            .min_width
            .is_none_or(|minimum| context.viewport.width >= minimum)
            && rule
                .when
                .max_width
                .is_none_or(|maximum| context.viewport.width <= maximum);
        let theme_matches = rule.when.theme.as_ref().is_none_or(|theme| {
            context
                .theme
                .as_ref()
                .is_some_and(|current| current == theme)
        });
        if width_matches && theme_matches {
            if let Some(value) = &rule.width {
                width = SizeIntentRef::from_intent(value);
            }
            if let Some(value) = &rule.height {
                height = SizeIntentRef::from_intent(value);
            }
            if let Some(value) = rule.direction {
                direction = value;
            }
            if let Some(value) = rule.gap {
                gap = value;
            }
        }
    }
    ResolvedAuthored {
        width,
        height,
        x: entity.authored.position.x,
        y: entity.authored.position.y,
        direction,
        gap,
    }
}

fn resolve_axis(intent: &SizeIntentRef, available: f64, intrinsic: f64) -> f64 {
    match intent {
        SizeIntentRef::Auto
        | SizeIntentRef::Intrinsic
        | SizeIntentRef::MinContent
        | SizeIntentRef::MaxContent => intrinsic,
        SizeIntentRef::Fixed(value) => *value,
        SizeIntentRef::Fill => available,
        SizeIntentRef::Percentage(percent) => available * percent / 100.0,
        SizeIntentRef::FitContent(limit) => intrinsic.min(*limit),
    }
}

fn intrinsic_size(document: &Document, entity: &Entity, context: &EvaluationContext) -> Size {
    if let Some(text) = &entity.authored.text {
        let line_count = hard_lines(&text.content).len();
        let direction = match context.writing_direction {
            WritingDirection::LeftToRight => TextDirection::LeftToRight,
            WritingDirection::RightToLeft => TextDirection::RightToLeft,
        };
        let shaped_width = shape_text(document, text, context, direction).map(|runs| {
            runs.into_iter().fold(0.0_f64, |maximum, run| {
                let advance = run
                    .glyphs
                    .iter()
                    .map(|glyph| f64::from(glyph.x_advance))
                    .sum::<f64>()
                    .abs()
                    * run.font_size
                    / f64::from(run.units_per_em);
                maximum.max(advance)
            })
        });
        let fallback_width = hard_lines(&text.content)
            .into_iter()
            .map(str::chars)
            .map(Iterator::count)
            .max()
            .map_or(f64::from(u32::MAX), |count| {
                f64::from(u32::try_from(count).unwrap_or(u32::MAX))
            })
            * text.size
            * 0.6;
        return Size {
            width: shaped_width.unwrap_or(fallback_width),
            height: f64::from(u32::try_from(line_count).unwrap_or(u32::MAX)) * text.line_height,
        };
    }
    Size::default()
}

fn shape_text(
    document: &Document,
    text: &nuif_core::TextContent,
    context: &EvaluationContext,
    direction: TextDirection,
) -> Option<Vec<ShapedRun>> {
    let binding = resolve_text_font_binding(document, text);
    let sha256 = binding.effective_sha256()?;
    if !context.font_hashes.contains(sha256) {
        return None;
    }
    let request = ShapeRequest {
        text: &text.content,
        font_sha256: sha256,
        font_size: text.size,
        direction,
        language: &context.locale,
    };
    if sha256 == PINNED_FONT_SHA256 {
        return shape_hard_lines(&request).ok();
    }
    let asset_id = match binding {
        TextFontBinding::Exact { asset, .. } | TextFontBinding::Substituted { asset, .. } => asset,
        TextFontBinding::Unbound { .. }
        | TextFontBinding::Unavailable { .. }
        | TextFontBinding::Invalid { .. } => return None,
    };
    let (family, license) = font_asset_metadata(document, asset_id)?;
    let bytes = context.font_resources.get(sha256)?;
    shape_hard_lines_resource(&request, bytes, family, license).ok()
}

fn font_asset_metadata(document: &Document, asset_id: AssetId) -> Option<(&str, &str)> {
    let asset = document.assets.get(&asset_id)?;
    let AssetKind::Font(font) = &asset.kind else {
        return None;
    };
    Some((
        font.names.first()?.as_str(),
        font.policy_evidence.get("license.expression")?.as_str(),
    ))
}

fn layout_children(
    document: &Document,
    entity: &Entity,
    rect: Rect,
    context: &EvaluationContext,
    snapshot: &mut LayoutSnapshot,
    resolved: ResolvedAuthored,
) {
    let padding = entity.authored.layout.padding;
    let content_bounds = Rect {
        x: rect.x + padding.left,
        y: rect.y + padding.top,
        width: (rect.width - padding.left - padding.right).max(0.0),
        height: (rect.height - padding.top - padding.bottom).max(0.0),
    };
    match entity.authored.layout.family {
        LayoutFamily::Freeform | LayoutFamily::Constraint => {
            if entity.authored.layout.family == LayoutFamily::Constraint {
                snapshot.diagnostics.push(Diagnostic {
                    code: "LAYOUT_CONSTRAINT_FALLBACK".to_owned(),
                    severity: Severity::Warning,
                    message: "constraint layout uses the freeform fallback in profile 0".to_owned(),
                    entity: Some(entity.id),
                    pointer: Some(format!("/entities/{}/authored/layout/family", entity.id)),
                    fidelity: Some(Fidelity::Approximated {
                        reason: "constraint solver is not part of profile 0".to_owned(),
                    }),
                });
            }
            for child in &entity.children {
                layout_entity(document, *child, content_bounds, context, snapshot, false);
            }
        }
        LayoutFamily::Stack | LayoutFamily::Flex => {
            if entity.authored.layout.family == LayoutFamily::Flex {
                snapshot.diagnostics.push(Diagnostic {
                    code: "LAYOUT_FAMILY_PROFILE0_FALLBACK".to_owned(),
                    severity: Severity::Warning,
                    message: format!(
                        "{:?} layout uses stack flow in profile 0",
                        entity.authored.layout.family
                    ),
                    entity: Some(entity.id),
                    pointer: Some(format!("/entities/{}/authored/layout/family", entity.id)),
                    fidelity: Some(Fidelity::Approximated {
                        reason: "the profile-0 flex evaluator currently uses stack flow".to_owned(),
                    }),
                });
            }
            layout_flow(
                document,
                entity,
                content_bounds,
                context,
                snapshot,
                resolved,
            );
        }
        LayoutFamily::Grid => {
            layout_grid(document, entity, content_bounds, context, snapshot);
        }
    }
}

fn layout_grid(
    document: &Document,
    entity: &Entity,
    bounds: Rect,
    context: &EvaluationContext,
    snapshot: &mut LayoutSnapshot,
) {
    let Ok(areas) = resolve_grid_placements(document, entity) else {
        return;
    };
    let gap = entity.authored.layout.gap.max(0.0);
    let columns = resolve_grid_tracks(&entity.authored.layout.grid.columns, bounds.width, gap);
    let rows = resolve_grid_tracks(&entity.authored.layout.grid.rows, bounds.height, gap);
    for child_id in &entity.children {
        let (Some(child), Some(area)) = (document.entities.get(child_id), areas.get(child_id))
        else {
            continue;
        };
        let area_bounds = grid_area_bounds(bounds, &columns, &rows, gap, *area);
        let resolved = resolved_authored(child, context);
        let intrinsic = intrinsic_size(document, child, context);
        let (x_offset, width) = resolve_grid_item_axis(
            &resolved.width,
            area_bounds.width,
            intrinsic.width,
            entity.authored.layout.align,
        );
        let (y_offset, height) = resolve_grid_item_axis(
            &resolved.height,
            area_bounds.height,
            intrinsic.height,
            entity.authored.layout.align,
        );
        layout_entity_in_flow(
            document,
            *child_id,
            Rect {
                x: area_bounds.x + x_offset,
                y: area_bounds.y + y_offset,
                width,
                height,
            },
            context,
            snapshot,
        );
    }
}

#[derive(Clone, Copy)]
struct GridTrackGeometry {
    offset: f64,
    size: f64,
}

fn resolve_grid_tracks(tracks: &[GridTrack], available: f64, gap: f64) -> Vec<GridTrackGeometry> {
    let gap_count = f64::from(u32::try_from(tracks.len().saturating_sub(1)).unwrap_or(u32::MAX));
    let fixed = tracks
        .iter()
        .map(|track| match track {
            GridTrack::Fixed(value) => *value,
            GridTrack::Fraction(_) => 0.0,
        })
        .sum::<f64>();
    let weight = tracks
        .iter()
        .map(|track| match track {
            GridTrack::Fixed(_) => 0.0,
            GridTrack::Fraction(value) => *value,
        })
        .sum::<f64>();
    let fraction = (available - fixed - gap * gap_count).max(0.0) / weight.max(1.0);
    let mut offset = 0.0;
    tracks
        .iter()
        .map(|track| {
            let size = match track {
                GridTrack::Fixed(value) => *value,
                GridTrack::Fraction(value) => value * fraction,
            };
            let geometry = GridTrackGeometry { offset, size };
            offset += size + gap;
            geometry
        })
        .collect()
}

fn grid_area_bounds(
    bounds: Rect,
    columns: &[GridTrackGeometry],
    rows: &[GridTrackGeometry],
    gap: f64,
    area: GridArea,
) -> Rect {
    let column = usize::try_from(area.column).unwrap_or(0);
    let row = usize::try_from(area.row).unwrap_or(0);
    let column_span = usize::try_from(area.column_span).unwrap_or(1);
    let row_span = usize::try_from(area.row_span).unwrap_or(1);
    let column_end = column + column_span;
    let row_end = row + row_span;
    Rect {
        x: bounds.x + columns[column].offset,
        y: bounds.y + rows[row].offset,
        width: columns[column..column_end]
            .iter()
            .map(|track| track.size)
            .sum::<f64>()
            + gap * f64::from(u32::try_from(column_span.saturating_sub(1)).unwrap_or(u32::MAX)),
        height: rows[row..row_end]
            .iter()
            .map(|track| track.size)
            .sum::<f64>()
            + gap * f64::from(u32::try_from(row_span.saturating_sub(1)).unwrap_or(u32::MAX)),
    }
}

fn resolve_grid_item_axis(
    intent: &SizeIntentRef,
    available: f64,
    intrinsic: f64,
    align: Align,
) -> (f64, f64) {
    let size = if matches!(intent, SizeIntentRef::Fill)
        || (align == Align::Stretch && matches!(intent, SizeIntentRef::Auto))
    {
        available
    } else {
        resolve_axis(intent, available, intrinsic)
    }
    .max(0.0);
    let offset = match align {
        Align::Start | Align::Stretch => 0.0,
        Align::Center => (available - size) / 2.0,
        Align::End => available - size,
    };
    (offset, size)
}

#[expect(
    clippy::too_many_lines,
    reason = "the profile-0 flow algorithm is kept contiguous for spec comparison"
)]
fn layout_flow(
    document: &Document,
    entity: &Entity,
    bounds: Rect,
    context: &EvaluationContext,
    snapshot: &mut LayoutSnapshot,
    resolved: ResolvedAuthored,
) {
    let is_row = resolved.direction == FlowDirection::Row;
    let available_main = if is_row { bounds.width } else { bounds.height };
    let gap_count =
        f64::from(u32::try_from(entity.children.len().saturating_sub(1)).unwrap_or(u32::MAX));
    let gap_total = resolved.gap.max(0.0) * gap_count;
    let mut fixed_main = 0.0;
    let mut fill_count = 0_u32;
    for child in &entity.children {
        let Some(item) = document.entities.get(child) else {
            continue;
        };
        let child_resolved = resolved_authored(item, context);
        let intent = if is_row {
            child_resolved.width
        } else {
            child_resolved.height
        };
        if matches!(intent, SizeIntentRef::Fill) {
            fill_count += 1;
        } else {
            let intrinsic = intrinsic_size(document, item, context);
            fixed_main += resolve_axis(
                &intent,
                available_main,
                if is_row {
                    intrinsic.width
                } else {
                    intrinsic.height
                },
            );
        }
    }
    let fill_main = if fill_count == 0 {
        0.0
    } else {
        ((available_main - fixed_main - gap_total).max(0.0)) / f64::from(fill_count)
    };
    let mut cursor = 0.0;
    for child in &entity.children {
        let Some(item) = document.entities.get(child) else {
            continue;
        };
        let child_resolved = resolved_authored(item, context);
        let intrinsic = intrinsic_size(document, item, context);
        let main_intent = if is_row {
            child_resolved.width
        } else {
            child_resolved.height
        };
        let main = if matches!(main_intent, SizeIntentRef::Fill) {
            fill_main
        } else {
            resolve_axis(
                &main_intent,
                available_main,
                if is_row {
                    intrinsic.width
                } else {
                    intrinsic.height
                },
            )
        };
        let cross_intent = if is_row {
            child_resolved.height
        } else {
            child_resolved.width
        };
        let available_cross = if is_row { bounds.height } else { bounds.width };
        let cross = if matches!(cross_intent, SizeIntentRef::Fill)
            || (matches!(entity.authored.layout.align, Align::Stretch)
                && matches!(cross_intent, SizeIntentRef::Auto))
        {
            available_cross
        } else {
            resolve_axis(
                &cross_intent,
                available_cross,
                if is_row {
                    intrinsic.height
                } else {
                    intrinsic.width
                },
            )
        };
        let cross_offset = match entity.authored.layout.align {
            Align::Start | Align::Stretch => 0.0,
            Align::Center => (available_cross - cross) / 2.0,
            Align::End => available_cross - cross,
        }
        .max(0.0);
        let child_available = if is_row {
            Rect {
                x: bounds.x + cursor,
                y: bounds.y + cross_offset,
                width: main,
                height: cross,
            }
        } else {
            Rect {
                x: bounds.x + cross_offset,
                y: bounds.y + cursor,
                width: cross,
                height: main,
            }
        };
        layout_entity_in_flow(document, *child, child_available, context, snapshot);
        cursor += main + resolved.gap.max(0.0);
    }
}

fn layout_entity_in_flow(
    document: &Document,
    id: EntityId,
    rect: Rect,
    context: &EvaluationContext,
    snapshot: &mut LayoutSnapshot,
) {
    let Some(entity) = document.entities.get(&id) else {
        return;
    };
    snapshot.boxes.insert(id, rect);
    record_entity_diagnostics(document, entity, context, snapshot);
    let resolved = resolved_authored(entity, context);
    layout_children(document, entity, rect, context, snapshot, resolved);
}

#[cfg(test)]
mod tests {
    use super::*;
    use nuif_core::{
        Edges, Entity, GridStyle, LayoutStyle, OpaqueEncoding, OpaquePayload, TextContent,
        UnknownKind,
    };
    use nuif_text::{PINNED_FONT_NAME, PINNED_FONT_SHA256};

    #[test]
    fn fingerprint_includes_content_addressed_font_set() {
        let first = EvaluationContext::viewport(100.0, 100.0);
        let mut second = first.clone();
        second.font_hashes.insert("a".repeat(64));
        assert_ne!(first.fingerprint(), second.fingerprint());
    }

    #[test]
    fn pinned_text_intrinsic_size_uses_shaped_hard_lines() {
        let mut text = Entity::new(EntityId::new(2), EntityKind::Text);
        text.authored.text = Some(TextContent {
            content: "A B\nAB".to_owned(),
            font: PINNED_FONT_NAME.to_owned(),
            font_sha256: PINNED_FONT_SHA256.to_owned(),
            font_asset: None,
            size: 18.0,
            line_height: 24.0,
        });
        let mut context = EvaluationContext::viewport(100.0, 100.0);
        context.font_hashes.insert(PINNED_FONT_SHA256.to_owned());
        let document = Document::empty(EntityId::new(1));
        assert_eq!(
            intrinsic_size(&document, &text, &context),
            Size {
                width: 54.0,
                height: 48.0,
            }
        );
    }

    #[test]
    fn responsive_stack_changes_direction() {
        let mut document = Document::empty(EntityId::new(1));
        let mut root = Entity::new(EntityId::new(2), EntityKind::Container);
        root.authored.width = SizeIntent::Fill;
        root.authored.height = SizeIntent::Fixed(100.0);
        root.authored.layout = LayoutStyle {
            family: LayoutFamily::Stack,
            direction: FlowDirection::Column,
            gap: 10.0,
            ..LayoutStyle::default()
        };
        root.authored
            .responsive
            .push(nuif_core::ResponsiveOverride {
                when: nuif_core::ContextPredicate {
                    min_width: Some(700.0),
                    max_width: None,
                    theme: None,
                },
                direction: Some(FlowDirection::Row),
                gap: None,
                width: None,
                height: None,
            });
        for id in [3, 4] {
            let mut child = Entity::new(EntityId::new(id), EntityKind::Container);
            child.authored.width = SizeIntent::Fill;
            child.authored.height = SizeIntent::Fill;
            root.children.push(child.id);
            document.entities.insert(child.id, child);
        }
        document.roots.push(root.id);
        document.entities.insert(root.id, root);

        let narrow = evaluate(&document, &EvaluationContext::viewport(360.0, 100.0));
        let wide = evaluate(&document, &EvaluationContext::viewport(768.0, 100.0));
        assert!(narrow.boxes[&EntityId::new(4)].y > narrow.boxes[&EntityId::new(3)].y);
        assert!(wide.boxes[&EntityId::new(4)].x > wide.boxes[&EntityId::new(3)].x);
    }

    #[test]
    fn stretch_preserves_a_definite_cross_size() {
        let mut document = Document::empty(EntityId::new(1));
        let mut root = Entity::new(EntityId::new(2), EntityKind::Container);
        root.authored.width = SizeIntent::Fill;
        root.authored.height = SizeIntent::Fixed(100.0);
        root.authored.layout = LayoutStyle {
            family: LayoutFamily::Stack,
            direction: FlowDirection::Column,
            align: Align::Stretch,
            ..LayoutStyle::default()
        };
        let mut child = Entity::new(EntityId::new(3), EntityKind::Container);
        child.authored.width = SizeIntent::Fixed(40.0);
        child.authored.height = SizeIntent::Fixed(20.0);
        root.children.push(child.id);
        document.roots.push(root.id);
        document.entities.insert(root.id, root);
        document.entities.insert(child.id, child);

        let snapshot = evaluate(&document, &EvaluationContext::viewport(200.0, 100.0));
        assert!((snapshot.boxes[&EntityId::new(3)].width - 40.0).abs() < f64::EPSILON);
    }

    #[test]
    fn explicit_grid_resolves_tracks_spans_and_two_axis_alignment() {
        let mut document = Document::empty(EntityId::new(1));
        let mut root = Entity::new(EntityId::new(2), EntityKind::Container);
        root.authored.width = SizeIntent::Fill;
        root.authored.height = SizeIntent::Fill;
        root.authored.layout = LayoutStyle {
            family: LayoutFamily::Grid,
            gap: 10.0,
            padding: Edges {
                top: 10.0,
                right: 10.0,
                bottom: 10.0,
                left: 10.0,
            },
            align: Align::Center,
            grid: GridStyle {
                columns: vec![GridTrack::Fixed(50.0), GridTrack::Fraction(1.0)],
                rows: vec![GridTrack::Fraction(1.0), GridTrack::Fixed(40.0)],
                ..GridStyle::default()
            },
            ..LayoutStyle::default()
        };
        let mut first = Entity::new(EntityId::new(3), EntityKind::Container);
        first.authored.width = SizeIntent::Fixed(20.0);
        first.authored.height = SizeIntent::Fixed(30.0);
        let mut second = Entity::new(EntityId::new(4), EntityKind::Container);
        second.authored.width = SizeIntent::Fill;
        second.authored.height = SizeIntent::Fill;
        let mut spanning = Entity::new(EntityId::new(5), EntityKind::Container);
        spanning.authored.width = SizeIntent::Fixed(100.0);
        spanning.authored.height = SizeIntent::Fixed(20.0);
        spanning.authored.grid_placement.column_span = 2;
        root.children = vec![first.id, second.id, spanning.id];
        document.roots.push(root.id);
        document.entities.insert(root.id, root);
        document.entities.insert(first.id, first);
        document.entities.insert(second.id, second);
        document.entities.insert(spanning.id, spanning);

        let snapshot = evaluate(&document, &EvaluationContext::viewport(220.0, 200.0));
        assert_eq!(
            snapshot.boxes[&EntityId::new(3)],
            Rect {
                x: 25.0,
                y: 60.0,
                width: 20.0,
                height: 30.0,
            }
        );
        assert_eq!(
            snapshot.boxes[&EntityId::new(4)],
            Rect {
                x: 70.0,
                y: 10.0,
                width: 140.0,
                height: 130.0,
            }
        );
        assert_eq!(
            snapshot.boxes[&EntityId::new(5)],
            Rect {
                x: 60.0,
                y: 160.0,
                width: 100.0,
                height: 20.0,
            }
        );
        assert!(
            snapshot
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code != "LAYOUT_FAMILY_PROFILE0_FALLBACK")
        );
    }

    #[test]
    fn fractional_weight_below_one_leaves_trailing_space() {
        let tracks = resolve_grid_tracks(&[GridTrack::Fraction(0.25)], 100.0, 0.0);
        assert_eq!(tracks.len(), 1);
        assert!((tracks[0].size - 25.0).abs() < f64::EPSILON);
    }

    #[test]
    fn flow_children_emit_text_and_unknown_fidelity_diagnostics() {
        let mut document = Document::empty(EntityId::new(1));
        let mut root = Entity::new(EntityId::new(2), EntityKind::Container);
        root.authored.layout.family = LayoutFamily::Stack;
        root.authored.layout.direction = FlowDirection::Column;
        let mut text = Entity::new(EntityId::new(3), EntityKind::Text);
        text.authored.text = Some(TextContent {
            content: "probe".to_owned(),
            font: "missing".to_owned(),
            font_sha256: "0".repeat(64),
            font_asset: None,
            size: 12.0,
            line_height: 16.0,
        });
        let unknown = Entity::new(
            EntityId::new(4),
            EntityKind::Unknown(UnknownKind {
                namespace: "vendor.probe".to_owned(),
                kind: "future_widget".to_owned(),
                schema_version: 1,
                payload: OpaquePayload {
                    encoding: OpaqueEncoding::Octets,
                    bytes: Vec::new(),
                },
            }),
        );
        root.children = vec![text.id, unknown.id];
        document.roots.push(root.id);
        document.entities.insert(root.id, root);
        document.entities.insert(text.id, text);
        document.entities.insert(unknown.id, unknown);
        document
            .extension_declarations
            .fallback_kind
            .insert("vendor.probe".to_owned(), "container".to_owned());
        document
            .extension_declarations
            .used
            .insert("vendor.probe".to_owned());

        let snapshot = evaluate(&document, &EvaluationContext::viewport(100.0, 100.0));
        let codes = snapshot
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<BTreeSet<_>>();
        assert!(codes.contains("TEXT_FONT_NOT_PINNED"));
        assert!(codes.contains("LAYOUT_UNKNOWN_KIND_FALLBACK"));
    }
}
