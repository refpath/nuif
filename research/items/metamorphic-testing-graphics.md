---
id: nuif:research:metamorphic-testing-graphics
kind: synthesis
status: reviewed
title: Metamorphic testing and metamorphic relations for graphics, layout and round-trip oracles
source:
  url: https://doi.org/10.1145/3133917
  doi: 10.1145/3133917
  repository: https://github.com/google/graphicsfuzz
  authors: [Alastair F. Donaldson, Hugues Evrard, Andrei Lascu, Paul Thomson, T. Y. Chen, S. C. Cheung, S. M. Yiu, Sergio Segura, Gordon Fraser, Ana B. Sanchez, Antonio Ruiz-Cortés]
  published_at: "2017-10-12"
  license: ACM copyrighted paper; GraphicsFuzz and SPIRV-Tools code Apache-2.0; Chen 1998 report on arXiv
retrieved_at: 2026-08-29
tags: [testing, metamorphic-testing, oracle-problem, graphics, shader-compilers, round-trip, layout, reduction]
confidence: 0.9
claims: [nuif:claim:semantic-automation, nuif:claim:authored-resolved, nuif:claim:opaque-preservation]
relations:
  - type: extends
    target: nuif:research:differential-testing
    note: Metamorphic relations replace a reference implementation when none exists; differential oracles cover the remaining cases.
  - type: depends_on
    target: nuif:research:delta-debugging-and-test-case-reduction
    note: GraphicsFuzz and spirv-fuzz reduce failing variants by removing transformations with delta debugging.
  - type: related_to
    target: nuif:research:property-based-testing-state-machines
    note: Operation-then-inverse relations are generated as property tests over operation sequences.
  - type: related_to
    target: nuif:research:golden-master-and-snapshot-testing
    note: Tolerant image comparison metrics from shader testing inform perceptual snapshot thresholds.
  - type: related_to
    target: nuif:research:vello
    note: Render-level relations require a deterministic reference rasterizer.
  - type: related_to
    target: nuif:research:taffy
    note: Layout-level relations are checked against the resolved boxes produced by the evaluator.
  - type: related_to
    target: nuif:research:encoding
    note: Encode-decode fixpoint relations apply to the canonical text and CBOR profiles.
  - type: related_to
    target: nuif:research:flip-perceptual-difference-metric
    note: A perceptual metric replaces the histogram chi-square distance for tolerant relations.
links:
  spec: [spec/00-conformance.md, spec/04-layout.md, spec/06-operations-and-patches.md, spec/08-serialization.md, spec/12-cli-api-and-automation.md]
  adr: []
  rfc: [rfcs/0004-headless-qa-contract.md]
  code: [crates/nuif-codec, crates/nuif-layout, crates/nuif-render, crates/nuif-protocol]
  experiments: [conformance/PLAN.md, conformance/fixtures/v0-responsive-card/README.md]
---

# Summary

Metamorphic testing (MT) checks necessary relations between the outputs of two or more related executions instead of checking one output against an expected value. It was proposed by Chen, Cheung and Yiu in 1998 as a way to derive new test cases from successful ones when no practical test oracle exists. Segura et al. (2016) surveyed 119 papers and report computer graphics and compilers among the most common application domains. Donaldson et al. (OOPSLA 2017) applied MT to graphics shader compilers: semantics-preserving transformations produce a family of variant shaders that must render the same image; deviations are detected with a tolerant histogram metric and reduced by reversing transformations. The later spirv-fuzz tool records transformations as a protobuf sequence, replays them, and shrinks failing sequences with delta debugging.

For NUIF, MT supplies the oracle for the trial-and-error loop where no reference implementation exists: encode-decode fixpoints, operation-then-inverse identity, translation equivariance of resolved boxes, and semantics-preserving document rewrites. The relation classes below are labelled as source-derived or as NUIF synthesis.

## Evidence

- MT was motivated by the oracle problem: the 1998 report states that test oracles are "pragmatically unattainable in most situations". Chen, Cheung, Yiu, HKUST-CS98-01, Abstract; arXiv 2002.12543, https://arxiv.org/abs/2002.12543, retrieved 2026-08-29.
- The 1998 report derives follow-up test cases from an input-output pair and the errors typically associated with the program; construction and checking must cost strictly less than executing the program. Same report, §2 Preliminaries.
- Canonical shortest-path example: reversing the query (y, x, G) must return the same distance, and splitting at an intermediate vertex must give distances that sum to the original. Same report, §3.3 and Table 3. The 1998 report does not use the phrase "metamorphic relation"; the formal term appears in later work.
- Segura et al. define a metamorphic relation as a relation among a series of inputs x1..xn (n > 1) and their outputs, and distinguish it from an invariant because it relates different executions. Segura, Fraser, Sanchez, Ruiz-Cortés, IEEE TSE 42(9), 2016, DOI 10.1109/TSE.2016.2532875, §2 (author preprint https://personal.us.es/sergiosegura/files/papers/segura16-tse.pdf, retrieved 2026-08-29).
- The survey covers 119 papers from 1998 to 2015; among case-study papers the leading domains are web services (16%), computer graphics (12%), simulation and modelling (12%) and embedded systems (10%). Same preprint, §1 and §5.1.
- The survey lists compilers as a domain, citing equivalence-preservation relations (replacing an expression by an equivalent one) and the EMI work that found 147 confirmed GCC/LLVM bugs. Same preprint, §5.1.10.
- MR construction is described as typically manual, with composition and automatic generation as reviewed alternatives. Same preprint, §4.2.
- The 2018 review gives Definition 1: an MR is a necessary property of f over a sequence of two or more inputs and their outputs, R ⊆ X^n × Y^n; it also states that not all MRs are equality relations. Chen et al., ACM Computing Surveys 51(1), 2018, DOI 10.1145/3143561, §2.2 Definition 1 and §3 Concept 3 (course mirror PDF https://homes.cs.washington.edu/~rjust/courses/CSE503/2021_02_12-reading2.pdf, retrieved 2026-08-29).
- Donaldson et al. state that GLSL is deliberately under-specified (denormal flushing, rounding, optimisation-induced differences), which makes pixel-exact comparison against a reference impossible. Donaldson, Evrard, Lascu, Thomson, PACMPL 1(OOPSLA), Art. 93, 2017, DOI 10.1145/3133917, §2.2 (open PDF https://www.doc.ic.ac.uk/~afd/homepages/papers/pdfs/2017/OOPSLA.pdf, retrieved 2026-08-29).
- The metamorphic form used is p(fI(x)) = fO(p(x)) checked with a tolerant equality; for a semantics-preserving fI, fO is the identity. The paper assumes P is deterministic and notes that MT finds bugs but cannot prove absence. Same paper, §2.3.
- Three phases: variant generation, detection of deviant variants, reduction of deviant variants. Same paper, §3.
- Opaque values come from a uniform `injSwitch` set to (0.0, 1.0) at run time, yielding expressions T, F, 0 and 1 the compiler cannot fold. Same paper, §3.1.1.
- Six transformation families: dead code injection, dead jump injection, live code injection, expression mutation (e + 0, e * 1, T ? e : d), vectorisation, and control-flow wrapping (single-iteration loops, `if(T){C}`). Transformations compose and are "easy to reverse during reduction". Same paper, §3.1.2 and §3.1.3.
- Image comparison uses the chi-squared distance between histograms (OpenCV `compareHist`, HSV, `HISTCMP_CHISQR`) with threshold 100 chosen empirically; pixel-per-pixel comparison is rejected. Same paper, §3.2 and §5.2.
- Reduction reverses random subsets of applied transformations until a minimal set is reached; it converges to a local minimum and is described as similar to delta debugging. Same paper, §3.3.
- Results: more than 60 distinct bugs across 17 GPU and driver configurations; §5.4 tabulates 71 logged issues. False positives were judged by three human raters over 975 reductions. Same paper, Abstract, §5.3, §5.4.
- GraphicsFuzz docs describe the pipeline: `glsl-generate` produces shader families from reference and donor shaders; workers render on devices; `glsl-reduce` shrinks with an interestingness script; default metric `HISTOGRAM_CHISQR`, threshold 100.0, alternative `PSNR`; reduction kinds include `ABOVE_THRESHOLD`, `NO_IMAGE`, `IDENTICAL`. https://github.com/google/graphicsfuzz, `docs/glsl-fuzz-intro.md`, `docs/glsl-fuzz-walkthrough.md`, `docs/glsl-fuzz-reduce.md`, `docs/glsl-reduce-intro.md`, master branch, retrieved 2026-08-29. The repository was archived on 2025-12-08.
- spirv-fuzz records each transformation as a protobuf message (`spvtoolsfuzz.proto`), writes `.transformations` and `.transformations_json`, and has FUZZ, REPLAY and SHRINK modes; `--shrink=<input.transformations> -- <interestingness_test>` where the script returns 0 iff the binary is interesting; `--replay-range`, `--shrinker-step-limit`, `--donors`, `--force-render-red`; the default pass selection is swarm testing. KhronosGroup/SPIRV-Tools, `tools/fuzz/fuzz.cpp` usage text (lines 60–176) and `docs/spirv-fuzz.md`, main branch, retrieved 2026-08-29.
- Transformation-based testing makes reduction and deduplication cheap: if transformations are small and independent, delta debugging shrinks the transformation subsequence, and the bug is reported as a delta between the original and a minimally transformed program. Donaldson et al., PLDI 2021, DOI 10.1145/3453483.3454092, Abstract, §2.1, §3.4 (open PDF https://www.doc.ic.ac.uk/~afd/papers/2021/PLDI.pdf, retrieved 2026-08-29).
- The PLDI 2021 reducer skips subsequences whose preconditions fail and halves chunk size until no single transformation can be removed; a set of facts (DeadBlock, Synonymous, Irrelevant, LiveSafe) is maintained to justify transformations. Same paper, §2.1, §3.2, §3.4.
- Janus (ICSE 2025) applies a delta-consistency oracle to browsers: for two HTML files differing by a minor modification, two browsers must agree on whether the renderings differ. Only the repository description was readable: https://github.com/ChijinZ/janus-browser-fuzzer, retrieved 2026-08-29; bug counts unverified.

## Mechanism

Definitions (from Segura 2016 §2 and Chen 2018 §2.2):

- A metamorphic relation R for a program P relates n ≥ 2 inputs and their outputs: R(x1..xn, P(x1)..P(xn)).
- A metamorphic test case is a source test case plus follow-up test cases derived from it by an input transformation fI; the check is R over the executions.
- R need not be an equality; subset, ordering and tolerant relations are admitted.

Transformation-based MT (from OOPSLA 2017 §3 and PLDI 2021 §2–3):

```
generate(reference, seed):
    T = []                                     # ordered transformation log
    facts = {}                                 # justifications (dead block, synonym, ...)
    repeat until budget:
        t = choose_transformation(seed, facts) # each t has precondition + effect
        if precondition(t, program, facts):
            program = apply(t, program); facts = update(facts, t); T.append(t)
    return program, T

check(reference, variant):
    img_r = run(reference); img_v = run(variant)
    return distance(hist(img_r), hist(img_v)) > threshold   # tolerant, not pixel-exact

reduce(reference, T, interesting):
    # delta debugging over T; program is rebuilt from reference each time
    n = 2
    while |T| >= 2:
        for chunk in split(T, n):
            T' = T \ chunk
            if replayable(T') and interesting(replay(reference, T')):
                T = T'; n = max(n - 1, 2); break
        else:
            if n >= |T|: break
            n = min(2n, |T|)
    return T
```

Invariants of the method:

- Every transformation is semantics-preserving modulo a declared tolerance (floating-point noise in shaders).
- Transformations are recorded, replayable by seed and reversible; the artifact under test is never edited directly, so reduced variants remain valid by construction.
- The oracle is a relation between executions, so no expected image is stored.

Relation classes for NUIF. Each class is tagged as source-derived (S) or NUIF synthesis (N).

1. Equivalence-preserving rewrite (S: OOPSLA 2017 §2.3, Segura §5.1.10). Wrapping a subtree in a no-op container, splitting a text run, or renaming entities must leave resolved boxes and rendered images within tolerance.
2. Encode→decode→encode fixpoint (N; matches the byte-stability criterion in `conformance/fixtures/v0-responsive-card/README.md`). For canonical encoders E and decoders D: E(D(E(d))) = E(d) and canonicalize is idempotent. The relation is exact, not tolerant.
3. Operation-then-inverse identity (N; reversal of recorded transformations in OOPSLA 2017 §3.3 is the closest source). For a transaction t with inverse t⁻¹ produced per `spec/06-operations-and-patches.md`: canon(apply(t⁻¹, apply(t, d))) = canon(d), and the inverse must satisfy its preconditions.
4. Commutativity of independent operations (S: Chen 1998 §3.3 reversal; N for layout). Operations on disjoint subtrees applied in either order must yield identical canonical hashes.
5. Translation and scale equivariance (N; general non-identity fO form from OOPSLA 2017 §2.3). Translating a freeform root by (dx, dy) must translate every resolved box by (dx, dy); scaling the viewport for a fully proportional layout must scale boxes proportionally within tolerance.
6. Additivity (S: Chen 1998 Table 3; N for layout). In a stack family, the resolved extent of a container equals the sum of child extents plus gaps and padding.
7. Monotone or subset relations (S: Chen 2018 Concept 3; N for documents). Removing an entity never increases the set of drawn commands; adding an opaque extension never changes resolved boxes.
8. Round-trip through an adapter (N). For an adapter export X and import Y over the representable subset: canon(Y(X(d))) = canon(d), fidelity report entries must explain every deviation, and opaque extension bytes must be identical.
9. Delta consistency across engines (S: Janus repository). For d and d' differing by one operation, NUIF and a browser must agree on whether resolved boxes changed.

## NUIF relevance

**Borrow**

- The three-phase structure generate variants → detect deviants → reduce by removing transformations maps directly onto NUIF operation logs, which are already ordered, serialisable and invertible (OOPSLA 2017 §3; PLDI 2021 §2).
- Record every generated variant as a replayable transformation sequence plus seed rather than as a mutated document, so reduction preserves validity (spirv-fuzz `.transformations`, `--replay`, `--shrink`).
- Use a declared tolerant metric for images and an exact metric for canonical bytes and hashes, mirroring the split between under-specified rendering and specified encoding (OOPSLA 2017 §2.2, §3.2).
- Treat MT as a bug-finding oracle with no completeness claim and pair it with differential and reference-model oracles (OOPSLA 2017 §2.3).

**Adapt**

- Replace the histogram chi-square metric with a perceptual metric bounded by the tolerances declared in `spec/00-conformance.md`; the shader metric ignores spatial position, which is unacceptable for layout.
- Extend equivalence-preserving rewrites with document-specific facts (entity is unreferenced, extension is opaque, subtree is invisible) analogous to the spirv-fuzz fact set, so that preconditions keep variants valid.
- Add exact-equality relations (fixpoint, idempotence, inverse identity) that shader testing does not need because compilers have no canonical form.

**Reject**

- Manual MR discovery per feature at survey scale (Segura §4.2) is too slow; NUIF should derive relations mechanically from the operation and layout family definitions.
- Human-rated false-positive adjudication (OOPSLA 2017 §5.3) is not automatable; NUIF should encode the tolerance policy in the report instead.

## Open questions

- Which layout families admit exact translation and scale equivariance, and which require tolerance because of rounding to device pixels?
- How should tolerant relations be expressed in the machine-readable report so that a threshold change is visible as a policy change rather than as a test change?
- Does the inverse-operation relation hold for `Move` across component instances with overrides, or must the relation be weakened to canonical equality modulo derived caches?
- Can delta consistency against a browser be made stable when the browser and NUIF differ in sub-pixel snapping?
