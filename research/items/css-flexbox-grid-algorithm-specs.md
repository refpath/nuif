---
id: nuif:research:css-flexbox-grid-algorithm-specs
kind: standard
status: reviewed
title: CSS Flexbox §9, Grid §12 and Box Sizing 3/4 - algorithm structure, phases and known implementation divergences
source:
  url: https://www.w3.org/TR/css-flexbox-1/
  authors: [W3C CSS Working Group]
  published_at: 2025-10-14
  license: W3C Document License (specifications); csswg-drafts issues under W3C software and document licence
retrieved_at: 2026-08-29
tags: [css, flexbox, grid, intrinsic-size, aspect-ratio, layout-algorithm, interoperability, standard]
confidence: 0.92
claims: [nuif:claim:authored-resolved]
relations:
  - type: extends
    target: nuif:research:css-formatting
    note: Details the normative algorithms and their under-specified regions; css-formatting covers box-tree generation and the split between element and box trees.
  - type: related_to
    target: nuif:research:taffy
    note: Taffy implements these algorithms; its fixtures encode Chrome's behaviour where the specification is incomplete.
  - type: related_to
    target: nuif:research:taffy-and-yoga-browser-generated-tests
  - type: related_to
    target: nuif:research:yoga
  - type: related_to
    target: nuif:research:cassius-web-layout-verification
    note: No mechanised semantics exists for these algorithms; Cassius stops at CSS 2.1.
links:
  spec: [spec/04-layout.md]
  adr: [adrs/0002-layout-engine.md]
  rfc: []
  code: [crates/nuif-layout]
  experiments: [nuif:experiment:layout-differential]
---

# Summary

CSS Flexible Box Layout Level 1 (Candidate Recommendation Draft, 14 October 2025) specifies flex layout as a sixteen-step algorithm across line length determination, main size determination, cross size determination and alignment, with §9.7 "Resolving Flexible Lengths" as an iterative loop that distributes free space proportionally to flex factors, clamps by min/max, and freezes violating items until none remain. CSS Grid Level 2 (CRD, 26 March 2025) specifies grid sizing as track sizing run for columns, then rows, then re-run once per axis if min-content contributions changed; track sizing itself has five phases (initialise, resolve intrinsic, maximise, expand flexible, stretch auto), with a detailed sub-algorithm for distributing extra space across spanned tracks. CSS Box Sizing Level 3 (Working Draft, 17 December 2021) defines min-content, max-content, fit-content and stretch-fit sizes and the treatment of cyclic percentages; Level 4 (WD, 20 May 2021) defines `aspect-ratio`, ratio-dependent and ratio-determining axes, automatic content-based minimums and min/max size transfer. The specifications state that implementations may use any algorithm producing the same results, but several regions are explicitly incomplete or known to diverge from shipping engines: the flex container intrinsic main size algorithm is labelled "not Web-compatible" with a placeholder for a replacement (issue #8884, open); flex intrinsic cross sizing for multi-line column containers is acknowledged as approximate; grid intrinsic sizing "may be updated"; percentage tracks under indefinite sizes changed after all engines had shipped the older behaviour (#1921); engines differ on whether the grid track sizing algorithm runs once or twice for intrinsic container sizes (#2303); the automatic minimum size of flex items with `aspect-ratio` produced Chrome/Firefox differences (#6794); and Box Sizing 3 leaves float sizes and several intrinsic sizes to CSS 2 "and/or existing implementations". NUIF interpretation follows.

## Evidence

CSS Flexible Box Layout Module Level 1, W3C CRD 14 October 2025 (https://www.w3.org/TR/css-flexbox-1/, retrieved 2026-08-29):

- Algorithm preamble: algorithms are "written to optimize readability"; "Implementations may use whatever actual algorithms they wish, but must produce the same results." Source: §9 introduction.
- Structure: §9.1 Initial Setup, §9.2 Line Length Determination, §9.3 Main Size Determination, §9.4 Cross Size Determination, §9.5 Main-Axis Alignment, §9.6 Cross-Axis Alignment, §9.7 Resolving Flexible Lengths, §9.8 Definite and Indefinite Sizes, §9.9 Intrinsic Sizes (§9.9.1 container main sizes with §9.9.1.1 Ideal Algorithm, §9.9.1.2 Web-compatible Intrinsic Sizing Algorithm, §9.9.1.3 Multi-line Min-content Algorithm; §9.9.2 container cross sizes; §9.9.3 item contributions).
- Step 3 flex base size cases A-E (definite flex basis; aspect ratio with definite cross size; min/max-content constraint on container; infinite available main size with parallel inline axis; otherwise size into available space treating content as max-content); min/max are ignored while computing flex base size; hypothetical main size = flex base size clamped by used min/max. Source: §9.2, step 3.
- Automatic minimum size (`min-width: auto`) for non-scroll-container items: larger of content size suggestion and transferred size suggestion, capped by the specified size suggestion (replaced elements: smaller of content and transferred); specified and transferred suggestions are "otherwise undefined" when their preconditions fail. Source: §4.5.
- §9.7 step structure: (1) determine used flex factor (grow if sum of hypothetical outer main sizes is less than inner main size, else shrink), target main size initialised to flex base size; (2) freeze inflexible items (flex factor 0, or base size already beyond hypothetical in the flexing direction); (3) initial free space; (4) loop: (a) exit if all frozen; (b) remaining free space, scaled by the sum of unfrozen flex factors when that sum is below 1; (c) distribute proportionally (grow: by flex grow factor; shrink: by scaled flex shrink factor = shrink factor × inner flex base size); (d) clamp to min/max, recording min and max violations; (e) total violation decides which items to freeze (zero: all; positive: min violators; negative: max violators); (5) used main size = target main size. A note states that at least one item freezes per iteration, guaranteeing termination.
- §9.8: definite main size of container implies definite post-flexing item main sizes; a definite flex basis makes the item's main size definite; a note says "definite" sizes in flex layout can require performing layout so that percentages inside items resolve.
- §9.9.1.1 note: the ideal algorithm "is not Web-compatible" and implementers and the working group "are investigating to what extent" engines can approach it. §9.9.1.2 contains only "Outline Web-compatible algorithm here, once we have one. [Issue #8884]".
- §9.9.2 notes for multi-line column containers: min-content "effectively assumes a single flex line"; the max-content approach is "not a perfect fit in some cases" and a fully correct computation is described as prohibitively expensive.
- Changes since the 2018 CR include: identification of §9.9.1 as ideal and not web-compatible (#8884); fixed flexing rules in §9.9.1 to avoid division by zero (#7189); reformed cross-size intrinsic sizing for column-wrap containers (#6777); aspect-ratio interaction with the automatic minimum via the transferred size suggestion (#6069, #6794); main size definite whenever flex basis is definite (#4311). Source: "Changes" section.

CSS Grid Layout Module Level 2, W3C CRD 26 March 2025 (https://www.w3.org/TR/css-grid-2/, retrieved 2026-08-29):

- §12 outer steps: placement, container size per §5.2 (note: cyclic percentages in track sizes treated as `auto`), grid sizing algorithm (percentages resolved against the resulting container size), item layout with definite grid areas.
- §12.1 Grid Sizing Algorithm: (1) size columns; (2) size rows using column sizes; (3) if any item's min-content contribution changed because of row sizes, re-run column sizing once; (4) likewise re-run row sizing once; (5) align tracks. A note lists the cases that trigger the re-run: column-wrap flex containers, orthogonal flows, multicol, aspect-ratio items.
- §5.2: max-content (min-content) size of a grid container is the sum of track sizes including gutters when sized under the corresponding constraint.
- §12.3 five phases: Initialize Track Sizes (§12.4), Resolve Intrinsic Track Sizes (§12.5), Maximize Tracks (§12.6), Expand Flexible Tracks (§12.7 with §12.7.1 Find the Size of an fr), Expand Stretched auto Tracks (§12.8).
- §12.5 order: baseline shims; span-1 items into intrinsic non-flexible tracks (minimums then maximums, with limited contributions capped by fixed max sizing functions); spanning items by increasing span, not crossing flexible tracks; items crossing flexible tracks all at once (flex-factor-proportional, with the sum-below-1 rule); infinite growth limits set to base size. §12.5.1 distributes extra space with per-track "planned increase" to avoid order dependence, freezes at limits, then distributes "beyond limits" into tracks with intrinsic maximums; "infinitely growable" tracks are those whose growth limit became finite in the intrinsic-maximums step.
- §12.5 closing note: "There is no single way to satisfy intrinsic sizing constraints" and the algorithm "may be updated in the future to take into account more advanced heuristics".
- §12.7.1: hypothetical fr size = leftover space ÷ (sum of flex factors floored at 1); restart treating any track whose factor × fr size is below its base size as inflexible.
- §7.2.1: percentage track sizes are relative to the container's inner size; if the container size depends on its tracks, the percentage "must be treated as auto" for intrinsic sizing and then resolved against the resulting size for layout.

CSS Box Sizing Module Level 3, W3C WD 17 December 2021 (https://www.w3.org/TR/css-sizing-3/, retrieved 2026-08-29):

- Definitions (§2.1): stretch-fit size (available space minus margins, border, padding, floored at zero; "Undefined if the available space is indefinite"); max-content size (size under infinite available space); min-content size (smallest size without avoidable overflow; formally the size under a min-content constraint); fit-content size = `clamp(min-content, stretch-fit, max-content)` when available space is definite, min-content under a min-content constraint, otherwise max-content. `fit-content(x)` = `min(max-content, max(min-content, x))` (§3.2).
- Intrinsic size contribution (§2.2): the outer size a box contributes, auto margins treated as zero.
- §5.2.1 cyclic percentages: non-replaced boxes treat cyclic percentages in `width`, `max-width`, `height`, `max-height` as the initial value for contributions; replaced boxes resolve them against zero for the min-content contribution; minimum sizes, margins, padding and gutters resolve against zero; the note states "These rules specify the previously-undefined behavior" of CSS 2.
- Under-specification: "This specification does not define how to determine the sizes of floats" (§5.1); intrinsic sizes of some boxes are deferred to CSS 2 "and/or existing implementations" (§5.1, §5.2); the UA "may enforce a minimum" on form-control intrinsic sizes and "may additionally floor the min-content contribution" for UI reasons (§5.1, §5.2.1). `stretch` and `fit-content` keywords are deferred to Level 4.

CSS Box Sizing Module Level 4, W3C WD 20 May 2021 (https://www.w3.org/TR/css-sizing-4/, retrieved 2026-08-29):

- §4.1 `aspect-ratio: auto || <ratio>`; `auto` uses the natural ratio of replaced elements, `<ratio>` uses the box selected by `box-sizing`, `auto && <ratio>` uses the content box; degenerate ratios behave as `auto`.
- §4.2 ratio-dependent axis (preferred size depends on the ratio, definite only if inputs are definite) and ratio-determining axis; an inline issue block records that the sizing text may move.
- §4.3 automatic content-based minimum size in the ratio-dependent axis: min-content size capped by the maximum size, for non-replaced, non-scroll-container boxes.
- §4.4 min/max transfer: the definite minimum is transferred first and "capped by any definite preferred or maximum size in the destination axis"; the maximum is then transferred and floored by definite preferred or minimum sizes and by the transferred minimum; definite sizes are never affected.
- Status: exploratory Working Draft; the document instructs implementers to use Level 3 as reference; one section carries the marker "This section might not be written correctly" (issue 6071).

csswg-drafts issues (github.com/w3c/csswg-drafts, retrieved 2026-08-29):

- #8884 (opened 2023-05-30, davidsgrogan): the §9.9.1 intrinsic main size algorithm for single-line row flexboxes is not web-compatible; the spec yields 300 px where existing content requires 100 px; labelled Agenda+ and Needs Edits; open.
- #7189 (2022-03-31, tabatkins): the intrinsic main size algorithm floors flex factors below 1 per item, creating discontinuities; closed, accepted by CSSWG resolution.
- #1147 (2017-03-30, tabatkins): implementations do not match the intrinsic main size algorithm, producing 200 px where the equivalent grid yields 150 px; closed, rejected as invalid.
- #6794 (2021-11-04, davidsgrogan): a flex item with `aspect-ratio` renders at 50 px in Chrome (per spec) and 100 px in Firefox (matching block layout); proposal to use min-intrinsic instead of min-content for the content size suggestion; closed, accepted by CSSWG resolution.
- #2303 (2018-02-12, mrego): Firefox runs the grid track sizing algorithm twice for min-content and max-content container sizes as specified; Blink, WebKit and Edge run it once with zero available space; closed.
- #1921 (2017-10-30, mrego): the specification changed percentage rows to resolve against the intrinsic container size, but all implementations had shipped the older "treated as auto" behaviour; closed, accepted by CSSWG resolution.
- #5566 (2020-10-01, mrego): proposal to resolve percentage row tracks as `auto` and gutters as `0` under indefinite height; Chromium and WebKit follow the spec for both, Firefox follows the proposal for tracks; closed, rejected as wontfix by CSSWG resolution.
- web-platform-tests/interop #139 (2022-09-21): `css/css-sizing/aspect-ratio/flex-aspect-ratio-004.html` "fails in the same identical way across the 3 engines"; open test-change proposal.

## Mechanism

Flex layout phases (Flexbox §9):

```
1  setup: generate flex items
2  available main/cross space
3  per item: flex base size (cases A-E) → hypothetical main size = clamp(base, min, max)
4  container main size
5  collect items into lines (single-line, or by outer hypothetical main size)
6  resolve flexible lengths (§9.7)            → used main sizes
7  hypothetical cross size per item (layout with used main size)
8  line cross sizes (baseline groups, largest outer hypothetical cross size)
9  align-content: stretch lines
10 visibility: collapse handling (strut, re-run)
11 used cross size (stretch → line cross size clamped; else hypothetical)
12 main-axis: auto margins, justify-content
13 cross-axis auto margins
14 align-self
15 container cross size
16 align-content
```

Flexible length resolution (§9.7):

```
factor    = (Σ outer hypothetical main < inner main) ? grow : shrink
target_i  = flex_base_i ; frozen_i = (factor_i == 0) or already beyond hypothetical
free_0    = inner_main − Σ (frozen ? outer target : outer flex base)
loop:
  if all frozen: break
  free = inner_main − Σ ...; if Σ unfrozen factors < 1: free = min(|free|, |free_0 × Σfactors|)
  grow:   target_i = base_i + free × grow_i / Σ grow_unfrozen
  shrink: scaled_i = shrink_i × inner_base_i ; target_i = base_i − |free| × scaled_i / Σ scaled_unfrozen
  clamp targets to [min_i, max_i], content box ≥ 0 ; violation_i = clamped − unclamped
  total = Σ violation_i ; freeze all if 0, min-violators if > 0, max-violators if < 0
used_main_i = target_i
```

Grid sizing (Grid §12):

```
size_columns(); size_rows(cols)
if any min-content contribution changed: size_columns() once more
if any min-content contribution changed: size_rows() once more
track_sizing(axis):
  initialize (base size, growth limit)
  resolve intrinsic (span 1 → increasing spans → flexible-crossing items; planned increases)
  maximize (distribute free space to base sizes up to growth limits)
  expand flexible (fr size = leftover / max(1, Σ flex); restart on under-sized tracks)
  stretch auto tracks
```

Intrinsic sizes (Sizing 3 §2, §3.2; Sizing 4 §4):

```
fit-content       = max(min-content, min(max-content, stretch-fit))   (definite available space)
fit-content(x)    = min(max-content, max(min-content, x))
stretch-fit       = available − margins − border − padding, floored at 0 (undefined if indefinite)
aspect-ratio transfer: min first (capped by dest preferred/max), then max (floored by dest preferred/min and transferred min)
```

## NUIF relevance

- **Borrow**: The phase structure of Flexbox §9 and Grid §12 gives NUIF a stable vocabulary for diagnostics from a CSS-family evaluator (flex base size, hypothetical main size, line cross size, base size, growth limit, fr size); NUIF resolved-layout diagnostics can name the phase in which a value was fixed.
- **Borrow**: Box Sizing 3's definitions of min-content, max-content, fit-content and stretch-fit are the definitions NUIF's shared sizing primitives should reference normatively, including the cyclic-percentage rules.
- **Adapt**: The re-run-once rule of Grid §12.1 and the "performing layout to make sizes definite" note of Flexbox §9.8 mean that a NUIF evaluator contract must allow multi-pass layout; the resolved snapshot should record whether a second pass occurred so differential tests can attribute divergences.
- **Adapt**: NUIF's `flex` family must state which intrinsic main size algorithm it uses for `min-content`/`max-content` containers, since the specification currently has none that is web-compatible (§9.9.1.2, #8884); the practical choice is to follow the reference browser's behaviour and label it `representable` rather than `lossless` relative to the specification.
- **Adapt**: Percentage tracks under indefinite size (#1921, #5566) and one-pass versus two-pass intrinsic grid sizing (#2303) are documented engine divergences; NUIF's differential experiment should include fixtures for each and classify results as "target semantic difference", not evaluator bugs.
- **Reject**: Treating the CRD text as a complete executable specification; the explicit "implementations may use whatever actual algorithms they wish" clause plus the open placeholder sections mean conformance can only be defined against test fixtures plus a named reference implementation.
- **Reject**: Adopting Box Sizing Level 4 `aspect-ratio` text as normative for NUIF's aspect-ratio primitive while the draft carries "might not be written correctly" markers; NUIF should specify its own transfer semantics and cite Level 4 as the intended alignment target.

## Open questions

- Which web-compatible intrinsic main size algorithm will replace §9.9.1.2, and does Taffy's current behaviour (Chrome-derived) match the eventual text?
- How many of Taffy's 17 excluded fixtures and 26 grids without track-list assertions correspond to the issues listed here versus Taffy-specific limits?
- Does any engine implement the Grid §12.1 double re-run exactly, or do all engines approximate it (as #2303 suggests for intrinsic sizes)?
- Should NUIF expose `stretch` and `fit-content` as keywords now (Sizing 4) even though Level 3 defers them?
