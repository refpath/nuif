use nuif_core::{
    Align, AuthoredProperties, Document, Edges, Entity, EntityId, EntityKind,
    ExtensionDeclarations, Extensions, Fidelity, FlowDirection, LayoutFamily, LayoutStyle, Point,
    Semantics, Severity, SizeIntent, TextContent, validate,
};

use crate::{CorrespondenceTarget, FidelityEntry};

pub const PROFILE_NAME: &str = "nuif-svelte-static-0";

#[must_use]
pub fn profile_fixture() -> Document {
    let document_id = EntityId::new(1);
    let root_id = EntityId::new(0x10);
    let text_id = EntityId::new(0x11);
    let mut document = Document::empty(document_id);

    let mut root = Entity::new(root_id, EntityKind::Container);
    root.name = Some("Card".to_owned());
    root.children.push(text_id);
    root.authored = AuthoredProperties {
        width: SizeIntent::Fixed(320.0),
        height: SizeIntent::Fixed(200.0),
        layout: LayoutStyle {
            family: LayoutFamily::Stack,
            direction: FlowDirection::Column,
            gap: 16.0,
            padding: Edges {
                top: 24.0,
                right: 24.0,
                bottom: 24.0,
                left: 24.0,
            },
            align: Align::Stretch,
        },
        ..AuthoredProperties::default()
    };

    let mut text = Entity::new(text_id, EntityKind::Text);
    text.name = Some("Copy".to_owned());
    text.authored.width = SizeIntent::Fill;
    text.authored.height = SizeIntent::Intrinsic;
    text.authored.text = Some(TextContent {
        content: "Portable authored intent".to_owned(),
        font: nuif_text::PINNED_FONT_NAME.to_owned(),
        font_sha256: nuif_text::PINNED_FONT_SHA256.to_owned(),
        size: 18.0,
        line_height: 24.0,
    });

    document.roots.push(root_id);
    document.entities.insert(root_id, root);
    document.entities.insert(text_id, text);
    document
}

pub(crate) fn profile_issues(document: &Document) -> Vec<FidelityEntry> {
    let mut issues = Vec::new();
    for diagnostic in validate(document) {
        if diagnostic.severity == Severity::Error {
            issues.push(FidelityEntry {
                target: CorrespondenceTarget::Document { id: document.id },
                pointer: diagnostic.pointer.unwrap_or_else(|| "/".to_owned()),
                status: Fidelity::Unsupported {
                    reason: diagnostic.message,
                },
            });
        }
    }
    if document.roots.len() != 1 {
        unsupported_document(
            &mut issues,
            document,
            "/roots",
            "profile requires exactly one Svelte root entity",
        );
    }
    if !document.tokens.is_empty()
        || !document.relations.is_empty()
        || document.extension_declarations != ExtensionDeclarations::default()
        || document.extensions != Extensions::default()
    {
        unsupported_document(
            &mut issues,
            document,
            "/",
            "profile does not map tokens, relations or document extensions",
        );
    }
    for entity in document.entities.values() {
        match entity.kind {
            EntityKind::Container => container_issues(entity, &mut issues),
            EntityKind::Text => text_issues(entity, &mut issues),
            _ => unsupported_entity(
                &mut issues,
                entity.id,
                "/kind",
                "profile maps only container and literal text entities",
            ),
        }
    }
    issues
}

fn container_issues(entity: &Entity, issues: &mut Vec<FidelityEntry>) {
    if !matches!(entity.authored.width, SizeIntent::Fixed(value) if value.is_finite()) {
        unsupported_entity(
            issues,
            entity.id,
            "/authored/width",
            "container width must be finite and fixed",
        );
    }
    if !matches!(entity.authored.height, SizeIntent::Fixed(value) if value.is_finite()) {
        unsupported_entity(
            issues,
            entity.id,
            "/authored/height",
            "container height must be finite and fixed",
        );
    }
    if entity.authored.layout.family != LayoutFamily::Stack
        || !entity.authored.layout.gap.is_finite()
        || !edges_finite(entity.authored.layout.padding)
    {
        unsupported_entity(
            issues,
            entity.id,
            "/authored/layout",
            "container requires finite stack layout values",
        );
    }
    if entity.name.is_none()
        || entity.authored.position != Point::default()
        || entity.authored.fill.is_some()
        || entity.authored.text.is_some()
        || !entity.authored.responsive.is_empty()
        || !entity.authored.values.is_empty()
        || entity.semantics != Semantics::default()
        || entity.extensions != Extensions::default()
    {
        unsupported_entity(
            issues,
            entity.id,
            "/authored",
            "container requires a name and otherwise only the mapped stack fields",
        );
    }
}

fn text_issues(entity: &Entity, issues: &mut Vec<FidelityEntry>) {
    let authored = &entity.authored;
    if entity.name.is_none()
        || authored.width != SizeIntent::Fill
        || authored.height != SizeIntent::Intrinsic
        || authored.position != Point::default()
        || authored.layout != LayoutStyle::default()
        || authored.fill.is_some()
        || !authored.responsive.is_empty()
        || !authored.values.is_empty()
        || !entity.children.is_empty()
        || entity.semantics != Semantics::default()
        || entity.extensions != Extensions::default()
    {
        unsupported_entity(
            issues,
            entity.id,
            "/authored",
            "text requires a name, fill/intrinsic sizing and otherwise default state",
        );
    }
    let Some(text) = &authored.text else {
        unsupported_entity(
            issues,
            entity.id,
            "/authored/text",
            "literal text content is required",
        );
        return;
    };
    if !text.size.is_finite()
        || !text.line_height.is_finite()
        || text.font != nuif_text::PINNED_FONT_NAME
        || text.font_sha256 != nuif_text::PINNED_FONT_SHA256
    {
        unsupported_entity(
            issues,
            entity.id,
            "/authored/text",
            "text requires finite metrics and the profile-pinned font identity",
        );
    }
}

fn edges_finite(edges: Edges) -> bool {
    edges.top.is_finite()
        && edges.right.is_finite()
        && edges.bottom.is_finite()
        && edges.left.is_finite()
}

fn unsupported_document(
    issues: &mut Vec<FidelityEntry>,
    document: &Document,
    pointer: &str,
    reason: &str,
) {
    issues.push(FidelityEntry {
        target: CorrespondenceTarget::Document { id: document.id },
        pointer: pointer.to_owned(),
        status: Fidelity::Unsupported {
            reason: reason.to_owned(),
        },
    });
}

fn unsupported_entity(issues: &mut Vec<FidelityEntry>, id: EntityId, suffix: &str, reason: &str) {
    issues.push(FidelityEntry {
        target: CorrespondenceTarget::Entity { id },
        pointer: crate::entity_pointer(id, suffix),
        status: Fidelity::Unsupported {
            reason: reason.to_owned(),
        },
    });
}
