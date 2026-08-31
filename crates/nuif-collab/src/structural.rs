//! Bounded cycle-safe tree-move collaboration above canonical NUIF.

use super::gc::{CompactionReceipt, StabilityFrontier};
use super::{ChangeId, CollaborationError, MAX_CHANGES, MAX_REPLICAS, validate_replica};
use nuif_codec::canonical_hash;
use nuif_core::{Document, EntityId, Severity, validate};
use nuif_protocol::Anchor;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const PROFILE_NAME: &str = "nuif-collab-tree-0";

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum PositionId {
    Base(EntityId),
    Change(ChangeId),
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "position")]
pub enum StructuralAnchor {
    Start,
    After(PositionId),
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "op")]
pub enum StructuralOperation {
    Move {
        entity: EntityId,
        new_parent: Option<EntityId>,
        anchor: StructuralAnchor,
    },
    Delete {
        entity: EntityId,
    },
}

impl StructuralOperation {
    const fn entity(&self) -> EntityId {
        match self {
            Self::Move { entity, .. } | Self::Delete { entity } => *entity,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuralChange {
    pub id: ChangeId,
    #[serde(default)]
    pub context: BTreeMap<String, u64>,
    pub operation: StructuralOperation,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum StructuralConflict {
    ConcurrentMove {
        entity: EntityId,
        left: ChangeId,
        right: ChangeId,
        selected: ChangeId,
    },
    DeleteMove {
        entity: EntityId,
        deletion: ChangeId,
        movement: ChangeId,
        selected: ChangeId,
    },
    DeleteDescendantMove {
        ancestor: EntityId,
        entity: EntityId,
        deletion: ChangeId,
        movement: ChangeId,
        selected: ChangeId,
    },
    DeletedParent {
        parent: EntityId,
        deletion: ChangeId,
        movement: ChangeId,
        selected: ChangeId,
    },
    CycleRejected {
        change: ChangeId,
        entity: EntityId,
        new_parent: EntityId,
    },
    AnchorUnavailable {
        change: ChangeId,
        anchor: PositionId,
    },
    SelfAnchor {
        change: ChangeId,
        entity: EntityId,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuralCheckpoint {
    pub profile: String,
    pub canonical_hash: String,
    pub document: Document,
    pub applied: Vec<ChangeId>,
    pub conflicts: Vec<StructuralConflict>,
    pub active_positions: BTreeMap<EntityId, PositionId>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuralCompaction {
    pub receipt: CompactionReceipt,
    pub checkpoint: StructuralCheckpoint,
}

impl StructuralCheckpoint {
    /// Resolves a canonical entity anchor to the stable profile position seen
    /// in this checkpoint.
    ///
    /// # Errors
    ///
    /// Returns [`StructuralError::ActivePositionMissing`] if the named entity
    /// is deleted or is not visible in this checkpoint.
    pub fn resolve_anchor(&self, anchor: Anchor) -> Result<StructuralAnchor, StructuralError> {
        match anchor {
            Anchor::Start => Ok(StructuralAnchor::Start),
            Anchor::After(entity) => self
                .active_positions
                .get(&entity)
                .cloned()
                .map(StructuralAnchor::After)
                .ok_or(StructuralError::ActivePositionMissing { entity }),
        }
    }

    /// Builds a stable structural move from a canonical protocol anchor.
    ///
    /// # Errors
    ///
    /// Returns the same error as [`Self::resolve_anchor`].
    pub fn move_operation(
        &self,
        entity: EntityId,
        new_parent: Option<EntityId>,
        anchor: Anchor,
    ) -> Result<StructuralOperation, StructuralError> {
        Ok(StructuralOperation::Move {
            entity,
            new_parent,
            anchor: self.resolve_anchor(anchor)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Error)]
pub enum StructuralError {
    #[error(transparent)]
    Causal(#[from] CollaborationError),
    #[error("structural change identifier {change:?} has conflicting contents")]
    DuplicateChange { change: ChangeId },
    #[error("base document is structurally invalid: {codes:?}")]
    InvalidBase { codes: Vec<String> },
    #[error("structural operation requires unknown entity {entity}")]
    EntityMissing { entity: EntityId },
    #[error("structural operation requires unknown parent {parent}")]
    ParentMissing { parent: EntityId },
    #[error("change {change:?} references missing anchor change {anchor:?}")]
    AnchorChangeMissing { change: ChangeId, anchor: ChangeId },
    #[error("change {change:?} references anchor change {anchor:?} outside its causal history")]
    AnchorNotCausal { change: ChangeId, anchor: ChangeId },
    #[error("entity {entity} has no visible collaboration position")]
    ActivePositionMissing { entity: EntityId },
    #[error("structural checkpoint is invalid: {codes:?}")]
    InvalidCheckpoint { codes: Vec<String> },
    #[error("structural base hashes differ: {left} != {right}")]
    BaseMismatch { left: String, right: String },
    #[error("canonical hashing failed: {0}")]
    Canonical(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructuralOperationSetEngine {
    base: Document,
    base_hash: String,
    changes: BTreeMap<ChangeId, StructuralChange>,
}

impl StructuralOperationSetEngine {
    /// Creates an operation-set materializer bound to one canonical base.
    ///
    /// # Errors
    ///
    /// Rejects an invalid or unhashable base document.
    pub fn new(base: Document) -> Result<Self, StructuralError> {
        TreeState::new(&base)?;
        let base_hash =
            canonical_hash(&base).map_err(|error| StructuralError::Canonical(error.to_string()))?;
        Ok(Self {
            base,
            base_hash,
            changes: BTreeMap::new(),
        })
    }

    /// Adds one idempotent structural change.
    ///
    /// # Errors
    ///
    /// Rejects malformed clocks, resource overflow, or conflicting identifier
    /// reuse before changing the operation set.
    pub fn ingest(&mut self, change: StructuralChange) -> Result<bool, StructuralError> {
        validate_change_shape(&change)?;
        validate_operation_identities(&self.base, &change.operation)?;
        if let Some(existing) = self.changes.get(&change.id) {
            return if existing == &change {
                Ok(false)
            } else {
                Err(StructuralError::DuplicateChange { change: change.id })
            };
        }
        if self.changes.len() == MAX_CHANGES {
            return Err(CollaborationError::TooManyChanges.into());
        }
        self.changes.insert(change.id.clone(), change);
        Ok(true)
    }

    /// Atomically joins another structural operation set.
    ///
    /// # Errors
    ///
    /// Returns the same typed failures as [`Self::ingest`].
    pub fn merge(&mut self, other: &Self) -> Result<(), StructuralError> {
        if self.base_hash != other.base_hash {
            return Err(StructuralError::BaseMismatch {
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

    /// Materializes all changes in their unique Lamport order.
    ///
    /// # Errors
    ///
    /// Rejects incomplete causal history, unknown identities, an invalid base,
    /// or a canonical checkpoint whose retained model invariants fail.
    pub fn checkpoint(&self) -> Result<StructuralCheckpoint, StructuralError> {
        validate_collection(self.changes.values())?;
        let mut state = TreeState::new(&self.base)?;
        for change in self.changes.values() {
            state.apply(change)?;
        }
        finish_checkpoint(&self.base, self.changes.values(), state)
    }

    /// Replaces a complete, causally stable structural history with its
    /// metadata-free canonical checkpoint.
    ///
    /// Profile 0 intentionally rejects partial collection. The frontier must
    /// exactly cover every locally observed structural change; this avoids
    /// rebasing position anchors and causal contexts without a versioned
    /// checkpoint protocol.
    ///
    /// # Errors
    ///
    /// Rejects incomplete causal history, an unsafe stability frontier or an
    /// invalid resulting checkpoint.
    pub fn compact_stable(
        &self,
        frontier: &StabilityFrontier,
    ) -> Result<StructuralCompaction, StructuralError> {
        let checkpoint = self.checkpoint()?;
        frontier.validate_complete(self.changes.keys())?;
        let receipt = CompactionReceipt::complete_history(
            PROFILE_NAME,
            self.base_hash.clone(),
            checkpoint.canonical_hash.clone(),
            frontier,
            self.changes.keys().cloned().collect(),
        );
        Ok(StructuralCompaction {
            receipt,
            checkpoint,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructuralUndoRedoEngine {
    base: Document,
    base_hash: String,
    changes: BTreeMap<ChangeId, StructuralChange>,
    order: Vec<ChangeId>,
    state: TreeState,
}

impl StructuralUndoRedoEngine {
    /// Creates an incremental rollback/replay materializer over one base.
    ///
    /// # Errors
    ///
    /// Rejects an invalid or unhashable base document.
    pub fn new(base: Document) -> Result<Self, StructuralError> {
        let initial = TreeState::new(&base)?;
        let base_hash =
            canonical_hash(&base).map_err(|error| StructuralError::Canonical(error.to_string()))?;
        Ok(Self {
            base,
            base_hash,
            changes: BTreeMap::new(),
            order: Vec::new(),
            state: initial,
        })
    }

    /// Inserts one change at its total-order position, rolls back to the prior
    /// state, then replays the affected suffix.
    ///
    /// # Errors
    ///
    /// Rejects malformed clocks, resource overflow, unknown identities, or
    /// conflicting identifier reuse without mutating this engine.
    pub fn ingest(&mut self, change: StructuralChange) -> Result<bool, StructuralError> {
        validate_change_shape(&change)?;
        validate_operation_identities(&self.base, &change.operation)?;
        if let Some(existing) = self.changes.get(&change.id) {
            return if existing == &change {
                Ok(false)
            } else {
                Err(StructuralError::DuplicateChange { change: change.id })
            };
        }
        if self.changes.len() == MAX_CHANGES {
            return Err(CollaborationError::TooManyChanges.into());
        }
        let index = self.order.partition_point(|id| id < &change.id);
        if index == self.order.len() {
            self.state.apply(&change)?;
            self.order.push(change.id.clone());
            self.changes.insert(change.id.clone(), change);
            return Ok(true);
        }
        let mut next_order = self.order.clone();
        next_order.insert(index, change.id.clone());
        let mut next_changes = self.changes.clone();
        next_changes.insert(change.id.clone(), change);
        let mut next_state = TreeState::new(&self.base)?;
        for id in &next_order {
            next_state.apply(&next_changes[id])?;
        }
        self.order = next_order;
        self.changes = next_changes;
        self.state = next_state;
        Ok(true)
    }

    /// Atomically joins an engine with the same canonical base.
    ///
    /// # Errors
    ///
    /// Rejects different bases or any failure from [`Self::ingest`].
    pub fn merge(&mut self, other: &Self) -> Result<(), StructuralError> {
        if self.base_hash != other.base_hash {
            return Err(StructuralError::BaseMismatch {
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

    /// Returns the metadata-free canonical checkpoint and explicit conflicts.
    ///
    /// # Errors
    ///
    /// Rejects incomplete causal history or an invalid canonical checkpoint.
    pub fn checkpoint(&self) -> Result<StructuralCheckpoint, StructuralError> {
        validate_collection(self.changes.values())?;
        finish_checkpoint(&self.base, self.changes.values(), self.state.clone())
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Parent {
    Root,
    Entity(EntityId),
    Trash,
}

#[derive(Clone, Debug, PartialEq)]
struct Position {
    entity: EntityId,
    parent: Parent,
    origin: Option<PositionId>,
    active: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct TreeState {
    parents: BTreeMap<EntityId, Parent>,
    active: BTreeMap<EntityId, PositionId>,
    positions: BTreeMap<PositionId, Position>,
    conflicts: BTreeSet<StructuralConflict>,
}

impl TreeState {
    fn new(base: &Document) -> Result<Self, StructuralError> {
        let codes = error_codes(base);
        if !codes.is_empty() {
            return Err(StructuralError::InvalidBase { codes });
        }
        let mut state = Self {
            parents: BTreeMap::new(),
            active: BTreeMap::new(),
            positions: BTreeMap::new(),
            conflicts: BTreeSet::new(),
        };
        state.add_base_list(&Parent::Root, &base.roots);
        for (parent, entity) in &base.entities {
            state.add_base_list(&Parent::Entity(*parent), &entity.children);
        }
        Ok(state)
    }

    fn add_base_list(&mut self, parent: &Parent, entities: &[EntityId]) {
        let mut origin = None;
        for entity in entities {
            let id = PositionId::Base(*entity);
            self.parents.insert(*entity, parent.clone());
            self.active.insert(*entity, id.clone());
            self.positions.insert(
                id.clone(),
                Position {
                    entity: *entity,
                    parent: parent.clone(),
                    origin,
                    active: true,
                },
            );
            origin = Some(id);
        }
    }

    fn apply(&mut self, change: &StructuralChange) -> Result<(), StructuralError> {
        let entity = change.operation.entity();
        if !self.parents.contains_key(&entity) {
            return Err(StructuralError::EntityMissing { entity });
        }
        match &change.operation {
            StructuralOperation::Delete { entity } => {
                self.delete(*entity);
                Ok(())
            }
            StructuralOperation::Move {
                entity,
                new_parent,
                anchor,
            } => self.move_entity(change, *entity, *new_parent, anchor),
        }
    }

    fn delete(&mut self, entity: EntityId) {
        self.deactivate(entity);
        self.parents.insert(entity, Parent::Trash);
    }

    fn move_entity(
        &mut self,
        change: &StructuralChange,
        entity: EntityId,
        new_parent: Option<EntityId>,
        anchor: &StructuralAnchor,
    ) -> Result<(), StructuralError> {
        if let Some(parent) = new_parent
            && !self.parents.contains_key(&parent)
        {
            return Err(StructuralError::ParentMissing { parent });
        }
        if new_parent.is_some_and(|parent| parent == entity || self.is_ancestor(entity, parent)) {
            self.conflicts.insert(StructuralConflict::CycleRejected {
                change: change.id.clone(),
                entity,
                new_parent: new_parent.expect("a cycle requires an entity parent"),
            });
            return Ok(());
        }
        let parent = new_parent.map_or(Parent::Root, Parent::Entity);
        let origin = match anchor {
            StructuralAnchor::Start => None,
            StructuralAnchor::After(position) => {
                let Some(anchor_position) = self.positions.get(position) else {
                    self.conflicts
                        .insert(StructuralConflict::AnchorUnavailable {
                            change: change.id.clone(),
                            anchor: position.clone(),
                        });
                    return Ok(());
                };
                if anchor_position.parent != parent {
                    self.conflicts
                        .insert(StructuralConflict::AnchorUnavailable {
                            change: change.id.clone(),
                            anchor: position.clone(),
                        });
                    return Ok(());
                }
                if anchor_position.entity == entity {
                    self.conflicts.insert(StructuralConflict::SelfAnchor {
                        change: change.id.clone(),
                        entity,
                    });
                    return Ok(());
                }
                Some(position.clone())
            }
        };
        self.deactivate(entity);
        let position_id = PositionId::Change(change.id.clone());
        self.positions.insert(
            position_id.clone(),
            Position {
                entity,
                parent: parent.clone(),
                origin,
                active: true,
            },
        );
        self.active.insert(entity, position_id);
        self.parents.insert(entity, parent);
        Ok(())
    }

    fn deactivate(&mut self, entity: EntityId) {
        if let Some(position) = self.active.remove(&entity)
            && let Some(position) = self.positions.get_mut(&position)
        {
            position.active = false;
        }
    }

    fn is_ancestor(&self, ancestor: EntityId, candidate: EntityId) -> bool {
        let mut current = Some(candidate);
        let mut visited = BTreeSet::new();
        while let Some(entity) = current {
            if entity == ancestor {
                return true;
            }
            if !visited.insert(entity) {
                return true;
            }
            current = match self.parents.get(&entity) {
                Some(Parent::Entity(parent)) => Some(*parent),
                Some(Parent::Root | Parent::Trash) | None => None,
            };
        }
        false
    }

    fn ordered_children(&self) -> BTreeMap<Parent, Vec<EntityId>> {
        let mut grouped = BTreeMap::<Parent, BTreeMap<Option<PositionId>, Vec<PositionId>>>::new();
        for (id, position) in &self.positions {
            grouped
                .entry(position.parent.clone())
                .or_default()
                .entry(position.origin.clone())
                .or_default()
                .push(id.clone());
        }
        grouped
            .into_iter()
            .map(|(parent, mut descendants)| {
                for positions in descendants.values_mut() {
                    positions.sort();
                }
                let mut stack = descendants.remove(&None).unwrap_or_default();
                let mut ordered = Vec::new();
                while let Some(id) = stack.pop() {
                    if let Some(position) = self.positions.get(&id) {
                        if position.active && self.parents.get(&position.entity) == Some(&parent) {
                            ordered.push(position.entity);
                        }
                        if let Some(mut children) = descendants.remove(&Some(id)) {
                            stack.append(&mut children);
                        }
                    }
                }
                (parent, ordered)
            })
            .collect()
    }

    fn materialize(
        &self,
        base: &Document,
    ) -> Result<(Document, BTreeMap<EntityId, PositionId>), StructuralError> {
        let mut document = base.clone();
        let mut ordered = self.ordered_children();
        document.roots = ordered.remove(&Parent::Root).unwrap_or_default();
        for (entity, value) in &mut document.entities {
            value.children = ordered.remove(&Parent::Entity(*entity)).unwrap_or_default();
        }
        let mut reachable = BTreeSet::new();
        let mut pending = document.roots.clone();
        while let Some(entity) = pending.pop() {
            if reachable.insert(entity)
                && let Some(value) = document.entities.get(&entity)
            {
                pending.extend(value.children.iter().copied());
            }
        }
        document
            .entities
            .retain(|entity, _| reachable.contains(entity));
        document.relations.retain(|relation| {
            reachable.contains(&relation.source) && reachable.contains(&relation.target)
        });
        let codes = error_codes(&document);
        if !codes.is_empty() {
            return Err(StructuralError::InvalidCheckpoint { codes });
        }
        let active = self
            .active
            .iter()
            .filter(|(entity, _)| reachable.contains(entity))
            .map(|(entity, position)| (*entity, position.clone()))
            .collect();
        Ok((document, active))
    }
}

fn finish_checkpoint<'a>(
    base: &Document,
    changes: impl Iterator<Item = &'a StructuralChange> + Clone,
    state: TreeState,
) -> Result<StructuralCheckpoint, StructuralError> {
    let mut conflicts = semantic_conflicts(base, changes.clone());
    let (document, active_positions) = state.materialize(base)?;
    conflicts.extend(state.conflicts);
    Ok(StructuralCheckpoint {
        profile: PROFILE_NAME.to_owned(),
        canonical_hash: canonical_hash(&document)
            .map_err(|error| StructuralError::Canonical(error.to_string()))?,
        document,
        applied: changes.map(|change| change.id.clone()).collect(),
        conflicts: conflicts.into_iter().collect(),
        active_positions,
    })
}

fn error_codes(document: &Document) -> Vec<String> {
    validate(document)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| diagnostic.code)
        .collect()
}

fn validate_change_shape(change: &StructuralChange) -> Result<(), StructuralError> {
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
    Ok(())
}

fn validate_operation_identities(
    base: &Document,
    operation: &StructuralOperation,
) -> Result<(), StructuralError> {
    let entity = operation.entity();
    if !base.entities.contains_key(&entity) {
        return Err(StructuralError::EntityMissing { entity });
    }
    if let StructuralOperation::Move {
        new_parent: Some(parent),
        ..
    } = operation
        && !base.entities.contains_key(parent)
    {
        return Err(StructuralError::ParentMissing { parent: *parent });
    }
    Ok(())
}

fn validate_collection<'a>(
    changes: impl Iterator<Item = &'a StructuralChange> + Clone,
) -> Result<(), StructuralError> {
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
    validate_contiguous(&received, &replicas)?;
    validate_dependencies(&changes, &received)?;
    validate_anchor_dependencies(&changes, &received)?;
    Ok(())
}

fn validate_contiguous(
    received: &BTreeMap<(&str, u64), &StructuralChange>,
    replicas: &BTreeSet<&str>,
) -> Result<(), StructuralError> {
    for replica in replicas {
        for (expected, counter) in (1_u64..).zip(
            received
                .keys()
                .filter_map(|(candidate, counter)| (candidate == replica).then_some(*counter)),
        ) {
            if counter != expected {
                return Err(CollaborationError::MissingReplicaChange {
                    replica: (*replica).to_owned(),
                    counter: expected,
                }
                .into());
            }
        }
    }
    Ok(())
}

fn validate_dependencies(
    changes: &[&StructuralChange],
    received: &BTreeMap<(&str, u64), &StructuralChange>,
) -> Result<(), StructuralError> {
    for change in changes {
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

fn validate_anchor_dependencies(
    changes: &[&StructuralChange],
    received: &BTreeMap<(&str, u64), &StructuralChange>,
) -> Result<(), StructuralError> {
    for change in changes {
        let StructuralOperation::Move {
            anchor: StructuralAnchor::After(PositionId::Change(anchor)),
            ..
        } = &change.operation
        else {
            continue;
        };
        let anchor_change = received
            .get(&(anchor.replica.as_str(), anchor.counter))
            .copied()
            .ok_or_else(|| StructuralError::AnchorChangeMissing {
                change: change.id.clone(),
                anchor: anchor.clone(),
            })?;
        if anchor_change.id != *anchor || !happens_before(anchor_change, change) {
            return Err(StructuralError::AnchorNotCausal {
                change: change.id.clone(),
                anchor: anchor.clone(),
            });
        }
    }
    Ok(())
}

fn happens_before(left: &StructuralChange, right: &StructuralChange) -> bool {
    right
        .context
        .get(&left.id.replica)
        .is_some_and(|counter| *counter >= left.id.counter)
}

fn concurrent(left: &StructuralChange, right: &StructuralChange) -> bool {
    !happens_before(left, right) && !happens_before(right, left)
}

fn semantic_conflicts<'a>(
    base: &Document,
    changes: impl Iterator<Item = &'a StructuralChange>,
) -> BTreeSet<StructuralConflict> {
    let changes = changes.collect::<Vec<_>>();
    let frontiers = structural_frontiers(&changes);
    let base_parents = base
        .entities
        .iter()
        .flat_map(|(parent, entity)| entity.children.iter().map(move |child| (*child, *parent)))
        .collect::<BTreeMap<_, _>>();
    let mut conflicts = BTreeSet::new();
    for frontier in frontiers.values() {
        for (index, left) in frontier.iter().enumerate() {
            for right in &frontier[index + 1..] {
                add_same_entity_conflict(&mut conflicts, left, right);
            }
        }
    }
    let deletions = deletion_frontiers(&frontiers);
    for frontier in frontiers.values() {
        for movement in frontier {
            let StructuralOperation::Move {
                entity, new_parent, ..
            } = &movement.operation
            else {
                continue;
            };
            if let Some(parent) = new_parent
                && let Some(parent_deletions) = deletions.get(parent)
            {
                for deletion in parent_deletions {
                    if concurrent(deletion, movement) {
                        conflicts.insert(StructuralConflict::DeletedParent {
                            parent: *parent,
                            deletion: deletion.id.clone(),
                            movement: movement.id.clone(),
                            selected: deletion.id.clone().max(movement.id.clone()),
                        });
                    }
                }
            }
            add_deleted_ancestor_conflicts(
                &mut conflicts,
                &base_parents,
                &deletions,
                *entity,
                movement,
            );
        }
    }
    conflicts
}

fn structural_frontiers<'a>(
    changes: &[&'a StructuralChange],
) -> BTreeMap<EntityId, Vec<&'a StructuralChange>> {
    let mut grouped = BTreeMap::<EntityId, Vec<&StructuralChange>>::new();
    for change in changes {
        grouped
            .entry(change.operation.entity())
            .or_default()
            .push(*change);
    }
    grouped
        .into_iter()
        .map(|(entity, candidates)| {
            let mut observed = BTreeMap::<&str, u64>::new();
            for candidate in &candidates {
                for (replica, counter) in &candidate.context {
                    observed
                        .entry(replica)
                        .and_modify(|value| *value = (*value).max(*counter))
                        .or_insert(*counter);
                }
            }
            let frontier = candidates
                .into_iter()
                .filter(|candidate| {
                    observed
                        .get(candidate.id.replica.as_str())
                        .copied()
                        .unwrap_or(0)
                        < candidate.id.counter
                })
                .collect();
            (entity, frontier)
        })
        .collect()
}

fn deletion_frontiers<'a>(
    frontiers: &BTreeMap<EntityId, Vec<&'a StructuralChange>>,
) -> BTreeMap<EntityId, Vec<&'a StructuralChange>> {
    frontiers
        .iter()
        .filter_map(|(entity, changes)| {
            let deletions = changes
                .iter()
                .copied()
                .filter(|change| matches!(change.operation, StructuralOperation::Delete { .. }))
                .collect::<Vec<_>>();
            (!deletions.is_empty()).then_some((*entity, deletions))
        })
        .collect()
}

fn add_same_entity_conflict(
    conflicts: &mut BTreeSet<StructuralConflict>,
    left: &StructuralChange,
    right: &StructuralChange,
) {
    if left.operation == right.operation || !concurrent(left, right) {
        return;
    }
    let entity = left.operation.entity();
    let selected = left.id.clone().max(right.id.clone());
    match (&left.operation, &right.operation) {
        (StructuralOperation::Move { .. }, StructuralOperation::Move { .. }) => {
            conflicts.insert(StructuralConflict::ConcurrentMove {
                entity,
                left: left.id.clone().min(right.id.clone()),
                right: left.id.clone().max(right.id.clone()),
                selected,
            });
        }
        (StructuralOperation::Delete { .. }, StructuralOperation::Move { .. }) => {
            conflicts.insert(StructuralConflict::DeleteMove {
                entity,
                deletion: left.id.clone(),
                movement: right.id.clone(),
                selected,
            });
        }
        (StructuralOperation::Move { .. }, StructuralOperation::Delete { .. }) => {
            conflicts.insert(StructuralConflict::DeleteMove {
                entity,
                deletion: right.id.clone(),
                movement: left.id.clone(),
                selected,
            });
        }
        (StructuralOperation::Delete { .. }, StructuralOperation::Delete { .. }) => {}
    }
}

fn add_deleted_ancestor_conflicts(
    conflicts: &mut BTreeSet<StructuralConflict>,
    base_parents: &BTreeMap<EntityId, EntityId>,
    deletions: &BTreeMap<EntityId, Vec<&StructuralChange>>,
    entity: EntityId,
    movement: &StructuralChange,
) {
    let mut ancestor = base_parents.get(&entity).copied();
    while let Some(current) = ancestor {
        if let Some(candidates) = deletions.get(&current) {
            for deletion in candidates {
                if concurrent(deletion, movement) {
                    conflicts.insert(StructuralConflict::DeleteDescendantMove {
                        ancestor: current,
                        entity,
                        deletion: deletion.id.clone(),
                        movement: movement.id.clone(),
                        selected: deletion.id.clone().max(movement.id.clone()),
                    });
                }
            }
        }
        ancestor = base_parents.get(&current).copied();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nuif_core::{Entity, EntityKind};

    fn base() -> Document {
        let mut document = Document::empty(EntityId::new(1));
        let root = EntityId::new(10);
        let left = EntityId::new(11);
        let right = EntityId::new(12);
        let leaf = EntityId::new(13);
        let mut root_entity = Entity::new(root, EntityKind::Container);
        root_entity.children.extend([left, right]);
        let mut left_entity = Entity::new(left, EntityKind::Container);
        left_entity.children.push(leaf);
        document.roots.push(root);
        document.entities.insert(root, root_entity);
        document.entities.insert(left, left_entity);
        document
            .entities
            .insert(right, Entity::new(right, EntityKind::Container));
        document
            .entities
            .insert(leaf, Entity::new(leaf, EntityKind::Container));
        document
    }

    fn change(
        replica: &str,
        counter: u64,
        context: &[(&str, u64)],
        operation: StructuralOperation,
    ) -> StructuralChange {
        StructuralChange {
            id: ChangeId::new(replica, counter),
            context: context
                .iter()
                .map(|(replica, counter)| ((*replica).to_owned(), *counter))
                .collect(),
            operation,
        }
    }

    fn move_to(
        replica: &str,
        counter: u64,
        entity: u128,
        parent: Option<u128>,
        anchor: StructuralAnchor,
    ) -> StructuralChange {
        change(
            replica,
            counter,
            &[],
            StructuralOperation::Move {
                entity: EntityId::new(entity),
                new_parent: parent.map(EntityId::new),
                anchor,
            },
        )
    }

    #[test]
    fn operation_set_and_undo_redo_converge_across_deliveries() {
        let changes = vec![
            move_to("alice", 1, 13, Some(12), StructuralAnchor::Start),
            move_to(
                "bob",
                1,
                11,
                None,
                StructuralAnchor::After(PositionId::Base(EntityId::new(10))),
            ),
            change(
                "carol",
                1,
                &[],
                StructuralOperation::Delete {
                    entity: EntityId::new(12),
                },
            ),
        ];
        let mut expected = None;
        for order in permutations(&changes) {
            let base = base();
            let mut set = StructuralOperationSetEngine::new(base.clone()).unwrap();
            let mut incremental = StructuralUndoRedoEngine::new(base).unwrap();
            for item in order {
                set.ingest(item.clone()).unwrap();
                incremental.ingest(item.clone()).unwrap();
            }
            let left = set.checkpoint().unwrap();
            let right = incremental.checkpoint().unwrap();
            assert_eq!(left, right);
            if let Some(expected) = &expected {
                assert_eq!(expected, &left);
            } else {
                expected = Some(left);
            }
        }
    }

    #[test]
    fn reciprocal_moves_keep_tree_acyclic_and_report_rejection() {
        let mut engine = StructuralOperationSetEngine::new(base()).unwrap();
        engine
            .ingest(move_to("alice", 1, 11, Some(12), StructuralAnchor::Start))
            .unwrap();
        engine
            .ingest(move_to("bob", 1, 12, Some(11), StructuralAnchor::Start))
            .unwrap();
        let checkpoint = engine.checkpoint().unwrap();
        assert!(
            checkpoint
                .conflicts
                .iter()
                .any(|conflict| matches!(conflict, StructuralConflict::CycleRejected { .. }))
        );
        assert!(error_codes(&checkpoint.document).is_empty());
    }

    #[test]
    fn concurrent_moves_and_delete_move_remain_explicit() {
        let mut engine = StructuralOperationSetEngine::new(base()).unwrap();
        for item in [
            move_to("alice", 1, 13, Some(12), StructuralAnchor::Start),
            move_to("bob", 1, 13, None, StructuralAnchor::Start),
            change(
                "carol",
                1,
                &[],
                StructuralOperation::Delete {
                    entity: EntityId::new(13),
                },
            ),
        ] {
            engine.ingest(item).unwrap();
        }
        let checkpoint = engine.checkpoint().unwrap();
        assert!(
            checkpoint
                .conflicts
                .iter()
                .any(|conflict| matches!(conflict, StructuralConflict::ConcurrentMove { .. }))
        );
        assert!(
            checkpoint
                .conflicts
                .iter()
                .any(|conflict| matches!(conflict, StructuralConflict::DeleteMove { .. }))
        );
    }

    #[test]
    fn same_origin_positions_use_descending_change_order() {
        let mut engine = StructuralOperationSetEngine::new(base()).unwrap();
        for item in [
            move_to(
                "alice",
                1,
                13,
                Some(10),
                StructuralAnchor::After(PositionId::Base(EntityId::new(11))),
            ),
            move_to(
                "bob",
                1,
                12,
                Some(10),
                StructuralAnchor::After(PositionId::Base(EntityId::new(11))),
            ),
        ] {
            engine.ingest(item).unwrap();
        }
        let checkpoint = engine.checkpoint().unwrap();
        assert_eq!(
            checkpoint.document.entities[&EntityId::new(10)].children,
            [EntityId::new(11), EntityId::new(12), EntityId::new(13),]
        );
    }

    #[test]
    fn stable_anchor_resolution_rejects_deleted_entities() {
        let mut engine = StructuralOperationSetEngine::new(base()).unwrap();
        engine
            .ingest(change(
                "alice",
                1,
                &[],
                StructuralOperation::Delete {
                    entity: EntityId::new(12),
                },
            ))
            .unwrap();
        let checkpoint = engine.checkpoint().unwrap();
        assert!(matches!(
            checkpoint.resolve_anchor(Anchor::After(EntityId::new(12))),
            Err(StructuralError::ActivePositionMissing { .. })
        ));
    }

    #[test]
    fn complete_stability_frontier_compacts_structural_history() {
        let mut engine = StructuralOperationSetEngine::new(base()).unwrap();
        engine
            .ingest(move_to("alice", 1, 13, Some(12), StructuralAnchor::Start))
            .unwrap();
        let before = engine.checkpoint().unwrap();
        let frontier = StabilityFrontier::new(BTreeMap::from([("alice".to_owned(), 1)])).unwrap();
        let compacted = engine.compact_stable(&frontier).unwrap();
        assert_eq!(compacted.checkpoint, before);
        assert_eq!(compacted.receipt.dropped.len(), 1);
        assert_eq!(compacted.receipt.compacted_base_hash, before.canonical_hash);
    }

    #[test]
    fn partial_structural_compaction_fails_closed() {
        let mut engine = StructuralOperationSetEngine::new(base()).unwrap();
        engine
            .ingest(move_to("alice", 1, 13, Some(12), StructuralAnchor::Start))
            .unwrap();
        let frontier = StabilityFrontier::new(BTreeMap::new()).unwrap();
        assert!(matches!(
            engine.compact_stable(&frontier),
            Err(StructuralError::Causal(
                CollaborationError::UnsafeCompaction {
                    replica,
                    observed: 1,
                    frontier: 0,
                }
            )) if replica == "alice"
        ));
    }

    fn permutations<T: Clone>(values: &[T]) -> Vec<Vec<T>> {
        fn visit<T: Clone>(values: &mut [T], index: usize, output: &mut Vec<Vec<T>>) {
            if index == values.len() {
                output.push(values.to_vec());
                return;
            }
            for candidate in index..values.len() {
                values.swap(index, candidate);
                visit(values, index + 1, output);
                values.swap(index, candidate);
            }
        }
        let mut values = values.to_vec();
        let mut output = Vec::new();
        visit(&mut values, 0, &mut output);
        output
    }
}
