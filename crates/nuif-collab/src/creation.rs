//! Bounded concurrent-creation collaboration above a canonical NUIF base.
//!
//! This profile deliberately supports only creation of leaf entities under
//! entities already present in the base document.  Keeping the first creation
//! profile small makes its ordering, identity-collision and canonicalization
//! rules testable without silently pretending that nested creation, deletion,
//! or mixed property transactions are solved.

use super::gc::{CompactionReceipt, StabilityFrontier};
use super::structural::PositionId;
use super::{ChangeId, CollaborationError, MAX_CHANGES, MAX_REPLICAS, validate_replica};
use nuif_codec::canonical_hash;
use nuif_core::{Document, Entity, EntityId, Severity, validate};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

/// Stable profile identifier for bounded concurrent leaf creation.
pub const PROFILE_NAME: &str = "nuif-collab-tree-create-0";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "entity")]
pub enum CreationAnchor {
    Start,
    After(EntityId),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "op")]
pub enum CreationOperation {
    Insert {
        parent: Option<EntityId>,
        anchor: CreationAnchor,
        entity: Box<Entity>,
    },
}

impl CreationOperation {
    fn entity(&self) -> &Entity {
        match self {
            Self::Insert { entity, .. } => entity,
        }
    }

    fn parent_and_anchor(&self) -> (Option<EntityId>, CreationAnchor) {
        match self {
            Self::Insert { parent, anchor, .. } => (*parent, *anchor),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreationChange {
    pub id: ChangeId,
    #[serde(default)]
    pub context: BTreeMap<String, u64>,
    pub operation: CreationOperation,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum CreationConflict {
    EntityIdCollision {
        entity: EntityId,
        candidates: Vec<ChangeId>,
        selected: ChangeId,
    },
    AnchorUnavailable {
        change: ChangeId,
        anchor: CreationAnchor,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreationCheckpoint {
    pub profile: String,
    pub canonical_hash: String,
    pub document: Document,
    pub applied: Vec<ChangeId>,
    pub conflicts: Vec<CreationConflict>,
    pub active_positions: BTreeMap<EntityId, PositionId>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreationCompaction {
    pub receipt: CompactionReceipt,
    pub checkpoint: CreationCheckpoint,
}

#[derive(Clone, Debug, PartialEq, Error)]
pub enum CreationError {
    #[error(transparent)]
    Causal(#[from] CollaborationError),
    #[error("creation change identifier {change:?} has conflicting contents")]
    DuplicateChange { change: ChangeId },
    #[error("base document is structurally invalid: {codes:?}")]
    InvalidBase { codes: Vec<String> },
    #[error("created entity {entity} already exists in the base document")]
    EntityAlreadyExists { entity: EntityId },
    #[error("created entity {entity} must be a leaf with no children")]
    NestedEntity { entity: EntityId },
    #[error("creation parent {parent} does not exist in the base document")]
    ParentMissing { parent: EntityId },
    #[error("creation anchor {anchor:?} does not identify a base sibling")]
    AnchorMissing { anchor: EntityId },
    #[error("creation engines are bound to different base hashes: {left} != {right}")]
    BaseMismatch { left: String, right: String },
    #[error("created entity {entity} is invalid: {codes:?}")]
    InvalidEntity {
        entity: EntityId,
        codes: Vec<String>,
    },
    #[error("creation checkpoint is invalid: {codes:?}")]
    InvalidCheckpoint { codes: Vec<String> },
    #[error("canonical hashing failed: {0}")]
    Canonical(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreationOperationSetEngine {
    base: Document,
    base_hash: String,
    changes: BTreeMap<ChangeId, CreationChange>,
}

impl CreationOperationSetEngine {
    /// Creates a creation engine bound to one canonical base document.
    ///
    /// # Errors
    ///
    /// Rejects invalid or unhashable base documents.
    pub fn new(base: Document) -> Result<Self, CreationError> {
        validate_base(&base)?;
        let base_hash =
            canonical_hash(&base).map_err(|error| CreationError::Canonical(error.to_string()))?;
        Ok(Self {
            base,
            base_hash,
            changes: BTreeMap::new(),
        })
    }

    /// Adds one idempotent leaf-creation change.
    ///
    /// # Errors
    ///
    /// Rejects malformed clocks, nested entities, unknown parents or anchors,
    /// resource overflow, and conflicting identifier reuse.
    pub fn ingest(&mut self, change: CreationChange) -> Result<bool, CreationError> {
        validate_change_shape(&self.base, &change)?;
        if let Some(existing) = self.changes.get(&change.id) {
            return if existing == &change {
                Ok(false)
            } else {
                Err(CreationError::DuplicateChange { change: change.id })
            };
        }
        if self.changes.len() == MAX_CHANGES {
            return Err(CollaborationError::TooManyChanges.into());
        }
        self.changes.insert(change.id.clone(), change);
        Ok(true)
    }

    /// Atomically joins another engine bound to the same base.
    ///
    /// # Errors
    ///
    /// Returns a typed base mismatch or any ingestion error.
    pub fn merge(&mut self, other: &Self) -> Result<(), CreationError> {
        if self.base_hash != other.base_hash {
            return Err(CreationError::BaseMismatch {
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

    /// Materializes a metadata-free canonical checkpoint.
    ///
    /// Concurrent creates with the same entity ID are represented as an
    /// explicit conflict and resolved provisionally by the greatest dot. New
    /// siblings sharing an anchor are ordered by descending change ID, while
    /// base sibling order remains unchanged.
    ///
    /// # Errors
    ///
    /// Rejects incomplete causal history or a resulting invalid checkpoint.
    pub fn checkpoint(&self) -> Result<CreationCheckpoint, CreationError> {
        validate_collection(self.changes.values())?;

        let mut by_entity = BTreeMap::<EntityId, Vec<&CreationChange>>::new();
        for change in self.changes.values() {
            by_entity
                .entry(change.operation.entity().id)
                .or_default()
                .push(change);
        }
        let mut selected = Vec::with_capacity(by_entity.len());
        let mut conflicts = Vec::new();
        for (entity, mut candidates) in by_entity {
            candidates.sort_by(|left, right| left.id.cmp(&right.id));
            let Some(winner) = candidates.last() else {
                continue;
            };
            if candidates.len() > 1 {
                conflicts.push(CreationConflict::EntityIdCollision {
                    entity,
                    candidates: candidates
                        .iter()
                        .map(|candidate| candidate.id.clone())
                        .collect(),
                    selected: winner.id.clone(),
                });
            }
            selected.push(*winner);
        }

        let mut document = self.base.clone();
        let mut active_positions = BTreeMap::new();
        let mut insertions =
            BTreeMap::<(Option<EntityId>, CreationAnchor), Vec<&CreationChange>>::new();
        for change in selected {
            let entity = change.operation.entity();
            let (parent, anchor) = change.operation.parent_and_anchor();
            if !anchor_available(&self.base, parent, anchor) {
                conflicts.push(CreationConflict::AnchorUnavailable {
                    change: change.id.clone(),
                    anchor,
                });
                continue;
            }
            document.entities.insert(entity.id, entity.clone());
            active_positions.insert(entity.id, PositionId::Change(change.id.clone()));
            insertions.entry((parent, anchor)).or_default().push(change);
        }
        for values in insertions.values_mut() {
            values.sort_by(|left, right| right.id.cmp(&left.id));
        }
        document.roots = materialize_list(&document.roots, None, &insertions);
        let base_children = self
            .base
            .entities
            .iter()
            .map(|(id, entity)| (*id, entity.children.clone()))
            .collect::<BTreeMap<_, _>>();
        for (id, entity) in &mut document.entities {
            if let Some(children) = base_children.get(id) {
                entity.children = materialize_list(children, Some(*id), &insertions);
            }
        }
        let codes = error_codes(&document);
        if !codes.is_empty() {
            return Err(CreationError::InvalidCheckpoint { codes });
        }
        let canonical_hash = canonical_hash(&document)
            .map_err(|error| CreationError::Canonical(error.to_string()))?;
        Ok(CreationCheckpoint {
            profile: PROFILE_NAME.to_owned(),
            canonical_hash,
            document,
            applied: self
                .changes
                .values()
                .map(|change| change.id.clone())
                .collect(),
            conflicts,
            active_positions,
        })
    }

    /// Replaces a complete, causally stable creation history with its
    /// metadata-free canonical checkpoint.
    ///
    /// Profile 0 intentionally rejects partial collection. The frontier must
    /// exactly cover every locally observed creation change; no nested payload
    /// or position-anchor rebasing is attempted.
    pub fn compact_stable(
        &self,
        frontier: &StabilityFrontier,
    ) -> Result<CreationCompaction, CreationError> {
        let checkpoint = self.checkpoint()?;
        frontier.validate_complete(self.changes.keys())?;
        let receipt = CompactionReceipt::complete_history(
            PROFILE_NAME,
            self.base_hash.clone(),
            checkpoint.canonical_hash.clone(),
            frontier,
            self.changes.keys().cloned().collect(),
        );
        Ok(CreationCompaction {
            receipt,
            checkpoint,
        })
    }
}

fn materialize_list(
    base: &[EntityId],
    parent: Option<EntityId>,
    insertions: &BTreeMap<(Option<EntityId>, CreationAnchor), Vec<&CreationChange>>,
) -> Vec<EntityId> {
    let mut output = Vec::with_capacity(base.len());
    if let Some(values) = insertions.get(&(parent, CreationAnchor::Start)) {
        output.extend(values.iter().map(|change| change.operation.entity().id));
    }
    for entity in base {
        output.push(*entity);
        if let Some(values) = insertions.get(&(parent, CreationAnchor::After(*entity))) {
            output.extend(values.iter().map(|change| change.operation.entity().id));
        }
    }
    output
}

fn validate_base(document: &Document) -> Result<(), CreationError> {
    let codes = error_codes(document);
    if codes.is_empty() {
        Ok(())
    } else {
        Err(CreationError::InvalidBase { codes })
    }
}

fn error_codes(document: &Document) -> Vec<String> {
    validate(document)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| diagnostic.code)
        .collect()
}

fn validate_change_shape(base: &Document, change: &CreationChange) -> Result<(), CreationError> {
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
    let entity = change.operation.entity();
    if base.entities.contains_key(&entity.id) {
        return Err(CreationError::EntityAlreadyExists { entity: entity.id });
    }
    if !entity.children.is_empty() {
        return Err(CreationError::NestedEntity { entity: entity.id });
    }
    let (parent, anchor) = change.operation.parent_and_anchor();
    if let Some(parent) = parent
        && !base.entities.contains_key(&parent)
    {
        return Err(CreationError::ParentMissing { parent });
    }
    if let CreationAnchor::After(anchor) = anchor
        && !base_parent_contains(base, parent, anchor)
    {
        return Err(CreationError::AnchorMissing { anchor });
    }
    let mut candidate = base.clone();
    candidate.roots.push(entity.id);
    candidate.entities.insert(entity.id, entity.clone());
    let codes = error_codes(&candidate);
    if !codes.is_empty() {
        return Err(CreationError::InvalidEntity {
            entity: entity.id,
            codes,
        });
    }
    Ok(())
}

fn base_parent_contains(base: &Document, parent: Option<EntityId>, anchor: EntityId) -> bool {
    match parent {
        Some(parent) => base
            .entities
            .get(&parent)
            .is_some_and(|entity| entity.children.contains(&anchor)),
        None => base.roots.contains(&anchor),
    }
}

fn anchor_available(base: &Document, parent: Option<EntityId>, anchor: CreationAnchor) -> bool {
    match anchor {
        CreationAnchor::Start => true,
        CreationAnchor::After(entity) => base_parent_contains(base, parent, entity),
    }
}

fn validate_collection<'a>(
    changes: impl Iterator<Item = &'a CreationChange> + Clone,
) -> Result<(), CreationError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use nuif_core::{EntityId, EntityKind};

    const ROOT: EntityId = EntityId::new(10);
    const BASE_CHILD: EntityId = EntityId::new(11);

    fn base() -> Document {
        let mut document = Document::empty(EntityId::new(1));
        let mut root = Entity::new(ROOT, EntityKind::Container);
        root.children.push(BASE_CHILD);
        document.roots.push(ROOT);
        document.entities.insert(ROOT, root);
        document.entities.insert(
            BASE_CHILD,
            Entity::new(
                BASE_CHILD,
                EntityKind::Shape(nuif_core::ShapeKind::Rectangle),
            ),
        );
        document
    }

    fn create(replica: &str, id: u128, anchor: CreationAnchor) -> CreationChange {
        CreationChange {
            id: ChangeId::new(replica, 1),
            context: BTreeMap::new(),
            operation: CreationOperation::Insert {
                parent: Some(ROOT),
                anchor,
                entity: Box::new(Entity::new(EntityId::new(id), EntityKind::Container)),
            },
        }
    }

    #[test]
    fn concurrent_leaf_creation_converges_and_preserves_base_order() {
        let changes = vec![
            create("alice", 20, CreationAnchor::After(BASE_CHILD)),
            create("bob", 21, CreationAnchor::After(BASE_CHILD)),
            create("carol", 22, CreationAnchor::Start),
        ];
        let mut engine = CreationOperationSetEngine::new(base()).unwrap();
        for change in changes.into_iter().rev() {
            engine.ingest(change).unwrap();
        }
        let checkpoint = engine.checkpoint().unwrap();
        assert_eq!(
            checkpoint.document.entities[&ROOT].children,
            vec![
                EntityId::new(22),
                BASE_CHILD,
                EntityId::new(21),
                EntityId::new(20)
            ]
        );
        assert_eq!(checkpoint.conflicts, Vec::new());
        assert_eq!(checkpoint.document.entities.len(), 5);
        assert!(
            checkpoint
                .document
                .entities
                .values()
                .all(|entity| entity.children.is_empty() || entity.id == ROOT)
        );
    }

    #[test]
    fn concurrent_id_collision_is_explicit_and_deterministic() {
        let mut engine = CreationOperationSetEngine::new(base()).unwrap();
        engine
            .ingest(create("alice", 20, CreationAnchor::Start))
            .unwrap();
        engine
            .ingest(create("bob", 20, CreationAnchor::Start))
            .unwrap();
        let checkpoint = engine.checkpoint().unwrap();
        assert_eq!(checkpoint.document.entities.len(), 3);
        assert!(matches!(
            checkpoint.conflicts.as_slice(),
            [CreationConflict::EntityIdCollision { entity, selected, .. }]
                if *entity == EntityId::new(20)
                    && *selected == ChangeId::new("bob", 1)
        ));
    }

    #[test]
    fn nested_creation_is_rejected_before_ingest() {
        let mut engine = CreationOperationSetEngine::new(base()).unwrap();
        let mut change = create("alice", 20, CreationAnchor::Start);
        let CreationOperation::Insert { entity, .. } = &mut change.operation;
        entity.children.push(BASE_CHILD);
        assert!(matches!(
            engine.ingest(change),
            Err(CreationError::NestedEntity { entity }) if entity == EntityId::new(20)
        ));
    }

    #[test]
    fn unknown_anchor_is_rejected_before_ingest() {
        let mut engine = CreationOperationSetEngine::new(base()).unwrap();
        assert!(matches!(
            engine.ingest(create("alice", 20, CreationAnchor::After(EntityId::new(99)))),
            Err(CreationError::AnchorMissing { anchor }) if anchor == EntityId::new(99)
        ));
    }

    #[test]
    fn incomplete_history_fails_closed() {
        let mut engine = CreationOperationSetEngine::new(base()).unwrap();
        let mut change = create("alice", 20, CreationAnchor::Start);
        change.id.counter = 2;
        change.context.insert("alice".to_owned(), 1);
        engine.ingest(change).unwrap();
        assert!(matches!(
            engine.checkpoint(),
            Err(CreationError::Causal(
                CollaborationError::MissingReplicaChange { .. }
            ))
        ));
    }

    #[test]
    fn complete_stability_frontier_compacts_creation_history() {
        let mut engine = CreationOperationSetEngine::new(base()).unwrap();
        engine
            .ingest(create("alice", 20, CreationAnchor::Start))
            .unwrap();
        let before = engine.checkpoint().unwrap();
        let frontier = StabilityFrontier::new(BTreeMap::from([("alice".to_owned(), 1)])).unwrap();
        let compacted = engine.compact_stable(&frontier).unwrap();
        assert_eq!(compacted.checkpoint, before);
        assert_eq!(compacted.receipt.dropped.len(), 1);
        assert!(compacted.receipt.retained.is_empty());
    }
}
