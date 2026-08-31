//! Explicit, conservative causal-stability compaction metadata.

use super::{
    Change, ChangeId, Checkpoint, CollaborationError, MAX_CHANGES, MAX_REPLICAS,
    OperationSetEngine, finish_checkpoint, register_key, validate_replica,
};
use nuif_core::Document;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Profile identifier for checkpoint-based collaboration history collection.
pub const PROFILE_NAME: &str = "nuif-collab-gc-0";
/// Profile identifier for register-only checkpoint-aware partial collection.
pub const PARTIAL_PROFILE_NAME: &str = "nuif-collab-gc-prefix-0";
/// Profile identifier for the external base handed to a resumed suffix.
pub const BASE_PROFILE_NAME: &str = "nuif-collab-causal-base-0";

/// A caller-attested causal frontier for one complete collaboration history.
///
/// The frontier is intentionally not inferred from a local operation set. A
/// host must merge the complete history first and then provide the replica
/// counters it has established as stable. The current profile only accepts a
/// frontier that exactly covers every locally observed change; partial pruning
/// and context rebasing are future work.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StabilityFrontier {
    pub counters: BTreeMap<String, u64>,
}

impl StabilityFrontier {
    /// Creates a validated stability frontier.
    ///
    /// # Errors
    ///
    /// Rejects invalid replica identifiers and oversized frontiers.
    pub fn new(counters: BTreeMap<String, u64>) -> Result<Self, CollaborationError> {
        if counters.len() > MAX_REPLICAS {
            return Err(CollaborationError::TooManyReplicas);
        }
        for replica in counters.keys() {
            validate_replica(replica)?;
        }
        Ok(Self { counters })
    }

    /// Verifies that this frontier is an exact clock for the supplied history.
    ///
    /// Exactness is the safe boundary for profile 0: every known replica must
    /// have the same counter in the frontier, and the frontier may not claim a
    /// counter for history that is not present locally.
    ///
    /// # Errors
    ///
    /// Returns a typed unsafe-compaction failure when the frontier is partial,
    /// behind, or ahead of the supplied history.
    pub fn validate_complete<'a, I>(&self, ids: I) -> Result<(), CollaborationError>
    where
        I: IntoIterator<Item = &'a ChangeId>,
    {
        let mut observed = BTreeMap::<&str, u64>::new();
        for id in ids {
            observed
                .entry(id.replica.as_str())
                .and_modify(|counter| *counter = (*counter).max(id.counter))
                .or_insert(id.counter);
        }

        for (replica, observed_counter) in &observed {
            let frontier_counter = self.counters.get(*replica).copied().unwrap_or(0);
            if frontier_counter != *observed_counter {
                return Err(CollaborationError::UnsafeCompaction {
                    replica: (*replica).to_owned(),
                    observed: *observed_counter,
                    frontier: frontier_counter,
                });
            }
        }
        for (replica, frontier_counter) in &self.counters {
            let observed_counter = observed.get(replica.as_str()).copied().unwrap_or(0);
            if observed_counter != *frontier_counter {
                return Err(CollaborationError::UnsafeCompaction {
                    replica: replica.clone(),
                    observed: observed_counter,
                    frontier: *frontier_counter,
                });
            }
        }
        Ok(())
    }

    /// Verifies a bounded stable prefix and a causally dominated retained
    /// suffix for checkpoint-aware register collection.
    ///
    /// The prefix consists of every change whose local counter is at most the
    /// frontier. Stable changes may not depend on the retained suffix, and
    /// every retained change must causally include the complete frontier. This
    /// deliberately excludes concurrent retained-vs-stable register conflicts
    /// and all structural position rebasing.
    ///
    /// # Errors
    ///
    /// Returns typed errors for ahead frontiers, non-closed stable prefixes or
    /// retained changes that do not causally follow the frontier.
    pub fn validate_prefix(&self, changes: &[Change]) -> Result<(), CollaborationError> {
        super::validate_collection(changes.iter())?;
        let mut observed = BTreeMap::<&str, u64>::new();
        for change in changes {
            observed
                .entry(change.id.replica.as_str())
                .and_modify(|counter| *counter = (*counter).max(change.id.counter))
                .or_insert(change.id.counter);
        }
        for (replica, frontier) in &self.counters {
            let max = observed.get(replica.as_str()).copied().unwrap_or(0);
            if *frontier > max {
                return Err(CollaborationError::UnsafeCompaction {
                    replica: replica.clone(),
                    observed: max,
                    frontier: *frontier,
                });
            }
        }
        for change in changes.iter().filter(|change| {
            self.counters.get(&change.id.replica).copied().unwrap_or(0) >= change.id.counter
        }) {
            for (replica, counter) in &change.context {
                if *counter > 0 && self.counters.get(replica).copied().unwrap_or(0) < *counter {
                    return Err(CollaborationError::StablePrefixNotClosed {
                        change: change.id.clone(),
                        dependency: ChangeId::new(replica.clone(), *counter),
                    });
                }
            }
        }
        for change in changes.iter().filter(|change| {
            self.counters.get(&change.id.replica).copied().unwrap_or(0) < change.id.counter
        }) {
            for (replica, frontier) in &self.counters {
                if *frontier > 0 && change.context.get(replica).copied().unwrap_or(0) < *frontier {
                    return Err(CollaborationError::RetainedChangeNotAfterFrontier {
                        change: change.id.clone(),
                        replica: replica.clone(),
                        frontier: *frontier,
                    });
                }
            }
        }
        Ok(())
    }
}

/// Evidence emitted when a complete operation history is replaced by a
/// metadata-free canonical checkpoint.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompactionReceipt {
    pub profile: String,
    pub source_profile: String,
    pub source_base_hash: String,
    pub compacted_base_hash: String,
    pub frontier: StabilityFrontier,
    pub dropped: Vec<ChangeId>,
    pub retained: Vec<ChangeId>,
}

impl CompactionReceipt {
    #[must_use]
    pub fn complete_history(
        source_profile: &str,
        source_base_hash: String,
        compacted_base_hash: String,
        frontier: &StabilityFrontier,
        dropped: Vec<ChangeId>,
    ) -> Self {
        Self {
            profile: PROFILE_NAME.to_owned(),
            source_profile: source_profile.to_owned(),
            source_base_hash,
            compacted_base_hash,
            frontier: frontier.clone(),
            dropped,
            retained: Vec::new(),
        }
    }

    #[must_use]
    pub fn partial_history(
        source_profile: &str,
        source_base_hash: String,
        compacted_base_hash: String,
        frontier: &StabilityFrontier,
        dropped: Vec<ChangeId>,
        retained: Vec<ChangeId>,
    ) -> Self {
        Self::partial_history_for_profile(
            PARTIAL_PROFILE_NAME,
            source_profile,
            source_base_hash,
            compacted_base_hash,
            frontier,
            dropped,
            retained,
        )
    }

    #[must_use]
    pub fn partial_history_for_profile(
        profile: &str,
        source_profile: &str,
        source_base_hash: String,
        compacted_base_hash: String,
        frontier: &StabilityFrontier,
        dropped: Vec<ChangeId>,
        retained: Vec<ChangeId>,
    ) -> Self {
        Self {
            profile: profile.to_owned(),
            source_profile: source_profile.to_owned(),
            source_base_hash,
            compacted_base_hash,
            frontier: frontier.clone(),
            dropped,
            retained,
        }
    }
}

/// A metadata-bearing causal base used to resume a retained register suffix.
///
/// The embedded checkpoint is not the canonical NUIF document: it is a local
/// synchronization handoff that keeps the stable frontier outside the wire
/// model while allowing a resumed materializer to compare later dots against
/// the collected prefix.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CausalCheckpointBase {
    pub profile: String,
    pub source_profile: String,
    pub source_base_hash: String,
    pub checkpoint: Checkpoint,
    pub frontier: StabilityFrontier,
}

/// Result of bounded register-only stable-prefix collection.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrefixCompaction {
    pub receipt: CompactionReceipt,
    pub base: CausalCheckpointBase,
    pub retained: Vec<Change>,
    pub checkpoint: Checkpoint,
}

/// Operation-set materializer for a causal suffix resumed from a collected
/// register checkpoint.
#[derive(Clone, Debug, PartialEq)]
pub struct ResumedOperationSetEngine {
    base: CausalCheckpointBase,
    changes: BTreeMap<ChangeId, Change>,
}

impl ResumedOperationSetEngine {
    /// Creates a resumed engine from a partial-compaction handoff.
    ///
    /// # Errors
    ///
    /// Rejects a malformed or mismatched causal base.
    pub fn new(base: CausalCheckpointBase) -> Result<Self, CollaborationError> {
        if base.profile != BASE_PROFILE_NAME {
            return Err(CollaborationError::Canonical(
                "unsupported causal checkpoint base profile".to_owned(),
            ));
        }
        if base.checkpoint.profile != super::PROFILE_NAME {
            return Err(CollaborationError::Canonical(
                "causal checkpoint base must contain a register checkpoint".to_owned(),
            ));
        }
        Ok(Self {
            base,
            changes: BTreeMap::new(),
        })
    }

    /// Adds one suffix change after the supplied stable frontier.
    ///
    /// # Errors
    ///
    /// Rejects unsupported operations, malformed clocks, duplicate IDs or a
    /// change that does not immediately continue its replica's retained log.
    pub fn ingest(&mut self, change: Change) -> Result<bool, CollaborationError> {
        super::validate_change_shape(&change)?;
        if let Some(existing) = self.changes.get(&change.id) {
            return if existing == &change {
                Ok(false)
            } else {
                Err(CollaborationError::DuplicateChange { change: change.id })
            };
        }
        let frontier = self
            .base
            .frontier
            .counters
            .get(&change.id.replica)
            .copied()
            .unwrap_or(0);
        let previous = self
            .changes
            .keys()
            .filter(|id| id.replica == change.id.replica)
            .map(|id| id.counter)
            .max()
            .unwrap_or(frontier);
        if change.id.counter != previous + 1 {
            return Err(CollaborationError::InvalidLocalContext {
                change: change.id,
                observed: previous,
            });
        }
        for (replica, counter) in &self.base.frontier.counters {
            if change.context.get(replica).copied().unwrap_or(0) < *counter {
                return Err(CollaborationError::RetainedChangeNotAfterFrontier {
                    change: change.id,
                    replica: replica.clone(),
                    frontier: *counter,
                });
            }
        }
        self.changes.insert(change.id.clone(), change);
        Ok(true)
    }

    /// Joins another resumed suffix with the same causal base.
    ///
    /// # Errors
    ///
    /// Rejects different bases or any ingestion failure.
    pub fn merge(&mut self, other: &Self) -> Result<(), CollaborationError> {
        if self.base != other.base {
            return Err(CollaborationError::Canonical(
                "resumed engines use different causal bases".to_owned(),
            ));
        }
        let mut candidate = self.clone();
        for change in other.changes.values() {
            candidate.ingest(change.clone())?;
        }
        *self = candidate;
        Ok(())
    }

    /// Materializes retained changes over the collected canonical checkpoint.
    ///
    /// # Errors
    ///
    /// Rejects incomplete suffix history or an operation that cannot apply to
    /// the collected document.
    pub fn checkpoint(&self) -> Result<Checkpoint, CollaborationError> {
        validate_resumed_collection(&self.base.frontier, self.changes.values())?;
        let mut grouped = BTreeMap::<super::RegisterKey, Vec<Change>>::new();
        for change in self.changes.values() {
            grouped
                .entry(register_key(&change.operation)?)
                .or_default()
                .push(change.clone());
        }
        finish_checkpoint(&self.base.checkpoint.document, grouped)
    }
}

impl OperationSetEngine {
    /// Collects a stable prefix and returns a resumable causal-base handoff.
    ///
    /// The profile is intentionally limited to register histories whose
    /// retained suffix causally follows the complete frontier. Structural
    /// anchors, concurrent stable-vs-retained conflicts and context rebasing
    /// are not inferred.
    ///
    /// # Errors
    ///
    /// Rejects incomplete history, unsafe prefix closure, an unhashable base or
    /// any retained operation that cannot resume over the compacted checkpoint.
    pub fn compact_stable_prefix(
        &self,
        base: &Document,
        frontier: &StabilityFrontier,
    ) -> Result<PrefixCompaction, CollaborationError> {
        let full = self.checkpoint(base)?;
        let changes = self.changes.values().cloned().collect::<Vec<_>>();
        frontier.validate_prefix(&changes)?;
        let stable = changes
            .iter()
            .filter(|change| {
                frontier
                    .counters
                    .get(&change.id.replica)
                    .copied()
                    .unwrap_or(0)
                    >= change.id.counter
            })
            .cloned()
            .collect::<Vec<_>>();
        let retained = changes
            .iter()
            .filter(|change| {
                frontier
                    .counters
                    .get(&change.id.replica)
                    .copied()
                    .unwrap_or(0)
                    < change.id.counter
            })
            .cloned()
            .collect::<Vec<_>>();
        let mut stable_engine = OperationSetEngine::default();
        for change in &stable {
            stable_engine.ingest(change.clone())?;
        }
        let stable_checkpoint = stable_engine.checkpoint(base)?;
        let source_base_hash = nuif_codec::canonical_hash(base)
            .map_err(|error| CollaborationError::Canonical(error.to_string()))?;
        let base = CausalCheckpointBase {
            profile: BASE_PROFILE_NAME.to_owned(),
            source_profile: super::PROFILE_NAME.to_owned(),
            source_base_hash: source_base_hash.clone(),
            checkpoint: stable_checkpoint.clone(),
            frontier: frontier.clone(),
        };
        let mut resumed = ResumedOperationSetEngine::new(base.clone())?;
        for change in &retained {
            resumed.ingest(change.clone())?;
        }
        let checkpoint = resumed.checkpoint()?;
        if checkpoint.canonical_hash != full.canonical_hash
            || checkpoint.document != full.document
            || checkpoint.conflicts != full.conflicts
        {
            return Err(CollaborationError::Canonical(
                "partial compaction replay changed the full checkpoint".to_owned(),
            ));
        }
        let receipt = CompactionReceipt::partial_history(
            super::PROFILE_NAME,
            source_base_hash,
            stable_checkpoint.canonical_hash.clone(),
            frontier,
            stable.iter().map(|change| change.id.clone()).collect(),
            retained.iter().map(|change| change.id.clone()).collect(),
        );
        Ok(PrefixCompaction {
            receipt,
            base,
            retained,
            checkpoint,
        })
    }
}

fn validate_resumed_collection<'a>(
    frontier: &StabilityFrontier,
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
        let start = frontier.counters.get(replica).copied().unwrap_or(0) + 1;
        for (expected, counter) in (start..).zip(
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
            if *counter == 0 {
                continue;
            }
            if *counter <= frontier.counters.get(replica).copied().unwrap_or(0) {
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
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nuif_core::EntityId;
    use nuif_protocol::Operation;

    fn rename(replica: &str, counter: u64, context: &[(&str, u64)], name: &str) -> Change {
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
    fn register_prefix_compaction_resumes_to_full_checkpoint() {
        let base = nuif_testing::responsive_card_fixture();
        let changes = vec![
            rename("alice", 1, &[], "stable"),
            rename("alice", 2, &[("alice", 1)], "retained"),
        ];
        let frontier = StabilityFrontier::new(BTreeMap::from([("alice".to_owned(), 1)])).unwrap();
        let mut engine = OperationSetEngine::default();
        for change in &changes {
            engine.ingest(change.clone()).unwrap();
        }
        let full = engine.checkpoint(&base).unwrap();
        let compacted = engine.compact_stable_prefix(&base, &frontier).unwrap();
        assert_eq!(compacted.checkpoint, full);
        assert_eq!(compacted.receipt.profile, PARTIAL_PROFILE_NAME);
        assert_eq!(compacted.receipt.dropped, vec![changes[0].id.clone()]);
        assert_eq!(compacted.receipt.retained, vec![changes[1].id.clone()]);
        assert_ne!(
            compacted.base.checkpoint.canonical_hash,
            full.canonical_hash
        );

        let mut resumed = ResumedOperationSetEngine::new(compacted.base.clone()).unwrap();
        for change in compacted.retained {
            resumed.ingest(change).unwrap();
        }
        let resumed_checkpoint = resumed.checkpoint().unwrap();
        assert_eq!(resumed_checkpoint, full);
    }

    #[test]
    fn stable_prefix_must_be_causally_closed() {
        let changes = vec![
            rename("alice", 1, &[("bob", 1)], "stable"),
            rename("bob", 1, &[], "dependency"),
        ];
        let frontier = StabilityFrontier::new(BTreeMap::from([("alice".to_owned(), 1)])).unwrap();
        assert!(matches!(
            frontier.validate_prefix(&changes),
            Err(CollaborationError::StablePrefixNotClosed { .. })
        ));
    }

    #[test]
    fn retained_suffix_must_follow_every_frontier_entry() {
        let changes = vec![
            rename("alice", 1, &[], "stable"),
            rename("bob", 1, &[], "concurrent"),
        ];
        let frontier = StabilityFrontier::new(BTreeMap::from([("alice".to_owned(), 1)])).unwrap();
        assert!(matches!(
            frontier.validate_prefix(&changes),
            Err(CollaborationError::RetainedChangeNotAfterFrontier { .. })
        ));
    }
}
