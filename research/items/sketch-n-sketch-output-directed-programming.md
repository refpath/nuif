---
id: nuif:research:sketch-n-sketch-output-directed-programming
kind: paper
status: reviewed
title: Sketch-n-Sketch (PLDI 2016, UIST 2016, UIST 2019) - trace-based program updates and output-directed programming for SVG
source:
  url: https://arxiv.org/abs/1507.02988
  doi: 10.1145/2908080.2908103
  repository: https://github.com/ravichugh/sketch-n-sketch
  authors: [Ravi Chugh, Brian Hempel, Mitchell Spradlin, Jacob Albers, Justin Lubin]
  published_at: 2016-06-02
  license: ACM (PLDI 2016 pp. 341-354; UIST 2016 pp. 379-390; UIST 2019 pp. 281-292); preprints under arXiv distribution license
retrieved_at: 2026-08-29
tags: [direct-manipulation, provenance, trace, program-synthesis, svg, bidirectional-editing, heuristics, output-directed-programming]
confidence: 0.9
claims: [nuif:claim:sync-not-regenerate, nuif:claim:authored-resolved]
relations:
  - type: related_to
    target: nuif:research:bidirectional-evaluation-direct-manipulation
    note: OOPSLA 2018 generalises the numeric trace mechanism to all value types; see its §1 Limitations A-D.
  - type: related_to
    target: nuif:research:retentive-lenses
    note: Local updates change only the constants on a trace; all other source text is retained by construction.
  - type: related_to
    target: nuif:research:lenses-foster-boomerang
    note: No lens laws are stated; "faithful" and "plausible" updates are the paper's own correctness criteria.
  - type: compares_to
    target: nuif:research:reverse-layout-inference
    note: Both recover authored parameters from output geometry; Sketch-n-Sketch has the program, ReverseORC does not.
  - type: related_to
    target: nuif:research:ui-code-generation-boundaries
links:
  spec: [spec/06-operations-and-patches.md, spec/09-provenance-and-fidelity.md]
  adr: []
  rfc: [rfcs/0003-authored-resolved-provenance.md]
  code: [crates/nuif-protocol]
  experiments: [nuif:experiment:v0-responsive-card]
---

# Summary

Three papers describe successive versions of Sketch-n-Sketch, an editor in which a program in a small functional language (`little`, later `Leo`) generates SVG output and direct manipulation of that output is translated into program edits. PLDI 2016 introduces trace-based program synthesis: every numeric literal carries a source location, primitive operations build data-flow traces, a user drag turns the trace of a manipulated attribute into a value-trace equation, and the system solves the equation by changing exactly one program constant per attribute, choosing the constant with a "fair" rotation heuristic that is fixed before the drag begins so that updates apply live. UIST 2016 adds tools that transform program structure rather than constants (Draw, Dig Hole, Fill Hole, Make Equal, Group, Abstract, Duplicate, Merge). UIST 2019 replaces location traces by general value provenance (each value tagged with its producing expression and pointers to the values it was computed from) and uses that provenance to fill "value holes", expose intermediate values as widgets, and drive refactorings such as Abstract, Repeat and Add Argument; sixteen parametric designs (427 lines) were built with no text editing. The reported limitations are consistent across the papers: only data-flow is traced (control flow is not), only constants are solved for, solvers accept only single-occurrence equations, ambiguity is resolved by heuristics rather than by asking, and the provenance-based tools are hand-coded one by one. Factual content below is separated from NUIF interpretation.

## Evidence

PLDI 2016, "Programmatic and Direct Manipulation, Together at Last" (arXiv:1507.02988v3, 18 April 2016; DOI 10.1145/2908080.2908103, PLDI 2016 pp. 341-354, verified via Crossref 2026-08-29):

- Two kinds of traces: locations `ℓ` annotating every numeric literal, and expression traces built by rule E-Op-Num during primitive operations; traces record data flow but not control flow, a deliberate design choice justified by the observation that visual programs have stable control flow. Source: §2.1 and Figure 2.
- Value-trace equations, for example `50 = (+ x0 (* ℓ0 sep))`, together with the substitution `ρ0` from locations to current values, relate program and output. Source: §2.1, equations (1)-(3).
- Local updates are substitutions from locations to numbers; only numeric constants change. Source: §2.2 "Local Updates".
- Frozen constants (`n!`) are excluded from updates; all Prelude literals are frozen automatically; range annotations `n{lo-hi}` produce sliders. Source: §2.2 "Frozen Constants", §2.4.
- Correctness criteria: a substitution is faithful if re-evaluation produces a structurally similar value context (`V′ ∼ V`) and every user-changed value is reproduced ("(c) implies (d)"); plausible if at least one user-changed value is reproduced. Source: §3, Definitions "Faithful Updates" and "Plausible Updates".
- Hard constraints are the j user-changed values, soft constraints the k−j unchanged ones. Source: §3 table "Program / Output / Updates / Constraints".
- Shape assignments map each shape and zone to a location set; the "fair" heuristic rotates through candidate assignments so each location set is chosen equally often; a "biased" heuristic preferring rarely used locations is described in the appendix. Source: §4.1 "Fair and Other Heuristics".
- Mouse trigger `τ = λ(dx, dy). ρ` is computed before the drag; `ComputeTrigger(ρ, γ, v)` solves one univariate equation per attribute with `SolveOne`; exactly one location per updated attribute is modified. Source: §4.1 "Computing Triggers" and "Recap: Design Decisions".
- Solutions are only plausible, not faithful, when one location feeds several manipulated attributes; substitutions are then applied in implementation-specific order. Source: §4.1 "Recap".
- `SolveOne` supports only single-occurrence equations, inverting primitives top-down; not all primitives have total inverses. Source: §5.1.
- Corpus: 68 programs, more than 2,000 lines; 3,772 shapes and 14,106 zones, of which 7% inactive, 34% unambiguous, 59% ambiguous with 3.83 candidates on average. Source: §5.2.1.
- Solvability: 4,574 unique pre-equations; 80% inside the solver fragment; 4% unsolvable for d = 1; 66% solvable for d = 100; failures include bounded functions such as `cos`. Source: §5.2.2.
- Performance (Chrome 49 / Firefox 45, i7 2.6 GHz): Solve < 1 ms median, Eval 5 ms median (12 ms average), Prepare 13 ms median with 6,789 ms maximum, Parse 53 ms median. Source: §5.2.3 table.
- Limitations: no shapes can be added through the GUI; no abstractions are inferred; no updates introduce new control flow; heuristics sometimes choose unintuitive locations (ferris wheel `numSpokes` becomes 0.3); rotation is poorly served by Cartesian drags. Source: §6.1, §6.2, §5.2.2.

UIST 2016, "Semi-Automated SVG Programming via Direct Manipulation" (arXiv:1608.02829v1, 9 August 2016; DOI 10.1145/2984511.2984575 printed on p. 1, UIST 2016 pp. 379-390):

- Draw inserts a definition `(def y ey)` and appends `y` to the `blobs` list when the program has the "simple" structure; otherwise it rewrites to `(let y ey (addShapeToCanvas e y))`. Source: "Tools for Drawing Shapes".
- Relate workflow: Select Features, Dig Hole, Fill Hole, Clean Up. Dig Hole lifts constants contributing to the selected features into variables in the nearest common scope without changing output; Fill Hole is manual; Clean Up inlines and renames. Source: "Tools for Relating Attributes".
- Make Equal = Dig Hole, automatic hole fill that eliminates one degree of freedom (one constant replaced by an expression over the others), then Clean Up; the `n?` annotation hints which constant to eliminate; otherwise the choice is arbitrary. Source: "Make Equal" and "Fill" paragraphs.
- Group rewrites member bounding boxes as percentages of a new group box (`scaleBetween`); Abstract turns a definition into a function over non-frozen named constants; Merge compares definitions modulo constants and abstracts over differing leaves. Source: "Tools for Grouping and Abstracting".
- The solver is "a prototype solver" over value-trace equations from PLDI 2016; the live synchronisation "one-equation, one-constant design" is inherited with its limitations. Source: "Related Work / Live Synchronization" paragraph.
- Implementation more than 13,000 lines of Elm and JavaScript; three worked examples; no timing or user study. Source: "Implementation", "Examples".

UIST 2019, "Sketch-n-Sketch: Output-Directed Programming for SVG" (arXiv:1907.10699v4, 10 August 2019; DOI 10.1145/3332165.3347925 printed on p. 1, UIST 2019 pp. 281-292):

- Provenance tracing: each value is "tagged with the expression being evaluated as well as pointers to the prior (tagged) values"; list elements carry pointers to their containing lists; pattern-match control flow is discarded. Source: "Provenance Tracing".
- Value holes: the intended value is inserted as a leaf, then filled "by inspecting the provenance of the value and choosing an expression that evaluates to the value", usually a variable. Source: Appendix "Value Holes".
- Tool inventory with per-example usage counts (Draw Shape 16, Snap Drawing 15, Rename in Output 15, Make Equal 12, Abstract 9, Draw Offset 9, Group 8, ...). Source: Figure 13.
- Make Equal ranking "prefers changes that rewrite terms near each other and later in the program"; Add Argument enumerates every expression that affected the selected value. Source: "Discussion" paragraphs on Make Equal and Add Argument.
- Repeat by Indexed Merge merges shape expressions into one function of an index `i` and fills holes by sketch-based synthesis over `i`. Source: "Repetition" section and Appendix.
- Evaluation: 16 parametric designs, 427 lines total, "built entirely via output-directed manipulations, without any text editing"; 4 of 15 WWID:PBD benchmark tasks fully completed. Source: "Case Study of ODP Examples", Figure 15.
- Limitations: large numbers of hard-to-distinguish Make Equal candidates; offsets require forethought; sluggish on larger examples due to trace comparison; each program transformation is hand-coded. Source: "Discussion", "Conclusion and Future Work".

## Mechanism

Trace syntax and update problem (PLDI 2016, Figure 2 and §3):

```
t ::= ℓ | (op_m t1 ... tm)            -- data-flow trace of a number n^t
ρ : location → number                  -- substitution (local update)
User changes j of k output numbers:
  hard:  n′_i = t_i   (1 ≤ i ≤ j)
  soft:  n_i  = t_i   (j < i ≤ k)
Faithful ρ:  ρe ⇓ V′(w″) with V′ ∼ V  ⟹  w″_i = w′_i for all i ≤ j
Plausible ρ: ... for some i ≤ j
```

Live synchronisation (PLDI 2016, §4.1):

```
γ(v)(zone)(attr) = ℓ                   -- location chosen before the drag ("fair" rotation)
ComputeTrigger(ρ, γ, v) = λ(dx,dy). ρ ⊕ (ℓx ↦ SolveOne(ρ, ℓx, nx+dx = tx))
                                        ⊕ (ℓy ↦ SolveOne(ρ, ℓy, ny+dy = ty))
SolveOne: univariate, single-occurrence equations, inverted top-down
```

Structural tools (UIST 2016) operate on the syntax tree with provenance only used to find the constants behind a selected feature: Dig Hole (lift constants to variables), Fill Hole (eliminate one degree of freedom), Group (re-parameterise by bounding box), Abstract (definition to function), Merge (anti-unification over constants).

Provenance-directed tools (UIST 2019): values carry `(expression, [parent values])` tags; a UI gesture produces a target value; the tool inserts a value hole and searches the provenance graph for an expression or variable to fill it, or enumerates all contributing expressions for the user to pick (Add Argument).

## NUIF relevance

- **Borrow**: Location-level provenance on numeric literals (PLDI 2016 §2.1) is the minimal provenance record that lets a resolved property be traced to an authored literal; NUIF correspondence records for source literals should carry the same information (file, span, literal value) so that a design-side geometry edit can be lowered to a literal replacement.
- **Borrow**: The hard/soft constraint split (§3) matches NUIF patch preconditions: user-edited properties are hard, all other resolved values are soft and may change only with an explicit fidelity report.
- **Borrow**: Freeze (`!`) and prefer-to-eliminate (`?`) annotations are per-literal editing policies; NUIF should support the same policies in correspondence metadata rather than in source syntax.
- **Adapt**: Dig Hole / Fill Hole / Clean Up (UIST 2016) is a three-phase refactoring that keeps output unchanged until the fill; NUIF's "relate" operations (token binding, constraint creation) should be specified the same way, with a no-op precondition check that lowering the intermediate state reproduces the current resolved layout.
- **Adapt**: UIST 2019 provenance (expression plus parent-value pointers) is richer than NUIF needs for declarative sources; for template languages (Svelte, JSX) NUIF adapters can restrict provenance to static literals and expression spans obtained from a syntax tree (nuif:research:tree-sitter) instead of an instrumented evaluator.
- **Reject**: Silent heuristic disambiguation ("fair" rotation, arbitrary constant elimination) is incompatible with NUIF's requirement that patches be deterministic and conflicts typed; NUIF must return ranked alternatives or a semantic-lowering conflict.
- **Reject**: The one-equation-one-constant solver as the only update mechanism; NUIF constraint layouts require simultaneous solving (nuif:research:cassowary) and stack/flex edits should lower to layout-property operations, not to numeric literal changes.

## Open questions

- The papers give no correctness result comparable to lens laws; which of the UIST 2019 transformations preserve output exactly (Group, Abstract, Merge claim to) and can that be checked by re-evaluation as a conformance oracle?
- Ambiguity statistics (59% of zones ambiguous, 3.83 candidates) were measured on hand-written `little` programs; comparable statistics for real component code are unknown.
- Provenance size and comparison cost caused sluggishness (UIST 2019); NUIF resolved snapshots need a bound on provenance payload per property.
- Whether Repeat by Indexed Merge (synthesising loops from repeated shapes) has an analogue for NUIF component instances with overrides is unexplored.
