# NUIF collaboration profiles

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

## Structural tree profile 0

`nuif-collab-tree-0` is a separate bounded profile for moves, reorders and
deletion of identities already present in the canonical base. It does not
weaken the register profile's rejection of structural operations or pretend a
move is an ordinary last-writer-wins property.

Each move has a unique Lamport-ordered dot, target parent and stable sibling
origin. Base positions are identified by entity ID; later positions are
identified by the change dot. Position identifiers, inactive origins and the
synthetic trash parent are collaboration metadata and never enter canonical
NUIF. Within one sibling list, entries sharing an origin are traversed in
descending identifier order and retain inactive origins, following the core
RGA rule. The public checkpoint resolves canonical `Anchor` values to stable
positions so a later operation cannot accidentally bind to a different move of
the same entity. Both materializers are bound to one canonical base hash;
different-base joins fail. A change-position anchor must exist and occur in the
author's transitive causal history.

Changes are replayed in ascending unique timestamp order. A move that would
make its destination a descendant of itself is retained but has no tree effect
and produces `CycleRejected`. Deletion moves an entity under profile trash;
its descendants remain available so a concurrent or later move can rescue
them. Canonical checkpoints contain only the forest reachable outside trash.
Concurrent move/move, delete/move, deleted-parent and delete/descendant-move
intent remains in typed conflicts even though a deterministic checkpoint is
available.

`StructuralOperationSetEngine` replays a sorted operation set.
`StructuralUndoRedoEngine` applies monotonic local changes directly and rolls
back/replays when a lower timestamp arrives. Gate H exhausts all 5,040 deliveries
of a seven-replica move/delete/cycle/stable-anchor fixture, checks join and idempotence, and
compares both paths. A 4,096-change/4,097-entity release trial guards the linear
checkpoint path.

Pinned `@automerge/automerge` 3.4.1 independently merges immutable structural
change records forward, reverse and in a different partition order, then
checks duplicate merge and save/load. Automerge is the foreign convergent
transport oracle only: it does not implement NUIF's tree move, cycle, trash or
semantic-conflict rules. Concurrent creation, partial causal garbage
collection, combined property/structure transactions and an independently
authored tree materializer remain outside this profile; complete-history
compaction is specified separately below.

## Concurrent creation profile 0

`nuif-collab-tree-create-0` is a deliberately smaller profile for creating
leaf entities concurrently under a parent that already exists in one
canonical base. It supports `Start` and `After(base-entity)` anchors. New
positions sharing an anchor are ordered by descending `(counter, replica)`;
the base sibling order is retained. Every accepted entity is inserted only
after the resulting document validates, and creation metadata is removed from
the canonical checkpoint.

An entity ID collision is not silently discarded: the checkpoint reports every
candidate and selects the greatest dot provisionally. Nested entities,
creation below a concurrently created parent, deletion/resurrection and mixed
property/structural transactions are rejected by the profile boundary. The
four-change conformance fixture exhausts all 24 delivery orders, checks merge
convergence and metadata absence, and exercises typed negative cases. This is
an executable bounded profile, not a claim that general tree creation is
solved.

## Nested creation profile 0

`nuif-collab-tree-create-nested-0` permits a creation change to use another
selected creation as its parent when the child context includes that parent's
dot. Parent chains are resolved before materialization and are capped at
`MAX_PARENT_DEPTH`. A created parent accepts only `Start`; base parents retain
the original `Start` and `After(base-entity)` anchors. Unknown or non-causal
parents, parent cycles, created-parent `After` anchors and depth overflow fail
with typed errors. The payload remains one leaf per change and collisions stay
explicit.

Gate H exhausts all six deliveries of a causal parent/child/base-sibling
fixture, checks merge convergence and metadata absence, and exercises every
declared negative boundary. This is a separate extension profile; the original
leaf-only creation profile remains unchanged.

## Nested creation arbitrary-anchor profile 1

`nuif-collab-tree-create-nested-1` keeps the same leaf payload and causal
parent rules while allowing `After(entity)` to name a selected created sibling.
The change must causally include the dot that won the entity-ID selection, and
the selected anchor must belong to the same parent. This permits deterministic
insertion chains below created parents without deriving order from delivery
order or leaking position metadata into the canonical document.

The profile remains bounded by the same change, replica and parent-depth
limits. Deletion/resurrection, mixed property/structure transactions and
anchors to collision losers remain outside its contract. Gate H exhausts all
24 deliveries of a four-change fixture and checks causal, unknown-anchor and
wrong-parent failures.

## Mixed property/structure profile 0

`nuif-collab-mixed-0` carries existing-tree structural changes and
register-like property changes in one causal operation set. The materializer
resolves structure first, then applies property registers to the resulting
document, so a property edit targeting an entity removed by structure is a
typed `PropertyTargetUnavailable` error rather than a silently lost update.
Property and structural conflict sets remain separate and the canonical
checkpoint contains no collaboration metadata. Creation changes and multiple
operations under one change dot remain outside this profile.

## Causal-stability compaction profile 0

`nuif-collab-gc-0` provides the first executable history-collection boundary.
`gc::StabilityFrontier` is caller-attested and must exactly cover every
locally observed replica clock. `OperationSetEngine`, `ReplicaLogEngine` and
`StructuralOperationSetEngine` expose `compact_stable`; each validates the
existing checkpoint first, then returns a `CompactionReceipt` alongside the
unchanged canonical checkpoint. The receipt is the audit trail for the source
base, compacted hash, frontier and dropped change IDs.

The profile collects a complete history only. Partial pruning, causal-context
rebasing, structural position-anchor rewriting and recovery from unseen remote
changes are refused with `CollaborationError::UnsafeCompaction` and remain
future protocol work. Compaction never mutates the canonical document or puts
collaboration metadata into it. Gate H writes
`target/collaboration-gc-report.json` and checks both successful and refused
paths.

## Causal register prefix profile 0

`nuif-collab-gc-prefix-0` is the first bounded partial-collection extension.
It is register-only and accepts a caller-attested frontier when the stable
prefix is causally closed and every retained change includes the complete
frontier. The stable prefix is materialized into a metadata-bearing
`CausalCheckpointBase`; retained changes are replayed through
`ResumedOperationSetEngine` and must reproduce the complete checkpoint's
canonical hash, document and conflicts. A `CompactionReceipt` records both
dropped and retained dots.

The profile deliberately refuses concurrent retained-versus-stable register
changes, structural position rebasing, frontier inference and unseen remote
history. These boundaries are typed (`StablePrefixNotClosed` and
`RetainedChangeNotAfterFrontier`) and are not inferred from delivery order.
The conformance report is `target/collaboration-gc-prefix-report.json`.
