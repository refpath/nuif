#![doc = "Stable headless engine and editor-session contract shared by every client."]

use nuif_codec::{CodecError, canonical_hash};
use nuif_core::{Diagnostic, Document, EntityId, validate};
use nuif_layout::{EvaluationContext, LayoutSnapshot, evaluate};
use nuif_protocol::{ApplyError, Patch, apply_patch_with_inverse};
use nuif_render::{RasterImage, RenderError, RenderScene, RenderTarget, build_scene, render_cpu};
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
    ) -> Result<RenderScene, Self::Error>;

    fn render_target_supported(&self, target: RenderTarget) -> bool;
}

#[derive(Clone, Debug, Default)]
pub struct ReferenceEngine;

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
        Ok(evaluate(document, context))
    }

    fn build_render_scene(
        &self,
        document: &Document,
        layout: &LayoutSnapshot,
    ) -> Result<RenderScene, Self::Error> {
        Ok(build_scene(document, layout))
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
            selection: Vec::new(),
            undo: Vec::new(),
            redo: Vec::new(),
        }
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

    /// Applies a transaction through the protocol and records its inverse.
    ///
    /// # Errors
    ///
    /// Returns a typed engine error if the patch cannot be applied atomically.
    pub fn apply(&mut self, patch: &Patch) -> Result<(), EngineError> {
        let inverse = apply_patch_with_inverse(&mut self.document, patch)?;
        self.undo.push(inverse);
        self.redo.clear();
        Ok(())
    }

    /// Undoes the last session patch and returns the semantic patch that was
    /// applied, suitable for an external replay log.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::HistoryEmpty`] when no undo entry exists.
    pub fn undo(&mut self) -> Result<Patch, EngineError> {
        let inverse = self.undo.pop().ok_or(EngineError::HistoryEmpty)?;
        let redo = apply_patch_with_inverse(&mut self.document, &inverse)?;
        self.redo.push(redo);
        Ok(inverse)
    }

    /// Redoes the last undone session patch and returns the semantic patch that
    /// was applied, suitable for an external replay log.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::HistoryEmpty`] when no redo entry exists.
    pub fn redo(&mut self) -> Result<Patch, EngineError> {
        let patch = self.redo.pop().ok_or(EngineError::HistoryEmpty)?;
        let inverse = apply_patch_with_inverse(&mut self.document, &patch)?;
        self.undo.push(inverse);
        Ok(patch)
    }

    /// Produces canonical, layout, scene and CPU raster artifacts in one frame.
    ///
    /// # Errors
    ///
    /// Returns an engine error if hashing or rasterization fails.
    pub fn snapshot(&self, context: &EvaluationContext) -> Result<Snapshot, EngineError> {
        let layout = self.engine.layout(&self.document, context)?;
        let scene = self.engine.build_render_scene(&self.document, &layout)?;
        let target = RenderTarget {
            width: target_dimension(context.viewport.width * context.scale_factor),
            height: target_dimension(context.viewport.height * context.scale_factor),
            scale_factor: target_scale(context.scale_factor),
        };
        let raster = render_cpu(&scene, target)?;
        Ok(Snapshot {
            canonical_hash: canonical_hash(&self.document)?,
            layout,
            scene,
            raster,
        })
    }
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
}
