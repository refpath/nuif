//! Bounded collaboration for one causal set carrying property and structure edits.
//!
//! Structure is materialized first, then register-like property changes are
//! applied to that result. This ordering makes deletion/property races
//! explicit instead of silently dropping a property update.

use super::structural::{
    StructuralChange, StructuralCheckpoint, StructuralError, StructuralOperation,
    StructuralOperationSetEngine,
};
use super::{
    Change, ChangeId, CollaborationError, MAX_CHANGES, MAX_REPLICAS, OperationSetEngine,
    SemanticConflict, validate_replica,
};
use nuif_codec::canonical_hash;
use nuif_core::{Document, EntityId};
use nuif_protocol::Operation;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

/// Profile identifier for mixed property/structure collaboration.
pub const PROFILE_NAME: &str = "nuif-collab-mixed-0";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "operation")]
pub enum MixedOperation {
    Property(Operation),
    Structure(StructuralOperation),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MixedChange {
    pub id: ChangeId,
    #[serde(default)]
    pub context: BTreeMap<String, u64>,
    pub operation: MixedOperation,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MixedCheckpoint {
    pub profile: String,
    pub canonical_hash: String,
    pub document: Document,
    pub applied: Vec<ChangeId>,
    pub property_conflicts: Vec<SemanticConflict>,
    pub structural_conflicts: Vec<super::structural::StructuralConflict>,
}

#[derive(Clone, Debug, PartialEq, Error)]
pub enum MixedError {
    #[error(transparent)]
    Causal(#[from] CollaborationError),
    #[error(transparent)]
    Structural(#[from] StructuralError),
    #[error("mixed change identifier {change:?} has conflicting contents")]
    DuplicateChange { change: ChangeId },
    #[error(
        "mixed change {change:?} targets entity {entity}, which is absent after structure materialization"
    )]
    PropertyTargetUnavailable { change: ChangeId, entity: EntityId },
    #[error("mixed engines are bound to different base hashes: {left} != {right}")]
    BaseMismatch { left: String, right: String },
    #[error("canonical hashing failed: {0}")]
    Canonical(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct MixedOperationSetEngine {
    base: Document,
    base_hash: String,
    changes: BTreeMap<ChangeId, MixedChange>,
}

impl MixedOperationSetEngine {
    /// Creates an engine bound to one canonical base document.
    ///
    /// # Errors
    ///
    /// Rejects an invalid or unhashable base document.
    pub fn new(base: Document) -> Result<Self, MixedError> {
        let structural = StructuralOperationSetEngine::new(base.clone())?;
        let base_hash =
            canonical_hash(&base).map_err(|error| MixedError::Canonical(error.to_string()))?;
        drop(structural);
        Ok(Self {
            base,
            base_hash,
            changes: BTreeMap::new(),
        })
    }

    /// Adds one idempotent mixed change.
    ///
    /// # Errors
    ///
    /// Rejects malformed clocks, unsupported property operations, unknown
    /// structural identities, resource overflow or conflicting identifier
    /// reuse.
    pub fn ingest(&mut self, change: MixedChange) -> Result<bool, MixedError> {
        validate_change_shape(&self.base, &change)?;
        if let Some(existing) = self.changes.get(&change.id) {
            return if existing == &change {
                Ok(false)
            } else {
                Err(MixedError::DuplicateChange { change: change.id })
            };
        }
        if self.changes.len() == MAX_CHANGES {
            return Err(CollaborationError::TooManyChanges.into());
        }
        self.changes.insert(change.id.clone(), change);
        Ok(true)
    }

    /// Atomically joins another engine bound to the same canonical base.
    ///
    /// # Errors
    ///
    /// Rejects a different base or any ingestion failure.
    pub fn merge(&mut self, other: &Self) -> Result<(), MixedError> {
        if self.base_hash != other.base_hash {
            return Err(MixedError::BaseMismatch {
                left: self.base_hash.clone(),
                right: other.base_hash.clone(),
            });
        }
        let mut candidate = self.clone();
        for change in other.changes.values() {
            candidate.ingest(change.clone())?;
        }
        *self = candidate;
        Ok(())
    }

    /// Materializes structure first and then property registers over the
    /// resulting tree.
    ///
    /// # Errors
    ///
    /// Rejects incomplete causal history, structural failures and property
    /// edits whose entity is no longer present after structure resolution.
    pub fn checkpoint(&self) -> Result<MixedCheckpoint, MixedError> {
        validate_collection(self.changes.values())?;
        let mut structural = StructuralOperationSetEngine::new(self.base.clone())?;
        let mut property = OperationSetEngine::default();
        for change in self.changes.values() {
            match &change.operation {
                MixedOperation::Property(operation) => {
                    property.ingest(Change {
                        id: change.id.clone(),
                        context: change.context.clone(),
                        operation: operation.clone(),
                    })?;
                }
                MixedOperation::Structure(operation) => {
                    structural.ingest(StructuralChange {
                        id: change.id.clone(),
                        context: change.context.clone(),
                        operation: operation.clone(),
                    })?;
                }
            }
        }
        let structural_checkpoint: StructuralCheckpoint = structural.checkpoint()?;
        for change in self.changes.values() {
            if let MixedOperation::Property(operation) = &change.operation
                && let Some(entity) = property_entity(operation)
                && !structural_checkpoint
                    .document
                    .entities
                    .contains_key(&entity)
            {
                return Err(MixedError::PropertyTargetUnavailable {
                    change: change.id.clone(),
                    entity,
                });
            }
        }
        let property_checkpoint = property.checkpoint(&structural_checkpoint.document)?;
        let canonical_hash = canonical_hash(&property_checkpoint.document)
            .map_err(|error| MixedError::Canonical(error.to_string()))?;
        Ok(MixedCheckpoint {
            profile: PROFILE_NAME.to_owned(),
            canonical_hash,
            document: property_checkpoint.document,
            applied: self.changes.keys().cloned().collect(),
            property_conflicts: property_checkpoint.conflicts,
            structural_conflicts: structural_checkpoint.conflicts,
        })
    }
}

fn validate_change_shape(base: &Document, change: &MixedChange) -> Result<(), MixedError> {
    validate_replica(&change.id.replica)?;
    if change.id.counter == 0 {
        return Err(CollaborationError::ZeroCounter {
            replica: change.id.replica.clone(),
        }
        .into());
    }
    for replica in change.context.keys() {
        validate_replica(replica)?;
    }
    if change.context.len() > MAX_REPLICAS {
        return Err(CollaborationError::TooManyReplicas.into());
    }
    let own = change.context.get(&change.id.replica).copied().unwrap_or(0);
    if own + 1 != change.id.counter {
        return Err(CollaborationError::InvalidLocalContext {
            change: change.id.clone(),
            observed: own,
        }
        .into());
    }
    match &change.operation {
        MixedOperation::Property(operation) => {
            let mut engine = OperationSetEngine::default();
            engine.ingest(Change {
                id: change.id.clone(),
                context: change.context.clone(),
                operation: operation.clone(),
            })?;
        }
        MixedOperation::Structure(operation) => {
            let mut engine = StructuralOperationSetEngine::new(base.to_owned())?;
            engine.ingest(StructuralChange {
                id: change.id.clone(),
                context: change.context.clone(),
                operation: operation.clone(),
            })?;
        }
    }
    Ok(())
}

fn validate_collection<'a>(
    changes: impl Iterator<Item = &'a MixedChange> + Clone,
) -> Result<(), MixedError> {
    let changes = changes.collect::<Vec<_>>();
    if changes.len() > MAX_CHANGES {
        return Err(CollaborationError::TooManyChanges.into());
    }
    let replicas = changes
        .iter()
        .map(|change| change.id.replica.as_str())
        .collect::<BTreeSet<_>>();
    if replicas.len() > MAX_REPLICAS {
        return Err(CollaborationError::TooManyReplicas.into());
    }
    let received = changes
        .iter()
        .map(|change| ((change.id.replica.as_str(), change.id.counter), *change))
        .collect::<BTreeMap<_, _>>();
    for replica in replicas {
        for (expected, counter) in (1_u64..).zip(
            received
                .keys()
                .filter_map(|(candidate, counter)| (*candidate == replica).then_some(*counter)),
        ) {
            if counter != expected {
                return Err(CollaborationError::MissingReplicaChange {
                    replica: replica.to_owned(),
                    counter: expected,
                }
                .into());
            }
        }
    }
    for change in &changes {
        for (replica, counter) in &change.context {
            if *counter == 0 {
                continue;
            }
            let dependency = received.get(&(replica.as_str(), *counter)).ok_or_else(|| {
                CollaborationError::MissingDependency {
                    change: change.id.clone(),
                    replica: replica.clone(),
                    counter: *counter,
                }
            })?;
            if dependency
                .context
                .iter()
                .any(|(causal_replica, causal_counter)| {
                    change.context.get(causal_replica).copied().unwrap_or(0) < *causal_counter
                })
            {
                return Err(CollaborationError::IncompleteCausalContext {
                    change: change.id.clone(),
                    dependency: dependency.id.clone(),
                }
                .into());
            }
        }
    }
    Ok(())
}

fn property_entity(operation: &Operation) -> Option<EntityId> {
    match operation {
        Operation::Rename { entity, .. }
        | Operation::SetSize { entity, .. }
        | Operation::SetLayout { entity, .. }
        | Operation::SetGridPlacement { entity, .. }
        | Operation::SetPosition { entity, .. }
        | Operation::SetFill { entity, .. }
        | Operation::SetText { entity, .. }
        | Operation::SetImage { entity, .. }
        | Operation::SetValue { entity, .. }
        | Operation::RemoveValue { entity, .. }
        | Operation::SetExtension { entity, .. }
        | Operation::RemoveExtension { entity, .. }
        | Operation::SetUnknownPayload { entity, .. } => Some(*entity),
        Operation::SetToken { .. }
        | Operation::RemoveToken { .. }
        | Operation::SetExtensionDeclarations { .. }
        | Operation::Insert { .. }
        | Operation::Remove { .. }
        | Operation::Move { .. }
        | Operation::SetAsset { .. }
        | Operation::RemoveAsset { .. }
        | Operation::BindAssetResource { .. }
        | Operation::RestoreSubtree { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nuif_core::{Entity, EntityKind};

    const ROOT: EntityId = EntityId::new(10);
    const LEFT: EntityId = EntityId::new(11);
    const RIGHT: EntityId = EntityId::new(12);

    fn base() -> Document {
        let mut document = Document::empty(EntityId::new(1));
        let mut root = Entity::new(ROOT, EntityKind::Container);
        root.children.extend([LEFT, RIGHT]);
        document.roots.push(ROOT);
        document.entities.insert(ROOT, root);
        document
            .entities
            .insert(LEFT, Entity::new(LEFT, EntityKind::Container));
        document
            .entities
            .insert(RIGHT, Entity::new(RIGHT, EntityKind::Container));
        document
    }

    fn changes() -> Vec<MixedChange> {
        vec![
            MixedChange {
                id: ChangeId::new("alice", 1),
                context: BTreeMap::new(),
                operation: MixedOperation::Structure(StructuralOperation::Move {
                    entity: RIGHT,
                    new_parent: Some(LEFT),
                    anchor: super::super::structural::StructuralAnchor::Start,
                }),
            },
            MixedChange {
                id: ChangeId::new("bob", 1),
                context: BTreeMap::new(),
                operation: MixedOperation::Property(Operation::Rename {
                    entity: RIGHT,
                    name: Some("right card".to_owned()),
                }),
            },
        ]
    }

    #[test]
    fn property_and_structure_converge_across_delivery_orders() {
        let changes = changes();
        let mut first = MixedOperationSetEngine::new(base()).unwrap();
        first.ingest(changes[0].clone()).unwrap();
        first.ingest(changes[1].clone()).unwrap();
        let expected = first.checkpoint().unwrap();
        let mut reversed = MixedOperationSetEngine::new(base()).unwrap();
        reversed.ingest(changes[1].clone()).unwrap();
        reversed.ingest(changes[0].clone()).unwrap();
        assert_eq!(reversed.checkpoint().unwrap(), expected);
        assert_eq!(expected.document.roots, vec![ROOT]);
        assert_eq!(expected.document.entities[&ROOT].children, vec![LEFT]);
        assert_eq!(expected.document.entities[&LEFT].children, vec![RIGHT]);
        assert_eq!(
            expected.document.entities[&RIGHT].name.as_deref(),
            Some("right card")
        );
    }

    #[test]
    fn property_deleted_by_structure_is_typed() {
        let mut engine = MixedOperationSetEngine::new(base()).unwrap();
        engine
            .ingest(MixedChange {
                id: ChangeId::new("alice", 1),
                context: BTreeMap::new(),
                operation: MixedOperation::Structure(StructuralOperation::Delete { entity: RIGHT }),
            })
            .unwrap();
        engine
            .ingest(MixedChange {
                id: ChangeId::new("bob", 1),
                context: BTreeMap::new(),
                operation: MixedOperation::Property(Operation::Rename {
                    entity: RIGHT,
                    name: Some("lost".to_owned()),
                }),
            })
            .unwrap();
        assert!(matches!(
            engine.checkpoint(),
            Err(MixedError::PropertyTargetUnavailable { entity, .. }) if entity == RIGHT
        ));
    }

    #[test]
    fn cross_kind_dependency_is_checked() {
        let mut engine = MixedOperationSetEngine::new(base()).unwrap();
        engine
            .ingest(MixedChange {
                id: ChangeId::new("bob", 1),
                context: BTreeMap::from([("alice".to_owned(), 1)]),
                operation: MixedOperation::Property(Operation::Rename {
                    entity: RIGHT,
                    name: Some("right card".to_owned()),
                }),
            })
            .unwrap();
        assert!(matches!(
            engine.checkpoint(),
            Err(MixedError::Causal(
                CollaborationError::MissingDependency { .. }
            ))
        ));
    }
}
