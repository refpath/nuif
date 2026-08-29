#![doc = "Canonical in-memory model for the NUIF reference implementation."]

use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EntityId(pub u128);

#[derive(Clone, Debug, PartialEq)]
pub struct Document {
    pub id: EntityId,
    pub entities: BTreeMap<EntityId, Entity>,
    pub roots: Vec<EntityId>,
    pub extensions: Extensions,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entity {
    pub id: EntityId,
    pub name: Option<String>,
    pub kind: EntityKind,
    pub children: Vec<EntityId>,
    pub authored: AuthoredProperties,
    pub extensions: Extensions,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EntityKind {
    Surface,
    Container,
    Shape(ShapeKind),
    Text,
    Image,
    Component,
    Instance { component: EntityId },
}

#[derive(Clone, Debug, PartialEq)]
pub enum ShapeKind {
    Rectangle,
    Ellipse,
    Path,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AuthoredProperties {
    pub width: SizeIntent,
    pub height: SizeIntent,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum SizeIntent {
    #[default]
    Auto,
    Fixed(f64),
    Fill,
    Intrinsic,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Extensions(pub BTreeMap<String, Vec<u8>>);

#[derive(Clone, Debug, PartialEq)]
pub enum Fidelity {
    Lossless,
    Representable,
    Approximated { reason: String },
    PreservedUnrenderable { extension: String },
    Unsupported { reason: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub entity: Option<EntityId>,
    pub fidelity: Option<Fidelity>,
}
