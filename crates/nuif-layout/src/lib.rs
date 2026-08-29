#![doc = "Layout evaluator boundary for NUIF."]

use nuif_core::{Diagnostic, Document, EntityId};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvaluationContext {
    pub viewport: Size,
    pub scale_factor: f64,
    pub locale: String,
    pub writing_direction: WritingDirection,
    pub theme: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum WritingDirection {
    #[default]
    LeftToRight,
    RightToLeft,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutSnapshot {
    pub boxes: BTreeMap<EntityId, Rect>,
    pub diagnostics: Vec<Diagnostic>,
}

pub trait LayoutEvaluator {
    type Error;

    fn evaluate(
        &self,
        document: &Document,
        context: &EvaluationContext,
    ) -> Result<LayoutSnapshot, Self::Error>;
}
