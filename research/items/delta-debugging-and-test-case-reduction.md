---
id: nuif:research:delta-debugging-and-test-case-reduction
kind: synthesis
status: reviewed
title: Delta debugging and test-case reduction (ddmin, HDD, C-Reduce, Hypothesis choice-sequence shrinking, proptest shrinking)
source:
  url: https://doi.org/10.1109/32.988498
  doi: 10.1109/32.988498
  repository: https://github.com/HypothesisWorks/hypothesis
  authors: [Andreas Zeller, Ralf Hildebrandt, Ghassan Misherghi, Zhendong Su, John Regehr, Yang Chen, Pascal Cuoq, Eric Eide, Chucky Ellison, Xuejun Yang, David R. MacIver, Alastair F. Donaldson]
  published_at: "2002-02-01"
  license: TSE 2002 IEEE copyrighted; HDD and C-Reduce ACM copyrighted; ECOOP 2020 CC-BY 3.0 (LIPIcs); Hypothesis MPL-2.0; proptest MIT OR Apache-2.0
retrieved_at: 2026-08-29
tags: [testing, delta-debugging, test-case-reduction, shrinking, hierarchical-delta-debugging, c-reduce, hypothesis, proptest, minimization]
confidence: 0.93
claims: [nuif:claim:semantic-automation]
relations:
  - type: extends
    target: nuif:research:property-based-testing-state-machines
    note: Supplies the reduction algorithms behind transition-sequence shrinking.
  - type: related_to
    target: nuif:research:metamorphic-testing-graphics
    note: GraphicsFuzz and spirv-fuzz reduce by removing recorded transformations, a ddmin instance.
  - type: related_to
    target: nuif:research:fuzzing-structured-inputs
    note: cargo fuzz tmin and libFuzzer -minimize_crash provide byte-level reduction; structure-aware reduction needs the generator.
  - type: related_to
    target: nuif:research:differential-testing
    note: McKeeman and Csmith require reduction before divergences are reportable, and reduction must preserve input validity.
  - type: related_to
    target: nuif:research:tree-sitter
    note: Grammar-guided reducers (HDD, Perses) operate on parse trees of the kind tree-sitter produces for source adapters.
links:
  spec: [spec/06-operations-and-patches.md, spec/12-cli-api-and-automation.md, spec/08-serialization.md]
  adr: []
  rfc: [rfcs/0004-headless-qa-contract.md]
  code: [crates/nuif-protocol, crates/nuif-core, crates/nuif-cli, crates/nuif-testing/src/reduction.rs, crates/nuif-testing/src/bin/reduction-profile.rs]
  experiments: [conformance/PLAN.md, conformance/HARNESS.md, research/experiments/index.yaml]
---

# Summary

Delta debugging (Zeller and Hildebrandt 2002) defines 1-minimality and the ddmin algorithm, which reduces a failing input by testing subsets and complements at increasing granularity; the worst case is |c|² + 3|c| tests, the best case 2·log₂|c|. Hierarchical delta debugging (Misherghi and Su 2006) applies ddmin level by level over a parse tree so that every candidate is syntactically valid and needs orders of magnitude fewer tests. C-Reduce (Regehr et al. 2012) generalises to a fixpoint over many pluggable transformation passes with an external interestingness test and validity checking. Hypothesis (MacIver and Donaldson 2020) reduces the sequence of random choices consumed by the generator instead of the generated value, so every reduced candidate is generatable by construction; shortlex order and adaptive passes drive it. proptest shrinks through `ValueTree::simplify`/`complicate` binary search, with collections deleting elements before shrinking them; proptest-state-machine deletes transitions while re-checking preconditions.

For NUIF the failing artefact is a seed plus an operation sequence over a tree document. The algorithm below combines ddmin over the operation list, HDD over the base document, and choice-sequence reduction so that the minimised fixture remains a valid document and a valid patch.

## Evidence

- A test case c is 1-minimal if removing any single change makes the failure disappear; determining a local minimum requires 2^|c| tests in general. Zeller and Hildebrandt, IEEE TSE 28(2), 2002, DOI 10.1109/32.988498, §III-A Definitions 8–10 (PDF https://www.st.cs.uni-saarland.de/papers/tse2002/tse2002.pdf, retrieved 2026-08-29).
- ddmin(c) = ddmin2(c, 2) with three rules: reduce to subset (continue with n = 2), reduce to complement (continue with max(n − 1, 2)), increase granularity (n = min(|c|, 2n)); Proposition 11: the result is 1-minimal; Proposition 12: worst case |c|² + 3|c| tests; Proposition 13: best case 2·log₂|c|. Same paper, §III-B Fig. 5 and Propositions 11–13.
- GCC case: 755 characters reduced to 77 after 731 further tests in 34 seconds; the failure-inducing option −ffast-math was isolated among 31 options in 7 tests. Same paper, §IV-A.
- Mozilla case: 95 user actions reduced to 3 after 82 runs; 896 lines of HTML reduced by a hierarchical approach (lines, then characters) to `<SELECT>` after 57 line-level runs. Same paper, Abstract and §IV-B.
- The isolating variant dd works on the pair (passing, failing) and needs only log₂|c| tests without unresolved outcomes; isolating the GCC difference took 59 tests where minimising took 731. Same paper, §V and §VI.
- HDD applies ddmin to each level of the tree from coarsest to finest, prunes irrelevant nodes, and "All the generated input configurations are syntactically valid"; it may not produce 1-minimal results, HDD* iterates to a 1-tree-minimal fixpoint in O(n³) worst case; finding a global minimum is NP-complete. Misherghi and Su, ICSE 2006, DOI 10.1145/1134285.1134307, Abstract, §3.2 Algorithm 1, §3.4 (PDF https://web.cs.ucdavis.edu/~su/publications/icse06-hdd.pdf, retrieved 2026-08-29).
- HDD numbers: bug.c ddmin 680 tests/53 tokens vs HDD 86/51; boom7.c 3727/102 vs 144/57; XSL case ddmin-line 1092 tests to 92 lines vs HDD 124 tests to 8 lines. Same paper, §4.1 Table 1, §4.2 Table 2.
- C-Reduce: a generic fixpoint over modular transformations, each an iterator with new/transform/advance, parameterised by a test that decides whether a variant is successful; outputs average more than 25 times smaller than Berkeley delta; validity is checked with Frama-C or KCC because unguarded reduction introduces undefined behaviour. Regehr, Chen, Cuoq, Eide, Ellison, Yang, PLDI 2012, DOI 10.1145/2254064.2254104, Abstract, §5, §6.3 Listing 2, §7 Table 1 (preprint https://users.cs.utah.edu/~regehr/papers/pldi12-preprint.pdf, retrieved 2026-08-29).
- Regehr's guidance: an interesting variant seeds further reduction, an uninteresting one is a dead end; order interestingness checks fastest first and run biggest-win passes first; "If these criteria contain any kind of loophole, C-Reduce is likely to find it." https://blog.regehr.org/archives/1678, retrieved 2026-08-29.
- Hypothesis reduces "the sequence of random choices made during generation", "ensuring that any reduced test case is one that could in principle have been generated"; generators are viewed as parsers of choice sequences; a too-short sequence is a parse error; order is shortlex. MacIver and Donaldson, ECOOP 2020, DOI 10.4230/LIPIcs.ECOOP.2020.13, Abstract, §2.1–2.2, §3.2 (PDF https://drops.dagstuhl.de/opus/volltexte/2020/13170/pdf/LIPIcs-ECOOP-2020-13.pdf, retrieved 2026-08-29).
- Hypothesis 5.15.1 used 15 passes: six deleting contiguous regions, region-to-subregion, region-to-zeroed sequence, four lexicographic, three combined, one float-specific; the list generator emits a "more" bit before each element so element deletion is a contiguous deletion. Same paper, §3.1, §3.3 Fig. 7.
- Evaluation: on Csmith programs Hypothesis reduced to 812 bytes (floor 410) versus C-Reduce 120 and Picire 345, using 762 SUT invocations versus 3968 (C-Reduce) and 3139 (Picire). Same paper, §4.1.2 Fig. 8 and Fig. 12.
- Hypothesis source (`hypothesis/src/hypothesis/internal/conjecture/shrinker.py`, master at 49a797b, retrieved 2026-08-29): `sort_key` implements shortlex over choice indices (lines 73–91); the `Shrinker` docstring requires that progress be deterministic, that passes not iterate to a fixed point internally, and recommends adaptive passes that turn O(m) successful calls into O(log m); the pass list includes `node_program("X"*k)` deletions for k = 5..1, `reorder_spans`, `minimize_duplicated_choices`, `minimize_individual_choices`, `redistribute_numeric_pairs`, `lower_integers_together` (lines 343–359); `fixate_shrink_passes` loops until no pass improves, with 20 consecutive failures per pass and length-reducing passes sorted first (lines 865–958).
- Haskell QuickCheck 2.18: `shrink :: a -> [a]` lists immediate shrinks; candidates are tried in list order, so aggressive steps go first; `genericShrink` tries subterms then recursive shrinks; `shrinkList` shrinks lists given an element shrinker. https://hackage-content.haskell.org/package/QuickCheck-2.18.0.0/docs/Test-QuickCheck.html, retrieved 2026-08-29.
- proptest 1.11.0: `ValueTree::simplify` moves current to a halfway point between low and high, `complicate` partially undoes the last simplification; `prop_map` shrinks in terms of the source value; `prop_flat_map` shrinks both the input and the derived values; `prop_filter` can largely prevent shrinking; `VecValueTree` uses `Shrink::DeleteElement` then `Shrink::ShrinkElement` ("delete elements from the list until we can do so no further, then to shrink each remaining element"). https://docs.rs/proptest/latest/proptest/strategy/trait.ValueTree.html, trait.Strategy.html, and https://docs.rs/proptest/latest/src/proptest/collection.rs.html, retrieved 2026-08-29.
- Perses guarantees that each reduction step considers only smaller, syntactically valid variants by reducing over a grammar in a normal form with quantifiers; results are 2% and 45% of the size of DD and HDD outputs on 20 C programs. Sun, Li, Zhang, Zhang, Su, ICSE 2018, DOI 10.1145/3180155.3180236, Abstract, §3, §4.3 (PDF https://web.cs.ucdavis.edu/~su/publications/perses.pdf, retrieved 2026-08-29).

## Mechanism

Definitions: an interestingness test `interesting(x) ∈ {FAIL, PASS, UNRESOLVED}`; a validity oracle `valid(x)` run before the system under test; a result is 1-minimal when no single element can be removed while preserving FAIL.

```
test(x) = memo( valid(x) ? interesting(x) : UNRESOLVED )        # C-Reduce §5, §6.3

ddmin(c, n = 2):                                                  # TSE 2002 Fig. 5
    chunks = split(c, n)
    if ∃i: test(chunks[i]) == FAIL:        return ddmin(chunks[i], 2)
    if ∃i: test(c − chunks[i]) == FAIL:    return ddmin(c − chunks[i], max(n − 1, 2))
    if n < |c|:                            return ddmin(c, min(|c|, 2n))
    return c                                                      # 1-minimal

hdd(tree):                                                        # HDD Alg. 1
    level = 0
    while nodes(tree, level) ≠ ∅:
        keep = ddmin(nodes(tree, level))    # test(prune(tree, level, keep))
        tree = prune(tree, level, keep)     # manipulator keeps required children
        level += 1
    return tree
repeat hdd until no node removed                                  # HDD*

reduce_choices(seq):                                              # ECOOP 2020, shrinker.py
    order = shortlex(seq)                   # length first, then choice indices
    passes = [zero_spans, delete_k_consecutive(5..1), subregion, reorder_spans,
              minimize_duplicates, minimize_individual (binary search), redistribute]
    until no pass improves:
        for p in passes: run p until 20 consecutive non-improvements
        sort passes: length-reducing first
    return generator.parse(seq)             # valid by construction
```

NUIF reducer over (seed, base document, operation sequence), synthesis with attribution:

1. Level A, operations: ddmin over the transaction list with `test` = replay on the base document followed by the failing comparison; validity = every operation's preconditions hold on the intermediate document (TSE 2002; eqc_statem precondition rule via nuif:research:property-based-testing-state-machines).
2. Level B, document: HDD over the base document tree by depth, pruning subtrees not referenced by remaining operations; the tree manipulator must keep component definitions referenced by instances, token definitions referenced by bindings, and parents of moved entities (HDD §3.2; Perses grammar validity).
3. Level C, values: choice-sequence reduction of the generator input so that names, sizes, extension payloads and viewport contexts shrink toward the simplest generatable values (ECOOP 2020; Hypothesis passes); for hand-written fixtures, per-value `ValueTree` binary search instead (proptest).
4. Isolation: when a passing base revision exists, run dd on the pair (passing, failing) to isolate the failure-inducing operation subset in log₂|c| tests before minimising (TSE 2002 §V).
5. Output: a fixture consisting of the reduced canonical document, the reduced patch, the seed, and the interestingness predicate identifier, as required by item 9 of `apps/editor/QA.md`.

Invariants: every candidate passed to the system under test is a valid document and a valid patch; the reducer never edits the document directly when a generator exists; progress is deterministic for a fixed seed; memoised results keep the test count within the ddmin bounds.

## NUIF relevance

**Borrow**

- ddmin as the operation-list reducer and dd as the isolator; both are small, proven 1-minimal and have known bounds (TSE 2002 Propositions 11–13, 16–18).
- Hypothesis's generator-as-parser principle: reducing the seed-derived choice sequence guarantees that minimised documents are generatable and valid, which is the property the task requires (ECOOP 2020 §2.2, §3.2).
- C-Reduce's separation of a domain-independent fixpoint driver from pluggable passes, and its rule that validity is checked before the system under test (PLDI 2012 §5–6).
- proptest-state-machine's delete-then-shrink order for transition lists and its precondition re-check on every deletion (proptest `VecValueTree`; proptest-state-machine `Shrink`).

**Adapt**

- HDD's tree levels map to NUIF entity depth, but pruning must respect graph references (instances to components, token bindings, relations) that a parse tree does not have; the tree manipulator becomes a document-aware pruner that also removes dangling references.
- The "more" bit trick from the Hypothesis list generator should be used in the NUIF operation generator so that deleting an operation is a contiguous choice-sequence deletion.
- Interestingness for tolerant oracles (image or box differences) must be stable under reduction; the predicate should compare against the same declared tolerance and record the maximum delta.

**Reject**

- Character- or line-based reduction of canonical text (Berkeley delta style); it produces invalid documents and is orders of magnitude slower than structural reduction (PLDI 2012 §3.2; HDD Table 2).
- Reduction without a validity oracle; Csmith and C-Reduce show that unguarded reducers converge to invalid inputs (PLDI 2011 §3.7; Regehr blog).

## Implemented decision

NUIF stores the reduced canonical document and semantic operations as the
durable regression fixture, together with the seed, predicate identifier,
content hashes and every accepted transformation. The choice sequence remains
reproduction evidence for generator/fuzzer failures but is not the only durable
form because generators evolve. The document reducer iterates subtree deletion
at progressively finer granularity and then runs explicit graph-collection,
extension and known-scalar passes; full structural validation precedes every
interestingness call. Unknown-kind opaque payload bytes are held fixed because
only their owner can define meaningful byte-level simplification, though a
whole irrelevant unknown entity or namespace can be removed. The bounded NUIF
profile does not need general cubic HDD*: the subtree pass returns only after no
remaining individual subtree can be removed, and all later passes are strictly
simplifying and content-hash memoized. `cargo xtask reduction-profile` records
the resulting three-entity ancestor path and emitted fixture in CI.
