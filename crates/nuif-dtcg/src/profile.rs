use crate::{CorrespondenceTarget, FidelityEntry};
use nuif_core::{
    Document, EntityId, ExtensionDeclarations, Extensions, Fidelity, PropertyValue, Severity,
    Token, validate,
};

pub(crate) fn profile_issues(document: &Document) -> Vec<FidelityEntry> {
    let mut issues = validate(document)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| FidelityEntry {
            target: CorrespondenceTarget::Document { id: document.id },
            pointer: diagnostic.pointer.unwrap_or_else(|| "/".to_owned()),
            status: Fidelity::Unsupported {
                reason: diagnostic.message,
            },
        })
        .collect::<Vec<_>>();
    if !document.roots.is_empty()
        || !document.entities.is_empty()
        || !document.relations.is_empty()
        || document.extension_declarations != ExtensionDeclarations::default()
        || document.extensions != Extensions::default()
    {
        unsupported_document(
            document,
            &mut issues,
            "entities, roots, relations and document extensions are outside the scalar-token profile",
        );
    }
    for token in document.tokens.values() {
        if !valid_name(&token.name) {
            unsupported_token(
                token.id,
                &mut issues,
                "/name",
                "DTCG token names cannot be empty, start with $, or contain dot or braces",
            );
        }
        let supported = matches!(
            token.value,
            PropertyValue::Boolean(_) | PropertyValue::Integer(_) | PropertyValue::String(_)
        ) || matches!(token.value, PropertyValue::Real(value) if value.is_finite());
        if !supported {
            unsupported_token(
                token.id,
                &mut issues,
                "/value",
                "only boolean, string, integer and finite real values are mapped",
            );
        }
    }
    issues
}

pub(crate) fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('$')
        && !name
            .chars()
            .any(|character| matches!(character, '.' | '{' | '}'))
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

fn unsupported_token(id: EntityId, issues: &mut Vec<FidelityEntry>, suffix: &str, reason: &str) {
    issues.push(FidelityEntry {
        target: CorrespondenceTarget::Token { id },
        pointer: format!("/tokens/{id}{suffix}"),
        status: Fidelity::Unsupported {
            reason: reason.to_owned(),
        },
    });
}

#[must_use]
pub fn profile_fixture() -> Document {
    let mut document = Document::empty(EntityId::new(1));
    for (id, name, value) in [
        (0x100, "enabled", PropertyValue::Boolean(true)),
        (0x101, "label", PropertyValue::String("Primary".to_owned())),
        (0x102, "count", PropertyValue::Integer(7)),
        (0x103, "spacing", PropertyValue::Real(24.0)),
    ] {
        let id = EntityId::new(id);
        document.tokens.insert(
            id,
            Token {
                id,
                name: name.to_owned(),
                value,
            },
        );
    }
    document
}
