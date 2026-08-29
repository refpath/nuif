use nuif_codec::canonical_hash;
use nuif_core::{
    Align, Document, Entity, EntityId, EntityKind, FlowDirection, PropertyValue, SizeIntent,
};
use std::fmt::{Arguments, Write as _};

use crate::profile::profile_issues;
use crate::syntax::{escape_html, number};
use crate::{AdapterError, AdapterReport, ExportedSource, PROFILE_NAME};

/// Exports a document in the deliberately bounded retentive HTML/CSS profile.
///
/// # Errors
///
/// Returns a typed fidelity report when the document uses semantics outside
/// the profile, or a parser/canonical error if self-verification fails.
pub fn export_document(document: &Document) -> Result<ExportedSource, AdapterError> {
    let issues = profile_issues(document);
    if !issues.is_empty() {
        let count = issues.len();
        return Err(AdapterError::UnsupportedProfile {
            issues: count,
            report: Box::new(AdapterReport {
                schema_version: 1,
                source_format: PROFILE_NAME.to_owned(),
                canonical_hash: canonical_hash(document).ok(),
                fidelity: issues,
                correspondences: Vec::new(),
                unmapped_source_preserved: false,
            }),
        });
    }

    let source = render(document);
    let imported = crate::import_source(&source)?;
    if imported.document != *document {
        return Err(AdapterError::SynchronizationMismatch);
    }
    Ok(ExportedSource {
        source,
        report: imported.retentive.report,
    })
}

fn render(document: &Document) -> String {
    let mut source = String::new();
    source.push_str("<!doctype html>\n");
    emit(
        &mut source,
        format_args!(
            "<html data-nuif-profile=\"{PROFILE_NAME}\" data-nuif-document=\"{}\">\n",
            document.id
        ),
    );
    source.push_str("<head>\n  <meta charset=\"utf-8\">\n  <style data-nuif-styles>\n");
    source.push_str("    :root {\n");
    for token in document.tokens.values() {
        let PropertyValue::Real(value) = token.value else {
            unreachable!("profile validation accepts only real tokens");
        };
        emit(
            &mut source,
            format_args!(
                "      --nuif-token-{}: {}px; /* nuif-name:{} */\n",
                token.id,
                number(value),
                token.name
            ),
        );
    }
    source.push_str("    }\n");
    for entity in document.entities.values() {
        if entity.kind == EntityKind::Container {
            render_container_css(&mut source, entity);
        }
    }
    source.push_str("  </style>\n</head>\n<body>\n");
    for root in &document.roots {
        render_entity(&mut source, document, *root, 1);
    }
    source.push_str("</body>\n</html>\n");
    source
}

fn render_container_css(source: &mut String, entity: &Entity) {
    let SizeIntent::Fixed(width) = entity.authored.width else {
        unreachable!("profile validation requires fixed container width");
    };
    let SizeIntent::Fixed(height) = entity.authored.height else {
        unreachable!("profile validation requires fixed container height");
    };
    let layout = &entity.authored.layout;
    emit(
        source,
        format_args!("    [data-nuif-id=\"{}\"] {{\n", entity.id),
    );
    emit(source, format_args!("      width: {}px;\n", number(width)));
    emit(
        source,
        format_args!("      height: {}px;\n", number(height)),
    );
    source.push_str("      box-sizing: border-box;\n      display: flex;\n");
    emit(
        source,
        format_args!(
            "      flex-direction: {};\n",
            match layout.direction {
                FlowDirection::Row => "row",
                FlowDirection::Column => "column",
            }
        ),
    );
    emit(
        source,
        format_args!("      gap: {}px;\n", number(layout.gap)),
    );
    emit(
        source,
        format_args!("      padding-top: {}px;\n", number(layout.padding.top)),
    );
    emit(
        source,
        format_args!("      padding-right: {}px;\n", number(layout.padding.right)),
    );
    emit(
        source,
        format_args!(
            "      padding-bottom: {}px;\n",
            number(layout.padding.bottom)
        ),
    );
    emit(
        source,
        format_args!("      padding-left: {}px;\n", number(layout.padding.left)),
    );
    emit(
        source,
        format_args!(
            "      align-items: {};\n",
            match layout.align {
                Align::Start => "flex-start",
                Align::Center => "center",
                Align::End => "flex-end",
                Align::Stretch => "stretch",
            }
        ),
    );
    source.push_str("    }\n");
}

fn render_entity(source: &mut String, document: &Document, id: EntityId, depth: usize) {
    let entity = &document.entities[&id];
    let indent = "  ".repeat(depth);
    match entity.kind {
        EntityKind::Container => {
            let PropertyValue::Token(token) = entity.authored.values["token.spacing"] else {
                unreachable!("profile validation requires token.spacing");
            };
            emit(
                source,
                format_args!(
                    "{indent}<section data-nuif-id=\"{}\" data-nuif-kind=\"container\"{} data-nuif-token-spacing=\"{token}\">\n",
                    entity.id,
                    name_attribute(entity)
                ),
            );
            for child in &entity.children {
                render_entity(source, document, *child, depth + 1);
            }
            emit(source, format_args!("{indent}</section>\n"));
        }
        EntityKind::Text => {
            let text = entity
                .authored
                .text
                .as_ref()
                .expect("profile validation requires text content");
            emit(
                source,
                format_args!(
                    "{indent}<p data-nuif-id=\"{}\" data-nuif-kind=\"text\"{} data-nuif-font=\"{}\" data-nuif-font-sha256=\"{}\" data-nuif-font-size=\"{}\" data-nuif-line-height=\"{}\">{}</p>\n",
                    entity.id,
                    name_attribute(entity),
                    escape_html(&text.font),
                    escape_html(&text.font_sha256),
                    number(text.size),
                    number(text.line_height),
                    escape_html(&text.content)
                ),
            );
        }
        _ => unreachable!("profile validation accepts only container and text entities"),
    }
}

fn name_attribute(entity: &Entity) -> String {
    entity.name.as_ref().map_or_else(String::new, |name| {
        format!(" data-nuif-name=\"{}\"", escape_html(name))
    })
}

fn emit(source: &mut String, arguments: Arguments<'_>) {
    source
        .write_fmt(arguments)
        .expect("writing to a String cannot fail");
}
