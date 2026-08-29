use crate::profile::valid_name;
use crate::{
    AdapterError, CorrespondenceRecord, CorrespondenceTarget, FidelityEntry, ImportedSource,
    MAX_SOURCE_BYTES, MAX_TOKENS, PROFILE_NAME, RetentiveSource, SourceSpan,
};
use nuif_adapter::AdapterReport;
use nuif_codec::canonical_hash;
use nuif_core::{Document, EntityId, Fidelity, PropertyValue, Token};
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use serde_json::value::RawValue;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

/// Imports bounded, marked DTCG 2025.10 scalar-token JSON.
///
/// # Errors
///
/// Returns typed resource, JSON, marker, value or canonicalization errors.
pub fn import_source(source: &str) -> Result<ImportedSource, AdapterError> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(AdapterError::SourceTooLarge);
    }
    let root: RawObject<'_> = serde_json::from_str(source).map_err(json_error)?;
    let extensions = root
        .entries
        .iter()
        .find(|(name, _)| name == "$extensions")
        .ok_or_else(|| AdapterError::ProfileMarker("root $extensions is required".to_owned()))?;
    let root_extensions = parse_extensions(extensions.1)?;
    let root_metadata: RootMetadata<'_> =
        serde_json::from_str(root_extensions.nuif.get()).map_err(json_error)?;
    required_canonical_string(root_metadata.profile, PROFILE_NAME, "/profile")?;
    let document_id = parse_id_raw(root_metadata.document, "/id")?;
    let mut document = Document::empty(document_id);
    let mut correspondences = vec![record(
        root_metadata.document,
        CorrespondenceTarget::Document { id: document_id },
        "/id".to_owned(),
        source,
    )?];
    let mut fidelity = vec![FidelityEntry {
        target: CorrespondenceTarget::Document { id: document_id },
        pointer: String::new(),
        status: Fidelity::Lossless,
    }];
    let mut token_ids = BTreeSet::new();
    let mut cursor = 0;
    for (name, raw) in root.entries {
        let raw_span = span(source, raw)?;
        let key_span = key_span(source, cursor, raw_span.start, &name)?;
        cursor = raw_span.end;
        if name == "$extensions" {
            continue;
        }
        if document.tokens.len() == MAX_TOKENS {
            return invalid("/tokens", "token count exceeds the profile limit");
        }
        if !valid_name(&name) {
            return invalid(
                "/tokens",
                "token names cannot be empty, start with $, or contain dot or braces",
            );
        }
        let wire: TokenWire<'_> = serde_json::from_str(raw.get()).map_err(json_error)?;
        let extensions = parse_extensions(wire.extensions)?;
        let metadata: TokenMetadata<'_> =
            serde_json::from_str(extensions.nuif.get()).map_err(json_error)?;
        let id = parse_id_raw(metadata.id, "/tokens/id")?;
        if !token_ids.insert(id) {
            return invalid("/tokens", "token identity is duplicated");
        }
        let pointer = format!("/tokens/{id}");
        let value = parse_scalar(wire.kind, wire.value, metadata.value_kind, &pointer)?;
        let target = CorrespondenceTarget::Token { id };
        correspondences.extend([
            CorrespondenceRecord {
                target: target.clone(),
                pointer: format!("{pointer}/name"),
                span: key_span,
            },
            record(metadata.id, target.clone(), format!("{pointer}/id"), source)?,
            record(
                wire.kind,
                target.clone(),
                format!("{pointer}/value"),
                source,
            )?,
            record(
                wire.value,
                target.clone(),
                format!("{pointer}/value"),
                source,
            )?,
            record(
                metadata.value_kind,
                target.clone(),
                format!("{pointer}/value"),
                source,
            )?,
        ]);
        fidelity.push(FidelityEntry {
            target,
            pointer,
            status: Fidelity::Lossless,
        });
        document.tokens.insert(id, Token { id, name, value });
    }
    finish_import(source, document, correspondences, fidelity)
}

fn finish_import(
    source: &str,
    document: Document,
    mut correspondences: Vec<CorrespondenceRecord>,
    mut fidelity: Vec<FidelityEntry>,
) -> Result<ImportedSource, AdapterError> {
    correspondences.sort_by(|left, right| {
        (&left.target, &left.pointer, left.span).cmp(&(&right.target, &right.pointer, right.span))
    });
    fidelity
        .sort_by(|left, right| (&left.target, &left.pointer).cmp(&(&right.target, &right.pointer)));
    let report = AdapterReport {
        schema_version: 1,
        source_format: PROFILE_NAME.to_owned(),
        canonical_hash: Some(
            canonical_hash(&document)
                .map_err(|error| AdapterError::Canonical(error.to_string()))?,
        ),
        fidelity,
        correspondences,
        unmapped_source_preserved: true,
    };
    Ok(ImportedSource {
        document: document.clone(),
        retentive: RetentiveSource {
            source: source.to_owned(),
            document,
            report,
        },
    })
}

pub(crate) fn encoded_scalar(
    source: &str,
    record: &CorrespondenceRecord,
) -> Result<String, AdapterError> {
    source
        .get(record.span.start..record.span.end)
        .map(str::to_owned)
        .ok_or_else(|| AdapterError::StaleSpan {
            pointer: record.pointer.clone(),
        })
}

fn parse_scalar(
    kind: &RawValue,
    value: &RawValue,
    value_kind: &RawValue,
    pointer: &str,
) -> Result<PropertyValue, AdapterError> {
    let kind_value = canonical_string(kind, &format!("{pointer}/value"))?;
    let value_kind_value = canonical_string(value_kind, &format!("{pointer}/value"))?;
    let parsed = match (kind_value.as_str(), value_kind_value.as_str()) {
        ("boolean", "boolean") => PropertyValue::Boolean(
            serde_json::from_str(value.get()).map_err(|error| scalar_error(pointer, error))?,
        ),
        ("string", "string") => {
            let string: String = serde_json::from_str(value.get()).map_err(json_error)?;
            if string.starts_with('{') && string.ends_with('}') {
                return invalid(
                    &format!("{pointer}/value"),
                    "alias strings are outside the scalar value profile",
                );
            }
            PropertyValue::String(string)
        }
        ("number", "integer") => PropertyValue::Integer(
            serde_json::from_str(value.get()).map_err(|error| scalar_error(pointer, error))?,
        ),
        ("number", "real") => {
            let real: f64 =
                serde_json::from_str(value.get()).map_err(|error| scalar_error(pointer, error))?;
            if !real.is_finite() {
                return invalid(&format!("{pointer}/value"), "real value must be finite");
            }
            PropertyValue::Real(real)
        }
        _ => {
            return invalid(
                &format!("{pointer}/value"),
                "type and org.nuif value_kind must describe the same supported scalar",
            );
        }
    };
    let expected = canonical_value(&parsed)?;
    if value.get() != expected {
        return invalid(
            &format!("{pointer}/value"),
            "mapped scalar must use the profile canonical JSON spelling",
        );
    }
    Ok(parsed)
}

fn canonical_value(value: &PropertyValue) -> Result<String, AdapterError> {
    match value {
        PropertyValue::Boolean(value) => Ok(value.to_string()),
        PropertyValue::String(value) => serde_json::to_string(value).map_err(json_error),
        PropertyValue::Integer(value) => Ok(value.to_string()),
        PropertyValue::Real(value) => serde_json::to_string(value).map_err(json_error),
        _ => invalid("/tokens", "non-scalar value reached the scalar encoder"),
    }
}

fn parse_extensions(raw: &RawValue) -> Result<ExtensionsWire<'_>, AdapterError> {
    serde_json::from_str(raw.get()).map_err(json_error)
}

fn parse_id_raw(raw: &RawValue, pointer: &str) -> Result<EntityId, AdapterError> {
    let value = canonical_string(raw, pointer)?;
    EntityId::from_str(&value).map_err(|error| AdapterError::InvalidValue {
        pointer: pointer.to_owned(),
        reason: error.to_string(),
    })
}

fn required_canonical_string(
    raw: &RawValue,
    expected: &str,
    pointer: &str,
) -> Result<(), AdapterError> {
    let value = canonical_string(raw, pointer)?;
    if value == expected {
        Ok(())
    } else {
        Err(AdapterError::ProfileMarker(format!(
            "{pointer} must equal {expected}"
        )))
    }
}

fn canonical_string(raw: &RawValue, pointer: &str) -> Result<String, AdapterError> {
    let value: String = serde_json::from_str(raw.get()).map_err(json_error)?;
    let canonical = serde_json::to_string(&value).map_err(json_error)?;
    if raw.get() != canonical {
        return invalid(pointer, "mapped string must use canonical JSON escaping");
    }
    Ok(value)
}

fn record(
    raw: &RawValue,
    target: CorrespondenceTarget,
    pointer: String,
    source: &str,
) -> Result<CorrespondenceRecord, AdapterError> {
    Ok(CorrespondenceRecord {
        target,
        pointer,
        span: span(source, raw)?,
    })
}

fn span(source: &str, raw: &RawValue) -> Result<SourceSpan, AdapterError> {
    let source_start = source.as_ptr() as usize;
    let raw_start = raw.get().as_ptr() as usize;
    let Some(start) = raw_start.checked_sub(source_start) else {
        return invalid("/", "parser returned a raw value outside the source buffer");
    };
    let end = start.saturating_add(raw.get().len());
    if end > source.len() || !source.is_char_boundary(start) || !source.is_char_boundary(end) {
        return invalid("/", "parser returned an invalid UTF-8 byte span");
    }
    Ok(SourceSpan { start, end })
}

fn key_span(
    source: &str,
    search_start: usize,
    value_start: usize,
    name: &str,
) -> Result<SourceSpan, AdapterError> {
    let encoded = serde_json::to_string(name).map_err(json_error)?;
    let region =
        source
            .get(search_start..value_start)
            .ok_or_else(|| AdapterError::InvalidValue {
                pointer: "/tokens".to_owned(),
                reason: "token key search range is invalid".to_owned(),
            })?;
    let offset = region
        .rfind(&encoded)
        .ok_or_else(|| AdapterError::InvalidValue {
            pointer: "/tokens".to_owned(),
            reason: "token key must use canonical JSON escaping".to_owned(),
        })?;
    let start = search_start + offset;
    Ok(SourceSpan {
        start,
        end: start + encoded.len(),
    })
}

fn invalid<T>(pointer: &str, reason: &str) -> Result<T, AdapterError> {
    Err(AdapterError::InvalidValue {
        pointer: pointer.to_owned(),
        reason: reason.to_owned(),
    })
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "the function is a direct Result::map_err callback"
)]
fn json_error(error: serde_json::Error) -> AdapterError {
    AdapterError::JsonSyntax(error.to_string())
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "the function consumes errors supplied by Result::map_err"
)]
fn scalar_error(pointer: &str, error: serde_json::Error) -> AdapterError {
    AdapterError::InvalidValue {
        pointer: format!("{pointer}/value"),
        reason: error.to_string(),
    }
}

struct RawObject<'a> {
    entries: Vec<(String, &'a RawValue)>,
}

impl<'de> Deserialize<'de> for RawObject<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(RawObjectVisitor)
    }
}

struct RawObjectVisitor;

impl<'de> Visitor<'de> for RawObjectVisitor {
    type Value = RawObject<'de>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON object with unique member names")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut entries = Vec::new();
        let mut names = BTreeSet::new();
        while let Some((name, value)) = map.next_entry::<String, &'de RawValue>()? {
            if !names.insert(name.clone()) {
                return Err(serde::de::Error::custom(format!(
                    "duplicate object member {name}"
                )));
            }
            entries.push((name, value));
        }
        Ok(RawObject { entries })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TokenWire<'a> {
    #[serde(rename = "$type", borrow)]
    kind: &'a RawValue,
    #[serde(rename = "$value", borrow)]
    value: &'a RawValue,
    #[serde(rename = "$extensions", borrow)]
    extensions: &'a RawValue,
}

#[derive(Deserialize)]
struct ExtensionsWire<'a> {
    #[serde(rename = "org.nuif", borrow)]
    nuif: &'a RawValue,
    #[serde(flatten)]
    _unknown: BTreeMap<String, Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RootMetadata<'a> {
    #[serde(borrow)]
    profile: &'a RawValue,
    #[serde(borrow)]
    document: &'a RawValue,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TokenMetadata<'a> {
    #[serde(borrow)]
    id: &'a RawValue,
    #[serde(borrow)]
    value_kind: &'a RawValue,
}
