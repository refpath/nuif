#![doc = "Stable headless engine and editor-session contract shared by every client."]

use nuif_codec::{CanonicalText, CodecError, DeterministicCbor, Encoder, canonical_hash};
use nuif_core::{Diagnostic, Document, EntityId, ResourceDigest, Severity, validate};
use nuif_layout::{EvaluationContext, LayoutSnapshot, evaluate};
use nuif_package::{NuifPackage, PackageCapabilityReport, PackageError, PackageMode};
use nuif_protocol::{ApplyError, Operation, Patch, Transaction, apply_patch_with_inverse};
use nuif_render::{
    RasterImage, RenderError, RenderScene, RenderTarget, build_scene, build_scene_with_resources,
    render_cpu,
};
use nuif_text::PINNED_FONT_SHA256;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use thiserror::Error;

pub const MAX_SESSION_RESOURCES: usize = 8_192;
pub const MAX_SESSION_RESOURCE_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_SESSION_TOTAL_RESOURCE_BYTES: usize = 64 * 1024 * 1024;
pub type SessionResources = BTreeMap<ResourceDigest, Arc<[u8]>>;

/// A canonical bare-document encoding accepted by every in-process client.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentEncoding {
    CanonicalText,
    DeterministicCbor,
}

impl DocumentEncoding {
    #[must_use]
    pub const fn profile(self) -> &'static str {
        match self {
            Self::CanonicalText => "nuif-text-0",
            Self::DeterministicCbor => "nuif-cbor-0",
        }
    }

    /// Resolves a public encoding profile without guessing from input bytes.
    ///
    /// # Errors
    ///
    /// Rejects every profile outside the two canonical bare encodings.
    pub fn from_profile(profile: &str) -> Result<Self, EngineError> {
        match profile {
            "nuif-text-0" => Ok(Self::CanonicalText),
            "nuif-cbor-0" => Ok(Self::DeterministicCbor),
            _ => Err(EngineError::EncodingUnsupported {
                profile: profile.to_owned(),
            }),
        }
    }

    fn decode_for_validation(self, bytes: &[u8]) -> Result<Document, EngineError> {
        match self {
            Self::CanonicalText => CanonicalText.decode_for_validation(bytes),
            Self::DeterministicCbor => DeterministicCbor.decode_for_validation(bytes),
        }
        .map_err(Into::into)
    }

    fn encode(self, document: &Document) -> Result<Vec<u8>, EngineError> {
        match self {
            Self::CanonicalText => CanonicalText.encode(document),
            Self::DeterministicCbor => DeterministicCbor.encode(document),
        }
        .map_err(Into::into)
    }
}

const CAPABILITIES: &[Capability] = &[
    Capability::Inspect,
    Capability::Query,
    Capability::Validate,
    Capability::Canonicalize,
    Capability::Diff,
    Capability::Patch,
    Capability::Layout,
    Capability::Render,
    Capability::Snapshot,
    Capability::Replay,
    Capability::Migrate,
    Capability::Import,
    Capability::Export,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Inspect,
    Query,
    Validate,
    Canonicalize,
    Diff,
    Patch,
    Layout,
    Render,
    Snapshot,
    Replay,
    Migrate,
    Import,
    Export,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationReport {
    pub diagnostics: Vec<Diagnostic>,
}

pub trait Engine {
    type Error;

    fn capabilities(&self) -> &[Capability];

    /// Validates a document against the engine's supported NUIF profile.
    ///
    /// # Errors
    ///
    /// Returns an implementation-defined error when validation itself cannot
    /// be completed. Conformance diagnostics belong in the returned report.
    fn validate(&self, document: &Document) -> Result<ValidationReport, Self::Error>;

    /// Applies a semantic patch to a document.
    ///
    /// # Errors
    ///
    /// Returns an implementation-defined error when preconditions fail, the
    /// patch is invalid, or a required operation cannot be applied.
    fn apply(&mut self, document: &mut Document, patch: &Patch) -> Result<(), Self::Error>;

    /// Evaluates authored layout under an explicit context.
    ///
    /// # Errors
    ///
    /// Returns an implementation-defined error when required layout semantics
    /// are unsupported or evaluation cannot complete safely.
    fn layout(
        &self,
        document: &Document,
        context: &EvaluationContext,
    ) -> Result<LayoutSnapshot, Self::Error>;

    /// Lowers a semantic document and resolved layout into a render scene.
    ///
    /// # Errors
    ///
    /// Returns an implementation-defined error when required visual semantics
    /// cannot be lowered or scene construction exceeds resource limits.
    fn build_render_scene(
        &self,
        document: &Document,
        layout: &LayoutSnapshot,
        context: &EvaluationContext,
    ) -> Result<RenderScene, Self::Error>;

    fn render_target_supported(&self, target: RenderTarget) -> bool;
}

#[derive(Clone, Debug, Default)]
pub struct ReferenceEngine;

/// Creates a profile-0 evaluation context with the built-in content-addressed
/// font resource registered.
#[must_use]
pub fn profile_zero_context(width: f64, height: f64) -> EvaluationContext {
    let mut context = EvaluationContext::viewport(width, height);
    context.font_hashes.insert(PINNED_FONT_SHA256.to_owned());
    context
}

/// Package-aware, byte-oriented façade over one authoritative session.
///
/// Language bindings and process adapters should translate their transport at
/// the edge, then delegate document semantics to this type. A loaded package
/// retains its verified resource descriptors and immutable embedded bytes
/// across semantic edits and subsequent package export.
#[derive(Clone, Debug)]
pub struct NuifDocument {
    session: Session,
    package: Option<NuifPackage>,
}

impl NuifDocument {
    #[must_use]
    pub fn from_document(document: Document) -> Self {
        Self {
            session: Session::new(document),
            package: None,
        }
    }

    /// Loads a canonical bare document while retaining structural diagnostics.
    ///
    /// # Errors
    ///
    /// Rejects malformed or resource-excessive input. Structurally invalid but
    /// decodable documents remain inspectable through [`Self::validate`].
    pub fn load(bytes: &[u8], encoding: DocumentEncoding) -> Result<Self, EngineError> {
        let document = encoding.decode_for_validation(bytes)?;
        Ok(Self::from_document(document))
    }

    /// Loads and structurally verifies a deterministic package and its embedded
    /// bytes. Required host capabilities remain available for explicit
    /// negotiation through [`Self::package_capability_report`].
    ///
    /// # Errors
    ///
    /// Rejects malformed packages, invalid manifests, resource-policy failures,
    /// digest mismatches and every declared package or session resource limit.
    pub fn load_package(bytes: &[u8]) -> Result<Self, EngineError> {
        let package = NuifPackage::decode(bytes)?;
        Self::from_package(package)
    }

    /// Loads a package only when the caller declares every capability required
    /// by its manifest.
    ///
    /// # Errors
    ///
    /// Returns structural package errors, session resource errors, or the
    /// exact set of unavailable required capabilities.
    pub fn load_package_with_capabilities(
        bytes: &[u8],
        supported: &BTreeSet<String>,
    ) -> Result<Self, EngineError> {
        let package = NuifPackage::decode(bytes)?;
        package.require_capabilities(supported)?;
        Self::from_package(package)
    }

    fn from_package(package: NuifPackage) -> Result<Self, EngineError> {
        let session =
            Session::with_resources(package.document.clone(), package.embedded_resources())?;
        Ok(Self {
            session,
            package: Some(package),
        })
    }

    #[must_use]
    pub const fn document(&self) -> &Document {
        self.session.document()
    }

    #[must_use]
    pub fn package_mode(&self) -> Option<PackageMode> {
        self.package.as_ref().map(|package| package.mode)
    }

    /// Compares a loaded package's declared requirements with one host's
    /// supported capability set. Bare documents return no package report.
    #[must_use]
    pub fn package_capability_report(
        &self,
        supported: &BTreeSet<String>,
    ) -> Option<PackageCapabilityReport> {
        self.package
            .as_ref()
            .map(|package| package.capability_report(supported))
    }

    /// Requires full capability support for an already structurally loaded
    /// package. Bare documents have no package requirements.
    ///
    /// # Errors
    ///
    /// Returns the exact unavailable requirement set.
    pub fn require_package_capabilities(
        &self,
        supported: &BTreeSet<String>,
    ) -> Result<(), EngineError> {
        self.package.as_ref().map_or(Ok(()), |package| {
            package.require_capabilities(supported).map_err(Into::into)
        })
    }

    #[must_use]
    pub fn resource(&self, digest: &ResourceDigest) -> Option<&[u8]> {
        self.session.resource(digest)
    }

    /// Returns structural diagnostics without mutating the loaded document.
    ///
    /// # Errors
    ///
    /// Returns an engine error only if validation cannot be executed.
    pub fn validate(&self) -> Result<ValidationReport, EngineError> {
        ReferenceEngine.validate(self.document())
    }

    /// Returns the canonical semantic-document revision.
    ///
    /// # Errors
    ///
    /// Rejects documents that cannot be canonically encoded.
    pub fn canonical_hash(&self) -> Result<String, EngineError> {
        canonical_hash(self.document()).map_err(Into::into)
    }

    /// Applies an already decoded semantic patch atomically.
    ///
    /// # Errors
    ///
    /// Rejects stale or invalid patches without changing the document.
    pub fn apply_patch(&mut self, patch: &Patch) -> Result<(), EngineError> {
        self.session.apply(patch)
    }

    /// Applies one typed transaction and returns its replayable patch.
    ///
    /// # Errors
    ///
    /// Rejects invalid operations atomically.
    pub fn apply_operations(
        &mut self,
        transaction: u128,
        operations: Vec<Operation>,
    ) -> Result<Patch, EngineError> {
        self.session.apply_transaction(transaction, operations)
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        self.session.can_undo()
    }

    #[must_use]
    pub fn can_redo(&self) -> bool {
        self.session.can_redo()
    }

    /// Undoes one semantic patch and returns the applied inverse.
    ///
    /// # Errors
    ///
    /// Rejects an empty or stale history entry without changing the document.
    pub fn undo(&mut self) -> Result<Patch, EngineError> {
        self.session.undo()
    }

    /// Redoes one semantic patch and returns the applied patch.
    ///
    /// # Errors
    ///
    /// Rejects an empty or stale history entry without changing the document.
    pub fn redo(&mut self) -> Result<Patch, EngineError> {
        self.session.redo()
    }

    /// Exports the current semantic document through one canonical bare codec.
    ///
    /// # Errors
    ///
    /// Rejects invalid documents and output-limit violations.
    pub fn export(&self, encoding: DocumentEncoding) -> Result<Vec<u8>, EngineError> {
        encoding.encode(self.document())
    }

    /// Exports a deterministic package while retaining loaded package metadata
    /// and resources. A document created from bare bytes starts an empty package.
    ///
    /// # Errors
    ///
    /// Rejects invalid documents, portability-policy failures and package limits.
    pub fn export_package(&self, mode: PackageMode) -> Result<Vec<u8>, EngineError> {
        let mut package = self
            .package
            .clone()
            .unwrap_or_else(|| NuifPackage::new(self.document().clone(), mode));
        package.document = self.document().clone();
        package.mode = mode;
        package.encode().map_err(Into::into)
    }

    /// Evaluates and rasterizes one explicit context through the shared engine.
    ///
    /// # Errors
    ///
    /// Rejects invalid documents, contexts, resources or render targets.
    pub fn snapshot(&self, context: &EvaluationContext) -> Result<Snapshot, EngineError> {
        self.session.snapshot(context)
    }
}

impl Engine for ReferenceEngine {
    type Error = EngineError;

    fn capabilities(&self) -> &[Capability] {
        CAPABILITIES
    }

    fn validate(&self, document: &Document) -> Result<ValidationReport, Self::Error> {
        Ok(ValidationReport {
            diagnostics: validate(document),
        })
    }

    fn apply(&mut self, document: &mut Document, patch: &Patch) -> Result<(), Self::Error> {
        nuif_protocol::apply_patch(document, patch).map_err(Into::into)
    }

    fn layout(
        &self,
        document: &Document,
        context: &EvaluationContext,
    ) -> Result<LayoutSnapshot, Self::Error> {
        validate_context(context)?;
        let snapshot = evaluate(document, context);
        let errors = snapshot
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Error)
            .count();
        if errors > 0 {
            Err(EngineError::DocumentInvalid { errors })
        } else {
            Ok(snapshot)
        }
    }

    fn build_render_scene(
        &self,
        document: &Document,
        layout: &LayoutSnapshot,
        context: &EvaluationContext,
    ) -> Result<RenderScene, Self::Error> {
        build_scene(document, layout, context).map_err(Into::into)
    }

    fn render_target_supported(&self, target: RenderTarget) -> bool {
        let pixels = u64::from(target.width) * u64::from(target.height);
        pixels > 0
            && pixels <= 16_777_216
            && target.scale_factor.is_finite()
            && target.scale_factor > 0.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum EngineError {
    #[error(transparent)]
    Apply(#[from] ApplyError),
    #[error(transparent)]
    Codec(#[from] CodecError),
    #[error(transparent)]
    Package(#[from] PackageError),
    #[error(transparent)]
    Render(#[from] RenderError),
    #[error("unsupported document encoding profile {profile:?}")]
    EncodingUnsupported { profile: String },
    #[error("undo or redo history is empty")]
    HistoryEmpty,
    #[error("invalid evaluation context: {reason}")]
    InvalidContext { reason: &'static str },
    #[error("document validation failed with {errors} error diagnostics")]
    DocumentInvalid { errors: usize },
    #[error("session resource limit exceeded for {resource}: limit {limit}, observed {observed}")]
    ResourceLimit {
        resource: &'static str,
        limit: usize,
        observed: usize,
    },
    #[error(
        "session resource bytes do not match declared digest {expected}; observed sha256:{actual}"
    )]
    ResourceDigestMismatch {
        expected: ResourceDigest,
        actual: String,
    },
}

fn validate_context(context: &EvaluationContext) -> Result<(), EngineError> {
    if !context.viewport.width.is_finite()
        || !context.viewport.height.is_finite()
        || context.viewport.width <= 0.0
        || context.viewport.height <= 0.0
        || context.viewport.width > f64::from(u32::MAX)
        || context.viewport.height > f64::from(u32::MAX)
    {
        return Err(EngineError::InvalidContext {
            reason: "viewport dimensions must be finite, positive, and fit u32",
        });
    }
    if !context.scale_factor.is_finite()
        || context.scale_factor <= 0.0
        || context.scale_factor > f64::from(f32::MAX)
    {
        return Err(EngineError::InvalidContext {
            reason: "scale factor must be finite, positive, and fit f32",
        });
    }
    if context.viewport.width * context.scale_factor > f64::from(u32::MAX)
        || context.viewport.height * context.scale_factor > f64::from(u32::MAX)
    {
        return Err(EngineError::InvalidContext {
            reason: "scaled viewport dimensions must fit u32",
        });
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct Session {
    engine: ReferenceEngine,
    document: Document,
    resources: SessionResources,
    revision: Option<String>,
    selection: Vec<EntityId>,
    undo: Vec<Patch>,
    redo: Vec<Patch>,
}

impl Session {
    #[must_use]
    pub fn new(document: Document) -> Self {
        Self {
            engine: ReferenceEngine,
            document,
            resources: BTreeMap::new(),
            revision: None,
            selection: Vec::new(),
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    /// Creates a session with an explicit, local set of immutable resources.
    ///
    /// Every byte sequence is digest-checked before the session becomes
    /// available. This grants no linked-resource or network authority.
    ///
    /// # Errors
    ///
    /// Rejects an invalid digest binding or a resource collection above the
    /// session count, per-resource or total-byte limit.
    pub fn with_resources(
        document: Document,
        resources: SessionResources,
    ) -> Result<Self, EngineError> {
        validate_session_resources(&resources)?;
        let mut session = Self::new(document);
        session.resources = resources;
        Ok(session)
    }

    #[must_use]
    pub const fn document(&self) -> &Document {
        &self.document
    }

    /// Returns an explicitly supplied, digest-verified resource without
    /// copying its immutable bytes.
    #[must_use]
    pub fn resource(&self, digest: &ResourceDigest) -> Option<&[u8]> {
        self.resources.get(digest).map(Arc::as_ref)
    }

    #[must_use]
    pub fn selection(&self) -> &[EntityId] {
        &self.selection
    }

    pub fn select(&mut self, entities: Vec<EntityId>) {
        self.selection = entities
            .into_iter()
            .filter(|id| self.document.entities.contains_key(id))
            .collect();
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Applies a transaction through the protocol and records its inverse.
    ///
    /// # Errors
    ///
    /// Returns a typed engine error if the patch cannot be applied atomically.
    pub fn apply(&mut self, patch: &Patch) -> Result<(), EngineError> {
        let inverse = apply_patch_with_inverse(&mut self.document, patch)?;
        self.revision.clone_from(&inverse.base_revision);
        self.undo.push(inverse);
        self.redo.clear();
        Ok(())
    }

    /// Authors and applies a transaction against the session-owned document.
    ///
    /// The returned patch includes the canonical pre-edit revision and is
    /// suitable for replay or transport. Because the session exclusively owns
    /// the document, the protocol application does not recompute that same
    /// revision as an external stale-base check.
    ///
    /// # Errors
    ///
    /// Returns a typed engine error if the current revision cannot be computed
    /// or the transaction cannot be applied atomically.
    pub fn apply_transaction(
        &mut self,
        transaction: u128,
        operations: Vec<Operation>,
    ) -> Result<Patch, EngineError> {
        let base_revision = self.document_revision()?;
        let mut patch = Patch {
            base_revision: None,
            transactions: vec![Transaction {
                id: transaction,
                operations,
            }],
        };
        let inverse = apply_patch_with_inverse(&mut self.document, &patch)?;
        self.revision.clone_from(&inverse.base_revision);
        self.undo.push(inverse);
        self.redo.clear();
        patch.base_revision = Some(base_revision);
        Ok(patch)
    }

    /// Undoes the last session patch and returns the semantic patch that was
    /// applied, suitable for an external replay log.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::HistoryEmpty`] when no undo entry exists.
    pub fn undo(&mut self) -> Result<Patch, EngineError> {
        let mut inverse = self.undo.pop().ok_or(EngineError::HistoryEmpty)?;
        match self.apply_history_patch(&mut inverse) {
            Ok(redo) => {
                self.redo.push(redo);
                Ok(inverse)
            }
            Err(error) => {
                self.undo.push(inverse);
                Err(error)
            }
        }
    }

    /// Redoes the last undone session patch and returns the semantic patch that
    /// was applied, suitable for an external replay log.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::HistoryEmpty`] when no redo entry exists.
    pub fn redo(&mut self) -> Result<Patch, EngineError> {
        let mut patch = self.redo.pop().ok_or(EngineError::HistoryEmpty)?;
        match self.apply_history_patch(&mut patch) {
            Ok(inverse) => {
                self.undo.push(inverse);
                Ok(patch)
            }
            Err(error) => {
                self.redo.push(patch);
                Err(error)
            }
        }
    }

    /// Produces canonical, layout, scene and CPU raster artifacts in one frame.
    ///
    /// # Errors
    ///
    /// Returns an engine error if hashing or rasterization fails.
    pub fn snapshot(&self, context: &EvaluationContext) -> Result<Snapshot, EngineError> {
        let layout = self.engine.layout(&self.document, context)?;
        let scene = build_scene_with_resources(&self.document, &layout, context, |digest| {
            self.resources.get(digest).map(Arc::as_ref)
        })?;
        let target = RenderTarget {
            width: target_dimension(context.viewport.width * context.scale_factor),
            height: target_dimension(context.viewport.height * context.scale_factor),
            scale_factor: target_scale(context.scale_factor),
        };
        let raster = render_cpu(&scene, target)?;
        Ok(Snapshot {
            canonical_hash: self.document_revision()?,
            layout,
            scene,
            raster,
        })
    }

    fn document_revision(&self) -> Result<String, EngineError> {
        self.revision
            .clone()
            .map_or_else(|| canonical_hash(&self.document).map_err(Into::into), Ok)
    }

    fn apply_history_patch(&mut self, patch: &mut Patch) -> Result<Patch, EngineError> {
        let expected = patch.base_revision.take();
        let precondition = match &expected {
            Some(expected) => self.document_revision().and_then(|actual| {
                if expected == &actual {
                    Ok(())
                } else {
                    Err(ApplyError::BaseRevisionMismatch {
                        expected: expected.clone(),
                        actual,
                    }
                    .into())
                }
            }),
            None => Ok(()),
        };
        if let Err(error) = precondition {
            patch.base_revision = expected;
            return Err(error);
        }
        let result = apply_patch_with_inverse(&mut self.document, patch);
        patch.base_revision = expected;
        let inverse = result?;
        self.revision.clone_from(&inverse.base_revision);
        Ok(inverse)
    }
}

fn validate_session_resources(resources: &SessionResources) -> Result<(), EngineError> {
    if resources.len() > MAX_SESSION_RESOURCES {
        return Err(EngineError::ResourceLimit {
            resource: "resource count",
            limit: MAX_SESSION_RESOURCES,
            observed: resources.len(),
        });
    }
    let mut total = 0_usize;
    for (expected, bytes) in resources {
        if bytes.len() > MAX_SESSION_RESOURCE_BYTES {
            return Err(EngineError::ResourceLimit {
                resource: "single resource bytes",
                limit: MAX_SESSION_RESOURCE_BYTES,
                observed: bytes.len(),
            });
        }
        total = total.saturating_add(bytes.len());
        if total > MAX_SESSION_TOTAL_RESOURCE_BYTES {
            return Err(EngineError::ResourceLimit {
                resource: "total resource bytes",
                limit: MAX_SESSION_TOTAL_RESOURCE_BYTES,
                observed: total,
            });
        }
        let actual = format!("{:x}", Sha256::digest(bytes));
        if expected.sha256_hex() != Some(actual.as_str()) {
            return Err(EngineError::ResourceDigestMismatch {
                expected: expected.clone(),
                actual,
            });
        }
    }
    Ok(())
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the rounded value is clamped to the complete u32 target domain"
)]
fn target_dimension(value: f64) -> u32 {
    value.round().clamp(1.0, f64::from(u32::MAX)) as u32
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "scale is clamped to the finite positive f32 target domain"
)]
fn target_scale(value: f64) -> f32 {
    value.clamp(f64::from(f32::MIN_POSITIVE), f64::from(f32::MAX)) as f32
}

#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub canonical_hash: String,
    pub layout: LayoutSnapshot,
    pub scene: RenderScene,
    pub raster: RasterImage,
}

#[cfg(test)]
mod tests {
    use super::*;
    use nuif_core::{Entity, EntityKind};
    use nuif_protocol::{Operation, Transaction};

    fn document() -> Document {
        let mut document = Document::empty(EntityId::new(1));
        let root = Entity::new(EntityId::new(2), EntityKind::Container);
        document.roots.push(root.id);
        document.entities.insert(root.id, root);
        document
    }

    #[test]
    fn editor_session_undo_redo_uses_protocol() {
        let mut session = Session::new(document());
        let patch = Patch {
            base_revision: None,
            transactions: vec![Transaction {
                id: 1,
                operations: vec![Operation::Rename {
                    entity: EntityId::new(2),
                    name: Some("card".to_owned()),
                }],
            }],
        };
        session.apply(&patch).unwrap();
        session.undo().unwrap();
        assert_eq!(session.document.entities[&EntityId::new(2)].name, None);
        session.redo().unwrap();
        assert_eq!(
            session.document.entities[&EntityId::new(2)].name.as_deref(),
            Some("card")
        );
    }

    #[test]
    fn local_transaction_returns_a_replayable_preconditioned_patch() {
        let base = document();
        let base_revision = canonical_hash(&base).unwrap();
        let mut session = Session::new(base.clone());
        let patch = session
            .apply_transaction(
                7,
                vec![Operation::Rename {
                    entity: EntityId::new(2),
                    name: Some("locally authored".to_owned()),
                }],
            )
            .unwrap();

        assert_eq!(patch.base_revision.as_deref(), Some(base_revision.as_str()));
        let mut replayed = base;
        nuif_protocol::apply_patch(&mut replayed, &patch).unwrap();
        assert_eq!(&replayed, session.document());
        session.undo().unwrap();
        assert_eq!(session.document.entities[&EntityId::new(2)].name, None);
    }

    #[test]
    fn consecutive_local_transactions_use_the_exact_session_revision() {
        let mut session = Session::new(document());
        session
            .apply_transaction(
                1,
                vec![Operation::Rename {
                    entity: EntityId::new(2),
                    name: Some("first".to_owned()),
                }],
            )
            .unwrap();
        let expected = canonical_hash(session.document()).unwrap();
        let second = session
            .apply_transaction(
                2,
                vec![Operation::Rename {
                    entity: EntityId::new(2),
                    name: Some("second".to_owned()),
                }],
            )
            .unwrap();

        assert_eq!(second.base_revision.as_deref(), Some(expected.as_str()));
        assert_eq!(
            session
                .snapshot(&EvaluationContext::viewport(10.0, 10.0))
                .unwrap()
                .canonical_hash,
            canonical_hash(session.document()).unwrap()
        );
    }

    #[test]
    fn failed_history_precondition_is_atomic_and_retains_the_entry() {
        let mut session = Session::new(document());
        session
            .apply_transaction(
                1,
                vec![Operation::Rename {
                    entity: EntityId::new(2),
                    name: Some("edited".to_owned()),
                }],
            )
            .unwrap();
        session.undo.last_mut().unwrap().base_revision = Some("stale".to_owned());
        let before = session.document().clone();

        assert!(matches!(
            session.undo(),
            Err(EngineError::Apply(ApplyError::BaseRevisionMismatch { .. }))
        ));
        assert_eq!(session.document(), &before);
        assert!(session.can_undo());
    }

    #[test]
    fn invalid_snapshot_context_is_rejected() {
        let session = Session::new(document());
        assert!(matches!(
            session.snapshot(&EvaluationContext::viewport(f64::NAN, 100.0)),
            Err(EngineError::InvalidContext { .. })
        ));
        assert!(matches!(
            session.snapshot(&EvaluationContext::viewport(-1.0, 100.0)),
            Err(EngineError::InvalidContext { .. })
        ));
    }

    #[test]
    fn snapshot_scale_factor_controls_device_dimensions() {
        let session = Session::new(document());
        let mut context = EvaluationContext::viewport(10.0, 12.0);
        context.scale_factor = 2.0;
        let snapshot = session.snapshot(&context).unwrap();
        assert_eq!(snapshot.raster.width, 20);
        assert_eq!(snapshot.raster.height, 24);
    }

    #[test]
    fn snapshot_rejects_a_document_with_validation_errors() {
        let mut invalid = document();
        invalid.roots.push(EntityId::new(999));
        assert!(matches!(
            Session::new(invalid).snapshot(&EvaluationContext::viewport(10.0, 12.0)),
            Err(EngineError::DocumentInvalid { errors: 1 })
        ));
    }

    #[test]
    fn session_rejects_resource_bytes_under_the_wrong_digest() {
        let resources = BTreeMap::from([(
            ResourceDigest::from_sha256_hex("0".repeat(64)),
            Arc::from(b"different bytes".as_slice()),
        )]);

        assert!(matches!(
            Session::with_resources(document(), resources),
            Err(EngineError::ResourceDigestMismatch { .. })
        ));
    }

    #[test]
    fn byte_facade_shares_canonical_operations_across_encodings() {
        let input = CanonicalText.encode(&document()).unwrap();
        let mut loaded = NuifDocument::load(&input, DocumentEncoding::CanonicalText).unwrap();
        let before = loaded.canonical_hash().unwrap();
        let patch = loaded
            .apply_operations(
                7,
                vec![Operation::Rename {
                    entity: EntityId::new(2),
                    name: Some("shared facade".to_owned()),
                }],
            )
            .unwrap();
        let after = loaded.canonical_hash().unwrap();

        assert_eq!(patch.base_revision.as_deref(), Some(before.as_str()));
        assert_ne!(after, before);
        assert_eq!(loaded.validate().unwrap().diagnostics.len(), 0);
        let cbor = loaded.export(DocumentEncoding::DeterministicCbor).unwrap();
        assert_eq!(
            NuifDocument::load(&cbor, DocumentEncoding::DeterministicCbor)
                .unwrap()
                .canonical_hash()
                .unwrap(),
            after
        );
        loaded.undo().unwrap();
        assert_eq!(loaded.canonical_hash().unwrap(), before);
        loaded.redo().unwrap();
        assert_eq!(loaded.canonical_hash().unwrap(), after);
    }

    #[test]
    fn byte_facade_retains_package_contract_across_edits() {
        let mut package = NuifPackage::new(document(), PackageMode::Authoring);
        package
            .required_capabilities
            .insert("nuif-layout-profile-0".to_owned());
        let bytes = package.encode().unwrap();
        let mut loaded = NuifDocument::load_package(&bytes).unwrap();

        assert_eq!(loaded.package_mode(), Some(PackageMode::Authoring));
        loaded
            .apply_operations(
                9,
                vec![Operation::Rename {
                    entity: EntityId::new(2),
                    name: Some("package edit".to_owned()),
                }],
            )
            .unwrap();
        let exported = loaded.export_package(PackageMode::Authoring).unwrap();
        let decoded = NuifPackage::decode(&exported).unwrap();

        assert!(
            decoded
                .required_capabilities
                .contains("nuif-layout-profile-0")
        );
        assert_eq!(
            decoded.document.entities[&EntityId::new(2)].name.as_deref(),
            Some("package edit")
        );
        assert_eq!(decoded.encode().unwrap(), exported);
    }

    #[test]
    fn package_capability_negotiation_is_explicit_and_typed() {
        let mut package = NuifPackage::new(document(), PackageMode::Portable);
        package.required_capabilities = BTreeSet::from([
            "nuif-behavior-state-machine-0".to_owned(),
            "nuif-layout-profile-0".to_owned(),
        ]);
        let bytes = package.encode().unwrap();
        let layout_only = BTreeSet::from(["nuif-layout-profile-0".to_owned()]);

        let loaded = NuifDocument::load_package(&bytes).unwrap();
        let report = loaded.package_capability_report(&layout_only).unwrap();
        assert!(!report.fully_supported);
        assert_eq!(
            report.missing_required,
            BTreeSet::from(["nuif-behavior-state-machine-0".to_owned()])
        );
        assert!(matches!(
            loaded.require_package_capabilities(&layout_only),
            Err(EngineError::Package(
                PackageError::RequiredCapabilitiesUnavailable { capabilities }
            )) if capabilities == report.missing_required
        ));
        assert!(matches!(
            NuifDocument::load_package_with_capabilities(&bytes, &layout_only),
            Err(EngineError::Package(
                PackageError::RequiredCapabilitiesUnavailable { .. }
            ))
        ));
        assert!(
            NuifDocument::load_package_with_capabilities(&bytes, &package.required_capabilities)
                .is_ok()
        );

        let bare = NuifDocument::from_document(document());
        assert_eq!(bare.package_capability_report(&BTreeSet::new()), None);
        assert!(bare.require_package_capabilities(&BTreeSet::new()).is_ok());
    }
}
