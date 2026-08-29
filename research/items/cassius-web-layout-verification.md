---
id: nuif:research:cassius-web-layout-verification
kind: paper
status: reviewed
title: Cassius, VizAssert and Troika - SMT formalisation of CSS layout and machine-checkable visual assertions
source:
  url: https://sandcat.cs.washington.edu/papers/Torlak-cassius-oopsla-2016.pdf
  doi: 10.1145/2983990.2984010
  repository: https://github.com/uwplse/cassius
  authors: [Pavel Panchekha, Emina Torlak, Adam T. Geller, Michael D. Ernst, Zachary Tatlock, Shoaib Kamil]
  published_at: 2016-10-19
  license: ACM (OOPSLA 2016, PLDI 2018); CC BY 4.0 (OOPSLA 2019 article); repository MIT
retrieved_at: 2026-08-29
tags: [css, layout, verification, smt, z3, oracle, accessibility, conformance, formal-semantics]
confidence: 0.9
claims: [nuif:claim:authored-resolved]
relations:
  - type: related_to
    target: nuif:research:css-formatting
    note: Formalises the CSS 2.1 box model, floats, line boxes and margin collapsing; flexbox, grid and tables are out of scope.
  - type: related_to
    target: nuif:research:taffy-and-yoga-browser-generated-tests
    note: Both use a browser as ground truth; Cassius validates a formal model on W3C conformance tests, the engines validate implementations on fixtures.
  - type: related_to
    target: nuif:research:css-flexbox-grid-algorithm-specs
    note: The fragment stops at CSS 2.1; the flex and grid algorithms have no comparable mechanised semantics.
  - type: compares_to
    target: nuif:research:cassowary
    note: Linear real arithmetic over box coordinates, solved by SMT for verification rather than by incremental simplex for layout.
  - type: related_to
    target: nuif:research:accessibility-semantics
    note: VizAssert encodes fourteen accessibility and usability guidelines as layout assertions.
links:
  spec: [spec/00-conformance.md, spec/04-layout.md, spec/13-semantics-accessibility-and-behavior.md]
  adr: [adrs/0002-layout-engine.md]
  rfc: [rfcs/0004-headless-qa-contract.md]
  code: [crates/nuif-layout]
  experiments: [nuif:experiment:layout-differential]
---

# Summary

Cassius (OOPSLA 2016) encodes a fragment of CSS 2.1 as a relation between an element tree, a rule set and a box tree, expressed in quantifier-free linear real arithmetic and solved with Z3. Every box coordinate is a real-valued constant, every layout rule of the standard becomes an equation or inequality over these constants, and the cascade is computed inside the solver. Because any field may be a hole, the same encoding verifies, debugs and synthesises stylesheets. The formalisation was validated against 2075 W3C CSS 2.1 conformance tests with Firefox 41.0.1 as the oracle, agreeing on all but six, all six traceable to Firefox's fixed-point rounding. VizAssert (PLDI 2018) extends the fragment (line height, margin collapsing, full float semantics, positioned layout, media queries) through finitisation techniques, defines a visual logic of universally quantified assertions over boxes with linear arithmetic and ancestor navigation, and checks assertions for all renderings within bounded ranges of window size and font size; on 62 pages and 502 page-assertion pairs it found 64 true violations with 13 false positives and 11 timeouts. Troika (OOPSLA 2019) makes verification modular: a page is decomposed into components with rely/guarantee specifications, well-formedness of the decomposition is a pure-logic check, and component obligations are discharged by per-component tools, giving 13-1469× speed-ups over whole-page verification. NUIF interpretation follows in the relevance section.

## Evidence

OOPSLA 2016, "Automated Reasoning for Web Page Layout" (DOI 10.1145/2983990.2984010, pp. 181-194; PDF from sandcat.cs.washington.edu retrieved 2026-08-29):

- Theory and solver: "theory of quantifier-free linear real arithmetic", Z3; high-level specification written in SMT-LIB2 with quantifiers and grounded per problem. Source: §1, §3.2, §4.
- Layout is a relation on element tree E, rules R and boxes B; box types root, block, inline, line, text, opaque; each box has position, width, height and per-side border widths; any field except text width and height may be a hole (`?`). Source: §3.2, Figure 2.
- Cascade is computed declaratively: `e[p] = r[p]` of the highest-scoring matching rule, else the default. Source: §3.2.
- Block layout distils "36 pages of the CSS standard into just 790 lines"; naive grounding is O(|B|²), rewritten with auxiliary uninterpreted functions to at most one quantifier per rule, giving an encoding linear in |B|. Source: §3.2, §4.1.
- Supported fragment: CSS 2.1 cascade and box model, block and inline boxes, floats, line boxes, margin collapsing, text-align; selectors limited to tag, id and universal; font metrics, line breaking and hyphenation are unmodelled; tables are opaque boxes; four restrictions on float interactions. Source: §3.1, §3.4, Figure 7.
- Conformance: 2075 W3C CSS 2.1 conformance tests within the fragment, oracle Firefox 41.0.1, agreement on all but six, all due to Firefox rounding (1/60 px fixed point, pixel-rounded borders and text); full suite 138 minutes, under 3 s per test. Source: §5.1, Table 1.
- Rejection (mutation) testing: 20,750 mutants, 152 accepted (99.3% rejected); 126 acceptances due to unmodelled font metrics, 26 due to shrink-to-fit non-determinism in CSS 2.1. Source: §5.1.2.
- Case studies on Amazon, Baidu, Google, Wikipedia, Yahoo! (18-45 elements, 35-54 boxes): verification 2-12 s, debugging 1-5 s, synthesis of 25 holes in minutes; unsat cores of 1-5 rules and 1-6 properties. Source: §5.2, Tables 2 and 3.
- Float scalability: pages with 0-13 floats complete "within a few minutes"; without the float restrictions nothing finishes within an hour. Source: §5.3, Figure 11.

PLDI 2018, "Verifying That Web Pages Have Accessible Layout" (DOI 10.1145/3192366.3192407, printed on p. 1; pp. 1-14 of the conference PDF from homes.cs.washington.edu/~mernst, retrieved 2026-08-29):

- Thirteen formalised subsystems (styles, cascade, selectors, box types, layout mode, vertical, clearance, flow width, horizontal, height, floating layout, shrink-to-fit, line height, margin collapse), seven new relative to Cassius; selector matching and cascading moved outside the solver, enabling descendant, child and pseudo-class selectors and em/ex units. Source: §3, Figure 2, §5.
- Finitisation: line height via running baseline accumulators (§4.1); margin collapsing reduced to six reals and one boolean (§4.2); floats via exclusion zones with a register bound (|L|, |R| ≤ 5 "suffices for most web pages", retry with more) and a per-run SMT proof that the float encoding satisfies the nine standard rules (§4.3).
- Visual logic grammar: `assertion ::= ∀ b1,... ∈ B : cond`; conditions over real arithmetic (`b.top`, `b.left`, ..., constants, multiplication by constants only), box navigation (`.parent`, `.first-child`, `.next`, `.ancestor(cond)`), box types (window, inline, line, text, block), selectors (`b ∈ $(sel)`), edge selection (`b.left[margin|border|padding|content]`), colours with gamma; universal quantification only, no recursion. Source: §3, Figure 3.
- Semantics: an assertion is verified "for all rendering parameters in a user-chosen bounded set" or a counterexample is produced consisting of boxes plus concrete window size and font size. Source: §1, §2.
- Fourteen encoded guidelines (Table 1) include minimum text size, 200% resizing, line length ≤ 80 characters, screen-reader-only content off-screen, no horizontal scroll, heading hierarchy, no text overlap, line spacing, contrast, no text over background image, dropdowns hidden, aligned columns, visible link text, minimum button size.
- Conformance: 1006 W3C CSS 2.1 tests for §§8.3, 8.3.1, 9.5-9.5.2, 10.8, 10.8.1; VizAssert passes 915 versus Cassius 271; all 91 failures use unsupported features; five passing tests differ from Firefox where Firefox is documented as incorrect; comparison tolerance one-sixth of a pixel. Source: §5.3, Table 3, footnote 7.
- Evaluation: 62 of the 100 most recent Free Website Templates pages fit the subset; parameter ranges width 1024-1920, height 800-1080, font 16-32 px; 30-minute timeout; Z3 4.5.1. 502 page-assertion pairs: 64 true positives, 13 false positives, 11 timeouts (2.2%); false positives arise from glyph shapes (descenders). Verification times 10-1000 s (CDF), instances of 488k-1052k terms. Source: §5.1-5.2, Table 2, Figure 10.
- Excluded: vertical alignment, right-to-left text, SVG, tables, JavaScript. Source: §6.

OOPSLA 2019, "Modular Verification of Web Page Layout" (DOI 10.1145/3360577, Article 151, pp. 151:1-151:26, CC BY 4.0; PDF from homes.cs.washington.edu/~mernst, retrieved 2026-08-29):

- Component = subtree with holes and a symbolic computed style; a modular layout proof is a decomposition C plus specifications P_c such that `(∧_c P_c) ⇒ Q` (Definition 4.3); well-formedness is "a matter of pure logic" (layout-agnostic) and checked by Z3 in 0.54 s; component specifications are layout-conscious, written as `∧ R_j ⇒ ∧ A_i` (rely/guarantee). Source: §4, §5.
- Tools per component: `admit`, `random-test[n]`, `model-check`, `whole-page`, `component-smt`; random-test and admit are unsound; `component-smt` inherits VizAssert's soundness by removing constraints only. Source: §5.3, §6.1.
- Extensions to the logic: `collapsed-margin`, `non-negative-margins`, `starts-float-free`, `ends-float-free`, `no-floats-enter`, `float-flow-across`. Source: §5.4.
- Case study on a page "11× larger" than prior work: proofs of 36 lines; speed-ups of 13-1469× over VizAssert (Table 1); eight re-proved properties from prior work at 1.9-67× (Table 2); overall 2.6× serial, 4.3× with 8 threads, 13× with caching. Source: §7, Tables 1 and 2.
- Unsupported in component-smt: `transform`, `:before/:after`, tables, flexbox, right-to-left. Source: §2.2, §7.

Repository: github.com/uwplse/cassius, Racket, MIT licence, requires Firefox with Geckodriver, Z3 ≥ 4.5, Racket ≥ 7.0 (README retrieved 2026-08-29). The project site cassius.uwplse.org did not resolve on 2026-08-29.

## Mechanism

Encoding (Cassius §3-4):

```
Inputs:  element tree E, rules R (with holes), boxes B (with holes)
Vars:    for each box b: b.x, b.y, b.w, b.h, border widths  ∈ ℝ
Cascade: e[p] = r[p] for the highest-specificity matching r, else default(p)
Layout:  per box type, equations from CSS 2.1, e.g.
         in-flow block:  b.x = parent.content-left + b.margin-left
Query:   ∃ holes . Layout(E, R, B)                       -- synthesis / debugging
         ¬∃ params . Layout(E, R, B) ∧ ¬P(B)              -- verification
Theory:  QF_LRA, Z3; grounding linear in |B| via uninterpreted helper functions
```

Visual logic (VizAssert §3):

```
assertion ::= ∀ b1 ... bn ∈ B : cond
cond      ::= cond ∧ cond | ¬cond | cond ∨ cond | real ⋈ real | box = box
           | box.type = type | box ∈ $(selector)
real      ::= k | real + real | k × real | box.dir[edge] | color.channel
box       ::= bi | root | null | box.parent | box.first-child | box.next | box.ancestor(cond)
Example:  onscreen(b) := b.right ≥ root.left ∧ b.bottom ≥ root.top
          ∀ b ∈ B : for_screenreader(b) ⇒ ¬onscreen(b)
Checked over: width ∈ [1024,1920], height ∈ [800,1080], font ∈ [16,32]
```

Modular proof (Troika §4):

```
page p, decomposition C = {c1..cn}, specs Pc = (∧ Rj ⇒ ∧ Ai), goal Q
well-formed(C, P, Q)  :⇔  (∧c Pc) ⇒ Q          -- pure logic, no layout
each Pc discharged by admit | random-test | model-check | whole-page | component-smt
```

## NUIF relevance

- **Borrow**: The visual logic's assertion forms (no overlap, on-screen, containment, alignment via equal edges, minimum size, text-fits via line width, contrast) are a ready-made oracle vocabulary over resolved boxes; NUIF conformance fixtures can adopt this grammar and evaluate it concretely on each resolved snapshot without any solver, with symbolic checking as an optional stronger mode.
- **Borrow**: The validation methodology - a formal model checked against a W3C conformance suite with a browser oracle and an explicit tolerance (1/6 px), plus mutation-based rejection testing - is the template for validating NUIF's CSS-family lowering and for nuif:experiment:layout-differential.
- **Borrow**: Rely/guarantee component specifications (Troika) map directly onto NUIF components with declared layout contracts; well-formedness as a pure-logic check is independent of any layout engine.
- **Adapt**: The quantified parameter ranges (window size, font size) correspond to NUIF evaluation contexts; NUIF should express ranges as context predicates and sample them concretely, since symbolic verification over ranges is available only for the CSS 2.1 fragment and costs 10-1000 s per assertion.
- **Adapt**: The fragment excludes flexbox, grid, tables, transforms and text shaping; NUIF's flex and grid families therefore cannot be verified symbolically with this work, and the encoding effort (790 lines for block layout) indicates what a mechanised flex/grid semantics would require.
- **Reject**: A single browser (Firefox) as ground truth for the model; NUIF must record which browser and version served as oracle and treat browser disagreements as separate divergence classes.
- **Reject**: SMT verification as a conformance requirement; the timeouts (2.2%), false positives from glyph shapes (17% of counterexamples) and instance sizes (up to 10^6 terms) make it a research tool rather than a normative test harness.

## Open questions

- Can the visual logic be evaluated concretely over NUIF resolved snapshots with identical semantics, so that the same assertion file serves both the concrete test oracle and a future symbolic checker?
- Which subset of the fourteen accessibility guidelines can be stated purely over NUIF resolved geometry and semantic annotations (spec/13) without CSS-specific box types?
- Is a mechanised semantics of CSS Flexbox §9 in QF_LRA feasible, given that flexible length resolution (§9.7) is an iterative freeze loop and intrinsic sizing is only partially specified (see nuif:research:css-flexbox-grid-algorithm-specs)?
- Troika's component specifications were written by hand over hours; could NUIF derive component layout contracts automatically from authored layout intent (stack/flex constraints) instead?
