use crate::{CorrespondenceTarget, FidelityEntry, entity_pointer};
use nuif_core::{
    AuthoredProperties, Color, ColorSpace, Document, Entity, EntityId, EntityKind, Fidelity,
    LayoutStyle, Point, Semantics, ShapeKind, SizeIntent, TextContent,
};
use std::collections::BTreeMap;

pub(crate) fn profile_issues(document: &Document) -> Vec<FidelityEntry> {
    let mut issues = Vec::new();
    if !document.assets.is_empty()
        || !document.tokens.is_empty()
        || !document.relations.is_empty()
        || !document.extension_declarations.used.is_empty()
        || !document.extension_declarations.required.is_empty()
        || !document.extension_declarations.fallback_kind.is_empty()
        || !document.extensions.0.is_empty()
    {
        unsupported_document(
            document,
            &mut issues,
            "assets, tokens, relations and extensions are outside nuif-svg-0",
        );
    }
    if document.roots.len() != 1 {
        unsupported_document(
            document,
            &mut issues,
            "nuif-svg-0 requires exactly one surface root",
        );
    }

    for entity in document.entities.values() {
        entity_issues(document, entity, &mut issues);
    }
    issues
}

fn entity_issues(document: &Document, entity: &Entity, issues: &mut Vec<FidelityEntry>) {
    if !entity.extensions.0.is_empty() {
        unsupported_entity(
            entity,
            issues,
            "/extensions",
            "entity extensions are not mapped",
        );
    }
    if !entity.semantics.states.is_empty() {
        unsupported_entity(
            entity,
            issues,
            "/semantics/states",
            "semantic states are not mapped",
        );
    }
    if !finite_point(entity.authored.position) {
        unsupported_entity(
            entity,
            issues,
            "/authored/position",
            "position must be finite",
        );
    }
    if entity.authored.layout != LayoutStyle::default()
        || !entity.authored.responsive.is_empty()
        || !entity.authored.values.is_empty()
    {
        unsupported_entity(
            entity,
            issues,
            "/authored",
            "layout, responsive rules and property values are not mapped",
        );
    }

    match &entity.kind {
        EntityKind::Surface => surface_issues(document, entity, issues),
        EntityKind::Container => container_issues(entity, issues),
        EntityKind::Shape(ShapeKind::Rectangle | ShapeKind::Ellipse) => {
            graphics_issues(entity, false, issues);
        }
        EntityKind::Text => graphics_issues(entity, true, issues),
        _ => unsupported_entity(
            entity,
            issues,
            "/kind",
            "only surface, container, rectangle, ellipse and text are mapped",
        ),
    }
}

fn surface_issues(document: &Document, entity: &Entity, issues: &mut Vec<FidelityEntry>) {
    if document.roots != [entity.id] || document.parent_of(entity.id).is_some() {
        unsupported_entity(
            entity,
            issues,
            "/kind",
            "the surface must be the sole document root",
        );
    }
    if entity.authored.position != Point::default()
        || entity.authored.fill.is_some()
        || entity.authored.text.is_some()
        || !fixed_positive(&entity.authored.width)
        || !fixed_positive(&entity.authored.height)
    {
        unsupported_entity(
            entity,
            issues,
            "/authored",
            "the surface requires positive fixed dimensions and default remaining authored fields",
        );
    }
}

fn container_issues(entity: &Entity, issues: &mut Vec<FidelityEntry>) {
    if entity.authored != AuthoredProperties::default() {
        unsupported_entity(
            entity,
            issues,
            "/authored",
            "SVG groups do not map authored geometry or paint in profile zero",
        );
    }
}

fn graphics_issues(entity: &Entity, text: bool, issues: &mut Vec<FidelityEntry>) {
    if !fixed_nonnegative(&entity.authored.width) || !fixed_nonnegative(&entity.authored.height) {
        unsupported_entity(
            entity,
            issues,
            "/authored",
            "graphics require finite non-negative fixed dimensions",
        );
    }
    if !entity.children.is_empty() {
        unsupported_entity(
            entity,
            issues,
            "/children",
            "graphics cannot contain mapped children",
        );
    }
    if let Some(fill) = entity.authored.fill
        && !opaque_quantized_srgb(fill)
    {
        unsupported_entity(
            entity,
            issues,
            "/authored/fill",
            "fill must be opaque sRGB with 8-bit-exact channels",
        );
    }
    match (text, &entity.authored.text) {
        (true, Some(value))
            if value.font == nuif_text::PINNED_FONT_NAME
                && value.font_sha256 == nuif_text::PINNED_FONT_SHA256
                && value.font_asset.is_none()
                && value.size.is_finite()
                && value.size > 0.0
                && value.line_height.is_finite()
                && value.line_height > 0.0 => {}
        (true, _) => unsupported_entity(
            entity,
            issues,
            "/authored/text",
            "text requires an unbound pinned font identity and positive finite metrics",
        ),
        (false, None) => {}
        (false, Some(_)) => unsupported_entity(
            entity,
            issues,
            "/authored/text",
            "shape text content is not mapped",
        ),
    }
}

fn fixed_positive(value: &SizeIntent) -> bool {
    matches!(value, SizeIntent::Fixed(value) if value.is_finite() && *value > 0.0)
}

fn fixed_nonnegative(value: &SizeIntent) -> bool {
    matches!(value, SizeIntent::Fixed(value) if value.is_finite() && *value >= 0.0)
}

fn finite_point(point: Point) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

fn opaque_quantized_srgb(color: Color) -> bool {
    color.space == ColorSpace::Srgb
        && (color.alpha - 1.0).abs() <= f32::EPSILON
        && [color.red, color.green, color.blue]
            .into_iter()
            .all(|channel| {
                channel.is_finite()
                    && (0.0..=1.0).contains(&channel)
                    && ((channel * 255.0).round() / 255.0 - channel).abs() <= f32::EPSILON
            })
}

fn unsupported_document(document: &Document, issues: &mut Vec<FidelityEntry>, reason: &str) {
    issues.push(FidelityEntry {
        target: CorrespondenceTarget::Document { id: document.id },
        pointer: String::new(),
        status: Fidelity::Unsupported {
            reason: reason.to_owned(),
        },
    });
}

fn unsupported_entity(
    entity: &Entity,
    issues: &mut Vec<FidelityEntry>,
    suffix: &str,
    reason: &str,
) {
    issues.push(FidelityEntry {
        target: CorrespondenceTarget::Entity { id: entity.id },
        pointer: entity_pointer(entity.id, suffix),
        status: Fidelity::Unsupported {
            reason: reason.to_owned(),
        },
    });
}

#[must_use]
pub fn profile_fixture() -> Document {
    let mut document = Document::empty(EntityId::new(1));
    let surface_id = EntityId::new(0x10);
    let group_id = EntityId::new(0x20);
    let rectangle_id = EntityId::new(0x21);
    let ellipse_id = EntityId::new(0x22);
    let text_id = EntityId::new(0x23);

    let mut surface = Entity::new(surface_id, EntityKind::Surface);
    surface.name = Some("SVG profile surface".to_owned());
    surface.authored.width = SizeIntent::Fixed(320.0);
    surface.authored.height = SizeIntent::Fixed(200.0);
    surface.children.push(group_id);

    let mut group = Entity::new(group_id, EntityKind::Container);
    group.name = Some("Artwork".to_owned());
    group.semantics = Semantics {
        role: Some("group".to_owned()),
        accessible_name: Some("Profile artwork".to_owned()),
        states: BTreeMap::default(),
    };
    group.children.extend([rectangle_id, ellipse_id, text_id]);

    let mut rectangle = Entity::new(rectangle_id, EntityKind::Shape(ShapeKind::Rectangle));
    rectangle.name = Some("Card".to_owned());
    rectangle.authored.position = Point { x: 16.0, y: 20.0 };
    rectangle.authored.width = SizeIntent::Fixed(288.0);
    rectangle.authored.height = SizeIntent::Fixed(160.0);
    rectangle.authored.fill = Some(rgb(239, 244, 255));

    let mut ellipse = Entity::new(ellipse_id, EntityKind::Shape(ShapeKind::Ellipse));
    ellipse.name = Some("Status".to_owned());
    ellipse.authored.position = Point { x: 36.0, y: 48.0 };
    ellipse.authored.width = SizeIntent::Fixed(24.0);
    ellipse.authored.height = SizeIntent::Fixed(24.0);
    ellipse.authored.fill = Some(rgb(34, 197, 94));

    let mut text = Entity::new(text_id, EntityKind::Text);
    text.name = Some("Label".to_owned());
    text.authored.position = Point { x: 76.0, y: 48.0 };
    text.authored.width = SizeIntent::Fixed(180.0);
    text.authored.height = SizeIntent::Fixed(24.0);
    text.authored.fill = Some(rgb(30, 41, 59));
    text.authored.text = Some(TextContent {
        content: "NUIF SVG profile".to_owned(),
        font: nuif_text::PINNED_FONT_NAME.to_owned(),
        font_sha256: nuif_text::PINNED_FONT_SHA256.to_owned(),
        font_asset: None,
        size: 16.0,
        line_height: 24.0,
    });

    document.roots.push(surface_id);
    for entity in [surface, group, rectangle, ellipse, text] {
        document.entities.insert(entity.id, entity);
    }
    document
}

fn rgb(red: u8, green: u8, blue: u8) -> Color {
    Color {
        space: ColorSpace::Srgb,
        red: f32::from(red) / 255.0,
        green: f32::from(green) / 255.0,
        blue: f32::from(blue) / 255.0,
        alpha: 1.0,
    }
}
