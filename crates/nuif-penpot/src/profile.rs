use nuif_adapter::{CorrespondenceTarget, FidelityEntry};
use nuif_core::{
    Color, ColorSpace, Document, Entity, EntityId, EntityKind, Fidelity, LayoutStyle, Point,
    Semantics, ShapeKind, SizeIntent, TextContent,
};

pub(crate) fn profile_issues(document: &Document) -> Vec<FidelityEntry> {
    let mut issues = Vec::new();
    if !document.tokens.is_empty()
        || !document.relations.is_empty()
        || !document.extension_declarations.used.is_empty()
        || !document.extension_declarations.required.is_empty()
        || !document.extension_declarations.fallback_kind.is_empty()
        || !document.extensions.0.is_empty()
    {
        unsupported_document(
            document,
            &mut issues,
            "tokens, relations and extensions are outside nuif-penpot-v3-0",
        );
    }
    if document.roots.len() != 1 {
        unsupported_document(
            document,
            &mut issues,
            "nuif-penpot-v3-0 requires exactly one surface root",
        );
    }
    for entity in document.entities.values() {
        entity_issues(document, entity, &mut issues);
    }
    issues
}

fn entity_issues(document: &Document, entity: &Entity, issues: &mut Vec<FidelityEntry>) {
    if entity.name.as_deref().is_none_or(str::is_empty) {
        unsupported_entity(
            entity,
            issues,
            "/name",
            "mapped Penpot shapes require a non-empty name",
        );
    }
    if !entity.extensions.0.is_empty() || entity.semantics != Semantics::default() {
        unsupported_entity(
            entity,
            issues,
            "",
            "extensions and portable semantics are not mapped",
        );
    }
    if entity.authored.layout != LayoutStyle::default()
        || !entity.authored.responsive.is_empty()
        || !entity.authored.values.is_empty()
        || !finite_point(entity.authored.position)
    {
        unsupported_entity(
            entity,
            issues,
            "/authored",
            "layout, responsive rules, property values and non-finite positions are not mapped",
        );
    }
    match &entity.kind {
        EntityKind::Surface => surface_issues(document, entity, issues),
        EntityKind::Shape(ShapeKind::Rectangle | ShapeKind::Ellipse) => {
            graphic_issues(document, entity, false, issues);
        }
        EntityKind::Text => graphic_issues(document, entity, true, issues),
        _ => unsupported_entity(
            entity,
            issues,
            "/kind",
            "only one surface with direct rectangle, ellipse and text children is mapped",
        ),
    }
}

fn surface_issues(document: &Document, entity: &Entity, issues: &mut Vec<FidelityEntry>) {
    if document.roots != [entity.id]
        || document.parent_of(entity.id).is_some()
        || entity.authored.position != Point::default()
        || entity
            .authored
            .fill
            .is_none_or(|fill| !opaque_quantized_srgb(fill))
        || entity.authored.text.is_some()
        || !fixed_positive(&entity.authored.width)
        || !fixed_positive(&entity.authored.height)
    {
        unsupported_entity(
            entity,
            issues,
            "/authored",
            "the sole surface requires positive fixed dimensions, one opaque solid fill and default remaining authored fields",
        );
    }
    for child in &entity.children {
        if document.parent_of(*child) != Some(entity.id) {
            unsupported_entity(
                entity,
                issues,
                "/children",
                "surface children must be direct and unique",
            );
        }
    }
}

fn graphic_issues(
    document: &Document,
    entity: &Entity,
    text: bool,
    issues: &mut Vec<FidelityEntry>,
) {
    let sole_root = document.roots.first().copied();
    if !entity.children.is_empty()
        || document.parent_of(entity.id) != sole_root
        || !fixed_nonnegative(&entity.authored.width)
        || !fixed_nonnegative(&entity.authored.height)
    {
        unsupported_entity(
            entity,
            issues,
            "/authored",
            "leaf graphics require finite non-negative fixed dimensions",
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
                && value.size.is_finite()
                && value.size > 0.0
                && value.line_height.is_finite()
                && value.line_height > 0.0 => {}
        (true, _) => unsupported_entity(
            entity,
            issues,
            "/authored/text",
            "text requires literal content and the pinned font identity",
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
        pointer: format!("/entities/{}{suffix}", entity.id),
        status: Fidelity::Unsupported {
            reason: reason.to_owned(),
        },
    });
}

#[must_use]
pub fn profile_fixture() -> Document {
    let mut document = Document::empty(EntityId::new(1));
    let surface_id = EntityId::new(0x10);
    let rectangle_id = EntityId::new(0x21);
    let ellipse_id = EntityId::new(0x22);
    let text_id = EntityId::new(0x23);

    let mut surface = Entity::new(surface_id, EntityKind::Surface);
    surface.name = Some("Surface".to_owned());
    surface.authored.width = SizeIntent::Fixed(320.0);
    surface.authored.height = SizeIntent::Fixed(200.0);
    surface.authored.fill = Some(rgb(255, 255, 255));
    surface.children.extend([rectangle_id, ellipse_id, text_id]);

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
    ellipse.authored.fill = Some(rgb(22, 139, 91));

    let mut text = Entity::new(text_id, EntityKind::Text);
    text.name = Some("Label".to_owned());
    text.authored.position = Point { x: 76.0, y: 44.0 };
    text.authored.width = SizeIntent::Fixed(180.0);
    text.authored.height = SizeIntent::Fixed(40.0);
    text.authored.fill = Some(rgb(30, 41, 59));
    text.authored.text = Some(TextContent {
        content: "NUIF Penpot profile".to_owned(),
        font: nuif_text::PINNED_FONT_NAME.to_owned(),
        font_sha256: nuif_text::PINNED_FONT_SHA256.to_owned(),
        size: 16.0,
        line_height: 24.0,
    });

    document.roots.push(surface_id);
    for entity in [surface, rectangle, ellipse, text] {
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
