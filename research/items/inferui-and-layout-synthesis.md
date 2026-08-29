---
id: nuif:research:inferui-and-layout-synthesis
kind: paper
status: reviewed
title: InferUI robust relational layout synthesis (OOPSLA 2018), with Scout (CHI 2020) and Rewire (CHI 2018)
source:
  url: https://pavol-bielik.github.io/data/papers/bielik18-inferui.pdf
  doi: 10.1145/3276526
  authors: [Pavol Bielik, Marc Fischer, Martin Vechev, Amanda Swearngin, Chenglong Wang, Alannah Oleson, James Fogarty, Amy J. Ko]
  published_at: 2018-10-24
  license: ACM (Proc. ACM Program. Lang. 2, OOPSLA, Article 156; CHI 2020; CHI 2018)
retrieved_at: 2026-08-29
tags: [layout, synthesis, constraints, smt, android, robustness, inference, design-tools]
confidence: 0.88
claims: [nuif:claim:authored-resolved]
relations:
  - type: extends
    target: nuif:research:reverse-layout-inference
    note: InferUI synthesises relational constraints from a single rendered device and enforces robustness on a device list; ReverseORC samples several sizes and fits flexible specifications.
  - type: compares_to
    target: nuif:research:cassowary
    note: Linear constraints over view edges, but solved by Z3 for synthesis rather than by incremental simplex at run time.
  - type: related_to
    target: nuif:research:cassius-web-layout-verification
    note: Both encode layout as linear real arithmetic in Z3; InferUI synthesises constraints, Cassius verifies properties.
  - type: related_to
    target: nuif:research:ui-code-generation-boundaries
  - type: related_to
    target: nuif:research:figma
    note: Scout targets designer workflows in vector design tools; its high-level constraints resemble auto-layout intent.
links:
  spec: [spec/04-layout.md, spec/09-provenance-and-fidelity.md]
  adr: []
  rfc: [rfcs/0003-authored-resolved-provenance.md]
  code: []
  experiments: [nuif:experiment:layout-inference]
---

# Summary

InferUI takes a set of views with absolute positions on one device (the "input specification") and synthesises an Android ConstraintLayout program: one horizontal and one vertical constraint per view, drawn from 26 constraint types (relative alignment, baseline, circular, fixed-size centring with bias, and dynamic-size centring), plus a size mode per axis. The rendering semantics of ConstraintLayout is written as linear equations, and synthesis is a satisfiability query in Z3 over booleans, integers and reals. Because a single device underdetermines the layout, the paper adds (i) robustness: the candidate layout is rendered symbolically on a list of additional devices and six properties (order, margin, centring, aspect-ratio preservation, pixel-perfectness, inside-screen) must hold, and (ii) a probabilistic model trained on ConstraintLayout files from top-500 GitHub and Google Play applications that scores candidate constraints and turns the query into maximum-score satisfiability with top-K candidate pruning. Single-device synthesis is always exact by construction; on held-out device sizes 86.5% (GitHub) and 92.3% (Play Store) of views generalise, 62% of synthesised constraints match the developer's, and synthetic user feedback resolves the rest with 0/1/2/3+ corrections in 63/25/8/4% of cases. Unguided synthesis times out beyond about ten views; guided multi-device synthesis succeeds on 98.7% of 2-3-view layouts and 41.7% of 16-19-view layouts. Scout (CHI 2020) addresses a different task, generating many layout alternatives from designer-authored high-level constraints (grouping, order, emphasis, alternates, repeats) compiled to low-level Z3 constraints with branch-and-bound enumeration. Rewire (CHI 2018) infers editable vector objects from screenshots; only its abstract was retrieved. NUIF interpretation follows.

## Evidence

InferUI, "Robust Relational Layout Synthesis from Examples for Android" (DOI 10.1145/3276526, Article 156, 29 pages; author PDF retrieved 2026-08-29):

- Input: N views with absolute positions (top-left and bottom-right points) inside a content frame ρ; output: N sizes and N horizontal plus N vertical constraints such that `v ⊨ ψ_layout(ρ, c_h, c_v, s)`. Source: §3 "Input Specification", §5 "Problem Statement".
- Target: Android ConstraintLayout only (version 1.0.2 for cross-checking). Source: §1, §8.
- View representation: five handle points `⟨x_L, x_R, y_T, y_B, y_baseline⟩`; constraint tuple `⟨type, A, B, C, m_L, m_R, bias, α, r⟩`. Source: §4, Figure 5.
- Constraint classes (Table 1): relative positioning ℛ_LL/ℛ_LR/ℛ_RL/ℛ_RR (and top/bottom analogues), baseline ℛ_B, circular ℛ_C (angle and distance), fixed-size centring ℱ_* with bias, dynamic-size centring 𝒟_* (view spans between two anchors); 26 types in total; size mode Fixed or MatchConstraint, the latter exactly when a 𝒟 constraint is used. Source: §4, Table 1, §5.
- A one-pixel Android solver bug is modelled explicitly for ℱ_LR/ℱ_RL against the content frame. Source: §4, discussion of Table 1.
- Rendering semantics `ψ_layout = φ_position ∧ φ_size ∧ φ_constraints` as linear equations. Source: Figure 7, Figure 8.
- Single-device synthesis formula `ψ_single_syn` with guards `g_i^k ⇒ ⟦c_i^k⟧`, exactly-one guard per view per axis (Z3 `PbEq`), acyclicity via integer distance variables; solver Z3 4.6.0, one-minute timeout. Source: §5, Figure 9, footnote 2, §8.
- Multi-device: `ψ_multi_syn = ψ_single_syn(ρ, v) ∧ ⋀_k ψ_gen(d_k, v, c, s)`; devices are an input list (or a maximum resize ratio); "input specification still consists of absolute view positions v only for a single device ρ". Source: §6, Figure 10.
- Robustness properties (§6.1): `φ_preserve_order` (pairwise handle ordering), `φ_preserve_margins` (distances in a set of common values such as 16 and multiples of 8), `φ_preserve_centering`, `φ_preserve_aspect_ratio` (only for ratios in {16/9, 3/2, 4/3, 1/1, 3/4, 2/3}), `φ_pixel_perfect` (non-negative integer handles), `φ_inside_screen`.
- Probabilistic model: `P(c, ρ, v) = (1/Z) ∏_k P_fk(c | f_k(c, v))^{w_k}` with MLE and additive smoothing; features margins, bias, distance, size (16 px buckets), orientation, type, intersection count, plus a regulariser on distinct constants and views; trained in under a second on all developer-written constraints. Source: §7, Table 2, Equation 1, §8.
- Guided search: top K = 5 candidates per view; on UNSAT add 10 more per view from the unsat core; top-5 sufficient in 69% of cases. Objective is maximum Σ score (Figure 12). Source: §7, §8.
- Results (Table 5, views generalising on held-out devices 341×518 to 384×640 dp after synthesis at 360×640 dp): GitHub 12.6% (single), 69.4% (single + guided), 86.5% (multi + guided); Play Store 12.9%, 75.5%, 92.3%. Vertical generalisation is higher than horizontal in every configuration. Source: §8.2.
- Developer-constraint match 62%; max-sat versus sat improves view generalisation by 35% and constraint match by 20% for single + guided. Source: §8.3.
- Scalability (Table 4): unguided `ψ_syn` times out in 87.2% of cases beyond about ten views; `ψ_multi_syn` unguided works only below four views; `ψ_multi_syn + guided` succeeds for 98.7% of layouts with 2-3 views and 41.7% with 16-19 views; runtimes 44 ms to 3 s. Source: §8.1.
- UNSAT causes: views cannot fit a smaller screen; robustness properties "too restrictive" when views are "centered simply by chance". Source: §8.1.
- Property violations in synthesised layouts without robustness (Table 6): for example `¬φ_inside_screen` in 86% (single) versus 52.3% (single + guided) of cases; violations also found in real applications. Source: §8.2.
- Feedback: user moves or resizes rendered views; changed views become additional input; evaluated with a synthetic user derived from developer constraints: 0/1/2/3+ rounds in 63/25/8/4% of cases; overall multi-device generalisation 89%. Source: §6.2, §8.4.
- Limitations stated: returns a single most likely layout; feedback study synthetic; no threats-to-validity section. Source: §8.4, §9-10.

Scout, "Rapid Exploration of Interface Layout Alternatives through High-Level Design Constraints" (DOI 10.1145/3313831.3376593; arXiv:2001.05424v1 retrieved 2026-08-29):

- High-level constraints: grouping, order (important/unimportant, first/last), emphasis (low/normal/high), alternate groups, repeat groups, Keep/Prevent feedback. Design variables: layout grid (margin, columns 2-4, gutter, column width), baseline grid, per-group alignment (six values), arrangement (horizontal, vertical, balanced rows/columns), padding, per-element x, y and precomputed size triples in 4 px steps. Source: §"Scout System", Table 1.
- Compilation: groups to alignment/arrangement/padding plus visual-hierarchy inequalities; order to ordering or bounding-box constraints; emphasis to size and relative-size/area constraints; repeats to equal arrangement across subgroups; every layout also satisfies in-bounds, pairwise non-overlap and 48×48 minimum touch targets, citing InferUI's robustness properties. Source: §"Constraint Solving".
- Solver: Z3 inside a modified branch-and-bound that assigns one variable at a time, backtracks on infeasibility, randomises assignment order and adds a blocking clause after each layout; size triples precomputed because Z3 "does not efficiently compute multiplication constraints". Source: §"Constraint Solving".
- Throughput: 20 solver threads, typically 15 layouts of 9 elements per request in under 5 s (Ryzen 7 1800X). Source: §"Implementation".
- Quality model: per-group size, balance and alignment scores, area-weighted with density, adapted from Riegler and Holzmann. Source: §"Quality Model".
- Study: 18 designers, within-subjects against Adobe XD; Scout layouts 12% more spatially diverse (p < 0.027), +35% for non-professionals, expert-rated quality not significantly different (5.37 vs 5.73). Source: §"Evaluation", Tables 1-2.

Rewire, "Interface Design Assistance from Examples" (DOI 10.1145/3173574.3174078, CHI 2018 pp. 1-12; DOI verified via Crossref 2026-08-29; only the abstract was retrieved): the abstract states that Rewire "automatically infers a vector representation of screenshots where each UI component is a separate object with editable shape and style properties". No mechanism or numbers are recorded here.

## Mechanism

InferUI (§4-7):

```
View v     = ⟨x_L, x_R, y_T, y_B, y_baseline⟩
Constraint = ⟨t ∈ 𝒞 (26 types), A, B, C ∈ View, m_L, m_R ∈ ℤ≥0, bias ∈ [0,1], α, r⟩
Size       = ⟨t_h, t_v ∈ {Fixed, MatchConstraint}, width, height⟩

ψ_layout(ρ, c_h, c_v, s) = φ_position ∧ φ_size ∧ φ_constraints        (linear)
ψ_single_syn = ψ_layout ∧ φ_valid ∧ φ_acyclic ∧ ⋀_i (Σ_k g_i^k = 1) ∧ ⋀_{i,k} (g_i^k ⇒ ⟦c_i^k⟧)
ψ_multi_syn  = ψ_single_syn(ρ, v) ∧ ⋀_{d ∈ devices} ( ψ_layout_syn(d, v_d, c, s) ∧ φ_robust(v, v_d) )
φ_robust     = φ_order ∧ φ_margins ∧ φ_centering ∧ φ_aspect_ratio ∧ φ_pixel_perfect ∧ φ_inside_screen
Objective    = max Σ_i score_i,  score_i = P(c_i^k, v)         (max-sat over guards)
Search       = top-K (K=5) candidates per view; on UNSAT add 10 from unsat core; repeat
P(c, ρ, v)   = (1/Z) ∏_k P_fk(c | f_k(c, v))^{w_k}
```

Scout:

```
high-level constraints (group, order, emphasis, alternate, repeat, keep/prevent)
  → low-level linear constraints over x, y, size triples, grid variables
  → branch-and-bound over variables with Z3 feasibility checks + blocking clauses
  → N diverse layouts, ranked by quality model (size, balance, alignment, density)
```

## NUIF relevance

- **Borrow**: The six robustness properties are a concrete, checkable definition of "the inferred layout generalises"; nuif:experiment:layout-inference should rank candidate stack/flex/grid/constraint reconstructions by these properties on held-out viewport widths.
- **Borrow**: Scoring inferred constraints with a probability (`P(c, v)`) and recording it is exactly the "inference confidence" that NUIF provenance must retain for reconstructed intent; the 62% developer-match figure shows why such confidence must never be reported as lossless.
- **Borrow**: Treating device sizes as an explicit input list of evaluation contexts, with the synthesised layout rendered symbolically on each, matches NUIF's context-keyed resolved snapshots.
- **Adapt**: The 26 ConstraintLayout types map onto NUIF's `constraint` family (edge equalities with margins, centring with bias, size modes) but NUIF must keep them as portable relations with identities and strengths rather than Android attribute names.
- **Adapt**: Scout's high-level constraints (grouping, order, emphasis, repeat) are close to authored intent in NUIF's `stack` family; Scout's compilation shows how intent can be lowered to linear constraints when a `constraint` evaluator is the target.
- **Adapt**: The single-device input assumption should be replaced by multi-context observations as in nuif:research:reverse-layout-inference; InferUI's own data show generalisation rising from 12.6% to 86.5% only when extra devices constrain the search.
- **Reject**: Exactly one constraint per view per axis; NUIF constraint layouts require multiple simultaneous relations (min/max, aspect ratio, distribution) and the restriction is an Android encoding choice.
- **Reject**: The Android-trained probabilistic prior as a NUIF default; the feature set (16 px margins, multiples of 8) encodes platform conventions and must remain a pluggable adapter heuristic.

## Open questions

- InferUI's robustness properties are stated for absolute-position views; which of them survive translation to flex/grid families where order and centring are structural rather than numeric?
- The paper's device range is narrow (341-384 dp width); generalisation to responsive breakpoints (360, 768, 1440 in nuif:experiment:v0-responsive-card) is untested.
- Scout enumerates alternatives; NUIF import needs a single ranked result with alternatives retained as provenance - how many candidates are worth storing?
- Rewire's screenshot-to-vector inference was not examined in detail; whether its component segmentation can seed NUIF entity identity for raster imports remains open.
