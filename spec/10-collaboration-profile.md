# 10 — Collaboration profile

Status: exploratory profile.

Collaboration is operation-based and layered above canonical NUIF.

A collaboration engine MUST be able to materialize a canonical NUIF snapshot without collaboration metadata. Replica IDs, clocks, tombstones and sync-state are profile data.

The profile defines convergence requirements, causal/change identifiers, transaction grouping, awareness/presence separation and checkpoint materialization. It does not mandate Automerge or Yjs.

Semantic conflicts that cannot be merged safely remain explicit conflict objects even if the underlying CRDT converges structurally.
