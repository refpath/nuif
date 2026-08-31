//! Explicit, conservative causal-stability compaction metadata.

use super::{ChangeId, CollaborationError, MAX_REPLICAS, validate_replica};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Profile identifier for checkpoint-based collaboration history collection.
pub const PROFILE_NAME: &str = "nuif-collab-gc-0";

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
}
