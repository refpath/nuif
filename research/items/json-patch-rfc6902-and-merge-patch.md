---
id: nuif:research:json-patch-rfc6902-and-merge-patch
kind: standard
status: reviewed
title: JSON Patch (RFC 6902), JSON Pointer (RFC 6901) and JSON Merge Patch (RFC 7396) versus identity-addressed operations
source:
  url: https://www.rfc-editor.org/rfc/rfc6902.html
  authors: [Paul C. Bryan, Mark Nottingham, Paul Hoffman, James M. Snell]
  published_at: "RFC 6901 and RFC 6902 2013-04; RFC 7386 2014-10 obsoleted by RFC 7396 2014-10"
  license: IETF Trust
retrieved_at: 2026-08-29
tags: [json-patch, json-pointer, merge-patch, path-addressing, precondition, patch-format, identity]
confidence: 0.95
claims: [nuif:claim:sync-not-regenerate]
relations:
  - type: compares_to
    target: nuif:research:crdt-tree-move-operation
    note: Path-indexed move versus identity-addressed move under concurrent reordering.
  - type: related_to
    target: nuif:research:canonicalization-rfc8785-and-cbor-deterministic
    note: Both rely on the I-JSON number and string model.
  - type: related_to
    target: nuif:research:command-pattern-undo-and-event-sourcing
    note: The `test` operation is the standardised form of an operation precondition.
links:
  spec: [spec/06-operations-and-patches.md]
  adr: []
  rfc: []
  code: [crates/nuif-protocol]
  experiments: []
---

# Summary

RFC 6902 defines a JSON Patch as an ordered array of operations `add`, `remove`, `replace`, `move`, `copy` and `test`, each addressing its target with a JSON Pointer (RFC 6901). `test` compares the addressed value with a supplied value and fails the patch on inequality, which makes it a precondition mechanism. A patch is applied sequentially and atomically: any failing operation makes the whole patch unsuccessful. Array positions are addressed by index or by `-` (append), and `add` shifts later elements. RFC 7396 (JSON Merge Patch, obsoleting RFC 7386) defines a recursive object overlay in which `null` deletes a member, arrays and non-object values are replaced wholesale, and a literal `null` cannot be stored. Both formats are path-addressed: a concurrent reorder or insert invalidates array indices, `move` cannot express "move entity X" independently of X's current path, and neither format defines an inverse. Identity-addressed operations, as used in nuif-protocol, remove path dependence but still carry an index in `Insert` and `Move`.

## Evidence

- RFC 6902, April 2013, Standards Track, Bryan and Nottingham; operations defined in §4: `add` (§4.1) inserts into an array at the index shifting later elements, or appends with `-`; "The specified index MUST NOT be greater than the number of elements in the array"; `remove` (§4.2) requires the target to exist; `replace` (§4.3) equals remove then add; `move` (§4.4): "The 'from' location MUST NOT be a proper prefix of the 'path' location"; `copy` (§4.5); `test` (§4.6) compares strings byte-wise, numbers numerically, arrays element-wise, objects member-wise regardless of order. https://www.rfc-editor.org/rfc/rfc6902.html (retrieved 2026-08-29).
- RFC 6902 §5 error handling: if an operation fails, "evaluation of the JSON Patch document SHOULD terminate and application of the entire patch document SHALL NOT be deemed successful"; HTTP PATCH is atomic. (retrieved 2026-08-29).
- RFC 6901, April 2013: reference tokens separated by `/`, `~` escaped as `~0` and `/` as `~1` (§3); array tokens are base-10 digits without leading zeros or the single character `-` denoting the position after the last element (§4); "This specification does not define how errors are handled" (§7). https://www.rfc-editor.org/rfc/rfc6901.html (retrieved 2026-08-29).
- RFC 7396, October 2014, Hoffman and Snell, obsoletes RFC 7386; MergePatch pseudocode in §2 (transcribed below); "It is not possible to patch part of a target that is not an object, such as to replace just some of the values in an array"; non-object patches replace the entire target; explicit `null` values in the target cannot be expressed (§1). https://www.rfc-editor.org/rfc/rfc7396.html (retrieved 2026-08-29). RFC 7386 text retrieved for comparison (https://www.rfc-editor.org/rfc/rfc7386.html, retrieved 2026-08-29); the RFC 7396 header shows the obsoletion.

## Mechanism

JSON Patch application (RFC 6902 §3-5):

```
apply(doc, ops):
  for op in ops (in order):
    target := resolve(doc, op.path)        -- RFC 6901; array index or "-"
    match op.op:
      add     : insert at index (shift right) | append if "-" | set/replace member
      remove  : target MUST exist; array elements shift left
      replace : target MUST exist; remove then add
      move    : from MUST exist; from not a proper prefix of path; remove(from) then add(path, value)
      copy    : add(path, value at from)
      test    : fail unless value(target) == op.value (type-specific equality)
    on failure: abort; entire patch unsuccessful
```

JSON Merge Patch (RFC 7396 §2):

```
MergePatch(Target, Patch):
  if Patch is an Object:
    if Target is not an Object: Target = {}
    for each Name/Value in Patch:
      if Value is null: remove Name from Target if present
      else Target[Name] = MergePatch(Target[Name], Value)
    return Target
  else: return Patch
```

Path dependence (NUIF interpretation of the source rules): an operation `{"op":"move","from":"/children/3","path":"/children/0"}` denotes whichever element occupies index 3 at application time; a concurrent `add` at `/children/1` shifts the intended element to index 4, and `test` can only detect the mismatch by comparing the entire element value. Identity addressing (`Move { entity: EntityId, .. }`) names the element regardless of position; the remaining position-dependent component is the destination index, which the same concurrent insert also shifts.

Inverse computation: neither RFC defines an inverse; `remove` and `replace` discard the prior value, so an inverse requires the applier to record it, which is the memento/inverse-recording problem treated in nuif:research:command-pattern-undo-and-event-sourcing.

## NUIF relevance

**Borrow**
- `test` as the canonical shape of a precondition: an operation-level guard that compares an addressed value with an expected value and aborts the patch on mismatch, matching spec/06 "Preconditions MAY guard expected prior values".
- Sequential, atomic patch semantics (RFC 6902 §5) for NUIF transactions: all operations of a transaction succeed or the transaction is not applied.
- The `move` prefix rule (§4.4) as the path-form of NUIF's acyclicity precondition for `Move`.

**Adapt**
- Replace path addressing with entity identity everywhere, and replace the destination `index` in `Insert` and `Move` with an order anchor (preceding sibling ID or order key) so that operations remain valid under concurrent sibling edits.
- Merge-patch style overlay semantics are usable only for unordered property maps of a single entity (set/unset property), never for containment sequences.
- Record removed values (or reference the base revision's content hash) in `Remove`/`SetProperty` so that patches are invertible; RFC 6902 leaves this to the application.

**Reject**
- JSON Merge Patch for structural edits: arrays are replaced wholesale and `null` is overloaded as delete, which conflicts with NUIF unset semantics and explicit null property values.
- JSON Pointer array indices as the identity of children in any NUIF patch encoding.

## Open questions

- Whether a NUIF `test`-style precondition should compare by value or by content hash of the addressed subtree; the latter is cheaper for large subtrees but couples preconditions to the canonical encoding.
- Whether a JSON Patch projection of NUIF patches (for tooling interoperability) should be generated against a fixed base snapshot with indices resolved at generation time, and marked as non-mergeable.
