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
    fn validate(&self, document: &Document) -> Result<ValidationReport, Self::Error>;
    fn apply(&mut self, document: &mut Document, patch: &Patch) -> Result<(), Self::Error>;
    fn layout(
        &self,
        document: &Document,
        context: &EvaluationContext,
    ) -> Result<LayoutSnapshot, Self::Error>;
    fn build_render_scene(
        &self,
        document: &Document,
        layout: &LayoutSnapshot,
    ) -> Result<RenderScene, Self::Error>;
    fn render_target_supported(&self, target: RenderTarget) -> bool;
}
