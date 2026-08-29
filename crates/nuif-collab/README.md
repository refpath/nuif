# NUIF collaboration register profile 0

`nuif-collab-registers-0` is a bounded operation-set collaboration profile above canonical NUIF. It proves convergence for register-like semantic operations without adding replica IDs, version vectors, histories or conflicts to `Document`.

## Change model

A change has a dot `(replica, counter)`, a version-vector context and one NUIF semantic operation. Replica counters are contiguous. Contexts must name received changes and transitively include their contexts; incomplete history fails closed. Replica identifiers and collection sizes have declared limits.

The profile maps these operations to multi-value registers:

- rename, horizontal/vertical size and layout;
- set/remove token;
- document extension declarations;
- set/remove authored property value;
- set/remove entity extension;
- set unknown payload.

Insert, remove, move and restore-subtree are rejected before ingestion. They require a tree/list CRDT with explicit cycle, deletion and sibling-order semantics; total-ordering them as ordinary registers would overstate correctness.

For each property key, causally superseded changes leave the frontier. Concurrent identical values coalesce without a conflict. Concurrent different values create a `SemanticConflict` containing every frontier candidate and a deterministic selected dot. The selected values materialize a canonical checkpoint in causal order; cross-register model invariant failures return a typed apply error rather than a partial checkpoint.

## Two materializers

`OperationSetEngine` joins a `BTreeMap<ChangeId, Change>` and computes maximal changes pairwise. `ReplicaLogEngine` joins per-replica logs and maintains each register's maximal causal frontier incrementally. Their merge methods are atomic on error. Both materialize the same public `Checkpoint`, but their frontier algorithms and storage representations are distinct.

This is algorithmic independence inside one repository, not an externally authored CRDT implementation and not an Automerge/Yjs interoperability claim.

## Automated evidence

`cargo xtask gate-h` runs a seven-change, three-replica responsive-card history with:

- a causal overwrite;
- concurrent card-name and variant edits producing two explicit property conflicts;
- all 5,040 delivery permutations through both materializers;
- different three-way merge orders and duplicate delivery;
- opaque unknown-payload preservation and canonical-text inspection for leaked collaboration metadata;
- negative cases for missing history, duplicate dots, invalid local context, structural operations and semantic apply failure.

The release-mode report is `target/collaboration-report.json` and is part of `cargo xtask all` and CI artifact upload.
