---
id: nuif:adr:0005
kind: adr
status: accepted
---

# ADR 0005 — Keep CRDT state out of canonical documents

Status: accepted

Collaboration engines operate over NUIF semantic operations and materialize canonical snapshots. Replica clocks/tombstones/history belong to a collaboration profile or sidecar, not every `.nuif` document.

This permits Automerge/Yjs experiments and independent non-collaborative implementations.
