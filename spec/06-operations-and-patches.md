# 06 — Operations, patches and merge

Status: draft.

Operations are stable, serializable semantic mutations. Core operations: create entity, delete entity, move entity, set/unset property, add/remove relation, set/remove extension, bind/unbind token, apply instance override and transaction.

## Position (RFC 0006)

Sibling order in the canonical document is an ordered array of entity identifiers without keys, tombstones or replica metadata. `Insert` and `Move` specify position as an anchor, `Start` or `After(entity)`, never as an integer index. An anchor MUST refer to a current child of the target parent; otherwise the operation fails with `AnchorMissing`. A move into the moved entity or its descendants fails with `CycleRejected`. Insertions at the same anchor within one patch apply in patch order.

## Patches

A patch's optional `base_revision`, when present, is the profile-qualified canonical content hash of the document to which it applies. An implementation MUST reject a mismatch before applying any transaction. Transactions and operations are ordered. Preconditions MAY guard expected prior values, including `ParentIs` and `Follows`.

Undo is represented as inverse semantic operations or transaction history; it is not part of canonical document state. A generated inverse patch declares the hash of the post-apply document as its `base_revision`. The invariant "undo, copy, redo leaves the document unchanged" is a conformance relation.

## Merge

Three-way merge MUST prefer stable identity; structural matching is reserved for adapters without identity. Implementations MUST surface typed conflicts rather than selecting arbitrary winners: property, delete/edit, ordering (`OrderAmbiguous`, informational, ordered by declared branch precedence), move (`MoveConflict`), relationship, extension and semantic-lowering conflicts. Conflict objects are document state until resolved.

Merge rules per property kind (ordered list, identity set, scalar with tolerance) are declared by the schema so that structural merges are deterministic.

## Unknown entities (RFC 0007)

`Remove`, `Move`, `Rename`, `SetExtension`, `RemoveExtension` and core-property operations apply to entities of unknown kind unchanged. `SetUnknownPayload` is valid only for implementations that declare the payload's namespace.
