---
id: nuif:research:bidirectional-evaluation-direct-manipulation
kind: paper
status: reviewed
title: Bidirectional evaluation with direct manipulation (evaluation update for a general-purpose functional language)
source:
  url: https://arxiv.org/abs/1809.04209
  doi: 10.1145/3276497
  repository: https://github.com/ravichugh/sketch-n-sketch
  authors: [Mikaël Mayer, Viktor Kunčak, Ravi Chugh]
  published_at: 2018-11
  license: ACM (Proc. ACM Program. Lang. 2, OOPSLA, Article 127); preprint under arXiv distribution license
retrieved_at: 2026-08-29
tags: [bidirectional-transformations, evaluation-update, direct-manipulation, lenses, program-repair, diff, provenance]
confidence: 0.9
claims: [nuif:claim:sync-not-regenerate, nuif:claim:authored-resolved]
relations:
  - type: extends
    target: nuif:research:sketch-n-sketch-output-directed-programming
    note: Replaces numeric trace-based synthesis (PLDI 2016) with an update relation over all value types and expression forms; the paper lists PLDI 2016 Limitations A-D in §1.
  - type: related_to
    target: nuif:research:lenses-foster-boomerang
    note: User-defined apply/update pairs are called lenses but round-trip laws are not required (§6, "Round-Trip Laws").
  - type: related_to
    target: nuif:research:symmetric-lenses
    note: Update is asymmetric (program is source, output is view); complement is the original program itself.
  - type: related_to
    target: nuif:research:retentive-lenses
    note: Structure preservation of update (values structurally equivalent, expressions structurally equivalent) is a weaker relative of retentiveness.
  - type: compares_to
    target: nuif:research:ui-code-generation-boundaries
    note: Demonstrates pushing output edits into source without regeneration for HTML-generating programs.
links:
  spec: [spec/06-operations-and-patches.md, spec/09-provenance-and-fidelity.md]
  adr: []
  rfc: [rfcs/0003-authored-resolved-provenance.md]
  code: [crates/nuif-protocol]
  experiments: [nuif:experiment:v0-responsive-card]
---

# Summary

The paper defines an evaluation update relation for LittleLeo, an ML-style lambda calculus with lists, records and dictionaries. Forward evaluation is the standard big-step judgement `E ⊢ e ⇒ v`; update is the judgement `(E ⊢ e) ⇐ v′ ⇝ (E′ ⊢ e′)`, read as "when the output is changed to v′, the program E ⊢ e becomes E′ ⊢ e′". Update rules retrace the evaluation derivation, replacing constants and closures at the leaves and propagating new bindings back through variables, let, application and conditionals. Conflicting bindings produced by different subderivations are reconciled by an environment merge; a conservative two-way merge yields a soundness theorem (re-evaluating the updated program produces v′), whereas the optimistic three-way merge used in the implementation abandons that guarantee in exchange for propagating a single edited use of a variable to all uses. List values are updated through a diff (Keep, Delete, Insert, Update) computed by dynamic programming, and the implementation propagates edit differences instead of whole values, which is reported as the decisive optimisation (70× average speed-up). Expert users may register custom lenses (apply/update pairs) that are invoked by `applyLens`, with access to the internal `updateApp`, `diff` and `merge` primitives. The update relation is nondeterministic; the Sketch-n-Sketch implementation enumerates solutions lazily and presents them as a menu with code and output previews. Across ten HTML-generating examples (about 1400 lines), 92 update calls produced 1.18 solutions on average and took 723 ms on average, close to the 833 ms average forward evaluation time. This record separates the paper's claims from NUIF interpretation in the sections below.

## Evidence

- Bibliographic data: Proc. ACM Program. Lang. 2, OOPSLA, Article 127 (November 2018), 28 pages, DOI 10.1145/3276497; arXiv:1809.04209v2, 18 October 2018. Source: arXiv preprint header and ACM reference format block, p. 127:1 (retrieved 2026-08-29).
- The PLDI 2016 system is characterised as having four limitations: only SVG-generating programs (A), only numeric values traced (B), no user customisation (C), and trace storage cost (D); the new approach addresses all four. Source: §1, p. 127:2-3.
- Syntax of LittleLeo (constants, closures, lists, records, `applyLens`, `updateApp`, `diff`, `merge`, `freeze`): Figure 6, §3, p. 127:9.
- Update judgement definition and the three rule families (replacement, primitive, propagation): §3.1, p. 127:9.
- Selected rules U-Const, U-Fun, U-Var, U-Let, U-App, U-If-True, U-Freeze: Figure 7, p. 127:10. U-Plus-1/U-Plus-2 (two valid updates for addition), U-Lt (operator flip), U-And: Figure 8, p. 127:11. List rules U-Cons, U-List with Diff operations Keep, Delete, Insert(v′), Update(v′): Figure 9, p. 127:13.
- Conservative two-way merge: Definition 3.1, p. 127:11; optimistic three-way merge: Definition 3.2, p. 127:12; Example 3.3 (`let x = 1 in [x, x]` updated to `[1, 2]` yields `let x = 2 in [x, x]` only under three-way merge), p. 127:12; Example 3.4 (control-flow deviation under three-way merge), p. 127:12-13.
- Theorem 3.5 (EvalUpdate): if `E ⊢ e ⇒ v` then `E ⊢ e ⇐ v ⇝ E ⊢ e`. Theorem 3.6 (Conservative UpdateEval): with two-way merge, if `E ⊢ e ⇐ v′ ⇝ E′ ⊢ e′` then `E′ ⊢ e′ ⇒ v′`. Proof sketch in supplementary appendices. Source: §3.1.5, p. 127:14.
- Structural updates are permitted only in list literals ("pretty local updates"); cons expressions are never added or removed by the core rules "because of the amount of ambiguity they would introduce". Source: §3.1.4, p. 127:13.
- The `map` pattern cannot be repaired by the core algorithm; motivation for user-defined lenses. Source: §3.2, p. 127:14-15. Lens type `{ apply: a -> b, update: {input: a, outputNew: b} -> {values: List a} }` and rules E-Lens, U-Lens, E-Update-App, E-Diff, E-Merge: Figure 10 and §3.2.1, p. 127:15. Example lenses for MaybeOne map (Figure 11) and control-flow repair `if_` (Figure 12), pp. 127:16-18.
- Implementation optimisations: continuation-passing style to avoid browser stack overflow; merging only bindings free in closure bodies to avoid exponential merge; propagation of edit differences instead of values, exposed to lenses through `outputOld` and `diffs` fields. Source: §4.1, pp. 127:18-19.
- Whitespace-preserving abstract syntax for readable updated programs: §4.1, p. 127:19.
- Ambiguity presentation: candidate repairs are shown in a nested "Update Program" menu with previews of code and output; users freeze expressions (`Update.freeze`) to remove undesired solutions. Source: §2.2 and Figure 3, pp. 127:5-6.
- Performance table: 10 examples, 1469 LOC total, average Eval 833±400 ms, 92 update calls, average 1.18 solutions, average optimised update 723±900 ms, 70× speed-up over the version without edit differences; Node.js 6.9.5, Intel i7-6820HQ. Source: Figure 13 and §5.2, pp. 127:22-23.
- Round-trip laws are explicitly not required; many implemented lenses "violate even the basic laws". Source: §6, "Round-Trip Laws", pp. 127:23-24.
- Diff alignment is a single heuristic; nested differences are unsupported (example `[x,y,z]` to `[x, ["b",[],[y]], z]`). Source: §6, "Alignment", p. 127:24.
- Follow-up: Mayer and Chugh, "A Bidirectional Krivine Evaluator", Bx 2019 (CEUR-WS Vol. 2355, paper 5, pp. 56-60) restates the call-by-value system with Theorem 1 (structure preservation) and Theorem 2 (soundness) and gives call-by-name and Krivine-machine variants with Theorems 3-9. Retrieved from https://ceur-ws.org/Vol-2355/paper5.pdf on 2026-08-29.
- Implementation: Sketch-n-Sketch v0.7.1, more than 12,000 lines of Elm and JavaScript added. Source: §4, p. 127:18.

## Mechanism

Judgements (§3.1):

```
Evaluation:        E ⊢ e ⇒ v
Evaluation update: (E ⊢ e) ⇐ v′ ⇝ (E′ ⊢ e′)
```

Replacement axioms (Figure 7):

```
[U-Const]  E ⊢ c ⇐ c′ ⇝ E ⊢ c′
[U-Fun]    E ⊢ λp.e ⇐ (E′, λp.e′) ⇝ E′ ⊢ λp.e′
[U-Var]    E = E1, x ↦ v, E2   ⟹   E ⊢ x ⇐ v′ ⇝ (E1, x ↦ v′, E2) ⊢ x
```

Propagation through binding forms (Figure 7): U-Let re-evaluates `e1` to `v1`, pushes `v2′` into `e2` under `E, x ↦ v1`, obtains an updated binding `v1′`, pushes `v1′` into `e1`, and merges the two resulting environments. U-App does the same through a closure: the new body and new closure environment are pushed back into the function expression, the new argument value into the argument expression, and the environments are merged. U-If-True pushes into the taken branch and assumes the guard is unchanged.

Environment merge (Definitions 3.1 and 3.2):

```
Two-way (conservative):  (E1, x↦v1) ⊕ (E2, x↦v2) = (E′, x↦v)
   v = v1 if v1 = v2;  v = v1 if x ∉ fv(e2);  v = v2 if x ∉ fv(e1);  otherwise fail
Three-way (optimistic):  x ↦ (v1 ⊕_v v2), base-case rule prefers v2 when v2 ≠ v, else v1
```

Correctness (§3.1.5):

```
Theorem 3.5 (EvalUpdate):             E ⊢ e ⇒ v  ⟹  E ⊢ e ⇐ v ⇝ E ⊢ e
Theorem 3.6 (Conservative UpdateEval): E ⊢ e ⇐ v′ ⇝ E′ ⊢ e′ (two-way merge)  ⟹  E′ ⊢ e′ ⇒ v′
```

Theorem 3.5 is the analogue of the lens law GetPut (putting back the unchanged view leaves the source unchanged). Theorem 3.6 is the analogue of PutGet (re-evaluating the updated program reproduces the edited output) and holds only for the conservative merge. No PutPut-style law and no determinism or completeness result is claimed; the relation is a set of candidate repairs.

List diff (Figure 9): `Diff(v, v′)` produces a sequence over `{Keep, Delete, Insert(v′), Update(v′)}` by dynamic programming that prefers long contiguous preserved runs; `U-List` walks the literal and the diff in parallel, inserting `exp(v′)` (a literal expression synthesised from a value) for insertions. Dictionaries and records use analogous difference operations without insertion/deletion for records.

User-defined lenses (Figure 10): `applyLens l e` evaluates `l.apply e`; in the backward direction, `l.update {input, outputOld, outputNew, diffs}` returns `{values = [...]}` and each candidate argument is pushed back into `e`. `updateApp` exposes U-App, `diff` exposes Diff, and `merge` exposes three-way value merge to lens code.

Ambiguity handling: all solutions are enumerated lazily; the editor previews each; `freeze e` (U-Freeze) pins subterms to prune the solution space.

## NUIF relevance

- **Borrow**: The judgement shape `(E ⊢ e) ⇐ v′ ⇝ (E′ ⊢ e′)` together with Theorems 3.5/3.6 is a precise template for specifying an NUIF adapter's design-to-source direction: an adapter's put must satisfy an EvalUpdate law (unchanged resolved view yields an empty patch) and, where it claims lossless fidelity, an UpdateEval law (re-lowering the patched source reproduces the edited document).
- **Borrow**: Propagating edit differences rather than whole values (§4.1, Optimisation 3) matches NUIF's patch model; the reported 70× speed-up supports designing the protocol around operation deltas with base-snapshot identity instead of full-document diffs.
- **Adapt**: The explicit distinction between conservative (sound, may fail) and optimistic (always succeeds, may change unrelated output) merge should surface in NUIF as a fidelity class on the patch: a patch produced under an optimistic policy must be reported as `approximated` with the affected uses listed, never as `lossless`.
- **Adapt**: The freeze primitive corresponds to NUIF correspondence records marking source regions as non-editable from the design side; NUIF should expose the same pruning to adapters but store it in the correspondence map rather than in the source program.
- **Adapt**: Solution enumeration with previews is an editor concern; the NUIF protocol should instead return a ranked candidate list of patches with provenance so that any editor can implement the menu.
- **Reject**: The absence of round-trip laws for user lenses (§6) is acceptable for an interactive programming environment but not for a conformance-tested interchange standard; NUIF adapters must declare which laws they satisfy per fidelity class.
- **Reject**: Update through general recursion and higher-order code (U-App into closure environments) presupposes an interpreter for the target language; NUIF adapters for Svelte, React or SwiftUI cannot assume this and should restrict the source side to a syntactic correspondence fragment (literal values, static structure) as in nuif:research:tree-sitter.

## Open questions

- Which subset of the U-rules can be realised without evaluating the target program, given only a syntax tree and correspondence records? The literal-replacement rules (U-Const, U-Var into let-bound literals, U-List on literals) appear feasible; U-App does not.
- Can Theorem 3.6 be checked mechanically per patch (re-lower and compare resolved geometry) as a conformance oracle for "minimal source patch after design edit" in nuif:experiment:v0-responsive-card?
- The Diff heuristic is fixed and alignment is not nested; NUIF entities carry stable identities, which removes most alignment ambiguity for the design side, but the source side still needs an alignment policy for lists without keys.
- Performance was measured on 37-534 line programs; scaling to component libraries with thousands of lines is not reported.
