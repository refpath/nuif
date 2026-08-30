use nuif_codec::canonical_hash;
use nuif_core::{Align, Document, EntityId, EntityKind, FlowDirection, SizeIntent};
use std::fmt::{Arguments, Write as _};

use crate::profile::profile_issues;
use crate::syntax::{escape_attribute, escape_text, number};
use crate::{AdapterError, AdapterReport, ExportedSource, PROFILE_NAME};

/// Exports a document in the bounded static Svelte profile.
///
/// # Errors
///
/// Returns typed fidelity when the document exceeds the profile and rejects
/// generated source that does not self-import to the same document.
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
    source.push_str("<!-- Generated bounded NUIF component; scalar spans remain retentive. -->\n");
    render_entity(
        &mut source,
        document,
        document.roots[0],
        0,
        Some(document.id),
    );
    source
}

fn render_entity(
    source: &mut String,
    document: &Document,
    id: EntityId,
    depth: usize,
    document_id: Option<EntityId>,
) {
    let entity = &document.entities[&id];
    let indent = "  ".repeat(depth);
    match entity.kind {
        EntityKind::Container => {
            let SizeIntent::Fixed(width) = entity.authored.width else {
                unreachable!("profile validation requires fixed width");
            };
            let SizeIntent::Fixed(height) = entity.authored.height else {
                unreachable!("profile validation requires fixed height");
            };
            emit(
                source,
                format_args!(
                    "{indent}<div data-nuif-id=\"{}\" data-nuif-kind=\"container\" data-nuif-name=\"{}\"",
                    entity.id,
                    escape_attribute(entity.name.as_deref().expect("profile requires name")),
                ),
            );
            if let Some(document_id) = document_id {
                emit(
                    source,
                    format_args!(
                        " data-nuif-profile=\"{PROFILE_NAME}\" data-nuif-document=\"{document_id}\""
                    ),
                );
            }
            let layout = &entity.authored.layout;
            emit(
                source,
                format_args!(
                    " style=\"width:{}px;height:{}px;box-sizing:border-box;display:flex;flex-direction:{};gap:{}px;padding-top:{}px;padding-right:{}px;padding-bottom:{}px;padding-left:{}px;align-items:{}\">\n",
                    number(width),
                    number(height),
                    direction(layout.direction),
                    number(layout.gap),
                    number(layout.padding.top),
                    number(layout.padding.right),
                    number(layout.padding.bottom),
                    number(layout.padding.left),
                    alignment(layout.align),
                ),
            );
            for child in &entity.children {
                render_entity(source, document, *child, depth + 1, None);
            }
            emit(source, format_args!("{indent}</div>\n"));
        }
        EntityKind::Text => {
            let text = entity
                .authored
                .text
                .as_ref()
                .expect("profile validation requires text");
            emit(
                source,
                format_args!(
                    "{indent}<span data-nuif-id=\"{}\" data-nuif-kind=\"text\" data-nuif-name=\"{}\" data-nuif-font-sha256=\"{}\" style=\"width:100%;font-family:{};font-size:{}px;line-height:{}px\">{}</span>\n",
                    entity.id,
                    escape_attribute(entity.name.as_deref().expect("profile requires name")),
                    text.font_sha256,
                    text.font,
                    number(text.size),
                    number(text.line_height),
                    escape_text(&text.content),
                ),
            );
        }
        _ => unreachable!("profile validation accepts only container and text entities"),
    }
}

fn direction(direction: FlowDirection) -> &'static str {
    match direction {
        FlowDirection::Row => "row",
        FlowDirection::Column => "column",
    }
}

fn alignment(alignment: Align) -> &'static str {
    match alignment {
        Align::Start => "flex-start",
        Align::Center => "center",
        Align::End => "flex-end",
        Align::Stretch => "stretch",
    }
}

fn emit(source: &mut String, arguments: Arguments<'_>) {
    source
        .write_fmt(arguments)
        .expect("writing to a String cannot fail");
}
