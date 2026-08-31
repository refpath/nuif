//! Bounded causal nested-creation collaboration above a canonical base.
//!
//! This profile extends the leaf-creation profile without changing its
//! contract. A created entity may be the parent of a later creation only when
//! the child causally names the selected parent dot. Created parents support a
//! `Start` child anchor; base-parent sibling insertion keeps the original
//! `Start`/`After(base)` semantics.

use super::creation::{CreationAnchor, CreationChange, CreationConflict, CreationOperation};
use super::structural::PositionId;
use super::{ChangeId, CollaborationError, MAX_CHANGES, MAX_REPLICAS, validate_replica};
use nuif_codec::canonical_hash;
use nuif_core::{Document, Entity, EntityId, Severity, validate};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

/// Stable profile identifier for causal nested creation.
pub const PROFILE_NAME: &str = "nuif-collab-tree-create-nested-0";
/// Maximum selected-parent depth in this bounded profile.
pub const MAX_PARENT_DEPTH: usize = 64;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NestedCreationCheckpoint {
    pub profile: String,
    pub canonical_hash: String,
    pub document: Document,
    pub applied: Vec<ChangeId>,
    pub conflicts: Vec<CreationConflict>,
    pub active_positions: BTreeMap<EntityId, PositionId>,
}

#[derive(Clone, Debug, PartialEq, Error)]
pub enum NestedCreationError {
    #[error(transparent)]
    Causal(#[from] CollaborationError),
    #[error("nested creation change identifier {change:?} has conflicting contents")]
    DuplicateChange { change: ChangeId },
    #[error("base document is structurally invalid: {codes:?}")]
    InvalidBase { codes: Vec<String> },
    #[error("created entity {entity} already exists in the base document")]
    EntityAlreadyExists { entity: EntityId },
    #[error("created entity {entity} must be a leaf with no children")]
    NestedEntity { entity: EntityId },
    #[error("creation parent {parent} is not present in the base or selected history")]
    ParentMissing { change: ChangeId, parent: EntityId },
    #[error("creation {change:?} does not causally include its selected parent {parent}")]
    ParentNotCausal { change: ChangeId, parent: EntityId },
    #[error("creation {change:?} creates a parent cycle through {parent}")]
    ParentCycle { change: ChangeId, parent: EntityId },
    #[error("creation {change:?} exceeds the {MAX_PARENT_DEPTH}-level parent depth limit")]
    ParentDepthExceeded { change: ChangeId },
    #[error("creation {change:?} anchor {anchor:?} is not a base sibling of its base parent")]
    AnchorMissing { change: ChangeId, anchor: EntityId },
    #[error("creation {change:?} may use only Start below a created parent")]
    CreatedParentAnchor { change: ChangeId },
    #[error("created entity {entity} is invalid: {codes:?}")]
    InvalidEntity {
        entity: EntityId,
        codes: Vec<String>,
    },
    #[error("nested creation checkpoint is invalid: {codes:?}")]
    InvalidCheckpoint { codes: Vec<String> },
    #[error("nested creation engines are bound to different base hashes: {left} != {right}")]
    BaseMismatch { left: String, right: String },
    #[error("canonical hashing failed: {0}")]
    Canonical(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct NestedCreationOperationSetEngine {
    base: Document,
    base_hash: String,
    changes: BTreeMap<ChangeId, CreationChange>,
}

impl NestedCreationOperationSetEngine {
    /// Creates an engine bound to one canonical base document.
    ///
    /// # Errors
    ///
    /// Rejects invalid or unhashable base documents.
    pub fn new(base: Document) -> Result<Self, NestedCreationError> {
        let codes = error_codes(&base);
        if !codes.is_empty() {
            return Err(NestedCreationError::InvalidBase { codes });
        }
        let base_hash = canonical_hash(&base)
            .map_err(|error| NestedCreationError::Canonical(error.to_string()))?;
        Ok(Self {
            base,
            base_hash,
            changes: BTreeMap::new(),
        })
    }

    /// Adds one idempotent creation change.
    ///
    /// # Errors
    ///
    /// Rejects malformed clocks, invalid payloads, base anchors and identifier
    /// reuse. A parent not present in the base is resolved at checkpoint time.
    pub fn ingest(&mut self, change: CreationChange) -> Result<bool, NestedCreationError> {
        validate_change_shape(&self.base, &change)?;
        if let Some(existing) = self.changes.get(&change.id) {
            return if existing == &change {
                Ok(false)
            } else {
                Err(NestedCreationError::DuplicateChange { change: change.id })
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
    pub fn merge(&mut self, other: &Self) -> Result<(), NestedCreationError> {
        if self.base_hash != other.base_hash {
            return Err(NestedCreationError::BaseMismatch {
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

    /// Materializes a metadata-free checkpoint with causally nested parents.
    ///
    /// # Errors
    ///
    /// Rejects incomplete causal history, unavailable/non-causal parents,
    /// invalid anchors, parent cycles or an invalid resulting checkpoint.
    ///
    /// # Panics
    ///
    /// Internal grouping invariants panic only if a non-empty selected change
    /// group unexpectedly has no winner.
    pub fn checkpoint(&self) -> Result<NestedCreationCheckpoint, NestedCreationError> {
        validate_collection(self.changes.values())?;
        let mut by_entity = BTreeMap::<EntityId, Vec<&CreationChange>>::new();
        for change in self.changes.values() {
            by_entity.entry(entity(change)).or_default().push(change);
        }
        let mut selected = BTreeMap::<EntityId, &CreationChange>::new();
        let mut conflicts = Vec::new();
        for (id, mut candidates) in by_entity {
            candidates.sort_by(|left, right| left.id.cmp(&right.id));
            let winner = candidates.last().expect("group is non-empty");
            if candidates.len() > 1 {
                conflicts.push(CreationConflict::EntityIdCollision {
                    entity: id,
                    candidates: candidates
                        .iter()
                        .map(|candidate| candidate.id.clone())
                        .collect(),
                    selected: winner.id.clone(),
                });
            }
            selected.insert(id, *winner);
        }

        let mut visiting = BTreeSet::new();
        let mut valid = BTreeSet::new();
        for id in selected.keys().copied() {
            validate_selected(&self.base, &selected, id, 0, &mut visiting, &mut valid)?;
        }

        let mut document = self.base.clone();
        let mut active_positions = BTreeMap::new();
        let mut insertions =
            BTreeMap::<(Option<EntityId>, CreationAnchor), Vec<&CreationChange>>::new();
        for (id, change) in &selected {
            if !valid.contains(id) {
                continue;
            }
            let inserted = change_entity(change);
            let (parent, anchor) = parent_and_anchor(change);
            document.entities.insert(*id, inserted);
            active_positions.insert(*id, PositionId::Change(change.id.clone()));
            insertions
                .entry((parent, anchor))
                .or_default()
                .push(*change);
        }
        for values in insertions.values_mut() {
            values.sort_by(|left, right| right.id.cmp(&left.id));
        }
        document.roots = materialize_list(&document.roots, None, &insertions);
        let base_children = self
            .base
            .entities
            .iter()
            .map(|(id, value)| (*id, value.children.clone()))
            .collect::<BTreeMap<_, _>>();
        for (id, value) in &mut document.entities {
            let children = base_children.get(id).map_or(&[][..], Vec::as_slice);
            value.children = materialize_list(children, Some(*id), &insertions);
        }
        let codes = error_codes(&document);
        if !codes.is_empty() {
            return Err(NestedCreationError::InvalidCheckpoint { codes });
        }
        let canonical_hash = canonical_hash(&document)
            .map_err(|error| NestedCreationError::Canonical(error.to_string()))?;
        Ok(NestedCreationCheckpoint {
            profile: PROFILE_NAME.to_owned(),
            canonical_hash,
            document,
            applied: self.changes.keys().cloned().collect(),
            conflicts,
            active_positions,
        })
    }
}

fn validate_selected(
    base: &Document,
    selected: &BTreeMap<EntityId, &CreationChange>,
    id: EntityId,
    depth: usize,
    visiting: &mut BTreeSet<EntityId>,
    valid: &mut BTreeSet<EntityId>,
) -> Result<(), NestedCreationError> {
    if valid.contains(&id) {
        return Ok(());
    }
    let change = selected.get(&id).copied().expect("selected ID exists");
    if depth >= MAX_PARENT_DEPTH {
        return Err(NestedCreationError::ParentDepthExceeded {
            change: change.id.clone(),
        });
    }
    if !visiting.insert(id) {
        let (Some(parent), _) = parent_and_anchor(change) else {
            return Ok(());
        };
        return Err(NestedCreationError::ParentCycle {
            change: change.id.clone(),
            parent,
        });
    }
    let (parent, anchor) = parent_and_anchor(change);
    if let Some(parent) = parent {
        if base.entities.contains_key(&parent) {
            if let CreationAnchor::After(anchor) = anchor
                && !base_parent_contains(base, Some(parent), anchor)
            {
                return Err(NestedCreationError::AnchorMissing {
                    change: change.id.clone(),
                    anchor,
                });
            }
        } else {
            let Some(parent_change) = selected.get(&parent).copied() else {
                return Err(NestedCreationError::ParentMissing {
                    change: change.id.clone(),
                    parent,
                });
            };
            if !causally_includes(change, &parent_change.id) {
                return Err(NestedCreationError::ParentNotCausal {
                    change: change.id.clone(),
                    parent,
                });
            }
            if !matches!(anchor, CreationAnchor::Start) {
                return Err(NestedCreationError::CreatedParentAnchor {
                    change: change.id.clone(),
                });
            }
            validate_selected(base, selected, parent, depth + 1, visiting, valid)?;
        }
    } else if let CreationAnchor::After(anchor) = anchor
        && !base_parent_contains(base, None, anchor)
    {
        return Err(NestedCreationError::AnchorMissing {
            change: change.id.clone(),
            anchor,
        });
    }
    visiting.remove(&id);
    valid.insert(id);
    Ok(())
}

fn validate_change_shape(
    base: &Document,
    change: &CreationChange,
) -> Result<(), NestedCreationError> {
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
    let entity = change_entity(change);
    if base.entities.contains_key(&entity.id) {
        return Err(NestedCreationError::EntityAlreadyExists { entity: entity.id });
    }
    if !entity.children.is_empty() {
        return Err(NestedCreationError::NestedEntity { entity: entity.id });
    }
    let (parent, anchor) = parent_and_anchor(change);
    if parent == Some(entity.id) {
        return Err(NestedCreationError::ParentCycle {
            change: change.id.clone(),
            parent: entity.id,
        });
    }
    if parent.is_some_and(|parent| !base.entities.contains_key(&parent))
        && !matches!(anchor, CreationAnchor::Start)
    {
        return Err(NestedCreationError::CreatedParentAnchor {
            change: change.id.clone(),
        });
    }
    let mut candidate = base.clone();
    candidate.roots.push(entity.id);
    candidate.entities.insert(entity.id, entity);
    let codes = error_codes(&candidate);
    if !codes.is_empty() {
        return Err(NestedCreationError::InvalidEntity {
            entity: change_entity(change).id,
            codes,
        });
    }
    Ok(())
}

fn validate_collection<'a>(
    changes: impl Iterator<Item = &'a CreationChange> + Clone,
) -> Result<(), NestedCreationError> {
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

fn entity(change: &CreationChange) -> EntityId {
    change_entity(change).id
}

fn change_entity(change: &CreationChange) -> Entity {
    match &change.operation {
        CreationOperation::Insert { entity, .. } => entity.as_ref().clone(),
    }
}

fn parent_and_anchor(change: &CreationChange) -> (Option<EntityId>, CreationAnchor) {
    match change.operation {
        CreationOperation::Insert { parent, anchor, .. } => (parent, anchor),
    }
}

fn causally_includes(change: &CreationChange, dependency: &ChangeId) -> bool {
    change
        .context
        .get(&dependency.replica)
        .is_some_and(|counter| *counter >= dependency.counter)
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

fn materialize_list(
    base: &[EntityId],
    parent: Option<EntityId>,
    insertions: &BTreeMap<(Option<EntityId>, CreationAnchor), Vec<&CreationChange>>,
) -> Vec<EntityId> {
    let mut output = Vec::with_capacity(base.len());
    if let Some(values) = insertions.get(&(parent, CreationAnchor::Start)) {
        output.extend(values.iter().map(|change| entity(change)));
    }
    for id in base {
        output.push(*id);
        if let Some(values) = insertions.get(&(parent, CreationAnchor::After(*id))) {
            output.extend(values.iter().map(|change| entity(change)));
        }
    }
    output
}

fn error_codes(document: &Document) -> Vec<String> {
    validate(document)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nuif_core::EntityKind;

    const ROOT: EntityId = EntityId::new(10);
    const BASE_CHILD: EntityId = EntityId::new(11);

    fn base() -> Document {
        let mut document = Document::empty(EntityId::new(1));
        let mut root = Entity::new(ROOT, EntityKind::Container);
        root.children.push(BASE_CHILD);
        document.roots.push(ROOT);
        document.entities.insert(ROOT, root);
        document
            .entities
            .insert(BASE_CHILD, Entity::new(BASE_CHILD, EntityKind::Container));
        document
    }

    fn create(
        replica: &str,
        counter: u64,
        context: &[(&str, u64)],
        id: u128,
        parent: Option<u128>,
        anchor: CreationAnchor,
    ) -> CreationChange {
        CreationChange {
            id: ChangeId::new(replica, counter),
            context: context
                .iter()
                .map(|(replica, counter)| ((*replica).to_owned(), *counter))
                .collect(),
            operation: CreationOperation::Insert {
                parent: parent.map(EntityId::new),
                anchor,
                entity: Box::new(Entity::new(EntityId::new(id), EntityKind::Container)),
            },
        }
    }

    #[test]
    fn causal_nested_creation_converges() {
        let changes = vec![
            create("alice", 1, &[], 20, Some(10), CreationAnchor::Start),
            create(
                "bob",
                1,
                &[("alice", 1)],
                21,
                Some(20),
                CreationAnchor::Start,
            ),
            create(
                "carol",
                1,
                &[],
                22,
                Some(10),
                CreationAnchor::After(BASE_CHILD),
            ),
        ];
        let mut engine = NestedCreationOperationSetEngine::new(base()).unwrap();
        for change in changes.into_iter().rev() {
            engine.ingest(change).unwrap();
        }
        let checkpoint = engine.checkpoint().unwrap();
        assert_eq!(checkpoint.document.roots, vec![ROOT]);
        assert_eq!(
            checkpoint.document.entities[&ROOT].children,
            vec![EntityId::new(20), BASE_CHILD, EntityId::new(22)]
        );
        assert_eq!(
            checkpoint.document.entities[&EntityId::new(20)].children,
            vec![EntityId::new(21)]
        );
    }

    #[test]
    fn non_causal_parent_is_typed() {
        let mut engine = NestedCreationOperationSetEngine::new(base()).unwrap();
        engine
            .ingest(create("alice", 1, &[], 20, Some(10), CreationAnchor::Start))
            .unwrap();
        engine
            .ingest(create("bob", 1, &[], 21, Some(20), CreationAnchor::Start))
            .unwrap();
        assert!(matches!(
            engine.checkpoint(),
            Err(NestedCreationError::ParentNotCausal { parent, .. })
                if parent == EntityId::new(20)
        ));
    }
}
