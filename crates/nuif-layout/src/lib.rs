#![doc = "Deterministic authored-to-resolved layout evaluation for NUIF."]

use nuif_core::{
    Align, Diagnostic, Document, Entity, EntityId, EntityKind, Fidelity, FlowDirection,
    LayoutFamily, Severity, SizeIntent, validate,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::convert::Infallible;

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
    let intrinsic = intrinsic_size(entity);
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
        && !text.font_sha256.is_empty()
        && !context.font_hashes.contains(&text.font_sha256)
    {
        snapshot.diagnostics.push(Diagnostic {
            code: "TEXT_FONT_NOT_PINNED".to_owned(),
            severity: Severity::Warning,
            message: format!("font {} is absent from the evaluation context", text.font),
            entity: Some(id),
            pointer: Some(format!("/entities/{id}/authored/text/font_sha256")),
            fidelity: Some(Fidelity::Approximated {
                reason: "font metrics use the deterministic fallback estimate".to_owned(),
            }),
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

fn intrinsic_size(entity: &Entity) -> Size {
    if let Some(text) = &entity.authored.text {
        let lines =
            f64::from(u32::try_from(text.content.lines().count().max(1)).unwrap_or(u32::MAX));
        let longest = f64::from(
            u32::try_from(
                text.content
                    .lines()
                    .map(|line| line.chars().count())
                    .max()
                    .unwrap_or(0),
            )
            .unwrap_or(u32::MAX),
        );
        return Size {
            width: longest * text.size * 0.6,
            height: lines * text.line_height,
        };
    }
    Size::default()
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
        LayoutFamily::Stack | LayoutFamily::Flex | LayoutFamily::Grid => {
            if entity.authored.layout.family != LayoutFamily::Stack {
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
                        reason: "the Taffy flex/grid evaluator is not wired yet".to_owned(),
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
    }
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
            let intrinsic = intrinsic_size(item);
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
        let intrinsic = intrinsic_size(item);
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
    use nuif_core::{Entity, LayoutStyle, OpaqueEncoding, OpaquePayload, TextContent, UnknownKind};

    #[test]
    fn fingerprint_includes_content_addressed_font_set() {
        let first = EvaluationContext::viewport(100.0, 100.0);
        let mut second = first.clone();
        second.font_hashes.insert("a".repeat(64));
        assert_ne!(first.fingerprint(), second.fingerprint());
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
