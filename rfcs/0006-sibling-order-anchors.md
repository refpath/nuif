---
id: nuif:rfc:0006
kind: rfc
status: accepted
---

# RFC 0006 — Sibling order: canonical arrays and anchored operations

Status: accepted (decision delegated to research on 2026-08-29; evidence in `nuif:research:list-ordering-fractional-indexing-vs-list-crdts`, `nuif:research:crdt-tree-move-operation`, `nuif:research:figma-multiplayer-and-rendering-engineering`)

## Motivation

`nuif-protocol` expresses `Insert` and `Move` with an integer `index`. Integer positions are not commutative under independent edits, make replay order-sensitive, and cannot be merged without renumbering. The collaboration profile (`spec/10`) needs positions that converge; the canonical document (`ADR 0005`) must not carry replica metadata.

## Prior art

OpenUSD stores resolved child order as an array and expresses authored reordering as sparse `reorder` list operations composed at index time (`pcp/composeSite.cpp`). Figma uses fractional string keys per child, deduplicated server-side, and reports key growth and interleaving as known costs. Automerge and RGA insert relative to a neighbour identifier. Penpot's change log names an `after-shape`. Fugue (Weidner and Kleppmann 2023) is the list CRDT with proved forward and backward non-interleaving; Logoot and LSEQ interleave; the PaPoC 2019 interleaving anomaly definition is unsatisfiable per the Fugue paper.

## Decision

### Canonical form

1. A canonical document MUST represent sibling order as an ordered array of entity identifiers (`Entity.children`) with no duplicates. It MUST NOT carry order keys, tombstones, replica identifiers or timestamps.
2. Integer indices are a resolved view computed from the array; they never appear in canonical state or in operations.

### Operations

3. `Insert` and `Move` MUST specify position as an anchor: `Anchor::Start` or `Anchor::After(EntityId)`.
4. An anchor MUST refer to a current child of the target parent at application time; otherwise the operation fails with the typed conflict `AnchorMissing`, and patch application reports it.
5. A `Move` whose target parent is the moved entity or one of its descendants MUST fail with `CycleRejected`.
6. Within one patch, insertions at the same anchor are applied in patch order.
7. Preconditions MAY assert `ParentIs { entity, parent }` and `Follows { entity, anchor }`.

### Merge

8. In three-way merge, insertions from different branches at the same anchor are ordered by declared branch precedence and emit the informational conflict `OrderAmbiguous` listing the entities.
9. Concurrent moves of one entity to different positions produce `MoveConflict` carrying both targets; a profile MAY converge on one, but the conflict object MUST be retained until resolved.

### Collaboration profile

10. A collaboration profile maps anchored operations onto a list CRDT that satisfies forward non-interleaving (Fugue Definition 2) and SHOULD satisfy maximal non-interleaving. Tree moves follow the undo/redo algorithm of Kleppmann et al. 2021 with cycle rejection.
11. Checkpoint materialization MUST emit the plain array of rule 1.
12. Fractional keys MAY be used as profile-internal transport and MUST NOT appear in `nuif-core` types or canonical hashes.

### Type changes (`nuif-protocol`, signatures only)

```rust
pub enum Anchor { Start, After(EntityId) }

pub enum Operation {
    Insert { parent: Option<EntityId>, anchor: Anchor, entity: Entity },
    Move { entity: EntityId, new_parent: Option<EntityId>, anchor: Anchor },
    // Remove, Rename, SetExtension unchanged
}

pub enum Precondition {
    ParentIs { entity: EntityId, parent: Option<EntityId> },
    Follows { entity: EntityId, anchor: Anchor },
}

pub enum Conflict {
    AnchorMissing { entity: EntityId, parent: Option<EntityId>, anchor: Anchor },
    CycleRejected { entity: EntityId, new_parent: EntityId },
    MoveConflict { entity: EntityId, targets: [(Option<EntityId>, Anchor); 2] },
    OrderAmbiguous { parent: Option<EntityId>, anchor: Anchor, entities: Vec<EntityId> },
}
```

`Entity.children: Vec<EntityId>` is unchanged and documented as the canonical order with a no-duplicates invariant.

## Compatibility

No persisted documents exist. Adapters that import index-based formats compute anchors from the imported order.

## Security

Anchor resolution is O(children); cycle rejection is a parent-chain walk bounded by the depth limit in `spec/11`.

## Conformance tests

- operations suite: independent inserts under different parents commute to identical hashes; `AnchorMissing` and `CycleRejected` fixtures; replay of anchored logs is deterministic.
- merge suite: same-anchor concurrent inserts produce `OrderAmbiguous` with branch-precedence order; concurrent moves produce `MoveConflict`; Kleppmann's concurrent-move test cases (cycle-forming pair) converge without cycles.
- collaboration experiment: two profile engines converge to the same checkpoint array and no interleaving of two concurrent runs of sequential inserts (Fugue forward non-interleaving check).

## Rejected alternatives

- Integer index in operations: non-commutative; order-sensitive replay.
- Fractional keys in the canonical form: history-dependent hashes, unbounded key growth at one gap, jitter randomness, interleaving, no rebalancing procedure documented by Figma or rocicorp.
- List-CRDT identifiers in the canonical form: require tombstones and replica metadata, contradicting ADR 0005.
- Logoot or LSEQ positions in the profile: interleave under concurrent sequential inserts.

## Unresolved

- Branch precedence for rule 8 is a merge-policy parameter, not derivable from sources.
- Whether the profile adopts Fugue or FugueMax (and how Yjs YATA's ordering compares) is settled by the collaboration experiment.
