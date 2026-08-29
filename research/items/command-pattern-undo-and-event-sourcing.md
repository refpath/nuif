---
id: nuif:research:command-pattern-undo-and-event-sourcing
kind: synthesis
status: reviewed
title: Undo models - Command and Memento patterns, event sourcing, and undo in Blender, Photoshop and Figma
source:
  url: https://martinfowler.com/eaaDev/EventSourcing.html
  authors: [Erich Gamma, Richard Helm, Ralph Johnson, John Vlissides, Martin Fowler, Blender Foundation, Adobe, Figma engineering (Evan Wallace)]
  published_at: "Design Patterns 1994 (Addison-Wesley, ISBN 0-201-63361-2); Fowler 2005-12-12; Blender developer documentation undated; Figma blog 2019-10"
  license: Book copyright Addison-Wesley; Fowler site copyright; Blender docs CC-BY-SA; Adobe help copyright; Figma blog copyright
retrieved_at: 2026-08-29
tags: [undo, redo, command-pattern, memento, event-sourcing, inverse-operation, snapshot, transaction, replay, editor-architecture]
confidence: 0.82
claims: [nuif:claim:sync-not-regenerate, nuif:claim:collab-profile]
relations:
  - type: related_to
    target: nuif:research:crdt-tree-move-operation
    note: The LogMove record with prior parent is an inverse-recording command log.
  - type: related_to
    target: nuif:research:json-patch-rfc6902-and-merge-patch
    note: JSON Patch `test` is a precondition; JSON Patch defines no inverse, which this record covers.
  - type: compares_to
    target: nuif:research:figma
    note: Figma's multiplayer undo semantics as described by Figma engineering.
  - type: related_to
    target: nuif:research:content-addressed-versioning
    note: Snapshot (memento) undo and content-addressed checkpoints are the same mechanism at different granularity.
links:
  spec: [spec/06-operations-and-patches.md, spec/10-collaboration-profile.md]
  adr: [adrs/0005-collaboration-profile.md]
  rfc: []
  code: [crates/nuif-protocol]
  experiments: []
---

# Summary

Gamma et al. describe two complementary mechanisms: Command encapsulates a request as an object so that requests can be queued, logged and undone, with undo implemented by storing enough state in the command to reverse its effect and a history list for undo/redo; Memento captures an object's internal state so it can be restored later without breaking encapsulation. Fowler's event sourcing records every state change as an event, from which state can be rebuilt, queried at any past time, or replayed after correcting an event; reversal is either by an inverse event (possible only when the event carries enough information, "add $10" rather than "set to $110") or by reverting to a snapshot and replaying. Editors combine these: Blender keeps a single undo stack of typed steps, stateful or differential, with global undo implemented as an in-memory `.blend` file written with the regular file-writing code; Photoshop keeps a bounded list of history states (snapshot-based, not saved with the document) with an optional non-linear mode; Figma's multiplayer undo is defined so that undoing and redoing back to the present leaves the document unchanged, with undo rewriting the redo history relative to the current shared state. The invariants that transfer to NUIF are listed under Mechanism.

## Evidence

- Command pattern: Gamma, Helm, Johnson, Vlissides, Design Patterns (1994), Command, pp. 233-242; intent is to encapsulate a request as an object, parameterise clients, queue or log requests, "and support undoable operations"; implementation notes cover storing state for reversal and a history list for undo and redo. Memento, pp. 283-291; intent is to capture and externalise internal state without violating encapsulation so the object can be restored later. (Book; page numbers from the table of contents; not retrieved online.)
- Event sourcing: definition "Capture all changes to an application state as a sequence of events"; capabilities Complete Rebuild, Temporal Query, Event Replay; reversal: "all the capabilities of reversing events can be done instead by reverting to a past snapshot and replaying the event stream"; the example contrasts "add $10 to Martin's account" with "set Martin's account to $110"; external-system updates and queries are the stated hazards of replay. Fowler, 2005-12-12, https://martinfowler.com/eaaDev/EventSourcing.html (retrieved 2026-08-29).
- Blender undo system: "Undo is organized as an 'undo stack' storing a list of 'undo steps'"; steps are relative (must be loaded in sequence) or absolute; "Currently, Blender undo stack is fully relative"; steps are stateful ("stores the state of some data, and can be loaded regardless of the direction") or differential ("only stores the difference to the previous step ... either applied (redo) or unapplied (undo)"), and with differential steps undoing requires unapplying step n+1; only data is stored, not UI; one stack gathers global undo, edit-mode undo, sculpt/paint undo; skipped steps are hidden intermediates; undo push is driven by the operator system; layers are `ed_undo.cc`, `undo_system.cc` (BKE), and per-type implementations such as `memfile_undo.cc` and `sculpt_undo.cc`, where memfile undo "uses BKE_memfile_ functions from blender_undo.c, which in turns uses read/write .blend file code from BLO". https://developer.blender.org/docs/features/core/undo/ (retrieved 2026-08-29; page marked WIP by its authors). Memfile improvement tracking: "Undo: support implicit-sharing in memfile undo step", https://projects.blender.org/blender/blender/pulls/106903 (search result, not retrieved).
- Photoshop history: Adobe help pages returned HTTP 403 and timeouts on retrieval; indexed text of https://helpx.adobe.com/photoshop/using/performance-preferences.html states Photoshop saves up to 1,000 history states with a default of 50, that "Allow Non-Linear History" permits editing from any state without deleting later ones, and that history states consume scratch memory (search snippets, 2026-08-29; treated as secondary evidence).
- Figma multiplayer undo: "if you undo a lot, copy something, and redo back to the present, the document should not change"; an undo "modifies redo history at the time of the undo, and likewise a redo operation modifies undo history at the time of the redo", stated as necessary so users do not overwrite others' later edits; changes are applied optimistically on the client, conflicting server updates are discarded while a client change is unacknowledged, "the server can define the order of events", per-property last-writer-wins; parent link and fractional position are one property updated atomically. https://www.figma.com/blog/how-figmas-multiplayer-technology-works/ (retrieved 2026-08-29).
- Undo integrated with transformation: Sun et al. (TOCHI 1998) §7 integrate their GOT control algorithm with an undo/do/redo scheme, undoing later operations, applying the new one and redoing (https://www.cs.cityu.edu.hk/~jia/research/reduce98.pdf, retrieved 2026-08-29); Kleppmann et al. use the same undo-do-redo shape with a recorded prior parent (nuif:research:crdt-tree-move-operation, Fig. 4 lines 32-49).

## Mechanism

Two undo representations:

```
inverse-operation undo (Command):    history = [(op_i, inv_i)]; undo = apply(inv_k); redo = apply(op_k)
   requires: inv_i computable at record time  ->  op must carry or capture prior state
memento / snapshot undo (Memento):   history = [state_i]; undo = restore(state_{k-1})
   requires: cheap snapshot (structural sharing, chunk dedup) ; no inverse needed
differential step (Blender):         stores delta to previous step; undo unapplies step k+1, redo applies step k
```

Recording inverse operations for a replay log (NUIF interpretation, derived from Command, LogMove and Fowler's "add $10" example):

```
record(op, doc):
  pre  := preconditions(op)          -- e.g. exists(entity), parent(entity) = p_old, prop(entity,k) = v_old
  inv  := inverse(op, doc)           -- Move{e,new}     -> Move{e, p_old, order_old}
                                     -- SetProperty{k,v} -> SetProperty{k, v_old} | UnsetProperty{k}
                                     -- Remove{e}       -> Insert{e, parent_old, order_old, payload_old}
                                     -- Insert{e}       -> Remove{e}
  log  += (op, pre, inv)
invariant: apply(inv, apply(op, doc)) = doc   whenever pre holds in doc
```

Invariants that the sources support:

1. Undo is a semantic inverse under preconditions: an inverse is only valid against the state the operation produced; if preconditions of the inverse fail (another actor changed the value), the editor must either transform, skip, or resolve relative to current state (Figma; Sun et al. §7).
2. Transactions are atomic undo units: an undo step corresponds to one user-level operation, possibly comprising many primitive operations (Blender undo push per operator; RFC 6902 §5 atomicity).
3. Redo stack invalidation: in linear history a new edit after undo discards the redo branch (Photoshop default; Blender relative stack); non-linear history retains it (Photoshop option); in multiplayer, undo rewrites redo entries against the current shared state so that undo-then-redo is the identity on the document (Figma).
4. Snapshot undo is equivalent to inverse undo in capability (Fowler) but differs in cost: inverse logs are proportional to change size, snapshots to state size unless deduplicated (Blender memfile reuses `.blend` writing).
5. Replay must be side-effect free: events that trigger external effects cannot be replayed naively (Fowler), which constrains what a NUIF operation may do (pure document mutation).
6. Undo history is not document state: Photoshop discards history on save; Blender stores steps in memory; spec/06 states undo "is not part of canonical document state".

## NUIF relevance

**Borrow**
- Record `(op, preconditions, inverse)` triples per transaction in the profile log, so a NUIF patch can be inverted mechanically and replayed deterministically to the same canonical hash.
- Blender's stateful/differential step distinction: NUIF checkpoints (content-addressed snapshots) are stateful steps; patches between checkpoints are differential steps.
- Figma's invariant that undo followed by redo returns the same document, as a test property for the collaboration profile.

**Adapt**
- Inverse computation must consult the base revision: `Remove` must capture the removed subtree (or its content hash plus a retrievable object) so that the inverse `Insert` is complete; nuif-protocol's `Remove { entity }` currently carries nothing to invert.
- Multiplayer undo semantics belong in spec/10: the profile must define whether undo of a property change restores the user's prior value, the current value's predecessor, or a conflict object when another actor has since written the property.
- Transaction granularity: nuif-protocol `Transaction { id, operations }` is the unit of undo; the log should also carry an actor and a base revision so that undo across replicas is well-defined.

**Reject**
- Snapshot-only undo for the canonical format: it is acceptable for editors but does not yield the serializable inverse operations that spec/06 requires.
- Storing undo history inside the canonical document.

## Open questions

- Whether inverse operations should be stored explicitly or derived at replay time from the base snapshot; explicit storage doubles log size but makes inverse validity checkable without the base.
- The precise multiplayer undo rule for NUIF (Figma's implementation details beyond the stated invariant are not public) and its interaction with CRDT reordering, where a later-timestamped undo may itself be reordered.
- Blender's memfile chunk deduplication and implicit sharing details are only referenced through issue trackers; a primary description of the chunk comparison algorithm was not retrieved.
