use crate::profile::profile_issues;
use crate::{AdapterError, ExportedSource, NUIF_EXTENSION, PROFILE_NAME, import_source};
use nuif_adapter::AdapterReport;
use nuif_codec::canonical_hash;
use nuif_core::{Document, PropertyValue};
use std::fmt::Write as _;

/// Exports a token-only document in the bounded DTCG scalar profile.
///
/// # Errors
///
/// Returns typed fidelity, serialization, parse or canonicalization errors.
pub fn export_document(document: &Document) -> Result<ExportedSource, AdapterError> {
    let issues = profile_issues(document);
    if !issues.is_empty() {
        return Err(AdapterError::UnsupportedProfile {
            issues: issues.len(),
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
    let mut output = String::from("{\n  \"$extensions\": {\n    ");
    write!(
        output,
        "{}: {{\"profile\": {}, \"document\": {}}}\n  }}",
        json_string(NUIF_EXTENSION)?,
        json_string(PROFILE_NAME)?,
        json_string(&document.id.to_string())?
    )
    .expect("writing to a string cannot fail");
    for token in document.tokens.values() {
        let (kind, value_kind, value) = scalar(&token.value)?;
        write!(
            output,
            ",\n  {}: {{\n    \"$type\": {},\n    \"$value\": {},\n    \"$extensions\": {{\n      {}: {{\"id\": {}, \"value_kind\": {}}}\n    }}\n  }}",
            json_string(&token.name)?,
            json_string(kind)?,
            value,
            json_string(NUIF_EXTENSION)?,
            json_string(&token.id.to_string())?,
            json_string(value_kind)?,
        )
        .expect("writing to a string cannot fail");
    }
    output.push_str("\n}\n");
    Ok(output)
}

fn scalar(value: &PropertyValue) -> Result<(&'static str, &'static str, String), AdapterError> {
    let (kind, value_kind) = match value {
        PropertyValue::Boolean(_) => ("boolean", "boolean"),
        PropertyValue::String(_) => ("string", "string"),
        PropertyValue::Integer(_) => ("number", "integer"),
        PropertyValue::Real(_) => ("number", "real"),
        _ => {
            return Err(AdapterError::InvalidValue {
                pointer: "/tokens".to_owned(),
                reason: "profile validation admitted a non-scalar token".to_owned(),
            });
        }
    };
    let encoded = match value {
        PropertyValue::Boolean(value) => value.to_string(),
        PropertyValue::String(value) => json_string(value)?,
        PropertyValue::Integer(value) => value.to_string(),
        PropertyValue::Real(value) => serde_json::to_string(value).map_err(json_error)?,
        _ => unreachable!("kind selection rejects non-scalar values"),
    };
    Ok((kind, value_kind, encoded))
}

fn json_string(value: &str) -> Result<String, AdapterError> {
    serde_json::to_string(value).map_err(json_error)
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "the function is a direct Result::map_err callback"
)]
fn json_error(error: serde_json::Error) -> AdapterError {
    AdapterError::JsonSyntax(error.to_string())
}
