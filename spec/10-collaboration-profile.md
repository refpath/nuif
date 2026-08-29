# 10 — Collaboration profile

Status: executable bounded register profile; structural collaboration remains exploratory.

Collaboration is operation-based and layered above canonical NUIF.

A collaboration engine MUST be able to materialize a canonical NUIF snapshot without collaboration metadata. Replica IDs, clocks, tombstones and sync-state are profile data.

The profile defines convergence requirements, causal/change identifiers, transaction grouping, awareness/presence separation and checkpoint materialization. It does not mandate Automerge or Yjs.

Semantic conflicts that cannot be merged safely remain explicit conflict objects even if the underlying CRDT converges structurally.

## Executable register profile 0

`nuif-collab-registers-0` represents each collaboration change as a replica/counter dot, a transitive version-vector context and one semantic operation. The metadata lives in the profile state and is stripped from the materialized `Document`.

Register-like operations use one multi-value register per entity/property pointer. Causally superseded values leave the frontier. Concurrent distinct values remain in an explicit `SemanticConflict`; a deterministic selected dot permits a provisional canonical checkpoint without discarding the candidates from the checkpoint report. The operation-set join is commutative, associative and idempotent, and incomplete causal histories fail closed.

Profile 0 supports rename, size, layout, token, authored-value, extension-declaration, entity-extension and unknown-payload registers. It rejects insert, remove, move and restore-subtree. Structural collaboration requires a declared tree move/list algorithm, cycle handling and tombstone policy and MUST NOT be inferred from register convergence.

`cargo xtask gate-h` compares an operation-set maximality materializer with an incremental replica-log frontier materializer over every delivery permutation of the bounded conflict fixture. These are algorithmically separate in-repository implementations, not foreign-engine interoperability evidence.
