#![doc = "Canonical text, deterministic CBOR and content hashes for NUIF."]

use ciborium::Value;
use nuif_core::{Document, Severity, validate};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::fmt::Write as _;
use std::io::Cursor;
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EncodingProfile {
    CanonicalTextV0,
    DeterministicCborV0,
}

pub trait Encoder {
    type Error;

    fn profile(&self) -> EncodingProfile;

    /// Encodes a semantic document using this encoder's declared profile.
    ///
    /// # Errors
    ///
    /// Returns an implementation-defined error when the document cannot be
    /// represented, validated, canonicalized, or written by the profile.
    fn encode(&self, document: &Document) -> Result<Vec<u8>, Self::Error>;
}

pub trait Decoder {
    type Error;

    fn profile(&self) -> EncodingProfile;

    /// Decodes bytes into a semantic document using this decoder's profile.
    ///
    /// # Errors
    ///
    /// Returns an implementation-defined error for malformed, unsupported,
    /// non-conforming, or resource-limit-exceeding input.
    fn decode(&self, bytes: &[u8]) -> Result<Document, Self::Error>;
}

pub trait Canonicalizer {
    type Error;

    /// Rewrites an encoded document into the profile's canonical byte form.
    ///
    /// # Errors
    ///
    /// Returns an implementation-defined error when the input cannot be
    /// decoded or canonical output cannot be produced.
    fn canonicalize(&self, bytes: &[u8]) -> Result<Vec<u8>, Self::Error>;
}

#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum CodecError {
    #[error("document is invalid: {0}")]
    InvalidDocument(String),
    #[error("input is malformed: {0}")]
    Malformed(String),
    #[error("input is valid but not canonical for the selected profile")]
    NonCanonical,
    #[error("numeric values must be finite")]
    NonFinite,
    #[error("document exceeds the profile input budget")]
    ResourceLimit,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CanonicalText;

impl Encoder for CanonicalText {
    type Error = CodecError;

    fn profile(&self) -> EncodingProfile {
        EncodingProfile::CanonicalTextV0
    }

    fn encode(&self, document: &Document) -> Result<Vec<u8>, Self::Error> {
        validate_for_encoding(document)?;
        let value = serde_json::to_value(document)
            .map_err(|error| CodecError::Malformed(error.to_string()))?;
        let mut output = String::new();
        write_text_value(&value, 0, &mut output)?;
        output.push('\n');
        Ok(output.into_bytes())
    }
}

impl Decoder for CanonicalText {
    type Error = CodecError;

    fn profile(&self) -> EncodingProfile {
        EncodingProfile::CanonicalTextV0
    }

    fn decode(&self, bytes: &[u8]) -> Result<Document, Self::Error> {
        decode_text(bytes, false)
    }
}

impl Canonicalizer for CanonicalText {
    type Error = CodecError;

    fn canonicalize(&self, bytes: &[u8]) -> Result<Vec<u8>, Self::Error> {
        let document = self.decode(bytes)?;
        self.encode(&document)
    }
}

impl CanonicalText {
    /// Parses the text surface without applying document invariants. This is
    /// intentionally limited to validators that need to report every model
    /// diagnostic; normal consumers should use [`Decoder::decode`].
    ///
    /// # Errors
    ///
    /// Returns a codec error when the byte budget, UTF-8, JSON/JSON5 syntax, or
    /// document shape is invalid.
    pub fn decode_for_validation(self, bytes: &[u8]) -> Result<Document, CodecError> {
        decode_text_document(bytes)
    }

    /// Decodes and requires the supplied text bytes to already be canonical.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError::NonCanonical`] when a decode/encode cycle changes
    /// any byte, and otherwise has the same errors as [`Decoder::decode`].
    pub fn decode_strict(self, bytes: &[u8]) -> Result<Document, CodecError> {
        decode_text(bytes, true)
    }
}

fn decode_text(bytes: &[u8], strict: bool) -> Result<Document, CodecError> {
    let document = decode_text_document(bytes)?;
    validate_for_encoding(&document)?;
    if strict && CanonicalText.encode(&document)? != bytes {
        return Err(CodecError::NonCanonical);
    }
    Ok(document)
}

fn decode_text_document(bytes: &[u8]) -> Result<Document, CodecError> {
    check_input_budget(bytes)?;
    let document: Document = match serde_json::from_slice(bytes) {
        Ok(document) => document,
        Err(json_error) => {
            let text = std::str::from_utf8(bytes)
                .map_err(|error| CodecError::Malformed(error.to_string()))?;
            json5::from_str(text).map_err(|json5_error| {
                CodecError::Malformed(format!("JSON: {json_error}; JSON5: {json5_error}"))
            })?
        }
    };
    Ok(document)
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DeterministicCbor;

impl Encoder for DeterministicCbor {
    type Error = CodecError;

    fn profile(&self) -> EncodingProfile {
        EncodingProfile::DeterministicCborV0
    }

    fn encode(&self, document: &Document) -> Result<Vec<u8>, Self::Error> {
        validate_for_encoding(document)?;
        let value = Value::serialized(document)
            .map_err(|error| CodecError::Malformed(error.to_string()))?;
        let value = canonical_value(value)?;
        encode_cbor_value(&value)
    }
}

impl Decoder for DeterministicCbor {
    type Error = CodecError;

    fn profile(&self) -> EncodingProfile {
        EncodingProfile::DeterministicCborV0
    }

    fn decode(&self, bytes: &[u8]) -> Result<Document, Self::Error> {
        check_input_budget(bytes)?;
        let value = decode_cbor_value(bytes)?;
        let canonical = canonical_value(value.clone())?;
        if encode_cbor_value(&canonical)? != bytes {
            return Err(CodecError::NonCanonical);
        }
        let document = value
            .deserialized()
            .map_err(|error| CodecError::Malformed(error.to_string()))?;
        validate_for_encoding(&document)?;
        Ok(document)
    }
}

impl Canonicalizer for DeterministicCbor {
    type Error = CodecError;

    fn canonicalize(&self, bytes: &[u8]) -> Result<Vec<u8>, Self::Error> {
        check_input_budget(bytes)?;
        let value = canonical_value(decode_cbor_value(bytes)?)?;
        let document = value
            .deserialized()
            .map_err(|error| CodecError::Malformed(error.to_string()))?;
        self.encode(&document)
    }
}

impl DeterministicCbor {
    /// Parses a canonical CBOR document without applying document invariants,
    /// for validators that need to return the complete diagnostic set.
    ///
    /// # Errors
    ///
    /// Returns a codec error for budget, malformed, trailing, or non-canonical
    /// CBOR and for data that cannot be deserialized as a NUIF document.
    pub fn decode_for_validation(self, bytes: &[u8]) -> Result<Document, CodecError> {
        check_input_budget(bytes)?;
        let value = decode_cbor_value(bytes)?;
        let canonical = canonical_value(value.clone())?;
        if encode_cbor_value(&canonical)? != bytes {
            return Err(CodecError::NonCanonical);
        }
        value
            .deserialized()
            .map_err(|error| CodecError::Malformed(error.to_string()))
    }
}

fn decode_cbor_value(bytes: &[u8]) -> Result<Value, CodecError> {
    let mut cursor = Cursor::new(bytes);
    let value = ciborium::from_reader(&mut cursor)
        .map_err(|error| CodecError::Malformed(error.to_string()))?;
    if cursor.position() != u64::try_from(bytes.len()).unwrap_or(u64::MAX) {
        return Err(CodecError::Malformed(
            "trailing bytes after the CBOR document".to_owned(),
        ));
    }
    Ok(value)
}

fn canonical_value(value: Value) -> Result<Value, CodecError> {
    match value {
        Value::Float(value) if !value.is_finite() => Err(CodecError::NonFinite),
        Value::Float(value) => Ok(Value::Float(if value == 0.0 { 0.0 } else { value })),
        Value::Array(values) => values
            .into_iter()
            .map(canonical_value)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        Value::Map(entries) => {
            let mut entries = entries
                .into_iter()
                .map(|(key, value)| Ok((canonical_value(key)?, canonical_value(value)?)))
                .collect::<Result<Vec<_>, CodecError>>()?;
            entries.sort_by(|(left, _), (right, _)| compare_encoded(left, right));
            for pair in entries.windows(2) {
                if compare_encoded(&pair[0].0, &pair[1].0) == Ordering::Equal {
                    return Err(CodecError::Malformed("duplicate CBOR map key".to_owned()));
                }
            }
            Ok(Value::Map(entries))
        }
        Value::Tag(_, _) => Err(CodecError::Malformed(
            "tags are not permitted in nuif-cbor-0".to_owned(),
        )),
        value => Ok(value),
    }
}

fn compare_encoded(left: &Value, right: &Value) -> Ordering {
    let left = encode_cbor_value(left).expect("in-memory CBOR key encoding cannot fail");
    let right = encode_cbor_value(right).expect("in-memory CBOR key encoding cannot fail");
    left.cmp(&right)
}

fn encode_cbor_value(value: &Value) -> Result<Vec<u8>, CodecError> {
    let mut bytes = Vec::new();
    ciborium::into_writer(value, &mut bytes)
        .map_err(|error| CodecError::Malformed(error.to_string()))?;
    Ok(bytes)
}

fn write_text_value(
    value: &serde_json::Value,
    depth: usize,
    output: &mut String,
) -> Result<(), CodecError> {
    match value {
        serde_json::Value::Null => output.push_str("null"),
        serde_json::Value::Bool(boolean) => {
            output.push_str(if *boolean { "true" } else { "false" });
        }
        serde_json::Value::Number(number) if number.is_f64() => {
            output.push_str(&format_text_real(
                number.as_f64().ok_or(CodecError::NonFinite)?,
            )?);
        }
        serde_json::Value::Number(number) => output.push_str(&number.to_string()),
        serde_json::Value::String(string) => output.push_str(
            &serde_json::to_string(string)
                .map_err(|error| CodecError::Malformed(error.to_string()))?,
        ),
        serde_json::Value::Array(values) => {
            if values.is_empty() {
                output.push_str("[]");
            } else {
                output.push_str("[\n");
                for (index, value) in values.iter().enumerate() {
                    write_indent(depth + 1, output);
                    write_text_value(value, depth + 1, output)?;
                    if index + 1 != values.len() {
                        output.push(',');
                    }
                    output.push('\n');
                }
                write_indent(depth, output);
                output.push(']');
            }
        }
        serde_json::Value::Object(values) => {
            if values.is_empty() {
                output.push_str("{}");
            } else {
                output.push_str("{\n");
                let mut entries = values.iter().collect::<Vec<_>>();
                entries.sort_by(|(left, _), (right, _)| left.as_bytes().cmp(right.as_bytes()));
                for (index, (key, value)) in entries.iter().enumerate() {
                    write_indent(depth + 1, output);
                    output.push_str(
                        &serde_json::to_string(key)
                            .map_err(|error| CodecError::Malformed(error.to_string()))?,
                    );
                    output.push_str(": ");
                    write_text_value(value, depth + 1, output)?;
                    if index + 1 != entries.len() {
                        output.push(',');
                    }
                    output.push('\n');
                }
                write_indent(depth, output);
                output.push('}');
            }
        }
    }
    Ok(())
}

fn write_indent(depth: usize, output: &mut String) {
    for _ in 0..depth {
        output.push_str("  ");
    }
}

fn format_text_real(value: f64) -> Result<String, CodecError> {
    if !value.is_finite() {
        return Err(CodecError::NonFinite);
    }
    if value == 0.0 {
        return Ok("0".to_owned());
    }
    let magnitude = value.abs();
    if (1e-6..1e21).contains(&magnitude) {
        return Ok(value.to_string());
    }
    let mut exponential = format!("{value:e}");
    let exponent = exponential
        .find('e')
        .expect("LowerExp formatting always includes an exponent marker");
    if !matches!(exponential.as_bytes().get(exponent + 1), Some(b'+' | b'-')) {
        exponential.insert(exponent + 1, '+');
    }
    Ok(exponential)
}

fn validate_for_encoding(document: &Document) -> Result<(), CodecError> {
    let codes = validate(document)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| diagnostic.code)
        .collect::<Vec<_>>();
    if codes.is_empty() {
        Ok(())
    } else {
        Err(CodecError::InvalidDocument(codes.join(",")))
    }
}

fn check_input_budget(bytes: &[u8]) -> Result<(), CodecError> {
    if bytes.len() > 64 * 1024 * 1024 {
        Err(CodecError::ResourceLimit)
    } else {
        Ok(())
    }
}

/// Computes the profile-qualified canonical document hash.
///
/// # Errors
///
/// Returns a codec error if the document is invalid or cannot be encoded.
pub fn canonical_hash(document: &Document) -> Result<String, CodecError> {
    let bytes = DeterministicCbor.encode(document)?;
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(64);
    for byte in digest {
        write!(hex, "{byte:02x}").expect("writing to a string cannot fail");
    }
    Ok(format!("nuif-cbor-0:sha256:{hex}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nuif_core::{Entity, EntityId, EntityKind, PropertyValue};

    fn document() -> Document {
        let mut document = Document::empty(EntityId::new(1));
        let entity = Entity::new(EntityId::new(2), EntityKind::Container);
        document.roots.push(entity.id);
        document.entities.insert(entity.id, entity);
        document
    }

    #[test]
    fn text_and_cbor_reach_fixpoints() {
        let document = document();
        let text = CanonicalText.encode(&document).unwrap();
        assert_eq!(CanonicalText.canonicalize(&text).unwrap(), text);
        assert_eq!(CanonicalText.decode_strict(&text).unwrap(), document);
        let cbor = DeterministicCbor.encode(&document).unwrap();
        assert_eq!(DeterministicCbor.canonicalize(&cbor).unwrap(), cbor);
        assert_eq!(DeterministicCbor.decode(&cbor).unwrap(), document);
    }

    #[test]
    fn encoded_key_order_is_not_utf8_order() {
        let value = Value::Map(vec![
            (Value::Text("aa".to_owned()), Value::Null),
            (Value::Text("z".to_owned()), Value::Null),
        ]);
        let encoded = encode_cbor_value(&canonical_value(value).unwrap()).unwrap();
        assert_eq!(encoded, [0xa2, 0x61, b'z', 0xf6, 0x62, b'a', b'a', 0xf6]);
    }

    #[test]
    fn integer_and_integral_real_remain_distinct() {
        let mut integer = document();
        integer
            .entities
            .get_mut(&EntityId::new(2))
            .unwrap()
            .authored
            .values
            .insert("value".to_owned(), PropertyValue::Integer(1));
        let mut real = integer.clone();
        real.entities
            .get_mut(&EntityId::new(2))
            .unwrap()
            .authored
            .values
            .insert("value".to_owned(), PropertyValue::Real(1.0));
        assert_ne!(
            DeterministicCbor.encode(&integer).unwrap(),
            DeterministicCbor.encode(&real).unwrap()
        );
        assert_ne!(
            canonical_hash(&integer).unwrap(),
            canonical_hash(&real).unwrap()
        );
    }

    #[test]
    fn strict_text_rejects_noncanonical_layout() {
        let text = serde_json::to_vec(&document()).unwrap();
        assert_eq!(
            CanonicalText.decode_strict(&text),
            Err(CodecError::NonCanonical)
        );
    }

    #[test]
    fn negative_real_zero_normalizes_without_becoming_an_integer() {
        let mut negative = document();
        negative
            .entities
            .get_mut(&EntityId::new(2))
            .unwrap()
            .authored
            .values
            .insert("value".to_owned(), PropertyValue::Real(-0.0));
        let mut positive = negative.clone();
        positive
            .entities
            .get_mut(&EntityId::new(2))
            .unwrap()
            .authored
            .values
            .insert("value".to_owned(), PropertyValue::Real(0.0));
        let mut integer = positive.clone();
        integer
            .entities
            .get_mut(&EntityId::new(2))
            .unwrap()
            .authored
            .values
            .insert("value".to_owned(), PropertyValue::Integer(0));

        let negative_bytes = DeterministicCbor.encode(&negative).unwrap();
        assert_eq!(negative_bytes, DeterministicCbor.encode(&positive).unwrap());
        assert_ne!(negative_bytes, DeterministicCbor.encode(&integer).unwrap());
        let decoded = DeterministicCbor.decode(&negative_bytes).unwrap();
        let PropertyValue::Real(value) =
            decoded.entities[&EntityId::new(2)].authored.values["value"]
        else {
            panic!("real zero changed logical numeric kind");
        };
        assert!(!value.is_sign_negative());
    }

    #[test]
    fn cbor_decoder_rejects_trailing_data() {
        let mut bytes = DeterministicCbor.encode(&document()).unwrap();
        bytes.push(0xf6);
        assert!(matches!(
            DeterministicCbor.decode(&bytes),
            Err(CodecError::Malformed(_))
        ));
    }

    #[test]
    fn text_real_spelling_obeys_profile_boundaries() {
        assert_eq!(format_text_real(-0.0).unwrap(), "0");
        assert_eq!(format_text_real(1.0).unwrap(), "1");
        assert_eq!(format_text_real(1e-6).unwrap(), "0.000001");
        assert_eq!(format_text_real(1e21).unwrap(), "1e+21");
        assert_eq!(format_text_real(5e-324).unwrap(), "5e-324");
    }

    #[test]
    fn lenient_text_accepts_comments_but_strict_text_rejects_them() {
        let canonical = String::from_utf8(CanonicalText.encode(&document()).unwrap()).unwrap();
        let commented = canonical.replacen('{', "{ // imported comment", 1);
        assert_eq!(
            CanonicalText.decode(commented.as_bytes()).unwrap(),
            document()
        );
        assert_eq!(
            CanonicalText.decode_strict(commented.as_bytes()),
            Err(CodecError::NonCanonical)
        );
    }

    #[test]
    fn cbor_reals_use_shortest_width_and_positive_zero() {
        assert_eq!(
            encode_cbor_value(&canonical_value(Value::Float(1.0)).unwrap()).unwrap(),
            [0xf9, 0x3c, 0x00]
        );
        assert_eq!(
            encode_cbor_value(&canonical_value(Value::Float(-0.0)).unwrap()).unwrap(),
            [0xf9, 0x00, 0x00]
        );
        let single = encode_cbor_value(&canonical_value(Value::Float(100_000.0)).unwrap()).unwrap();
        assert_eq!(single.first(), Some(&0xfa));
        let double =
            encode_cbor_value(&canonical_value(Value::Float(f64::MIN_POSITIVE)).unwrap()).unwrap();
        assert_eq!(double.first(), Some(&0xfb));
    }

    #[test]
    fn strict_cbor_rejects_a_widened_integral_real() {
        let mut document = document();
        document
            .entities
            .get_mut(&EntityId::new(2))
            .unwrap()
            .authored
            .values
            .insert("value".to_owned(), PropertyValue::Real(1.0));
        let canonical = DeterministicCbor.encode(&document).unwrap();
        let offset = canonical
            .windows(3)
            .position(|window| window == [0xf9, 0x3c, 0x00])
            .expect("fixture contains the half-precision encoding of real 1.0");
        let mut widened = canonical.clone();
        widened.splice(
            offset..offset + 3,
            [0xfb, 0x3f, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        );

        assert_eq!(
            DeterministicCbor.decode(&widened),
            Err(CodecError::NonCanonical)
        );
        assert_eq!(DeterministicCbor.canonicalize(&widened).unwrap(), canonical);
    }

    #[test]
    fn validation_parser_exposes_structurally_invalid_documents() {
        let mut invalid = document();
        let detached = Entity::new(EntityId::new(3), EntityKind::Container);
        invalid.entities.insert(detached.id, detached);
        let bytes = serde_json::to_vec(&invalid).unwrap();

        assert_eq!(
            CanonicalText.decode_for_validation(&bytes).unwrap(),
            invalid
        );
        assert!(matches!(
            CanonicalText.decode(&bytes),
            Err(CodecError::InvalidDocument(_))
        ));
    }
}
