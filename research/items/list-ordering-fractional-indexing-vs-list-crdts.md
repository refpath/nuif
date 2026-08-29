---
id: nuif:research:list-ordering-fractional-indexing-vs-list-crdts
kind: synthesis
status: reviewed
title: Sibling order representation for authored documents; fractional indexing, list CRDT positions and anchor-based operations compared
source:
  url: https://arxiv.org/abs/2305.00583
  doi: 10.1145/3301419.3323972
  repository: https://github.com/rocicorp/fractional-indexing
  authors: [Matthew Weidner, Martin Kleppmann, Victor B. F. Gomes, Dominic P. Mulligan, Alastair R. Beresford, Hyun-Gul Roh, Petru Nicolaescu, Kevin Jahns, Evan Wallace, David Greenspan]
  published_at: "2009-06 (Logoot, ICDCS), 2011-03 (RGA, JPDC 71(3)), 2013-09 (LSEQ, DocEng), 2016-11 (YATA, GROUP), 2017-03-06 (Figma ordered sequences), 2019-03-25 (interleaving anomalies, PaPoC), 2023-04-30/2025-10-21 (Fugue, arXiv v1/v3)"
  license: "ACM and IEEE copyright for papers (author-hosted PDFs); arXiv non-exclusive licence for Fugue; CC0-1.0 for rocicorp/fractional-indexing 4.0.0; MIT for yjs and mweidner037/list-positions; MPL-2.0 for Penpot; proprietary blog content for Figma"
retrieved_at: 2026-08-29
tags: [ordering, fractional-indexing, list-crdt, rga, yata, fugue, logoot, lseq, interleaving, commutativity, canonical-form, replay, merge-conflict, penpot, figma, automerge, yjs, openusd]
confidence: 0.9
claims: [nuif:claim:collab-profile, nuif:claim:sync-not-regenerate]
relations:
  - type: extends
    target: nuif:research:crdt-tree-move-operation
    note: Resolves the open question on which order key the metadata field of a tree move should carry.
  - type: related_to
    target: nuif:research:figma-multiplayer-and-rendering-engineering
    note: Figma's base-95 fractional position is one of the three candidates compared here.
  - type: related_to
    target: nuif:research:automerge-yjs
    note: Automerge (RGA-derived) and Yjs (YATA) are the list-CRDT candidates for the collaboration profile.
  - type: related_to
    target: nuif:research:operational-transformation-vs-crdt
    note: Index-based operations require transformation; anchor-based operations commute without it.
  - type: related_to
    target: nuif:research:penpot
    note: Penpot stores child order as an id vector and moves by integer index or after-shape anchor.
  - type: related_to
    target: nuif:research:openusd-composition-and-crate
    note: USD composes child order weakest-to-strongest with sparse reorder statements per layer.
links:
  spec: [spec/01-model.md, spec/06-operations-and-patches.md, spec/08-serialization.md, spec/10-collaboration-profile.md]
  adr: [adrs/0005-collaboration-profile.md]
  rfc: []
  code: [crates/nuif-core, crates/nuif-protocol]
  experiments: []
---

# Summary

Three families represent the order of siblings under a parent. Fractional indexing assigns each child a key from a dense ordered set (a fraction in (0, 1) written as a string over a base-95 or base-62 alphabet); insertion between two neighbours picks a key strictly between their keys, so a reorder is one register write. List CRDTs (RGA, Logoot, LSEQ, YATA, Fugue) assign each element an identifier that is either a dense path (Logoot, LSEQ) or a reference to an existing element plus a Lamport-style identifier (RGA, YATA, Fugue); the order is recovered by a deterministic traversal that requires tombstones for deleted elements. Anchor-based operations ("insert after element x") are the operation form used by RGA, Automerge, Yjs and Fugue and, in Penpot, as an optional `after-shape` alternative to an integer index.

The interleaving anomaly (Kleppmann, Gomes, Mulligan and Beresford, PaPoC 2019) affects every dense-identifier scheme, fractional indexing included: two concurrent runs inserted at one gap can be shuffled element by element. RGA and YATA are proved forward non-interleaving; Weidner and Kleppmann's FugueMax is proved maximally non-interleaving (forward and, where achievable, backward). Figma and the `fractional-indexing` library accept interleaving and unbounded key growth; neither source describes rebalancing. USD does not store keys at all: each layer holds an ordered child list plus a sparse `reorder nameChildren` statement, and composition appends names weakest-to-strongest and applies each layer's reorder in turn.

Source statements are reported in `## Evidence`; the NUIF interpretation follows in `## NUIF relevance`.

## Evidence

Fractional indexing.

- Figma stores position as "a fraction between 0 and 1 exclusive"; "Each index is stored as a string"; base 95 over printable ASCII with the leading "0." omitted; insertion "set[s] the index for the new object to the average index of the two objects on either side"; "Averaging between two identical indices doesn't work", resolved by the server "generating and assigning a unique position to the second insert operation"; "Index length can grow over time", accepted because "the number of reordering operations is bounded by user activity"; "Merging new elements from multiple clients may interleave them", accepted because "the new objects likely don't overlap". No rebalancing is described. E. Wallace, "Realtime Editing of Ordered Sequences", https://www.figma.com/blog/realtime-editing-of-ordered-sequences/, retrieved 2026-08-29.
- Parent link and position are one property "so they update atomically"; the server "reject[s] parent property updates that would cause a cycle". E. Wallace, "How Figma's multiplayer technology works", 2019-10-16, retrieved 2026-08-29 (see nuif:research:figma-multiplayer-and-rendering-engineering).
- Wallace's later algorithm note: objects are sorted "by their positions (using object id as a tie-breaker)"; "add a random offset to the end of the fraction during each insert operation" to avoid identical keys with high probability; "If two peers both simultaneously insert a run of objects at the same location, the resulting objects may be interleaved. So this algorithm is not appropriate in situations where object adjacency is critical"; "Index length can become long in pathological scenarios"; "Floating-point numbers are insufficient". https://madebyevan.com/algos/crdt-fractional-indexing/, retrieved 2026-08-29.
- `rocicorp/fractional-indexing` 4.0.0 (CC0-1.0): `generateKeyBetween(a, b)` and `generateNKeysBetween(a, b, n)`; default digits `0-9A-Za-z` with integer-part heads `A-Z`/`a-z`; keys "sort correctly using ordinary lexicographic comparison because the digits do"; `localeCompare` "will give an incorrect ordering"; repeated insertion at one gap lengthens keys (`a0`, `a1`, `a2`, `Zz`, `a1V` in the README example); jitter is not in the core library and is delegated to `nathanhleung/jittered-fractional-indexing`. `README.md` lines 4, 29, 61, 91, 114, 135, 172–178; `package.json` lines 3, 34; retrieved 2026-08-29.
- Jittered variant: after computing the unjittered midpoint, "with 50% probability each, we either generate a key between the original lower bound `a` and the `midpoint`, or a key between the `midpoint` and the original upper bound `b`", repeated `jitterBits` times; the README gives a birthday-bound example of "~4.5% chance of collision" at 30 bits and 10 000 concurrent keys. https://github.com/nathanhleung/jittered-fractional-indexing README, retrieved 2026-08-29.

List CRDTs.

- RGA: Roh, Jeon, Kim, Lee, "Replicated abstract data types: Building blocks for collaborative applications", J. Parallel Distrib. Comput. 71(3), 354–368, 2011, DOI 10.1016/j.jpdc.2010.12.006 (dblp record retrieved 2026-08-29). In Attiya et al.'s formulation an insertion is a triple (a, t, r) with r the timestamp of the reference (predecessor) character; deletions retain tombstones; multiple insertions anchored to one character "are sorted in descending timestamp order". Kleppmann et al., PaPoC 2019, §3 (PDF p. 3).
- Logoot: Weiss, Urso, Molli, ICDCS 2009, pp. 404–412, DOI 10.1109/ICDCS.2009.75. LSEQ: Nédelec, Molli, Mostéfaoui, Desmontils, DocEng 2013, pp. 37–46, DOI 10.1145/2494266.2494278 (dblp records retrieved 2026-08-29). Both assign "a unique position identifier from a dense ordered set"; identifiers "are paths through a tree". PaPoC 2019, §2.
- YATA: Nicolaescu, Jahns, Derntl, Klamma, "Near Real-Time Peer-to-Peer Shared Editing on Extensible Data Types", GROUP 2016, DOI 10.1145/2957276.2957310 (Semantic Scholar record retrieved 2026-08-29). Yjs: "Everything inserted in a Yjs document is given a unique ID, formed from a ID(clientID, clock) pair"; an item "stores a reference to the IDs of the preceding and succeeding item" in `origin` and `originRight`; a deleted item "is flagged as deleted"; "No data is kept on when an item was deleted". `INTERNALS.md` lines 7–13, 45–50, 62–69, 104–108, yjs main, retrieved 2026-08-29. Conflict resolution in `Item#integrate`: items with equal `origin` are ordered by `o.id.client < this.id.client` (case 1) and by transitive origin membership (case 2). `src/structs/Item.js` lines 168–245, retrieved 2026-08-29.
- Automerge: "When you insert elements into a list the insert operation references the ID of the element you are inserting after"; concurrent inserts after one element are resolved by "arbitrarily choose one to insert first and then insert the other immediately afterwards"; no list move operation is documented. https://automerge.org/docs/reference/under-the-hood/merge-rules/, retrieved 2026-08-29. The Rust crate exposes `Cursor { Start, End, Op(OpCursor) }`, "An identifier of a position in a Sequence"; automerge 0.11.0, https://docs.rs/automerge/latest/automerge/enum.Cursor.html, retrieved 2026-08-29.
- Fugue: Weidner and Kleppmann, "The Art of the Fugue: Minimizing Interleaving in Collaborative Text Editing", arXiv 2305.00583 (v1 2023-04-30, v2 2023-11-17, v3 2025-10-21). Algorithm 1 (PDF p. 6): `ID := (RID × N) ∪ {null}`; a node is `(id, value, parent, side)`; `insert(i, x)` takes `leftOrigin` (the (i−1)-th value) and `rightOrigin` ("next node after leftOrigin in the tree traversal that includes tombstones"); the node becomes a right child of `leftOrigin` if it has no right children, otherwise a left child of `rightOrigin`; on delivery siblings are ordered by `node.id <`; `delete` sets `value ← ⊥`. "We cannot remove a deleted element's node entirely: it may be an ancestor to non-deleted nodes" (p. 6). Definition 2 (forward non-interleaving) and Definition 4 (maximal non-interleaving, conditions (1)–(3)), §5.2 (p. 8); Definition 6: FugueMax "visits right-side siblings in the reverse order of their right origins, breaking ties using the lexicographic order of their IDs", §5.3 (p. 9); Theorem 9: FugueMax is maximally non-interleaving, §5.4. Table 1 (p. 3): Logoot, LSEQ and Treedoc interleave forward and backward; RGA is proved forward non-interleaving and interleaves backward; Yjs is proved forward non-interleaving, backward one-replica unproven, backward multi-replica interleaves; Fugue and FugueMax are proved non-interleaving in all three columns. Evaluation (§6, Tables 2–3, p. 11–12): on a 260 k-operation trace Fugue used 2.4 MB, 46 network bytes per operation and 94 k operations/s; Yjs 13.6.8 used 3.3 MB, 29 bytes/op, 39 k ops/s; Automerge-Wasm 0.5.0 used 126 bytes/op and 52 k ops/s.
- Interleaving anomaly: Kleppmann, Gomes, Mulligan, Beresford, "Interleaving anomalies in collaborative text editors", PaPoC 2019, DOI 10.1145/3301419.3323972 (author PDF https://martin.kleppmann.com/papers/interleaving-papoc19.pdf, retrieved 2026-08-29). §2: Logoot and LSEQ "suffer from this problem"; Figure 3 shows the anomaly with rational-number identifiers, the model that fractional indexing instantiates. §2.1 adds clause 1(d) to the strong list specification: for concurrent insertion sets X and Y at one location, "either all X insertions appear before all Y insertions ... or vice versa, but they are never interleaved". §3: RGA "does not suffer from" the character-level anomaly under sequential insertion but exhibits a "lesser" anomaly for non-sequential insertion; §3.1 proposes a 4-tuple (a, t, r, e) with e the set of timestamps of prior insertions at the same reference, and a session-grouping order, stated as a conjecture. The Fugue paper (§3.2) reports that this 2019 definition "cannot be satisfied by any algorithm" and that the proposed fix is flawed.
- `mweidner037/list-positions` (MIT) implements Fugue for application lists: `Position = { bunchID: string; innerIndex: number }`, each bunch carries `BunchMeta` describing "its location in the tree"; "if two users concurrently insert a (forward or backward) sequence at the same place, their sequences will not be interleaved"; `lexicographicString(pos)` yields strings whose lexicographic order matches the list order; fractional indexing is described as "a related but less general idea". README, retrieved 2026-08-29.
- Tree move with list order: the move-operation paper delegates sibling order to "an additional list CRDT for each branch node, e.g. using RGA [14] or Logoot [15]", carried in the metadata field, so a reorder is a move with unchanged parent (nuif:research:crdt-tree-move-operation, PDF §3.7). Kleppmann, "Moving Elements in List CRDTs", PaPoC 2020, DOI 10.1145/3380787.3393677, shows delete-and-reinsert duplicating an element under concurrent moves.

Design and scene documents.

- Penpot: a parent's children are an ordered vector of shape ids (`(:shapes shape)`), `common/src/app/common/types/container.cljc` `get-direct-children`; the `:mov-objects` change schema carries `parent-id`, `shapes`, `index` (optional int) and `after-shape` (optional); at application `index` is computed as `(or (some-> (d/index-of (:shapes parent) after-shape) inc) index)` and the shapes are inserted with `d/insert-at-index` or appended; a move is rejected when the target is a descendant of the moved shape, and skipped when the parent no longer exists ("race condition when an inflight move operations lands when parent is deleted"). `common/src/app/common/files/changes.cljc` lines 229–239, 735–770, 815–823, develop branch, retrieved 2026-08-29. `:add-obj` likewise carries an optional integer `index` (lines 189–198, 595–600).
- OpenUSD: `SdfPrimSpec::SetNameChildrenOrder(names)` "Given a list of (possibly sparse) child names, authors a reorder nameChildren statement for this prim. The reorder statement can modify the order of name children during composition. This order doesn't affect GetNameChildren(), InsertNameChild(), SetNameChildren(), et al."; `ApplyNameChildrenOrder` "employs the standard list editing operation for ordered items in a ListEditor"; `InsertNameChild(child, index)` documents that `index` "is ignored except for range checking". `pxr/usd/sdf/primSpec.h` lines 160–210, release branch, retrieved 2026-08-29. `SdfListOpType` has `Explicit, Added, Deleted, Ordered, Prepended, Appended`, and `ApplyOperations` is well defined only when neither operand "use[s] the 'ordered' or 'added' item lists" (`pxr/usd/sdf/listOp.h` lines 30–36, 205–217). Composition: `PcpComposeSiteChildNames` iterates layers with `TF_REVERSE_FOR_ALL` (weakest first), appends names not yet seen, then applies the layer's `PrimOrder` field with `SdfApplyListOrdering` (`pxr/usd/pcp/composeSite.cpp` lines 487–540); `_ComposePrimChildNames` performs a "Reverse strength-order traversal (weak-to-strong)" over prim-index nodes (`pxr/usd/pcp/primIndex.cpp` lines 5648, 5669–5671, 5783–5791). The glossary states list-edited elements "always resolve to a set" with no repetition (https://openusd.org/release/glossary.html, "List Editing", retrieved 2026-08-29).

## Mechanism

Fractional keys. Order is the lexicographic order of keys; a key is a variable-length numeral in a fixed alphabet. Insertion between keys `a < b` computes a key `k` with `a < k < b`; when `a` and `b` share a long common prefix the new key is at least one digit longer, so `n` repeated insertions at the same gap produce keys of length Θ(n) digits. Two concurrent insertions at one gap can produce equal keys, so a tie-break (object id, server rewrite, or random jitter) is required, and two concurrent runs at one gap sort by key value, not by run, which is the interleaving anomaly of Figure 3 in the PaPoC 2019 paper. A key is a per-child register: reorder and reparent are one register write, moves commute with unrelated writes, and two concurrent moves of one child resolve by last-writer-wins on the register with no conflict signalled. Keys are replica-generated; the same visible order admits infinitely many key assignments, so a document's hash depends on editing history unless keys are renormalised, and no retrieved source specifies renormalisation.

Anchor-based positions with tombstones (RGA, YATA, Fugue). An insertion names an existing element (left origin; YATA and Fugue also a right origin) and carries a unique identifier (replica, counter). The visible order is a depth-first traversal of the origin tree with a fixed sibling rule: descending identifier (RGA), client identifier (YATA case 1), identifier order among siblings (Fugue), or reverse right-origin order then identifier (FugueMax). Deleted elements stay as tombstones because later insertions may name them; Fugue states this explicitly. Insertions commute under causal delivery; order is a pure function of the operation set. Forward non-interleaving holds for RGA, YATA and Fugue; maximal non-interleaving for FugueMax. Metadata per element is one identifier plus one or two origin references; save size in Fugue's benchmark was 60 % of the literal text.

Anchor-based operations over a plain array. The operation carries `after: Option<EntityId>` (RGA's reference element) and the canonical document stores the resolved array. Two operations with different anchors commute; two insertions with the same anchor need a tie-break rule to be replay-deterministic; a move whose anchor was concurrently deleted needs a fallback. The collaboration profile supplies the tie-break and the fallback through a list CRDT whose tombstones are profile data; a sequential patch supplies them through preconditions and typed conflicts. Penpot's `after-shape` and Automerge's `Cursor::Op` are instances of this form.

USD reorder statements. Each layer stores the ordered child list it authored plus a sparse ordering list; composition appends new names in weakest-to-strongest order and applies each stronger layer's `reorder` to the running list. Order is derived, deterministic and free of replica identifiers, and the ordered-items list op is not closed under composition, which is why USD excludes it from `ApplyOperations`.

Comparison against the evaluation criteria (sources as above).

| Criterion | Integer index (current NUIF, Penpot) | Fractional key (Figma, rocicorp) | List-CRDT identifier (RGA, YATA, Fugue) | Anchor operation over ordered array (proposed) |
|-----------|--------------------------------------|----------------------------------|------------------------------------------|-----------------------------------------------|
| Commutativity of independent operations | No; indices shift | Yes (register per child) | Yes under causal delivery | Yes when anchors differ; same-anchor case needs a tie-break |
| Canonical form without replica metadata | Yes | No; keys are history-dependent, jitter is random | No; tombstones and identifiers required | Yes; array only |
| Human-readable text form | Position implicit in listing | Opaque strings (`a1V`) | Opaque identifier pairs | Position implicit in listing; operation names a neighbour |
| Interleaving of concurrent runs | Not applicable (sequential) | Interleaves (PaPoC 2019 Fig. 3; Wallace) | RGA/YATA forward-safe; FugueMax maximal | Delegated to profile CRDT; sequential patches never interleave |
| Growth bound | None | Key length Θ(n) at one gap; no rebalancing specified | One identifier per element; tombstones retained | None in canonical form |
| Replay determinism | Order-sensitive | Deterministic given tie-break | Deterministic | Deterministic given anchor resolution and same-anchor tie-break |
| Detection of two moves to one slot | Index collision, ambiguous | Register last-writer-wins, silent | Register over positions (Kleppmann 2020), silent | Same-anchor moves are detectable and reportable as a typed conflict |

## NUIF relevance

**Borrow**
- The RGA/Automerge operation form: an insertion or move names the sibling it follows, not an integer position; `Option<EntityId>` with `None` meaning "first child" is the RGA `head` value.
- The USD principle that the canonical document stores a resolved ordered list and no order keys, so `nuif-text-0` lists children in order and the hash covers the array, not replica-generated strings.
- Fugue's tree formulation (left origin, right origin, identifier tie-break) as the list CRDT the collaboration profile should specify, since it is the only candidate with proved forward and backward non-interleaving and a published, benchmarked implementation.
- The move-operation paper's rule that a reorder is a move with unchanged parent and new order metadata, so one operation type covers reparent and reorder.
- Penpot's precondition that a move whose parent no longer exists is dropped with a diagnostic rather than applied to a stale index.

**Adapt**
- Same-anchor concurrency: in a sequential patch the two insertions are applied in patch order (no ambiguity); in a three-way merge the merge tool orders same-anchor insertions from different branches by branch precedence declared in the merge input and reports `OrderAmbiguous` as an informational diagnostic; in the collaboration profile the CRDT's sibling rule applies. NUIF must state all three rules; no retrieved source covers the three-way case.
- Anchor deleted concurrently: the collaboration profile resolves through tombstones (RGA semantics); a sequential patch or three-way merge whose `after` anchor is absent from the target parent fails the operation with a typed conflict `AnchorMissing` that carries the intended anchor and the last known neighbours, matching spec/06's requirement to surface typed conflicts rather than pick winners.
- Two concurrent moves of one entity are a semantic conflict in NUIF (`MoveConflict { entity, targets: [(parent, anchor); 2] }`), not a last-writer-wins register write as in Figma; the profile may converge structurally on one winner but must retain the conflict object (spec/10).
- Fractional keys may still appear as a profile-internal or transport optimisation (Figma's single-register reparent-and-reorder) provided checkpoint materialisation strips them; they must not appear in `nuif-core` types.

**Reject**
- Integer `index` in `Insert` and `Move` (current `crates/nuif-protocol/src/lib.rs`): non-commutative and replay order-sensitive; Penpot's `after-shape` shows the migration path.
- Fractional keys in the canonical form: history-dependent hashes, unbounded key growth without a specified rebalance, interleaving, and jitter that introduces randomness into a document that must be deterministic (spec/08).
- List-CRDT identifiers in the canonical form: require tombstones and replica identifiers that spec/10 assigns to profile data.
- Logoot and LSEQ for the profile: interleave in every column of Fugue's Table 1.

## Open questions

- Whether the collaboration profile should mandate FugueMax or accept any algorithm satisfying Definition 2 (forward non-interleaving); Yjs and Automerge satisfy only the weaker property, and requiring FugueMax excludes both without an adapter layer.
- Branch precedence for same-anchor insertions in three-way merge: by branch identity, by operation identifier, or by entity identifier; each is deterministic but none has a source in the retrieved literature.
- Whether `AnchorMissing` should fall back to the nearest surviving predecessor when the patch carries an ordered anchor chain (Fugue's right origin suggests a two-anchor form `between: (Option<EntityId>, Option<EntityId>)`); the cost is a larger operation and a second failure mode.
- The Fugue paper's claim that the PaPoC 2019 non-interleaving definition is unsatisfiable was not independently verified here; only the Fugue paper's statement (§3.2) was retrieved.
