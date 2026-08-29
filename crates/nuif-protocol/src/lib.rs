#![doc = "Semantic operations and transaction primitives for NUIF."]

use nuif_core::{Entity, EntityId};

#[derive(Clone, Debug, PartialEq)]
pub struct Transaction {
    pub id: u128,
    pub operations: Vec<Operation>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Operation {
    Insert {
        parent: Option<EntityId>,
        index: usize,
        entity: Entity,
    },
    Remove {
        entity: EntityId,
    },
    Move {
        entity: EntityId,
        new_parent: Option<EntityId>,
        new_index: usize,
    },
    Rename {
        entity: EntityId,
        name: Option<String>,
    },
    SetExtension {
        entity: EntityId,
        namespace: String,
        payload: Vec<u8>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Patch {
    pub base_revision: Option<String>,
    pub transactions: Vec<Transaction>,
}
