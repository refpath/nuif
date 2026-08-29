---
id: nuif:research:crdt-tree-move-operation
kind: paper
status: reviewed
title: A highly-available move operation for replicated trees (Kleppmann, Mulligan, Gomes, Beresford)
source:
  url: https://doi.org/10.1109/TPDS.2021.3118603
  doi: 10.1109/TPDS.2021.3118603
  repository: https://github.com/trvedata/move-op
  authors: [Martin Kleppmann, Dominic P. Mulligan, Victor B. F. Gomes, Alastair R. Beresford]
  published_at: "2021-10 (IEEE TPDS vol. 33 no. 7, pp. 1711-1724)"
  license: IEEE copyright; author-hosted open-access PDF; proofs and code MIT
retrieved_at: 2026-08-29
tags: [crdt, tree, move-operation, undo-redo, isabelle, convergence, automerge, fractional-indexing, rga, collaboration]
confidence: 0.93
claims: [nuif:claim:collab-profile, nuif:claim:sync-not-regenerate]
relations:
  - type: extends
    target: nuif:research:automerge-yjs
    note: Supplies the tree-move algorithm that Automerge lacks and that its JSON CRDT (RGA lists) is being extended with.
  - type: supports
    target: nuif:claim:collab-profile
    note: Shows a convergent tree CRDT whose replica metadata (timestamps, log, trash) is separable from the materialised tree.
  - type: compares_to
    target: nuif:research:figma
    note: Figma resolves cycles by server rejection and orders children by fractional indexing; this paper resolves cycles without a server.
  - type: related_to
    target: nuif:research:operational-transformation-vs-crdt
    note: Instance of an operation-based CRDT whose commutation is obtained by undo-do-redo rather than by transformation functions.
  - type: related_to
    target: nuif:research:command-pattern-undo-and-event-sourcing
    note: Uses recorded inverse information (old parent) in a log to undo and redo operations mechanically.
links:
  spec: [spec/06-operations-and-patches.md, spec/10-collaboration-profile.md]
  adr: [adrs/0005-collaboration-profile.md]
  rfc: []
  code: [crates/nuif-protocol]
  experiments: []
---

# Summary

The paper defines an operation-based CRDT for trees whose only operation is `Move t p m c` (timestamp, new parent, metadata, child). Node creation is a first move; deletion is a move under a designated trash node. Each replica keeps the tree as a set of `(parent, meta, child)` triples plus a log of applied moves in descending timestamp order, each log entry recording the child's previous parent. A remote operation with timestamp `t` is applied by undoing every logged operation with timestamp greater than `t`, applying the new operation, and redoing the undone operations. Applying an operation whose child is an ancestor of its destination is a no-op. The result is a state identical to sequential application in timestamp order, so any permutation of the same operation set converges. Convergence, acyclicity and unique-parent invariants are mechanised in Isabelle/HOL, and Scala code is extracted from the proofs. Automerge does not yet ship this algorithm; Da and Kleppmann (PaPoC 2024) adapt it to the Automerge operation set and report that Automerge currently models a move as delete-plus-reinsert. Sibling order is delegated to a list CRDT identifier stored in the metadata field, the alternative being fractional indices as used by Figma.

NUIF interpretation is separated from source statements in the relevance section.

## Evidence

- Bibliographic data: IEEE TPDS vol. 33, no. 7, pp. 1711-1724, DOI 10.1109/TPDS.2021.3118603; open-access PDF at https://martin.kleppmann.com/papers/move-op.pdf; code and proofs at https://github.com/trvedata/move-op (author page https://martin.kleppmann.com/2021/10/07/crdt-tree-move-operation.html, retrieved 2026-08-29).
- Motivation: concurrent moves of the same directory produced duplication in Dropbox (Fig. 1a) and one of the two intended outcomes in Google Drive (Fig. 1c/d); concurrent reciprocal moves (A under B, B under A) can create a cycle (Fig. 2). PDF §2.1-2.2, pp. 2-3.
- Algorithm definition: Fig. 4 (59 lines of Isabelle/HOL) defines `state = log_op list × (n × m × n) set` (line 14), `get_parent` (lines 16-20), the inductive `ancestor` relation (lines 22-24), `do_op` (lines 26-30) with the guard `if ancestor tree c newp ∨ c = newp then tree` (line 29), `undo_op` (lines 32-35), `redo_op` (lines 37-40), `apply_op` (lines 42-49), `apply_ops` as `foldl` (lines 51-52), and the `unique_parent` and `acyclic` predicates (lines 54-59). PDF p. 6.
- Undo-do-redo: "it first undoes the effect of any operations with a timestamp greater than t, then performs the new operation, and finally re-applies the undone operations." PDF §3.4, p. 7. The log is kept in descending timestamp order; `LogMove` adds an `oldp :: (n × m) option` field (PDF §3.2, p. 7).
- Conflict semantics: concurrent moves of one node are resolved by the greater timestamp; an operation that would close a cycle is ignored because `do_op` checks against the tree produced by all lower-timestamped operations; ignored operations must remain in the log because later lower-timestamped operations can change their safety. PDF §3.5, pp. 7-8.
- Creation/deletion: creation is the first move of a fresh node; deletion moves to a trash node; children of deleted nodes are retained so a concurrent move can bring them back. Node creation may bypass undo-redo under three stated assumptions, proved safe in Isabelle; deletion cannot. PDF §3.6, p. 8.
- Log truncation and garbage collection use causal stability: operations with timestamp at or below the causally stable threshold can be dropped; trashed subtrees can be discarded once the trashing operation is stable. PDF §3.7, p. 8.
- Sibling ordering: "This can be implemented by maintaining an additional list CRDT for each branch node, e.g. using RGA [14] or Logoot [15]"; the list element ID is placed in the metadata field, and reordering is a move with unchanged parent and a new ID. PDF §3.7, pp. 8-9.
- Theorems (all machine-checked): `apply_ops_unique_parent`, `apply_ops_acyclic`, and `apply_ops_commutes` (assumes `set ops1 = set ops2` and distinct timestamps, shows `apply_ops ops1 = apply_ops ops2`); strong eventual consistency is obtained through the framework of Gomes et al. PDF §4.1-4.2, p. 9.
- Proof size: 59 lines of definitions plus 2,495 lines of proof (203 unique parent, 443 acyclicity, 450 commutation/convergence, 327 SEC, 743 executable refinement, 779 creation optimisation); checking takes about 3 minutes. PDF §5.3, p. 11. Repository layout: `proof/Move.thy`, `proof/Move_Acyclic.thy`, `proof/Move_SEC.thy`, `proof/Move_Code.thy`, `evaluation/` Scala, MIT licence (GitHub README, retrieved 2026-08-29).
- Complexity: worst case `O(nd)` per applied operation, `n` being the number of logged operations to undo and redo and `d` the tree depth. PDF §5.1, p. 10. Local operations need no undo/redo because their Lamport timestamp exceeds all logged ones (median 1-2 µs hand-written, about 50 µs generated code); remote saturation at 5,700 ops/s hand-written versus 600 ops/s generated, with about 200 undos/redos per remote operation at peak. PDF §5.1, p. 11.
- Comparison to state machine replication: leader ordering reaches 22,000 ops/s but requires a 145-176 ms round trip per operation and no offline editing. PDF §5.2, pp. 11-12.
- Automerge status: Automerge merge-rules documentation lists no move operation and orders concurrent inserts at one position by an arbitrary but deterministic choice (https://automerge.org/docs/reference/under-the-hood/merge-rules/, retrieved 2026-08-29). The Automerge JSON CRDT states "Our approach for handling insertions is based on the RGA algorithm" (Kleppmann and Beresford, IEEE TPDS 2017, DOI 10.1109/TPDS.2017.2697382, §4, INSERT1/INSERT2 rules; arXiv 1608.03960 PDF retrieved 2026-08-29).
- Automerge extension: Da and Kleppmann, "Extending JSON CRDTs with Move Operations", PaPoC 2024 (arXiv 2311.14007; Cambridge repository PDF retrieved 2026-08-29): "Currently, Automerge handles moves by deletion and reinsertion" (§1); operations are applied in ascending operation-ID order with a `tree` map (child to parent, deletion as parent `null`) and a `winners` map (greatest move ID per element) (§3.1, Algorithm 1); optimisations are batch updating and lifecycle tracking (§3.3); a Go prototype exists and integration into the Rust implementation is planned (§4). Kleppmann's 2024-01-04 review states the algorithm "is not yet fully implemented within Automerge" (https://martin.kleppmann.com/2024/01/04/year-in-review.html, retrieved 2026-08-29).
- List move: naive delete-and-reinsert duplicates an element under concurrent moves; the fix treats the element's position as a register over list-CRDT positions. Kleppmann, "Moving Elements in List CRDTs", PaPoC 2020, DOI 10.1145/3380787.3393677, §2 (PDF retrieved 2026-08-29).
- Fractional indexing: "An object's position in its parent's array of children is represented as a fraction between 0 and 1 exclusive"; parent link and position are stored as a single property so they update atomically; the server rejects parent updates that would cause a cycle. Figma engineering blog (https://www.figma.com/blog/how-figmas-multiplayer-technology-works/, retrieved 2026-08-29).

## Mechanism

Types (Fig. 4):

```
Move    t p m c            -- timestamp, new parent, metadata, child
LogMove t oldp p m c       -- oldp : (parent × meta) option, recorded at apply time
state = LogMove list × (parent × meta × child) set
```

Core functions (Fig. 4, lines 22-49, transcribed):

```
ancestor tree a c  ⇔  (a, _, c) ∈ tree  ∨  ∃p. (p, _, c) ∈ tree ∧ ancestor tree a p

do_op (Move t newp m c, tree) =
  (LogMove t (get_parent tree c) newp m c,
   if ancestor tree c newp ∨ c = newp then tree
   else {(p', m', c') ∈ tree | c' ≠ c} ∪ {(newp, m, c)})

undo_op (LogMove t None        newp m c, tree) = {(p', m', c') ∈ tree | c' ≠ c}
undo_op (LogMove t (Some(op,om)) newp m c, tree) = {(p', m', c') ∈ tree | c' ≠ c} ∪ {(op, om, c)}

redo_op (LogMove t p m c) (ops, tree) =
  let (op2, tree2) = do_op (Move t p m c, tree) in (op2 # ops, tree2)

apply_op op1 ([], tree1)          = let (op2, tree2) = do_op (op1, tree1) in ([op2], tree2)
apply_op op1 (logop # ops, tree1) =
  if move_time op1 < log_time logop
  then redo_op logop (apply_op op1 (ops, undo_op (logop, tree1)))
  else let (op2, tree2) = do_op (op1, tree1) in (op2 # logop # ops, tree2)

apply_ops ops = foldl (λs o. apply_op o s) ([], {}) ops
```

Invariants proved for every reachable state: every child has at most one `(parent, meta)` pair (`unique_parent`); no node is its own ancestor (`acyclic`); `apply_ops` is invariant under permutation of an operation set with distinct timestamps (`apply_ops_commutes`). Timestamps must form a total order with unique values (Lamport timestamps suffice). Preconditions are not user-visible: an unsafe move is silently ignored rather than rejected, and its log entry preserves the information needed to re-evaluate it.

Sibling order is external to the proof: metadata `m` carries a list-CRDT identifier (RGA or Logoot), so reordering is `Move t sameparent newid c`. Fractional indexing (Figma) is the other established choice; it requires an authority or a tie-break rule for equal fractions and periodic renormalisation, which the Figma post does not detail.

Cost model: local operations are `O(d)` for the ancestor check; remote operations are `O(nd)` where `n` grows with the number of in-flight concurrent operations; the log can be truncated at the causally stable timestamp.

## NUIF relevance

**Borrow**
- The single-operation model (create as first move, delete as move to trash) because it yields a small proof surface and identical conflict handling for all structural edits.
- The `LogMove` shape (operation plus recorded prior parent and metadata) as the canonical way to record inverse information in a replay log, matching spec/06's "inverse semantic operations or transaction history".
- The acyclicity precondition expressed as an ancestor check evaluated against the state produced by all lower-ordered operations, and the rule that ignored operations stay in the log.
- Causal stability as the criterion for truncating profile-level logs and discarding tombstoned subtrees.

**Adapt**
- NUIF's `Operation::Move { entity, new_parent, new_index }` (crates/nuif-protocol/src/lib.rs) uses an integer index, which is not commutative under concurrent sibling edits; the metadata field should carry a stable order key (list-CRDT ID or fractional index) so that reordering becomes a move with unchanged parent.
- Silent ignoring of cycle-inducing moves is correct for convergence but must be surfaced as a typed conflict object in the collaboration profile (spec/10) rather than lost, because NUIF requires semantic conflicts to remain explicit.
- The timestamp total order belongs to the collaboration profile; canonical NUIF documents must not carry Lamport timestamps or trash subtrees, so checkpoint materialisation must strip them (spec/10 "materialize a canonical NUIF snapshot without collaboration metadata").

**Reject**
- Making undo-do-redo the semantics of the canonical patch format: a NUIF patch is applied against a declared base revision in order, and operations with failed preconditions are reported, not reordered by timestamp.
- Unbounded operation logs in the document: log retention is a profile concern.

## Open questions

- Which order key should the collaboration profile mandate for sibling order: RGA-style identifiers (require tombstones) or fractional indices (require renormalisation and an equal-key tie-break)?
- How should a cycle-rejected move be represented to a user: as a conflict object with both intended parents, or as an automatic loss with an audit entry?
- Da and Kleppmann's lifecycle tracking changes validity as later operations arrive; whether an equivalent incremental validity check can be defined for NUIF patches with preconditions is untested.
- The paper leaves the interaction between move and concurrent property edits inside a moved subtree to the enclosing data model; NUIF property operations need their own commutation argument.
