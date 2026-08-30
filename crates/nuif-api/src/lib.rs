#![doc = "Stable headless engine and editor-session contract shared by every client."]

use nuif_codec::{CodecError, canonical_hash};
use nuif_core::{Diagnostic, Document, EntityId, ResourceDigest, Severity, validate};
use nuif_layout::{EvaluationContext, LayoutSnapshot, evaluate};
use nuif_protocol::{ApplyError, Operation, Patch, Transaction, apply_patch_with_inverse};
use nuif_render::{
    RasterImage, RenderError, RenderScene, RenderTarget, build_scene, build_scene_with_resources,
    render_cpu,
};
use nuif_text::PINNED_FONT_SHA256;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::Arc;
use thiserror::Error;

pub const MAX_SESSION_RESOURCES: usize = 8_192;
pub const MAX_SESSION_RESOURCE_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_SESSION_TOTAL_RESOURCE_BYTES: usize = 64 * 1024 * 1024;
pub type SessionResources = BTreeMap<ResourceDigest, Arc<[u8]>>;

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
    Render(#[from] RenderError),
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
}
