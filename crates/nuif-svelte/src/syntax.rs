use nuif_core::EntityId;

use crate::AdapterError;

#[must_use]
pub fn number(value: f64) -> String {
    if value == 0.0 {
        return "0".to_owned();
    }
    let mut rendered = value.to_string();
    if rendered.ends_with(".0") {
        rendered.truncate(rendered.len() - 2);
    }
    rendered
}

#[must_use]
pub fn escape_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('{', "&#123;")
        .replace('}', "&#125;")
}

#[must_use]
pub fn escape_attribute(value: &str) -> String {
    escape_text(value).replace('"', "&quot;")
}

pub fn unescape(value: &str, pointer: &str) -> Result<String, AdapterError> {
    let mut output = String::with_capacity(value.len());
    let mut remainder = value;
    while let Some(index) = remainder.find('&') {
        output.push_str(&remainder[..index]);
        remainder = &remainder[index..];
        let (entity, decoded) = [
            ("&amp;", '&'),
            ("&lt;", '<'),
            ("&gt;", '>'),
            ("&quot;", '"'),
            ("&#123;", '{'),
            ("&#125;", '}'),
        ]
        .into_iter()
        .find(|(entity, _)| remainder.starts_with(entity))
        .ok_or_else(|| AdapterError::InvalidValue {
            pointer: pointer.to_owned(),
            reason: "profile accepts only its canonical Svelte entity escapes".to_owned(),
        })?;
        output.push(decoded);
        remainder = &remainder[entity.len()..];
    }
    output.push_str(remainder);
    Ok(output)
}

pub fn parse_id(value: &str, pointer: &str) -> Result<EntityId, AdapterError> {
    value
        .parse::<EntityId>()
        .map_err(|error| AdapterError::InvalidValue {
            pointer: pointer.to_owned(),
            reason: error.to_string(),
        })
}

pub fn parse_number(value: &str, pointer: &str) -> Result<f64, AdapterError> {
    let parsed = value
        .parse::<f64>()
        .map_err(|error| AdapterError::InvalidValue {
            pointer: pointer.to_owned(),
            reason: error.to_string(),
        })?;
    if !parsed.is_finite() {
        return Err(AdapterError::InvalidValue {
            pointer: pointer.to_owned(),
            reason: "number must be finite".to_owned(),
        });
    }
    Ok(parsed)
}
