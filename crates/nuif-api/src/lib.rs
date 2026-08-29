#![doc = "Stable headless engine contract shared by CLI, editor and automation clients."]

use nuif_core::{Diagnostic, Document};
use nuif_layout::{EvaluationContext, LayoutSnapshot};
use nuif_protocol::Patch;
use nuif_render::{RenderScene, RenderTarget};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

#[derive(Clone, Debug, Default, PartialEq)]
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
