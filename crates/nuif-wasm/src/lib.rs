#![doc = "Byte-oriented WebAssembly bindings over the authoritative NUIF core API."]

use nuif_api::{DocumentEncoding, EngineError, NuifDocument, profile_zero_context};
use nuif_codec::MAX_INPUT_BYTES;
use nuif_core::{Diagnostic, Severity, is_identifier};
use nuif_font::OPENTYPE_VARIABLE_TRUETYPE_PROFILE;
use nuif_package::{
    MAX_CAPABILITY_BYTES, MAX_PACKAGE_BYTES, MAX_REQUIRED_CAPABILITIES, PackageCapabilityReport,
    PackageMode,
};
use nuif_protocol::{Patch, PatchLimits, enforce_patch_limits};
use serde::Serialize;
use std::collections::BTreeSet;
use thiserror::Error;
use wasm_bindgen::prelude::*;

pub const API_PROFILE: &str = "nuif-wasm-api-0";
pub const MAX_PATCH_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_PATCH_TRANSACTIONS: usize = 1_024;
pub const MAX_PATCH_OPERATIONS: usize = 16_384;
pub const MAX_CAPABILITY_SET_BYTES: usize = 64 * 1024;

fn document_encoding(value: &str) -> Result<DocumentEncoding, BindingError> {
    DocumentEncoding::from_profile(value)
        .map_err(|error| BindingError::new("NUIF_ENCODING_UNSUPPORTED", error.to_string()))
}

fn package_mode(value: &str) -> Result<PackageMode, BindingError> {
    match value {
        "portable" => Ok(PackageMode::Portable),
        "authoring" => Ok(PackageMode::Authoring),
        _ => Err(BindingError::new(
            "NUIF_PACKAGE_MODE_UNSUPPORTED",
            format!("unsupported package mode {value:?}"),
        )),
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

fn engine_binding_error(error: &EngineError, fallback_code: &'static str) -> BindingError {
    let code = if matches!(error, EngineError::PackageCapabilitiesRequired { .. }) {
        "NUIF_PACKAGE_CAPABILITIES_REQUIRED"
    } else {
        fallback_code
    };
    BindingError::new(code, error.to_string())
}

#[derive(Debug)]
struct CoreDocument {
    document: NuifDocument,
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
        let encoding = document_encoding(encoding)?;
        let document = NuifDocument::load(bytes, encoding)
            .map_err(|error| BindingError::new("NUIF_DOCUMENT_DECODE_FAILED", error.to_string()))?;
        Ok(Self { document })
    }

    fn load_package(bytes: &[u8]) -> Result<Self, BindingError> {
        if bytes.len() > MAX_PACKAGE_BYTES {
            return Err(BindingError::new(
                "NUIF_PACKAGE_LIMIT_EXCEEDED",
                format!(
                    "package input exceeds {MAX_PACKAGE_BYTES} bytes (observed {})",
                    bytes.len()
                ),
            ));
        }
        let document = NuifDocument::load_package(bytes)
            .map_err(|error| BindingError::new("NUIF_PACKAGE_DECODE_FAILED", error.to_string()))?;
        Ok(Self { document })
    }

    fn load_package_with_capabilities(
        bytes: &[u8],
        supported: &BTreeSet<String>,
    ) -> Result<Self, BindingError> {
        let mut document = Self::load_package(bytes)?;
        document.require_package_capabilities(supported)?;
        Ok(document)
    }

    fn validation_report(&self) -> Result<ValidationReport, BindingError> {
        let diagnostics = self
            .document
            .validate()
            .map_err(|error| BindingError::new("NUIF_VALIDATE_FAILED", error.to_string()))?
            .diagnostics;
        let errors = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Error)
            .count();
        Ok(ValidationReport {
            schema_version: 1,
            status: if errors == 0 { "passed" } else { "failed" },
            errors,
            diagnostics,
        })
    }

    fn export(&self, encoding: &str) -> Result<Vec<u8>, BindingError> {
        self.document
            .export(document_encoding(encoding)?)
            .map_err(|error| BindingError::new("NUIF_DOCUMENT_ENCODE_FAILED", error.to_string()))
    }

    fn export_package(&self, mode: &str) -> Result<Vec<u8>, BindingError> {
        self.document
            .export_package(package_mode(mode)?)
            .map_err(|error| engine_binding_error(&error, "NUIF_PACKAGE_ENCODE_FAILED"))
    }

    fn package_capability_report(
        &self,
        supported: &BTreeSet<String>,
    ) -> Option<PackageCapabilityReport> {
        self.document.package_capability_report(supported)
    }

    fn require_package_capabilities(
        &mut self,
        supported: &BTreeSet<String>,
    ) -> Result<(), BindingError> {
        self.document
            .require_package_capabilities(supported)
            .map_err(|error| {
                BindingError::new("NUIF_PACKAGE_CAPABILITIES_UNAVAILABLE", error.to_string())
            })
    }

    fn canonical_hash(&self) -> Result<String, BindingError> {
        self.document
            .canonical_hash()
            .map_err(|error| BindingError::new("NUIF_CANONICAL_HASH_FAILED", error.to_string()))
    }

    fn snapshot_report(&self, width: f64, height: f64) -> Result<Vec<u8>, BindingError> {
        let snapshot = self
            .document
            .snapshot(&profile_zero_context(width, height))
            .map_err(|error| engine_binding_error(&error, "NUIF_SNAPSHOT_FAILED"))?;
        serde_json::to_vec(&snapshot.report())
            .map_err(|error| BindingError::new("NUIF_REPORT_ENCODE_FAILED", error.to_string()))
    }

    fn apply_patch(&mut self, bytes: &[u8]) -> Result<String, BindingError> {
        let patch = decode_patch(bytes)?;
        self.document
            .apply_patch(&patch)
            .map_err(|error| engine_binding_error(&error, "NUIF_PATCH_APPLY_FAILED"))?;
        self.canonical_hash()
    }

    fn undo(&mut self) -> Result<(String, Patch), BindingError> {
        let patch = self
            .document
            .undo()
            .map_err(|error| engine_binding_error(&error, "NUIF_UNDO_FAILED"))?;
        Ok((self.canonical_hash()?, patch))
    }

    fn redo(&mut self) -> Result<(String, Patch), BindingError> {
        let patch = self
            .document
            .redo()
            .map_err(|error| engine_binding_error(&error, "NUIF_REDO_FAILED"))?;
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

    /// Structurally loads a bounded, deterministic `.nuif` package while
    /// retaining its verified resources and manifest requirements.
    ///
    /// Structural loading is appropriate for inert inspection and migration.
    /// A host that evaluates the package must additionally call
    /// `requirePackageCapabilities` or use `fromPackageWithCapabilities`.
    ///
    /// # Errors
    ///
    /// Throws for malformed, excessive, non-canonical or policy-invalid
    /// packages. It does not fetch linked resources or execute capabilities.
    #[wasm_bindgen(js_name = fromPackage)]
    pub fn from_package(bytes: &[u8]) -> Result<WasmDocument, JsError> {
        CoreDocument::load_package(bytes)
            .map(|inner| Self { inner })
            .map_err(Into::into)
    }

    /// Loads a package only when every manifest requirement is present in the
    /// supplied bounded JSON string array.
    ///
    /// # Errors
    ///
    /// Throws for an invalid capability-set transport, a structurally invalid
    /// package, or the exact set of unavailable required capabilities.
    #[wasm_bindgen(js_name = fromPackageWithCapabilities)]
    pub fn from_package_with_capabilities(
        bytes: &[u8],
        supported: &[u8],
    ) -> Result<WasmDocument, JsError> {
        let supported = decode_capability_set(supported).map_err(JsError::from)?;
        CoreDocument::load_package_with_capabilities(bytes, &supported)
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
        serde_json::to_vec(&self.inner.validation_report().map_err(JsError::from)?)
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

    /// Encodes a deterministic package while preserving loaded descriptors,
    /// embedded bytes and required capabilities.
    ///
    /// # Errors
    ///
    /// Throws for an unsupported mode or any package-policy violation.
    #[wasm_bindgen(js_name = exportPackage)]
    pub fn export_package(&self, mode: &str) -> Result<Vec<u8>, JsError> {
        self.inner.export_package(mode).map_err(Into::into)
    }

    /// Evaluates the loaded document or authorized package through the shared
    /// layout/render runtime and returns the transport-neutral snapshot report.
    ///
    /// Raster bytes remain outside the JSON report; `rgba_sha256` commits the
    /// report to the exact reference pixels.
    ///
    /// # Errors
    ///
    /// Throws for unauthorized package capabilities, invalid viewport values,
    /// document errors, unresolved resources, or render failures.
    #[wasm_bindgen(js_name = snapshotReport)]
    pub fn snapshot_report(&self, width: f64, height: f64) -> Result<Vec<u8>, JsError> {
        self.inner
            .snapshot_report(width, height)
            .map_err(Into::into)
    }

    /// Reports exact required, supported-required and missing-required sets as
    /// JSON. A bare document returns JSON `null`.
    ///
    /// # Errors
    ///
    /// Throws for malformed or excessive capability-set transport, or if the
    /// report cannot be represented as JSON.
    #[wasm_bindgen(js_name = packageCapabilityReport)]
    pub fn package_capability_report(&self, supported: &[u8]) -> Result<Vec<u8>, JsError> {
        let supported = decode_capability_set(supported).map_err(JsError::from)?;
        serde_json::to_vec(&self.inner.package_capability_report(&supported))
            .map_err(|error| JsError::new(&format!("NUIF_REPORT_ENCODE_FAILED: {error}")))
    }

    /// Fails unless the loaded package's complete required-capability set is
    /// present in the supplied bounded JSON string array. Bare documents have
    /// no package requirements.
    ///
    /// # Errors
    ///
    /// Throws for malformed transport or unavailable requirements.
    #[wasm_bindgen(js_name = requirePackageCapabilities)]
    pub fn require_package_capabilities(&mut self, supported: &[u8]) -> Result<(), JsError> {
        let supported = decode_capability_set(supported).map_err(JsError::from)?;
        self.inner
            .require_package_capabilities(&supported)
            .map_err(Into::into)
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
        self.inner.document.can_undo()
    }

    #[wasm_bindgen(js_name = canRedo)]
    #[must_use]
    pub fn can_redo(&self) -> bool {
        self.inner.document.can_redo()
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
        self.inner.document.document().entities.len()
    }

    #[wasm_bindgen(getter, js_name = packageMode)]
    #[must_use]
    pub fn package_mode(&self) -> Option<String> {
        self.inner.document.package_mode().map(|mode| match mode {
            PackageMode::Portable => "portable".to_owned(),
            PackageMode::Authoring => "authoring".to_owned(),
        })
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
        "containers": ["nuif-text-0", "nuif-cbor-0", "nuif-package-0"],
        "package_modes": ["portable", "authoring"],
        "package_capabilities": [OPENTYPE_VARIABLE_TRUETYPE_PROFILE],
        "operations": ["load", "load_package", "validate", "canonical_hash", "export", "export_package", "package_capability_report", "require_package_capabilities", "snapshot_report", "apply_patch", "undo", "redo"],
        "limits": {
            "document_bytes": MAX_INPUT_BYTES,
            "package_bytes": MAX_PACKAGE_BYTES,
            "capability_set_bytes": MAX_CAPABILITY_SET_BYTES,
            "required_capabilities": MAX_REQUIRED_CAPABILITIES,
            "capability_bytes": MAX_CAPABILITY_BYTES,
            "patch_bytes": MAX_PATCH_BYTES,
            "patch_transactions": MAX_PATCH_TRANSACTIONS,
            "patch_operations": MAX_PATCH_OPERATIONS,
        },
        "authorities": [],
    }))
    .map_err(|error| JsError::new(&format!("NUIF_REPORT_ENCODE_FAILED: {error}")))
}

fn decode_capability_set(bytes: &[u8]) -> Result<BTreeSet<String>, BindingError> {
    if bytes.len() > MAX_CAPABILITY_SET_BYTES {
        return Err(BindingError::new(
            "NUIF_CAPABILITY_SET_LIMIT_EXCEEDED",
            format!(
                "capability-set input exceeds {MAX_CAPABILITY_SET_BYTES} bytes (observed {})",
                bytes.len()
            ),
        ));
    }
    let capabilities: BTreeSet<String> = serde_json::from_slice(bytes).map_err(|error| {
        BindingError::new("NUIF_CAPABILITY_SET_DECODE_FAILED", error.to_string())
    })?;
    if capabilities.len() > MAX_REQUIRED_CAPABILITIES {
        return Err(BindingError::new(
            "NUIF_CAPABILITY_SET_LIMIT_EXCEEDED",
            format!(
                "capability set exceeds {MAX_REQUIRED_CAPABILITIES} entries (observed {})",
                capabilities.len()
            ),
        ));
    }
    if let Some(capability) = capabilities
        .iter()
        .find(|capability| capability.len() > MAX_CAPABILITY_BYTES || !is_identifier(capability))
    {
        return Err(BindingError::new(
            "NUIF_CAPABILITY_SET_INVALID",
            format!("capability {capability:?} is not a bounded identifier"),
        ));
    }
    Ok(capabilities)
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
    use nuif_codec::{CanonicalText, Encoder};
    use nuif_core::{Document, Entity, EntityId, EntityKind};
    use nuif_package::NuifPackage;
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
        assert_eq!(bound.validation_report().unwrap().errors, 0);
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
        assert!(bound.validation_report().unwrap().errors > 0);
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

    #[test]
    fn package_surface_preserves_bytes_and_negotiates_capabilities() {
        let required = BTreeSet::from(["feature.example".to_owned()]);
        let mut package = NuifPackage::new(document(), PackageMode::Portable);
        package.required_capabilities.clone_from(&required);
        let bytes = package.encode().unwrap();

        let mut structural = CoreDocument::load_package(&bytes).unwrap();
        assert_eq!(structural.export_package("portable").unwrap(), bytes);
        let unsupported = structural
            .package_capability_report(&BTreeSet::new())
            .unwrap();
        assert!(!unsupported.fully_supported);
        assert_eq!(unsupported.missing_required, required);
        assert!(matches!(
            structural.require_package_capabilities(&BTreeSet::new()),
            Err(BindingError {
                code: "NUIF_PACKAGE_CAPABILITIES_UNAVAILABLE",
                ..
            })
        ));
        assert!(matches!(
            structural.apply_patch(&rename_patch(None)),
            Err(BindingError {
                code: "NUIF_PACKAGE_CAPABILITIES_REQUIRED",
                ..
            })
        ));
        assert!(matches!(
            structural.snapshot_report(10.0, 12.0),
            Err(BindingError {
                code: "NUIF_PACKAGE_CAPABILITIES_REQUIRED",
                ..
            })
        ));
        structural.require_package_capabilities(&required).unwrap();
        let report: nuif_api::SnapshotReport =
            serde_json::from_slice(&structural.snapshot_report(10.0, 12.0).unwrap()).unwrap();
        assert_eq!(report.raster.width, 10);
        assert_eq!(report.raster.height, 12);
        assert!(structural.apply_patch(&rename_patch(None)).is_ok());
        assert!(CoreDocument::load_package_with_capabilities(&bytes, &required).is_ok());
        assert!(CoreDocument::load_package_with_capabilities(&bytes, &BTreeSet::new()).is_err());
    }

    #[test]
    fn capability_transport_and_package_limits_are_typed() {
        assert_eq!(
            decode_capability_set(br#"["feature.z","feature.a"]"#).unwrap(),
            BTreeSet::from(["feature.a".to_owned(), "feature.z".to_owned()])
        );
        assert!(matches!(
            decode_capability_set(br#"["not valid"]"#),
            Err(BindingError {
                code: "NUIF_CAPABILITY_SET_INVALID",
                ..
            })
        ));
        assert!(matches!(
            decode_capability_set(&vec![b' '; MAX_CAPABILITY_SET_BYTES + 1]),
            Err(BindingError {
                code: "NUIF_CAPABILITY_SET_LIMIT_EXCEEDED",
                ..
            })
        ));
        assert!(matches!(
            CoreDocument::load_package(&vec![0; MAX_PACKAGE_BYTES + 1]),
            Err(BindingError {
                code: "NUIF_PACKAGE_LIMIT_EXCEEDED",
                ..
            })
        ));
    }
}
