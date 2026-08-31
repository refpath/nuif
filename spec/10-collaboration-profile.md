---
id: nuif:spec:collaboration-profile
kind: specification
status: executable
---

# 10 — Collaboration profile

Status: executable bounded register and existing-tree structural profiles.

Collaboration is operation-based and layered above canonical NUIF.

A collaboration engine MUST be able to materialize a canonical NUIF snapshot without collaboration metadata. Replica IDs, clocks, tombstones and sync-state are profile data.

The profile defines convergence requirements, causal/change identifiers, transaction grouping, awareness/presence separation and checkpoint materialization. It does not mandate Automerge or Yjs.

Semantic conflicts that cannot be merged safely remain explicit conflict objects even if the underlying CRDT converges structurally.

## Executable register profile 0

`nuif-collab-registers-0` represents each collaboration change as a replica/counter dot, a transitive version-vector context and one semantic operation. The metadata lives in the profile state and is stripped from the materialized `Document`.

Register-like operations use one multi-value register per entity/property pointer. Causally superseded values leave the frontier. Concurrent distinct values remain in an explicit `SemanticConflict`; a deterministic selected dot permits a provisional canonical checkpoint without discarding the candidates from the checkpoint report. The operation-set join is commutative, associative and idempotent, and incomplete causal histories fail closed.

Profile 0 supports rename, size, container layout, grid-item placement, token, authored-value, extension-declaration, entity-extension and unknown-payload registers. It rejects insert, remove, move and restore-subtree. Structural collaboration requires a declared tree move/list algorithm, cycle handling and tombstone policy and MUST NOT be inferred from register convergence.

`cargo xtask gate-h` compares an operation-set maximality materializer with an incremental replica-log frontier materializer over every delivery permutation of the bounded conflict fixture. These are algorithmically separate in-repository implementations, not foreign-engine interoperability evidence.

## Executable existing-tree structural profile 0

`nuif-collab-tree-0` accepts only `Move` and `Delete` for entity identities
already present in one validated canonical base. Creation, subtree payloads,
relations, property changes and mixed structural/property transactions are not
part of this profile. A structural change uses the same dot and transitive
version-vector requirements as the register profile. Every engine and joined
operation set is bound to the canonical hash of exactly one base; merging
different bases is a typed failure. The dot's total order is `(counter,
replica)`.

A move carries `(entity, new_parent, anchor)`. `anchor` is either `Start` or a
stable position identifier. A base position is `Base(entity_id)`; every move
creates `Change(dot)`. An authoring surface MUST resolve a canonical
`After(entity_id)` against its current checkpoint and persist the resulting
stable position. It MUST NOT reconstruct a stale anchor from the entity's
current position after synchronization. A `Change(dot)` anchor MUST name a
received change included in the author's transitive causal context; a missing
or non-causal anchor change fails the checkpoint as incomplete history.

The materializer MUST behave as if all changes were applied in ascending dot
order:

1. An unknown entity or parent is a typed failure and produces no checkpoint.
2. Moving an entity below itself or its current descendant has no structural
   effect, remains in the applied history and emits `CycleRejected`.
3. A missing anchor, an anchor from another parent, or the entity's own
   position has no structural effect and emits its typed anchor conflict.
4. A valid move deactivates the entity's prior position and creates an active
   position under the target parent. Inactive positions remain as sibling
   origins.
5. Delete deactivates the prior position and assigns the entity to synthetic
   profile trash. Descendant relationships are retained in profile state.
6. A later valid move may restore a trashed entity or rescue one of its
   descendants.

Sibling order is an RGA-style origin traversal. Positions with the same origin
sort by descending position identifier, followed recursively by their own
descendants. Base sibling order is represented as an origin chain. Position
IDs, inactive positions, clocks and trash are profile metadata. The canonical
checkpoint stores only ordinary ordered child arrays and removes every entity
not reachable from a visible root; it MUST validate and hash as canonical NUIF.

Concurrent distinct moves of one entity, delete/move of one entity, deletion
of a destination parent and deletion of a moved entity's base ancestor MUST
remain typed semantic conflicts. Total ordering selects a provisional tree but
does not erase those candidates.

Gate H compares full sorted replay with an incremental local/rollback-replay
engine for all 5,040 deliveries of the bounded fixture, multiple joins, duplicate
delivery and a 4,096-change scaling case. Pinned Automerge 3.4.1 must reproduce
the exact immutable operation set through different merge orders and
save/load. That foreign check covers convergent transport only; it is not an
independent implementation of these tree semantics.

## Executable concurrent-creation profile 0

`nuif-collab-tree-create-0` is a separate bounded profile for concurrent
creation of leaf entities under a parent already present in one canonical
base. A creation change carries a dot, transitive version-vector context, a
base parent (or the root), a `Start` or `After(base-entity)` anchor and one
entity with an empty `children` list. The entity ID MUST not already exist in
the base. The parent and anchor MUST resolve against the base; creation below
a concurrently created parent is outside this profile.

The materializer groups changes by entity ID. A group with more than one
candidate produces `EntityIdCollision` containing every dot and selects the
greatest dot provisionally. For selected, valid creations, positions sharing
an anchor are ordered by descending dot and are placed around the unchanged
base sibling list. The canonical checkpoint contains ordinary entities and
child arrays only; dots, contexts and collision candidates remain profile
metadata. The checkpoint MUST validate and hash as canonical NUIF.

`cargo xtask gate-h` exhausts all 24 deliveries of the four-change fixture,
checks merge convergence, same-anchor ordering, explicit ID-collision
diagnostics, canonical metadata absence and typed failures for nested entities,
unknown parents/anchors and incomplete causal history. This profile does not
claim nested creation, deletion/resurrection, mixed property/structure
transactions or an independently authored tree materializer.

## Executable nested-creation profile 0

`nuif-collab-tree-create-nested-0` extends creation to a bounded causal parent
chain without changing `nuif-collab-tree-create-0`. A creation may name an
entity created by another change as its parent only when the child context
includes the selected parent dot. The parent chain is resolved before
materialization, cycles and unavailable parents fail as typed errors, and the
chain is capped at the profile's declared depth limit.

`Start` is the only supported anchor below a created parent. A base parent may
still use `Start` or `After(base-entity)`, preserving the base sibling order;
created-parent `After` anchors are rejected rather than guessed from delivery
order. The payload remains one leaf entity per change, so descendants are
separate changes and ordinary canonical child arrays are produced only after
all selected parent relationships validate. Entity-ID collisions remain
explicit `EntityIdCollision` conflicts with the greatest dot selected
provisionally.

The profile is intentionally one operation-set materializer. Its conformance
fixture exhausts all six deliveries of a causal parent, nested child and base
sibling, checks merge convergence and canonical metadata absence, and rejects
non-causal/unknown parents, created-parent anchors and incomplete history.
Deletions, resurrection, arbitrary created-parent anchors, mixed
property/structure transactions and a second independently authored tree
materializer remain future work.

## Executable nested-creation arbitrary-anchor profile 1

`nuif-collab-tree-create-nested-1` preserves profile 0's wire operation and
causal parent contract while admitting `After(entity)` anchors under base or
selected created parents. If the anchor is created, the change MUST causally
include the selected anchor dot and the anchor's selected parent MUST equal the
new change's parent. An anchor that is absent, non-causal, or owned by another
parent fails with a typed diagnostic. Same-anchor candidates retain descending
dot order, and anchor chains are materialized recursively under the bounded
`MAX_PARENT_DEPTH` limit.

The canonical checkpoint contains only ordinary entities and child arrays; the
anchor dots, causal contexts and active positions remain outside the document.
This profile still excludes deletion/resurrection, mixed property/structure
transactions and anchors to non-selected collision candidates. Gate H exhausts
all 24 deliveries of a causal sibling-chain fixture and checks merge
convergence, metadata absence and the typed negative boundaries.

## Executable mixed property/structure profile 0

`nuif-collab-mixed-0` uses one causal operation set for existing-tree structural
changes and register-like property changes. A checkpoint materializes the
structural operation-set result first, then applies the property register
frontiers to that document. Property and structural conflicts remain distinct,
and a property change whose entity was removed by the selected structural
result fails with `PropertyTargetUnavailable`; it is never silently discarded.

The profile does not admit creation operations or multiple semantic operations
under one change dot. Gate H exhausts all 24 deliveries of a four-change
fixture, checks merge convergence, explicit conflict sets and metadata-free
output, and rejects deleted targets and cross-kind missing dependencies.

## Causal-stability compaction profile 0

`nuif-collab-gc-0` is an explicit, conservative history-collection profile. It
does not infer stability from a local log and it never deletes canonical
document data. A host supplies a `StabilityFrontier`, a replica-to-counter map
attesting that every participant has observed the corresponding history. The
frontier MUST exactly match every locally observed replica clock. A missing,
behind or ahead counter is an unsafe-compaction error and MUST leave the
operation set unchanged.

When the frontier is exact, a register, concurrent-creation or existing-tree
materializer may emit a
`CompactionReceipt` and replace its complete operation history with the
metadata-free canonical checkpoint it already materializes. The receipt records
the source profile, source base hash, compacted checkpoint hash, frontier and
every dropped change identifier. The checkpoint's canonical hash and semantic
content MUST equal the pre-compaction checkpoint; replica IDs, clocks, conflict
candidates, positions, tombstones and receipts MUST remain outside the
canonical `Document`.

Profile 0 intentionally supports complete-history checkpoint compaction only.
It MUST NOT prune a stable prefix while retaining later changes, rebase causal
contexts, or rewrite structural position anchors. Those operations require a
versioned checkpoint-as-causal-base protocol and remain a future profile. A
caller MUST retain or archive the receipt and checkpoint according to its sync
and recovery policy; compaction is not an interoperability claim for arbitrary
CRDT logs.

`cargo xtask gate-h` runs the register operation-set and replica-log,
concurrent-creation and structural materializers through complete-history
compaction. It checks exact checkpoint equivalence, complete dropped-history
receipts, empty-history behavior, metadata absence and typed refusal of partial
and ahead frontiers. The release report is
`target/collaboration-gc-report.json`.
