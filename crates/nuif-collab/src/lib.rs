#![doc = "Bounded operation-set collaboration profile for canonical NUIF checkpoints."]

use nuif_codec::canonical_hash;
use nuif_core::{Document, EntityId};
use nuif_protocol::{ApplyError, Axis, Operation, Patch, Transaction, apply_patch};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const PROFILE_NAME: &str = "nuif-collab-registers-0";
pub const MAX_CHANGES: usize = 100_000;
pub const MAX_REPLICAS: usize = 1_024;
pub const MAX_REPLICA_BYTES: usize = 64;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeId {
    pub counter: u64,
    pub replica: String,
}

impl ChangeId {
    #[must_use]
    pub fn new(replica: impl Into<String>, counter: u64) -> Self {
        Self {
            counter,
            replica: replica.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Change {
    pub id: ChangeId,
    #[serde(default)]
    pub context: BTreeMap<String, u64>,
    pub operation: Operation,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum RegisterTarget {
    Document,
    Entity { id: EntityId },
    Token { id: EntityId },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisterKey {
    pub target: RegisterTarget,
    pub pointer: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConflictCandidate {
    pub change: ChangeId,
    pub operation: Operation,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticConflict {
    pub key: RegisterKey,
    pub candidates: Vec<ConflictCandidate>,
    pub selected: ChangeId,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Checkpoint {
    pub profile: String,
    pub canonical_hash: String,
    pub document: Document,
    pub applied: Vec<ChangeId>,
    pub conflicts: Vec<SemanticConflict>,
}

#[derive(Clone, Debug, PartialEq, Error)]
pub enum CollaborationError {
    #[error("replica identifier {replica:?} is invalid")]
    InvalidReplica { replica: String },
    #[error("change counter must be non-zero for replica {replica}")]
    ZeroCounter { replica: String },
    #[error("change {change:?} does not use the next local counter after {observed}")]
    InvalidLocalContext { change: ChangeId, observed: u64 },
    #[error("change identifier {change:?} has conflicting contents")]
    DuplicateChange { change: ChangeId },
    #[error("change collection exceeds the {MAX_CHANGES}-change profile limit")]
    TooManyChanges,
    #[error("change collection exceeds the {MAX_REPLICAS}-replica profile limit")]
    TooManyReplicas,
    #[error("replica {replica} log is missing counter {counter}")]
    MissingReplicaChange { replica: String, counter: u64 },
    #[error("change {change:?} depends on missing {replica}:{counter}")]
    MissingDependency {
        change: ChangeId,
        replica: String,
        counter: u64,
    },
    #[error("change {change:?} does not include the causal context of {dependency:?}")]
    IncompleteCausalContext {
        change: ChangeId,
        dependency: ChangeId,
    },
    #[error("operation {operation} is outside the property-register profile")]
    UnsupportedOperation { operation: &'static str },
    #[error("change {change:?} could not apply: {source}")]
    Apply {
        change: ChangeId,
        #[source]
        source: ApplyError,
    },
    #[error("checkpoint hashing failed: {0}")]
    Canonical(String),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct OperationSetEngine {
    changes: BTreeMap<ChangeId, Change>,
}

impl OperationSetEngine {
    /// Adds one idempotent collaboration change.
    ///
    /// # Errors
    ///
    /// Rejects malformed identifiers, unsupported operations, resource overflow,
    /// or reuse of an identifier with different contents.
    pub fn ingest(&mut self, change: Change) -> Result<bool, CollaborationError> {
        validate_change_shape(&change)?;
        if let Some(existing) = self.changes.get(&change.id) {
            return if existing == &change {
                Ok(false)
            } else {
                Err(CollaborationError::DuplicateChange { change: change.id })
            };
        }
        if self.changes.len() == MAX_CHANGES {
            return Err(CollaborationError::TooManyChanges);
        }
        self.changes.insert(change.id.clone(), change);
        Ok(true)
    }

    /// Joins another operation set. The join is commutative, associative and idempotent.
    ///
    /// # Errors
    ///
    /// Returns the same typed validation failures as [`Self::ingest`].
    pub fn merge(&mut self, other: &Self) -> Result<(), CollaborationError> {
        let mut candidate = self.clone();
        for change in other.changes.values() {
            candidate.ingest(change.clone())?;
        }
        *self = candidate;
        Ok(())
    }

    /// Materializes a metadata-free canonical checkpoint with explicit register conflicts.
    ///
    /// # Errors
    ///
    /// Rejects incomplete causal history or an operation that cannot apply to `base`.
    pub fn checkpoint(&self, base: &Document) -> Result<Checkpoint, CollaborationError> {
        validate_collection(self.changes.values())?;
        let mut grouped = BTreeMap::<RegisterKey, Vec<&Change>>::new();
        for change in self.changes.values() {
            grouped
                .entry(register_key(&change.operation)?)
                .or_default()
                .push(change);
        }
        let frontiers = grouped
            .into_iter()
            .map(|(key, changes)| {
                let mut observed_context = BTreeMap::<&str, u64>::new();
                for change in &changes {
                    for (replica, counter) in &change.context {
                        observed_context
                            .entry(replica)
                            .and_modify(|observed| *observed = (*observed).max(*counter))
                            .or_insert(*counter);
                    }
                }
                let maximal = changes
                    .into_iter()
                    .filter(|candidate| {
                        observed_context
                            .get(candidate.id.replica.as_str())
                            .copied()
                            .unwrap_or(0)
                            < candidate.id.counter
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                (key, maximal)
            })
            .collect();
        finish_checkpoint(base, frontiers)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ReplicaLogEngine {
    logs: BTreeMap<String, BTreeMap<u64, Change>>,
    changes: usize,
}

impl ReplicaLogEngine {
    /// Adds one idempotent collaboration change to a replica log.
    ///
    /// # Errors
    ///
    /// Rejects malformed identifiers, unsupported operations, resource overflow,
    /// or reuse of an identifier with different contents.
    pub fn ingest(&mut self, change: Change) -> Result<bool, CollaborationError> {
        validate_change_shape(&change)?;
        if let Some(existing) = self
            .logs
            .get(&change.id.replica)
            .and_then(|log| log.get(&change.id.counter))
        {
            return if existing == &change {
                Ok(false)
            } else {
                Err(CollaborationError::DuplicateChange { change: change.id })
            };
        }
        if self.changes == MAX_CHANGES {
            return Err(CollaborationError::TooManyChanges);
        }
        if !self.logs.contains_key(&change.id.replica) && self.logs.len() == MAX_REPLICAS {
            return Err(CollaborationError::TooManyReplicas);
        }
        self.logs
            .entry(change.id.replica.clone())
            .or_default()
            .insert(change.id.counter, change);
        self.changes += 1;
        Ok(true)
    }

    /// Joins another collection of per-replica logs.
    ///
    /// # Errors
    ///
    /// Returns the same typed validation failures as [`Self::ingest`].
    pub fn merge(&mut self, other: &Self) -> Result<(), CollaborationError> {
        let mut candidate = self.clone();
        for log in other.logs.values() {
            for change in log.values() {
                candidate.ingest(change.clone())?;
            }
        }
        *self = candidate;
        Ok(())
    }

    /// Materializes through an incremental causal-frontier algorithm.
    ///
    /// # Errors
    ///
    /// Rejects incomplete causal history or an operation that cannot apply to `base`.
    pub fn checkpoint(&self, base: &Document) -> Result<Checkpoint, CollaborationError> {
        let changes = self.logs.values().flat_map(BTreeMap::values);
        validate_collection(changes.clone())?;
        let mut frontiers = BTreeMap::<RegisterKey, Vec<Change>>::new();
        for change in changes {
            let frontier = frontiers
                .entry(register_key(&change.operation)?)
                .or_default();
            if frontier
                .iter()
                .any(|existing| happens_before(change, existing))
            {
                continue;
            }
            frontier.retain(|existing| !happens_before(existing, change));
            frontier.push(change.clone());
        }
        finish_checkpoint(base, frontiers)
    }
}

fn finish_checkpoint(
    base: &Document,
    mut frontiers: BTreeMap<RegisterKey, Vec<Change>>,
) -> Result<Checkpoint, CollaborationError> {
    let mut selected = Vec::new();
    let mut conflicts = Vec::new();
    for (key, frontier) in &mut frontiers {
        frontier.sort_by(|left, right| left.id.cmp(&right.id));
        let winner = frontier.last().expect("register frontier is non-empty");
        selected.push(winner.clone());
        if frontier
            .iter()
            .skip(1)
            .any(|candidate| candidate.operation != frontier[0].operation)
        {
            conflicts.push(SemanticConflict {
                key: key.clone(),
                candidates: frontier
                    .iter()
                    .map(|candidate| ConflictCandidate {
                        change: candidate.id.clone(),
                        operation: candidate.operation.clone(),
                    })
                    .collect(),
                selected: winner.id.clone(),
            });
        }
    }
    selected = causal_order(selected);
    let mut document = base.clone();
    for change in &selected {
        let patch = Patch {
            base_revision: None,
            transactions: vec![Transaction {
                id: 0,
                operations: vec![change.operation.clone()],
            }],
        };
        apply_patch(&mut document, &patch).map_err(|source| CollaborationError::Apply {
            change: change.id.clone(),
            source,
        })?;
    }
    Ok(Checkpoint {
        profile: PROFILE_NAME.to_owned(),
        canonical_hash: canonical_hash(&document)
            .map_err(|error| CollaborationError::Canonical(error.to_string()))?,
        document,
        applied: selected.into_iter().map(|change| change.id).collect(),
        conflicts,
    })
}

fn validate_change_shape(change: &Change) -> Result<(), CollaborationError> {
    validate_replica(&change.id.replica)?;
    if change.id.counter == 0 {
        return Err(CollaborationError::ZeroCounter {
            replica: change.id.replica.clone(),
        });
    }
    for replica in change.context.keys() {
        validate_replica(replica)?;
    }
    if change.context.len() > MAX_REPLICAS {
        return Err(CollaborationError::TooManyReplicas);
    }
    let own = change.context.get(&change.id.replica).copied().unwrap_or(0);
    if own + 1 != change.id.counter {
        return Err(CollaborationError::InvalidLocalContext {
            change: change.id.clone(),
            observed: own,
        });
    }
    register_key(&change.operation)?;
    Ok(())
}

fn causal_order(mut changes: Vec<Change>) -> Vec<Change> {
    let mut ordered = Vec::with_capacity(changes.len());
    while !changes.is_empty() {
        let next = changes
            .iter()
            .enumerate()
            .filter(|(candidate_index, candidate)| {
                !changes.iter().enumerate().any(|(other_index, other)| {
                    candidate_index != &other_index && happens_before(other, candidate)
                })
            })
            .min_by(|(_, left), (_, right)| left.id.cmp(&right.id))
            .map(|(index, _)| index)
            .expect("validated vector contexts are acyclic");
        ordered.push(changes.remove(next));
    }
    ordered
}

fn validate_replica(replica: &str) -> Result<(), CollaborationError> {
    if replica.is_empty()
        || replica.len() > MAX_REPLICA_BYTES
        || !replica
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        Err(CollaborationError::InvalidReplica {
            replica: replica.to_owned(),
        })
    } else {
        Ok(())
    }
}

fn validate_collection<'a>(
    changes: impl Iterator<Item = &'a Change> + Clone,
) -> Result<(), CollaborationError> {
    let changes = changes.collect::<Vec<_>>();
    if changes.len() > MAX_CHANGES {
        return Err(CollaborationError::TooManyChanges);
    }
    let replicas = changes
        .iter()
        .map(|change| change.id.replica.as_str())
        .collect::<BTreeSet<_>>();
    if replicas.len() > MAX_REPLICAS {
        return Err(CollaborationError::TooManyReplicas);
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
                });
            }
        }
    }
    for change in &changes {
        for (replica, counter) in &change.context {
            if *counter > 0 {
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
                    });
                }
            }
        }
    }
    Ok(())
}

fn happens_before(left: &Change, right: &Change) -> bool {
    right
        .context
        .get(&left.id.replica)
        .is_some_and(|counter| *counter >= left.id.counter)
}

fn register_key(operation: &Operation) -> Result<RegisterKey, CollaborationError> {
    let key = match operation {
        Operation::Rename { entity, .. } => entity_key(*entity, "/name"),
        Operation::SetSize { entity, axis, .. } => entity_key(
            *entity,
            match axis {
                Axis::Horizontal => "/authored/width",
                Axis::Vertical => "/authored/height",
            },
        ),
        Operation::SetLayout { entity, .. } => entity_key(*entity, "/authored/layout"),
        Operation::SetPosition { entity, .. } => entity_key(*entity, "/authored/position"),
        Operation::SetFill { entity, .. } => entity_key(*entity, "/authored/fill"),
        Operation::SetText { entity, .. } => entity_key(*entity, "/authored/text"),
        Operation::SetImage { entity, .. } => entity_key(*entity, "/authored/image"),
        Operation::SetToken { token } => RegisterKey {
            target: RegisterTarget::Token { id: token.id },
            pointer: format!("/tokens/{}", token.id),
        },
        Operation::RemoveToken { token } => RegisterKey {
            target: RegisterTarget::Token { id: *token },
            pointer: format!("/tokens/{token}"),
        },
        Operation::SetExtensionDeclarations { .. } => RegisterKey {
            target: RegisterTarget::Document,
            pointer: "/extension_declarations".to_owned(),
        },
        Operation::SetValue { entity, key, .. } | Operation::RemoveValue { entity, key } => {
            entity_key(
                *entity,
                &format!("/authored/values/{}", pointer_segment(key)),
            )
        }
        Operation::SetExtension {
            entity, namespace, ..
        }
        | Operation::RemoveExtension { entity, namespace } => entity_key(
            *entity,
            &format!("/extensions/{}", pointer_segment(namespace)),
        ),
        Operation::SetUnknownPayload { entity, .. } => entity_key(*entity, "/kind/data/payload"),
        Operation::Insert { .. }
        | Operation::Remove { .. }
        | Operation::Move { .. }
        | Operation::SetAsset { .. }
        | Operation::RemoveAsset { .. }
        | Operation::BindAssetResource { .. }
        | Operation::RestoreSubtree { .. } => {
            return Err(CollaborationError::UnsupportedOperation {
                operation: operation_name(operation),
            });
        }
    };
    Ok(key)
}

fn entity_key(id: EntityId, pointer: &str) -> RegisterKey {
    RegisterKey {
        target: RegisterTarget::Entity { id },
        pointer: format!("/entities/{id}{pointer}"),
    }
}

fn pointer_segment(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

fn operation_name(operation: &Operation) -> &'static str {
    match operation {
        Operation::Insert { .. } => "insert",
        Operation::Remove { .. } => "remove",
        Operation::Move { .. } => "move",
        Operation::Rename { .. } => "rename",
        Operation::SetSize { .. } => "set_size",
        Operation::SetLayout { .. } => "set_layout",
        Operation::SetPosition { .. } => "set_position",
        Operation::SetFill { .. } => "set_fill",
        Operation::SetText { .. } => "set_text",
        Operation::SetImage { .. } => "set_image",
        Operation::SetAsset { .. } => "set_asset",
        Operation::RemoveAsset { .. } => "remove_asset",
        Operation::BindAssetResource { .. } => "bind_asset_resource",
        Operation::SetToken { .. } => "set_token",
        Operation::RemoveToken { .. } => "remove_token",
        Operation::SetExtensionDeclarations { .. } => "set_extension_declarations",
        Operation::SetValue { .. } => "set_value",
        Operation::RemoveValue { .. } => "remove_value",
        Operation::SetExtension { .. } => "set_extension",
        Operation::RemoveExtension { .. } => "remove_extension",
        Operation::SetUnknownPayload { .. } => "set_unknown_payload",
        Operation::RestoreSubtree { .. } => "restore_subtree",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn change(replica: &str, counter: u64, context: &[(&str, u64)], name: &str) -> Change {
        Change {
            id: ChangeId::new(replica, counter),
            context: context
                .iter()
                .map(|(replica, counter)| ((*replica).to_owned(), *counter))
                .collect(),
            operation: Operation::Rename {
                entity: EntityId::new(0x20),
                name: Some(name.to_owned()),
            },
        }
    }

    #[test]
    fn independent_materializers_converge_with_explicit_conflict() {
        let alice = change("alice", 1, &[], "Alice");
        let bob = change("bob", 1, &[], "Bob");
        let mut set = OperationSetEngine::default();
        let mut logs = ReplicaLogEngine::default();
        for item in [bob, alice] {
            set.ingest(item.clone()).unwrap();
            logs.ingest(item).unwrap();
        }
        let base = nuif_testing::responsive_card_fixture();
        let left = set.checkpoint(&base).unwrap();
        let right = logs.checkpoint(&base).unwrap();
        assert_eq!(left, right);
        assert_eq!(left.conflicts.len(), 1);
        assert_eq!(
            left.document.entities[&EntityId::new(0x20)].name.as_deref(),
            Some("Bob")
        );
    }

    #[test]
    fn incomplete_causal_history_fails_closed() {
        let mut engine = OperationSetEngine::default();
        engine
            .ingest(change("alice", 2, &[("alice", 1)], "second"))
            .unwrap();
        assert!(matches!(
            engine.checkpoint(&nuif_testing::responsive_card_fixture()),
            Err(CollaborationError::MissingReplicaChange { .. })
        ));
    }

    #[test]
    fn structural_operations_are_outside_register_profile() {
        let change = Change {
            id: ChangeId::new("alice", 1),
            context: BTreeMap::new(),
            operation: Operation::Move {
                entity: EntityId::new(0x20),
                new_parent: None,
                anchor: nuif_protocol::Anchor::Start,
            },
        };
        assert!(matches!(
            OperationSetEngine::default().ingest(change),
            Err(CollaborationError::UnsupportedOperation { operation: "move" })
        ));
    }

    #[test]
    fn failed_merge_is_atomic() {
        let original = change("alice", 1, &[], "first");
        let mut conflicting = original.clone();
        conflicting.operation = Operation::Rename {
            entity: EntityId::new(0x20),
            name: Some("different".to_owned()),
        };
        let mut left = OperationSetEngine::default();
        left.ingest(original.clone()).unwrap();
        let before = left.clone();
        let mut right = OperationSetEngine::default();
        right.ingest(conflicting).unwrap();
        assert!(matches!(
            left.merge(&right),
            Err(CollaborationError::DuplicateChange { .. })
        ));
        assert_eq!(left, before);

        let mut left = ReplicaLogEngine::default();
        left.ingest(original).unwrap();
        let before = left.clone();
        let mut right = ReplicaLogEngine::default();
        right.ingest(change("alice", 1, &[], "different")).unwrap();
        assert!(matches!(
            left.merge(&right),
            Err(CollaborationError::DuplicateChange { .. })
        ));
        assert_eq!(left, before);
    }

    #[test]
    fn vector_context_must_include_dependency_context() {
        let alice = change("alice", 1, &[], "alice");
        let bob = Change {
            id: ChangeId::new("bob", 2),
            context: BTreeMap::from([("alice".to_owned(), 1), ("bob".to_owned(), 1)]),
            operation: Operation::Rename {
                entity: EntityId::new(0x20),
                name: Some("bob".to_owned()),
            },
        };
        let bob_first = change("bob", 1, &[], "bob first");
        let carol = Change {
            id: ChangeId::new("carol", 1),
            context: BTreeMap::from([("bob".to_owned(), 2)]),
            operation: Operation::Rename {
                entity: EntityId::new(0x20),
                name: Some("carol".to_owned()),
            },
        };
        let mut engine = OperationSetEngine::default();
        for item in [alice, bob_first, bob, carol] {
            engine.ingest(item).unwrap();
        }
        assert!(matches!(
            engine.checkpoint(&nuif_testing::responsive_card_fixture()),
            Err(CollaborationError::IncompleteCausalContext { .. })
        ));
    }
}
