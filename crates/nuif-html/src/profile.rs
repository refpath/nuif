use nuif_core::{
    Align, AuthoredProperties, Document, Edges, Entity, EntityId, EntityKind,
    ExtensionDeclarations, Extensions, Fidelity, FlowDirection, LayoutFamily, LayoutStyle, Point,
    PropertyValue, Semantics, Severity, SizeIntent, TextContent, Token, validate,
};

use crate::{CorrespondenceTarget, FidelityEntry, entity_pointer, token_pointer};

pub const PROFILE_NAME: &str = "nuif-html-css-0";

#[must_use]
pub fn profile_fixture() -> Document {
    let document_id = EntityId::new(1);
    let root_id = EntityId::new(0x10);
    let text_id = EntityId::new(0x11);
    let token_id = EntityId::new(0x100);
    let mut document = Document::empty(document_id);
    document.tokens.insert(
        token_id,
        Token {
            id: token_id,
            name: "space.card".to_owned(),
            value: PropertyValue::Real(24.0),
        },
    );

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
        values: [("token.spacing".to_owned(), PropertyValue::Token(token_id))]
            .into_iter()
            .collect(),
        ..AuthoredProperties::default()
    };

    let mut text = Entity::new(text_id, EntityKind::Text);
    text.name = Some("Copy".to_owned());
    text.authored.width = SizeIntent::Fill;
    text.authored.height = SizeIntent::Intrinsic;
    text.authored.text = Some(TextContent {
        content: "Portable authored intent".to_owned(),
        font: "Ahem".to_owned(),
        font_sha256: "f0a92cd0cc45735591c9b5b1fa8aecd5194e8dc518895ca22af94a46c23550dc".to_owned(),
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
            "profile requires exactly one HTML body root",
        );
    }
    if !document.relations.is_empty() {
        unsupported_document(
            &mut issues,
            document,
            "/relations",
            "profile does not map relations",
        );
    }
    if document.extension_declarations != ExtensionDeclarations::default()
        || document.extensions != Extensions::default()
    {
        unsupported_document(
            &mut issues,
            document,
            "/extensions",
            "profile does not map document extensions",
        );
    }
    for token in document.tokens.values() {
        if !safe_name(&token.name) {
            unsupported_token(
                &mut issues,
                token.id,
                "/name",
                "token names are limited to ASCII letters, digits, dot, underscore and hyphen",
            );
        }
        if !matches!(token.value, PropertyValue::Real(value) if value.is_finite()) {
            unsupported_token(
                &mut issues,
                token.id,
                "/value",
                "profile maps only finite real-valued CSS length tokens",
            );
        }
    }
    for entity in document.entities.values() {
        match entity.kind {
            EntityKind::Container => container_issues(document, entity, &mut issues),
            EntityKind::Text => text_issues(entity, &mut issues),
            _ => unsupported_entity(
                &mut issues,
                entity.id,
                "/kind",
                "profile maps only container and text entities",
            ),
        }
    }
    issues
}

fn container_issues(document: &Document, entity: &Entity, issues: &mut Vec<FidelityEntry>) {
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
    if entity.authored.layout.family != LayoutFamily::Stack {
        unsupported_entity(
            issues,
            entity.id,
            "/authored/layout/family",
            "container layout must be stack",
        );
    }
    if !entity.authored.layout.gap.is_finite() || !edges_finite(entity.authored.layout.padding) {
        unsupported_entity(
            issues,
            entity.id,
            "/authored/layout",
            "gap and padding must be finite",
        );
    }
    let shared_default = entity.authored.position == Point::default()
        && entity.authored.fill.is_none()
        && entity.authored.text.is_none()
        && entity.authored.responsive.is_empty()
        && entity.semantics == Semantics::default()
        && entity.extensions == Extensions::default();
    if !shared_default {
        unsupported_entity(
            issues,
            entity.id,
            "/authored",
            "container position, fill, text, responsive, semantics and extensions must be default",
        );
    }
    if entity.authored.values.len() != 1 {
        unsupported_entity(
            issues,
            entity.id,
            "/authored/values",
            "container must carry exactly one token.spacing binding",
        );
    } else if let Some(value) = entity.authored.values.get("token.spacing") {
        if !matches!(value, PropertyValue::Token(id) if document.tokens.contains_key(id)) {
            unsupported_entity(
                issues,
                entity.id,
                "/authored/values/token.spacing",
                "token.spacing must reference a mapped token",
            );
        }
    } else {
        unsupported_entity(
            issues,
            entity.id,
            "/authored/values",
            "container token binding must be named token.spacing",
        );
    }
}

fn text_issues(entity: &Entity, issues: &mut Vec<FidelityEntry>) {
    let authored = &entity.authored;
    if authored.width != SizeIntent::Fill
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
            "text profile requires fill/intrinsic sizing and otherwise default authored state",
        );
    }
    let Some(text) = &authored.text else {
        unsupported_entity(
            issues,
            entity.id,
            "/authored/text",
            "text content is required",
        );
        return;
    };
    if !text.size.is_finite() || !text.line_height.is_finite() {
        unsupported_entity(
            issues,
            entity.id,
            "/authored/text",
            "text size and line height must be finite",
        );
    }
}

fn edges_finite(edges: Edges) -> bool {
    edges.top.is_finite()
        && edges.right.is_finite()
        && edges.bottom.is_finite()
        && edges.left.is_finite()
}

fn safe_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn unsupported_document(
    issues: &mut Vec<FidelityEntry>,
    document: &Document,
    suffix: &str,
    reason: &str,
) {
    issues.push(FidelityEntry {
        target: CorrespondenceTarget::Document { id: document.id },
        pointer: suffix.to_owned(),
        status: Fidelity::Unsupported {
            reason: reason.to_owned(),
        },
    });
}

fn unsupported_entity(issues: &mut Vec<FidelityEntry>, id: EntityId, suffix: &str, reason: &str) {
    issues.push(FidelityEntry {
        target: CorrespondenceTarget::Entity { id },
        pointer: entity_pointer(id, suffix),
        status: Fidelity::Unsupported {
            reason: reason.to_owned(),
        },
    });
}

fn unsupported_token(issues: &mut Vec<FidelityEntry>, id: EntityId, suffix: &str, reason: &str) {
    issues.push(FidelityEntry {
        target: CorrespondenceTarget::Token { id },
        pointer: token_pointer(id, suffix),
        status: Fidelity::Unsupported {
            reason: reason.to_owned(),
        },
    });
}
