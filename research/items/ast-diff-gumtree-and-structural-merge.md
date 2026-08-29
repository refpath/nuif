---
id: nuif:research:ast-diff-gumtree-and-structural-merge
kind: paper
status: reviewed
title: Tree differencing with moves (Chawathe et al. 1996, GumTree 2014) and structural merge/diff tools (Mergiraf, difftastic)
source:
  url: https://doi.org/10.1145/2642937.2642982
  doi: 10.1145/2642937.2642982
  repository: https://github.com/GumTreeDiff/gumtree
  authors: [Jean-Rémy Falleri, Floréal Morandat, Xavier Blanc, Matias Martinez, Martin Monperrus, Sudarshan S. Chawathe, Anand Rajaraman, Hector Garcia-Molina, Jennifer Widom, Antonin Delpeuch, Wilfred Hughes]
  published_at: "2014-09 (ASE 2014, pp. 313-324); Chawathe et al. 1996-06 (SIGMOD 1996, pp. 493-504)"
  license: ACM copyright with author preprints; GumTree LGPL-3.0; Mergiraf GPL-3.0; difftastic MIT
retrieved_at: 2026-08-29
tags: [tree-differencing, edit-script, ast, gumtree, move-detection, structural-merge, mergiraf, difftastic, tree-sitter, matching]
confidence: 0.9
claims: [nuif:claim:sync-not-regenerate, nuif:claim:semantic-automation]
relations:
  - type: extends
    target: nuif:research:structured-merge
    note: Gives the concrete matching algorithms and edit-script generators behind AST-aware merge, and current tool behaviour.
  - type: related_to
    target: nuif:research:tree-sitter
    note: Mergiraf and difftastic both obtain trees from tree-sitter grammars.
  - type: related_to
    target: nuif:research:patch-theory-darcs-pijul
    note: Mergiraf keeps conflicts as nodes in the merged tree, the tree analogue of Pijul's conflict states.
  - type: related_to
    target: nuif:research:crdt-tree-move-operation
    note: Move detection is the inferred substitute for the explicit move operation that identity-bearing models provide.
links:
  spec: [spec/06-operations-and-patches.md]
  adr: []
  rfc: []
  code: [crates/nuif-protocol]
  experiments: []
---

# Summary

Chawathe, Rajaraman, Garcia-Molina and Widom (SIGMOD 1996) defined the tree change-detection problem as finding a minimum-cost edit script over insert, delete, update and subtree move, split it into finding a matching and then generating a conforming script, and gave an `O(ne + e²)` algorithm under domain assumptions. GumTree (ASE 2014) keeps the Chawathe script generator but replaces matching with a two-phase heuristic: a greedy top-down search for the largest isomorphic subtrees, then a bottom-up phase that matches containers by Dice similarity of already matched descendants and recovers further matches with an optimal tree-edit-distance algorithm on small subtrees; worst case `O(n²)`. Mergiraf (2024-) applies GumTree classic matching to base/left/right trees, converts them to parent-child-successor triples, merges the triple sets, and emits conflict nodes or falls back to diff3 for the affected element; it treats designated "commutative parents" specially and identifies their children by signatures. Difftastic computes a diff as a lowest-cost path with Dijkstra's algorithm over pairs of tree positions and does not detect moves. Across these systems the expensive and heuristic step is matching; every subsequent step (edit script, three-way merge) is defined relative to a matching. When entities carry stable identifiers, the matching is the identity map and the residual problems are ordering and move conflicts.

## Evidence

- Chawathe et al.: DOI 10.1145/233269.233366, SIGMOD 1996 pp. 493-504 (SIGMOD Record 25(2), PDF retrieved 2026-08-29 from https://sigmodrecord.org/1996/06/24/change-detection-in-hierarchically-structured-information/). Edit operations `INS`, `DEL` (leaf only; interior deletion requires moving descendants first), `UPD`, `MOV` of a subtree (§3.1); an edit script conforms to a partial matching if it does not insert or delete matched nodes (§3.2); cost model with a `compare` function in `[0, 2]` for updates (§3.3); five phases update, align, insert, move, delete in one breadth-first scan of the new tree plus a post-order delete pass (§4.1, §4.2); LCS-based child alignment yields the minimum number of moves (§4.2); Matching Criterion 1 (leaves: equal labels and `compare ≤ f`, `0 ≤ f < 1`), Criterion 2 (internal nodes: fraction of common leaves `> t`, `1/2 ≤ t < 1`), Assumption 1 (acyclic labels), Assumption 2 (at most one close leaf), Theorem 5.1 (unique maximal matching is the best matching), algorithm FastMatch (§5); running time `O(ne + e²)` with `n` leaves and `e` the weighted edit distance (§1).
- GumTree problem statement: actions `update`, `add`, `delete`, `move(t, tp, i)` moving a subtree; the shortest script with moves is NP-hard; the best add/delete/update algorithm (RTED) is `O(n³)`. ASE 2014 PDF §2 (retrieved 2026-08-29 from https://www.labri.fr/perso/xblanc/data/papers/ASE14.pdf).
- GumTree top-down phase (Algorithm 1): height-indexed priority lists, processing nodes of equal greatest height, isomorphism by hash then exact test, ambiguous candidates ranked by `dice(parent(t1), parent(t2), M)`, only nodes with height greater than `minHeight`. §3.1. Dice: `dice(t1, t2, M) = 2·|{t1' ∈ s(t1) | (t1', t2') ∈ M}| / (|s(t1)| + |s(t2)|)`.
- GumTree bottom-up phase (Algorithm 2): a candidate `c` for unmatched internal `t1` requires equal labels, `c` unmatched, and matched descendants; the candidate with greatest Dice is matched if `dice > minDice`; when the remaining subtrees are both smaller than `maxSize` an optimal algorithm `opt` (RTED) recovers descendant mappings for same-label nodes. §3.2.
- GumTree script generation: "RTED does not handle moves," so the script is produced with Chawathe et al.'s algorithm from the mappings. §2 and §3.3 (Complexity Analysis).
- GumTree complexity: worst case `O(n²)`, `n = max(|T1|, |T2|)`, from the Cartesian products in both phases. §3.3. Replication settings: `minHeight = 2`, `minDice = 0.5`, `maxSize = 100`. §5.2.3.
- GumTree implementation (main branch, retrieved 2026-08-29): `core/src/main/java/com/github/gumtreediff/matchers/heuristic/gt/AbstractSubtreeMatcher.java` uses `PriorityTreeQueue`, `HashBasedMapper`, `DEFAULT_MIN_PRIORITY = 1`, options `st_minprio`, `st_priocalc` (default `height`); `GreedySubtreeMatcher.java` resolves ambiguous mappings by sorting on maximum subtree size then `FullMappingComparator`; `GreedyBottomUpMatcher.java` uses `DEFAULT_SIM_THRESHOLD = 0.5`, `DEFAULT_SIZE_THRESHOLD = 1000`, options `bu_minsim`, `bu_minsize`, and `ZsMatcher` for last-chance matching; `actions/ChawatheScriptGenerator.java` walks the destination tree breadth-first emitting `Insert`, `Update`, `Move`, aligns children with LCS, computes positions with `findPos`, and emits `Delete` in post-order. The code defaults (min priority 1, size threshold 1000) differ from the paper's (`minHeight` 2, `maxSize` 100).
- GumTree README cites hyperparameter optimisation (Martinez et al., IEEE TSE 2023, DOI 10.1109/TSE.2023.3315935) and a scalable variant (Falleri and Martinez, ICSE 2024, DOI 10.1145/3597503.3639148). https://github.com/GumTreeDiff/gumtree (retrieved 2026-08-29).
- Mergiraf architecture: tree-sitter parsing with multi-line leaves split into lines; "the GumTree classic algorithm" in top-down and bottom-up phases applied to base-left, base-right and left-right; class mapping with leader preference base, left, right; conversion to parent-child-successor triples `(p, c, s)` with sentinels; quadruplets tagged by revision merged into a possibly inconsistent set with base triples removed when contradicted; commutative parents merged by applying right's deletions and appending right's additions; signature-based duplicate detection; delete/modify conflicts decided by a covering check that distinguishes moves; fallback to diff3 on the parent element's source; output node kinds `ExactTree`, `Conflict`, `LineBasedMerge`, `MixedTree`, `CommutativeChildSeparator`; fast mode cannot resolve "moving edited elements". https://mergiraf.org/architecture.html (retrieved 2026-08-29).
- Mergiraf conflict classes: commutative insertions (e.g. class members) are auto-resolved; order-dependent insertions (statements in a block, function arguments) are not; duplicate signatures under commutative parents are flagged; moved-and-edited code is replayed at the new location. https://mergiraf.org/conflicts.html (retrieved 2026-08-29). Repository: Rust, GPL-3.0, 24 releases. https://codeberg.org/mergiraf/mergiraf (retrieved 2026-08-29).
- Difftastic: a diff is "a route finding problem on a directed acyclic graph"; a vertex is a pair of positions in the two trees; edges marking a node novel cost more than matching; Dijkstra's algorithm finds the lowest-cost route with vertices constructed lazily. https://difftastic.wilfred.me.uk/diffing.html (retrieved 2026-08-29). Tricky cases: no move detection; sliders; preference for matches at the same nesting depth. https://difftastic.wilfred.me.uk/tricky_cases.html (retrieved 2026-08-29). Repository: Rust, MIT, tree-sitter parsers, no merge or patch output. https://github.com/Wilfred/difftastic (retrieved 2026-08-29).

## Mechanism

Problem decomposition (Chawathe et al. §1, §3; adopted by GumTree §2):

```
input:  T1 (old), T2 (new)
step 1: matching  M ⊆ nodes(T1) × nodes(T2), partial injective, label-preserving
step 2: edit script E conforming to M, minimising Σ cost(op)
        ops: INS(x, parent, k, l, v) | DEL(x) | UPD(x, v) | MOV(x, parent, k)
```

Script generation from a matching (Chawathe §4.1; GumTree `ChawatheScriptGenerator`):

```
for y in BFS(T2):
  if y unmatched:            x := INS(new, partner(parent(y)), findPos(y)); M += (x, y)
  else x := partner(y):
     if value(x) ≠ value(y): UPD(x, value(y))
     if partner(parent(y)) ≠ parent(x): MOV(x, partner(parent(y)), findPos(y))
  alignChildren(x, y): keep LCS of matched children fixed, MOV the rest
for x in postorder(T1): if x unmatched: DEL(x)
```

GumTree matching:

```
top-down:  process nodes by decreasing height; isomorphic subtrees (hash, then exact) are mapped wholesale;
           ambiguous candidates ranked by Dice of their parents; stop below minHeight
bottom-up: for unmatched internal t1 in post-order: candidates c with label(c)=label(t1), c unmatched,
           matched descendants; match argmax dice if dice > minDice;
           if |t1|,|t2| < maxSize: run optimal edit distance (RTED / Zhang-Shasha) on the residue
```

Three-way structured merge (Mergiraf, PCS form):

```
PCS(T) = {(p, c, s) | s is the immediate successor of c under p}  ∪ sentinels
merge  = PCS(base)^tagged ∪ PCS(left) ∪ PCS(right)   minus base triples contradicted by a side
inconsistency (two successors for one (p, c), two parents for one (c, s)) → Conflict node or diff3 fallback
commutative parent: children set = left ∪ (right additions) minus (right deletions); duplicates by signature → conflict
```

Identity observation (NUIF interpretation): with stable entity identifiers `M = {(x, y) | id(x) = id(y)}`, which removes both GumTree phases and the ambiguity they resolve heuristically. The residue is exactly the class Mergiraf cannot resolve automatically: order-dependent insertions and moved-and-edited elements, i.e. sibling-order and move conflicts.

## NUIF relevance

**Borrow**
- The Chawathe edit-operation vocabulary (insert, delete, update, move-subtree) and the conforming-script discipline: NUIF operations in nuif-protocol already mirror it (`Insert`, `Remove`, `Rename`/`SetExtension`, `Move`).
- Mergiraf's commutative-parent and signature concepts: NUIF relation sets and unordered property maps are commutative parents by construction, and entity IDs are exact signatures.
- Conflict nodes embedded in the merged tree (Mergiraf `Conflict`) as the representation for spec/06's typed conflicts.

**Adapt**
- GumTree's matcher is still needed at the boundary: importing documents from formats without stable IDs (SVG, Figma exports, generated code) requires a matching step, and GumTree-style heuristics with tuned thresholds are the reference for that import path.
- The LCS-based child alignment (Chawathe §4.2) should be reused to convert two child sequences into a minimal move set when NUIF diffs snapshots rather than replaying operations.
- Mergiraf's diff3 fallback on the enclosing element corresponds to NUIF's textual fallback for human review in the canonical text form (`nuif-text-0`).

**Reject**
- Treating move detection as a heuristic: NUIF records moves explicitly; inferred moves are only for import.
- Difftastic's shortest-path diff as a merge primitive: it optimises display, ignores moves, and produces no patch.

## Open questions

- Which alignment cost (Chawathe's move-minimising LCS versus a Dice-weighted variant) produces the fewest spurious `Move` operations when diffing NUIF snapshots that differ by reordering.
- Whether Mergiraf's covering algorithm for delete/modify conflicts has an identity-based analogue that distinguishes "moved out then deleted" from "edited while deleted".
- How to calibrate GumTree thresholds for design-document trees (wide, shallow, many identical leaves) where the paper's defaults were tuned on Java ASTs.
