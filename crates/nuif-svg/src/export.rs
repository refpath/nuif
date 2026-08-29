use crate::profile::profile_issues;
use crate::{AdapterError, ExportedSource, PROFILE_NAME, SVG_NAMESPACE, import_source};
use nuif_adapter::AdapterReport;
use nuif_core::{Color, Document, Entity, EntityId, EntityKind, ShapeKind, SizeIntent};
use std::fmt::Write as _;

/// Exports a document in the bounded `nuif-svg-0` profile.
///
/// # Errors
///
/// Returns typed profile, value, parse or canonicalization errors when the
/// document cannot produce an exact SVG round trip.
pub fn export_document(document: &Document) -> Result<ExportedSource, AdapterError> {
    let issues = profile_issues(document);
    if !issues.is_empty() {
        return Err(AdapterError::UnsupportedProfile {
            issues: issues.len(),
            report: Box::new(AdapterReport {
                schema_version: 1,
                source_format: PROFILE_NAME.to_owned(),
                canonical_hash: None,
                fidelity: issues,
                correspondences: Vec::new(),
                unmapped_source_preserved: false,
            }),
        });
    }
    let source = render(document)?;
    let imported = import_source(&source)?;
    if imported.document != *document {
        return Err(AdapterError::SynchronizationMismatch);
    }
    Ok(ExportedSource {
        source,
        report: imported.retentive.report,
    })
}

fn render(document: &Document) -> Result<String, AdapterError> {
    let surface = &document.entities[&document.roots[0]];
    let width = fixed(&surface.authored.width);
    let height = fixed(&surface.authored.height);
    let mut output = String::new();
    write!(
        output,
        "<svg xmlns=\"{}\" data-nuif-profile=\"{}\" data-nuif-document=\"{}\"",
        SVG_NAMESPACE, PROFILE_NAME, document.id
    )
    .expect("writing to a string cannot fail");
    write_common_attributes(surface, &mut output);
    writeln!(
        output,
        " width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        number(width),
        number(height),
        number(width),
        number(height)
    )
    .expect("writing to a string cannot fail");
    for child in &surface.children {
        render_entity(document, *child, 1, &mut output)?;
    }
    output.push_str("</svg>\n");
    Ok(output)
}

fn render_entity(
    document: &Document,
    id: EntityId,
    depth: usize,
    output: &mut String,
) -> Result<(), AdapterError> {
    let entity = &document.entities[&id];
    output.push_str(&"  ".repeat(depth));
    match entity.kind {
        EntityKind::Container => {
            output.push_str("<g");
            write_common_attributes(entity, output);
            output.push_str(">\n");
            for child in &entity.children {
                render_entity(document, *child, depth + 1, output)?;
            }
            output.push_str(&"  ".repeat(depth));
            output.push_str("</g>\n");
        }
        EntityKind::Shape(ShapeKind::Rectangle) => render_rectangle(entity, output),
        EntityKind::Shape(ShapeKind::Ellipse) => render_ellipse(entity, output),
        EntityKind::Text => render_text(entity, output),
        _ => {
            return Err(AdapterError::InvalidValue {
                pointer: format!("/entities/{id}/kind"),
                reason: "profile validation admitted an unsupported kind".to_owned(),
            });
        }
    }
    Ok(())
}

fn render_rectangle(entity: &Entity, output: &mut String) {
    output.push_str("<rect");
    write_common_attributes(entity, output);
    writeln!(
        output,
        " x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
        number(entity.authored.position.x),
        number(entity.authored.position.y),
        number(fixed(&entity.authored.width)),
        number(fixed(&entity.authored.height)),
        fill(entity.authored.fill)
    )
    .expect("writing to a string cannot fail");
}

fn render_ellipse(entity: &Entity, output: &mut String) {
    let x = entity.authored.position.x;
    let y = entity.authored.position.y;
    let width = fixed(&entity.authored.width);
    let height = fixed(&entity.authored.height);
    write!(output, "<ellipse").expect("writing to a string cannot fail");
    write_common_attributes(entity, output);
    writeln!(
        output,
        " data-nuif-x=\"{}\" data-nuif-y=\"{}\" data-nuif-width=\"{}\" data-nuif-height=\"{}\" cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\"/>",
        number(x),
        number(y),
        number(width),
        number(height),
        number(x + width / 2.0),
        number(y + height / 2.0),
        number(width / 2.0),
        number(height / 2.0),
        fill(entity.authored.fill)
    )
    .expect("writing to a string cannot fail");
}

fn render_text(entity: &Entity, output: &mut String) {
    let text = entity
        .authored
        .text
        .as_ref()
        .expect("profile validation requires text content");
    output.push_str("<text");
    write_common_attributes(entity, output);
    writeln!(
        output,
        " x=\"{}\" y=\"{}\" data-nuif-width=\"{}\" data-nuif-height=\"{}\" font-family=\"{}\" data-nuif-font-sha256=\"{}\" font-size=\"{}\" data-nuif-line-height=\"{}\" dominant-baseline=\"text-before-edge\" fill=\"{}\">{}</text>",
        number(entity.authored.position.x),
        number(entity.authored.position.y),
        number(fixed(&entity.authored.width)),
        number(fixed(&entity.authored.height)),
        escape_attribute(&text.font),
        escape_attribute(&text.font_sha256),
        number(text.size),
        number(text.line_height),
        fill(entity.authored.fill),
        escape_text(&text.content)
    )
    .expect("writing to a string cannot fail");
}

fn write_common_attributes(entity: &Entity, output: &mut String) {
    let kind = match entity.kind {
        EntityKind::Surface => "surface",
        EntityKind::Container => "container",
        EntityKind::Shape(ShapeKind::Rectangle) => "rectangle",
        EntityKind::Shape(ShapeKind::Ellipse) => "ellipse",
        EntityKind::Text => "text",
        _ => unreachable!("profile validation rejects unsupported kinds"),
    };
    write!(
        output,
        " data-nuif-id=\"{}\" data-nuif-kind=\"{kind}\"",
        entity.id
    )
    .expect("writing to a string cannot fail");
    if let Some(name) = &entity.name {
        write!(output, " data-nuif-name=\"{}\"", escape_attribute(name))
            .expect("writing to a string cannot fail");
    }
    if let Some(role) = &entity.semantics.role {
        write!(output, " role=\"{}\"", escape_attribute(role))
            .expect("writing to a string cannot fail");
    }
    if let Some(name) = &entity.semantics.accessible_name {
        write!(output, " aria-label=\"{}\"", escape_attribute(name))
            .expect("writing to a string cannot fail");
    }
}

pub(crate) fn number(value: f64) -> String {
    if value == 0.0 {
        "0".to_owned()
    } else {
        value.to_string()
    }
}

pub(crate) fn fill(value: Option<Color>) -> String {
    value.map_or_else(
        || "none".to_owned(),
        |color| {
            format!(
                "#{:02x}{:02x}{:02x}",
                color_byte(color.red),
                color_byte(color.green),
                color_byte(color.blue)
            )
        },
    )
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "profile validation restricts channels to exact values in the u8 domain"
)]
fn color_byte(channel: f32) -> u8 {
    (channel * 255.0).round() as u8
}

pub(crate) fn escape_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub(crate) fn escape_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub(crate) fn fixed(value: &SizeIntent) -> f64 {
    let SizeIntent::Fixed(value) = value else {
        unreachable!("profile validation requires fixed dimensions");
    };
    *value
}
