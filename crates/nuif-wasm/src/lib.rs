#![doc = "Byte-oriented WebAssembly bindings over the authoritative NUIF core API."]

use nuif_api::Session;
use nuif_codec::{CanonicalText, DeterministicCbor, Encoder, MAX_INPUT_BYTES, canonical_hash};
use nuif_core::{Diagnostic, Document, Severity, validate};
use nuif_protocol::{Patch, PatchLimits, enforce_patch_limits};
use serde::Serialize;
use thiserror::Error;
use wasm_bindgen::prelude::*;

pub const API_PROFILE: &str = "nuif-wasm-api-0";
pub const MAX_PATCH_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_PATCH_TRANSACTIONS: usize = 1_024;
pub const MAX_PATCH_OPERATIONS: usize = 16_384;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DocumentEncoding {
    Text,
    Cbor,
}

impl DocumentEncoding {
    fn parse(value: &str) -> Result<Self, BindingError> {
        match value {
            "nuif-text-0" => Ok(Self::Text),
            "nuif-cbor-0" => Ok(Self::Cbor),
            _ => Err(BindingError::new(
                "NUIF_ENCODING_UNSUPPORTED",
                format!("unsupported encoding profile {value:?}"),
            )),
        }
    }

    fn decode_for_validation(self, bytes: &[u8]) -> Result<Document, BindingError> {
        match self {
            Self::Text => CanonicalText.decode_for_validation(bytes),
            Self::Cbor => DeterministicCbor.decode_for_validation(bytes),
        }
        .map_err(|error| BindingError::new("NUIF_DOCUMENT_DECODE_FAILED", error.to_string()))
    }

    fn encode(self, document: &Document) -> Result<Vec<u8>, BindingError> {
        match self {
            Self::Text => CanonicalText.encode(document),
            Self::Cbor => DeterministicCbor.encode(document),
        }
        .map_err(|error| BindingError::new("NUIF_DOCUMENT_ENCODE_FAILED", error.to_string()))
    }
}

#[derive(Debug, Error)]
#[error("{code}: {message}")]
struct BindingError {
    code: &'static str,
    message: String,
}

impl BindingError {
    fn new(code: &'static str, message: String) -> Self {
        Self { code, message }
    }
}

#[derive(Debug)]
struct CoreDocument {
    session: Session,
}

impl CoreDocument {
    fn load(bytes: &[u8], encoding: &str) -> Result<Self, BindingError> {
        if bytes.len() > MAX_INPUT_BYTES {
            return Err(BindingError::new(
                "NUIF_DOCUMENT_LIMIT_EXCEEDED",
                format!(
                    "document input exceeds {MAX_INPUT_BYTES} bytes (observed {})",
                    bytes.len()
                ),
            ));
        }
        let encoding = DocumentEncoding::parse(encoding)?;
        let document = encoding.decode_for_validation(bytes)?;
        Ok(Self {
            session: Session::new(document),
        })
    }

    fn validation_report(&self) -> ValidationReport {
        let diagnostics = validate(self.session.document());
        let errors = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Error)
            .count();
        ValidationReport {
            schema_version: 1,
            status: if errors == 0 { "passed" } else { "failed" },
            errors,
            diagnostics,
        }
    }

    fn export(&self, encoding: &str) -> Result<Vec<u8>, BindingError> {
        DocumentEncoding::parse(encoding)?.encode(self.session.document())
    }

    fn canonical_hash(&self) -> Result<String, BindingError> {
        canonical_hash(self.session.document())
            .map_err(|error| BindingError::new("NUIF_CANONICAL_HASH_FAILED", error.to_string()))
    }

    fn apply_patch(&mut self, bytes: &[u8]) -> Result<String, BindingError> {
        let patch = decode_patch(bytes)?;
        self.session
            .apply(&patch)
            .map_err(|error| BindingError::new("NUIF_PATCH_APPLY_FAILED", error.to_string()))?;
        self.canonical_hash()
    }

    fn undo(&mut self) -> Result<(String, Patch), BindingError> {
        let patch = self
            .session
            .undo()
            .map_err(|error| BindingError::new("NUIF_UNDO_FAILED", error.to_string()))?;
        Ok((self.canonical_hash()?, patch))
    }

    fn redo(&mut self) -> Result<(String, Patch), BindingError> {
        let patch = self
            .session
            .redo()
            .map_err(|error| BindingError::new("NUIF_REDO_FAILED", error.to_string()))?;
        Ok((self.canonical_hash()?, patch))
    }
}

#[derive(Debug, Serialize)]
struct ValidationReport {
    schema_version: u32,
    status: &'static str,
    errors: usize,
    diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Serialize)]
struct HistoryResult {
    canonical_hash: String,
    patch: Patch,
}

/// An in-memory NUIF document owned by the WebAssembly module.
///
/// The JavaScript boundary accepts and returns bytes. It never exposes a second
/// mutable object model that could diverge from `nuif-core`.
#[wasm_bindgen(js_name = NuifDocument)]
pub struct WasmDocument {
    inner: CoreDocument,
}

#[wasm_bindgen(js_class = NuifDocument)]
impl WasmDocument {
    /// Loads a bounded NUIF text or deterministic-CBOR document.
    ///
    /// # Errors
    ///
    /// Throws a coded JavaScript `Error` for an unknown encoding, malformed
    /// input or a declared resource limit.
    #[wasm_bindgen(constructor)]
    pub fn new(bytes: &[u8], encoding: &str) -> Result<WasmDocument, JsError> {
        CoreDocument::load(bytes, encoding)
            .map(|inner| Self { inner })
            .map_err(Into::into)
    }

    /// Serializes structural diagnostics without mutating the document.
    ///
    /// # Errors
    ///
    /// Throws only if the report cannot be represented as JSON.
    #[wasm_bindgen(js_name = validationReport)]
    pub fn validation_report(&self) -> Result<Vec<u8>, JsError> {
        serde_json::to_vec(&self.inner.validation_report())
            .map_err(|error| JsError::new(&format!("NUIF_REPORT_ENCODE_FAILED: {error}")))
    }

    /// Returns the canonical document hash.
    ///
    /// # Errors
    ///
    /// Throws when the loaded semantic document is invalid.
    #[wasm_bindgen(js_name = canonicalHash)]
    pub fn canonical_hash(&self) -> Result<String, JsError> {
        self.inner.canonical_hash().map_err(Into::into)
    }

    /// Encodes through the authoritative canonical codec.
    ///
    /// # Errors
    ///
    /// Throws for an unsupported encoding or invalid document.
    #[wasm_bindgen(js_name = exportBytes)]
    pub fn export_bytes(&self, encoding: &str) -> Result<Vec<u8>, JsError> {
        self.inner.export(encoding).map_err(Into::into)
    }

    /// Applies a bounded semantic patch atomically and returns the new hash.
    ///
    /// # Errors
    ///
    /// Throws for malformed, excessive, stale or semantically invalid patches.
    #[wasm_bindgen(js_name = applyPatch)]
    pub fn apply_patch(&mut self, bytes: &[u8]) -> Result<String, JsError> {
        self.inner.apply_patch(bytes).map_err(Into::into)
    }

    #[wasm_bindgen(js_name = canUndo)]
    #[must_use]
    pub fn can_undo(&self) -> bool {
        self.inner.session.can_undo()
    }

    #[wasm_bindgen(js_name = canRedo)]
    #[must_use]
    pub fn can_redo(&self) -> bool {
        self.inner.session.can_redo()
    }

    /// Undoes one patch and returns its new hash and replayable patch as JSON.
    ///
    /// # Errors
    ///
    /// Throws when history is empty or its exact precondition is stale.
    pub fn undo(&mut self) -> Result<Vec<u8>, JsError> {
        history_json(self.inner.undo().map_err(JsError::from)?)
    }

    /// Redoes one patch and returns its new hash and replayable patch as JSON.
    ///
    /// # Errors
    ///
    /// Throws when history is empty or its exact precondition is stale.
    pub fn redo(&mut self) -> Result<Vec<u8>, JsError> {
        history_json(self.inner.redo().map_err(JsError::from)?)
    }

    #[wasm_bindgen(getter, js_name = entityCount)]
    #[must_use]
    pub fn entity_count(&self) -> usize {
        self.inner.session.document().entities.len()
    }
}

#[wasm_bindgen(js_name = apiVersion)]
#[must_use]
pub fn api_version() -> String {
    format!("{API_PROFILE}/{}", env!("CARGO_PKG_VERSION"))
}

#[wasm_bindgen]
/// Returns the static byte-oriented binding contract.
///
/// # Errors
///
/// Throws only if the static record cannot be represented as JSON.
pub fn capabilities() -> Result<Vec<u8>, JsError> {
    serde_json::to_vec(&serde_json::json!({
        "schema_version": 1,
        "api_profile": API_PROFILE,
        "binding_version": env!("CARGO_PKG_VERSION"),
        "encodings": ["nuif-text-0", "nuif-cbor-0"],
        "operations": ["load", "validate", "canonical_hash", "export", "apply_patch", "undo", "redo"],
        "limits": {
            "document_bytes": MAX_INPUT_BYTES,
            "patch_bytes": MAX_PATCH_BYTES,
            "patch_transactions": MAX_PATCH_TRANSACTIONS,
            "patch_operations": MAX_PATCH_OPERATIONS,
        },
        "authorities": [],
    }))
    .map_err(|error| JsError::new(&format!("NUIF_REPORT_ENCODE_FAILED: {error}")))
}

fn decode_patch(bytes: &[u8]) -> Result<Patch, BindingError> {
    if bytes.len() > MAX_PATCH_BYTES {
        return Err(BindingError::new(
            "NUIF_PATCH_LIMIT_EXCEEDED",
            format!(
                "patch input exceeds {MAX_PATCH_BYTES} bytes (observed {})",
                bytes.len()
            ),
        ));
    }
    let patch: Patch = serde_json::from_slice(bytes)
        .map_err(|error| BindingError::new("NUIF_PATCH_DECODE_FAILED", error.to_string()))?;
    enforce_patch_limits(
        &patch,
        PatchLimits {
            transactions: MAX_PATCH_TRANSACTIONS,
            operations: MAX_PATCH_OPERATIONS,
        },
    )
    .map_err(|error| BindingError::new("NUIF_PATCH_LIMIT_EXCEEDED", error.to_string()))?;
    Ok(patch)
}

fn history_json((canonical_hash, patch): (String, Patch)) -> Result<Vec<u8>, JsError> {
    serde_json::to_vec(&HistoryResult {
        canonical_hash,
        patch,
    })
    .map_err(|error| JsError::new(&format!("NUIF_REPORT_ENCODE_FAILED: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nuif_core::{Entity, EntityId, EntityKind};
    use nuif_protocol::{Operation, Transaction};

    fn document() -> Document {
        let mut document = Document::empty(EntityId::new(1));
        let root = Entity::new(EntityId::new(2), EntityKind::Container);
        document.roots.push(root.id);
        document.entities.insert(root.id, root);
        document
    }

    fn rename_patch(base_revision: Option<&str>) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "base_revision": base_revision,
            "transactions": [{
                "id": 1,
                "operations": [{
                    "op": "rename",
                    "entity": "00000000000000000000000000000002",
                    "name": "WASM edited"
                }]
            }]
        }))
        .unwrap()
    }

    #[test]
    fn byte_surface_matches_core_patch_and_history() {
        let input = CanonicalText.encode(&document()).unwrap();
        let mut bound = CoreDocument::load(&input, "nuif-text-0").unwrap();
        let before = bound.canonical_hash().unwrap();
        let after = bound
            .apply_patch(&rename_patch(Some(before.as_str())))
            .unwrap();
        assert_ne!(after, before);
        assert_eq!(bound.validation_report().errors, 0);
        let cbor = bound.export("nuif-cbor-0").unwrap();
        assert_eq!(
            CoreDocument::load(&cbor, "nuif-cbor-0")
                .unwrap()
                .canonical_hash()
                .unwrap(),
            after
        );
        assert_eq!(bound.undo().unwrap().0, before);
        assert_eq!(bound.redo().unwrap().0, after);
    }

    #[test]
    fn invalid_document_can_be_inspected_but_not_exported() {
        let mut invalid = document();
        invalid.roots.push(EntityId::new(99));
        let bytes = serde_json::to_vec(&invalid).unwrap();
        let bound = CoreDocument::load(&bytes, "nuif-text-0").unwrap();
        assert!(bound.validation_report().errors > 0);
        assert!(bound.export("nuif-text-0").is_err());
    }

    #[test]
    fn byte_and_cardinality_limits_fail_before_mutation() {
        assert!(matches!(
            CoreDocument::load(&vec![b' '; MAX_INPUT_BYTES + 1], "nuif-text-0"),
            Err(BindingError {
                code: "NUIF_DOCUMENT_LIMIT_EXCEEDED",
                ..
            })
        ));
        assert!(matches!(
            decode_patch(&vec![b' '; MAX_PATCH_BYTES + 1]),
            Err(BindingError {
                code: "NUIF_PATCH_LIMIT_EXCEEDED",
                ..
            })
        ));
        let transactions = (0..=MAX_PATCH_TRANSACTIONS)
            .map(|id| serde_json::json!({"id": id, "operations": []}))
            .collect::<Vec<_>>();
        let bytes = serde_json::to_vec(&serde_json::json!({
            "base_revision": null,
            "transactions": transactions,
        }))
        .unwrap();
        assert!(matches!(
            decode_patch(&bytes),
            Err(BindingError {
                code: "NUIF_PATCH_LIMIT_EXCEEDED",
                ..
            })
        ));

        let operations = (0..=MAX_PATCH_OPERATIONS)
            .map(|_| Operation::Rename {
                entity: EntityId::new(2),
                name: Some("bounded".to_owned()),
            })
            .collect();
        let bytes = serde_json::to_vec(&Patch {
            base_revision: None,
            transactions: vec![Transaction { id: 1, operations }],
        })
        .unwrap();
        assert!(bytes.len() < MAX_PATCH_BYTES);
        assert!(matches!(
            decode_patch(&bytes),
            Err(BindingError {
                code: "NUIF_PATCH_LIMIT_EXCEEDED",
                ..
            })
        ));
    }
}
