---
id: nuif:research:taffy-and-yoga-browser-generated-tests
kind: repository
status: verified
title: Browser-derived layout fixtures in Taffy and Yoga (Chrome as reference oracle)
source:
  url: https://github.com/DioxusLabs/taffy/tree/main/scripts/gentest
  repository: https://github.com/DioxusLabs/taffy; https://github.com/facebook/yoga
  authors: [Taffy contributors, Meta and Yoga contributors]
  published_at: 2026-08-26
  license: MIT (both repositories)
retrieved_at: 2026-08-29
tags: [layout, testing, oracle, flexbox, grid, webdriver, chrome, rounding, conformance]
confidence: 0.95
claims: [nuif:claim:authored-resolved]
relations:
  - type: extends
    target: nuif:research:taffy
    note: Documents the test-generation pipeline (scripts/gentest, test_fixtures, tests/xml) rather than the layout algorithms.
  - type: extends
    target: nuif:research:yoga
    note: Documents gentest (selenium-webdriver, Chrome, three emitters) and its signed generated tests.
  - type: related_to
    target: nuif:research:css-flexbox-grid-algorithm-specs
    note: The fixtures encode browser behaviour, including places where Chrome and the specification diverge.
  - type: related_to
    target: nuif:research:cassius-web-layout-verification
    note: Cassius uses browser output as ground truth for a formal model; Taffy and Yoga use it as unit-test expectations.
  - type: related_to
    target: nuif:research:css-formatting
links:
  spec: [spec/00-conformance.md, spec/04-layout.md]
  adr: [adrs/0002-layout-engine.md]
  rfc: [rfcs/0004-headless-qa-contract.md]
  code: [crates/nuif-layout, crates/nuif-testing/src/layout_differential.rs, conformance/browser-oracle.lock.json, tools/browser/install-chrome-for-testing.sh, xtask/src/main.rs]
  experiments: [nuif:experiment:layout-differential]
---

# Summary

Both Taffy and Yoga treat Chrome as the reference implementation of CSS Flexbox (and, for Taffy, Block and Grid). Each repository keeps HTML fixtures in which the tree structure and inline `style` attributes are the test input; a generator loads each fixture in headless Chrome through WebDriver, reads back the DOM geometry with `getBoundingClientRect()`, and emits unit tests whose expectations are the browser's numbers. Taffy (Rust, `scripts/gentest`, `fantoccini` WebDriver client) downloads a Chrome for Testing build and matching ChromeDriver, measures each fixture under four variants (border-box/content-box × ltr/rtl), records unrounded, naively rounded and "smart" rounded layouts, and writes XML test descriptions that a Rust harness replays with a tolerance of 0.1 px. Yoga (TypeScript, `gentest/src/cli.ts`, `selenium-webdriver`) injects each fixture into a template that sets Yoga's defaults as CSS, measures ltr and rtl trees with edges rounded to integers, and emits C++, Java and JavaScript tests asserting exact equality; generated files are signed and CI regenerates them to detect drift. Neither project uses a numeric tolerance against the browser beyond rounding; known divergences are handled by excluding or flagging fixtures rather than by relaxing assertions. Facts below were verified against the repositories at the commits stated.

## Evidence

Taffy at commit `b3b387132be1dda0e9d08d5044692236532c166d` (2026-08-26, crate version 0.14.0), retrieved 2026-08-29:

- Generator dependencies: `fantoccini = "0.22.0"` (WebDriver client), local crate `getchrome`, `tokio`, `walkdir`, `xmlwriter`, `serde_json`. Source: `scripts/gentest/Cargo.toml`.
- `getchrome` downloads the latest Stable "Chrome for Testing" browser and matching ChromeDriver from `googlechromelabs.github.io/chrome-for-testing/last-known-good-versions-with-downloads.json` into `.chrome-for-testing/<version>`; the Chrome version is not pinned in the repository (`CHANNEL = "Stable"`). Source: `scripts/getchrome/src/lib.rs`, lines 1-30 and `scripts/getchrome/Cargo.toml`.
- Fixture discovery walks `test_fixtures/`, skips `_scratch` directories and any file whose name starts with `x`. Source: `scripts/gentest/src/main.rs`, lines 37-49. At this commit there are 1533 HTML fixtures (block 235, blockflex 11, blockgrid 14, contain 8, flex 674, float 27, grid 543, gridflex 7, leaf 14) of which 17 are `x`-prefixed and excluded; the excluded files carry no explanatory comment (titles are the placeholder "Test description").
- Chrome is launched headless with `--headless --no-sandbox --disable-gpu` and a per-run profile directory; ChromeDriver is started on a free port with 10 s timeouts. Source: `main.rs`, lines 197-284.
- Before measuring, the generator asserts that scrollbars occupy space (15 px) and aborts otherwise; the stylesheet forces `::-webkit-scrollbar { width: 15px; height: 15px }` and the comment states the width must match the test runner. Source: `main.rs`, lines 484-505; `scripts/gentest/test_base_style.css`.
- Each fixture is loaded from a `file://` URL, the load event is awaited, and `getTestData()` is executed; it toggles `body.className` through `border-box ltr`, `content-box ltr`, `border-box rtl`, `content-box rtl` and describes `#test-root` under each. Source: `main.rs`, lines 507-554; `scripts/gentest/test_helper.js`, `getTestData`, lines 1308-1321 of the concatenated listing (function at the end of the file).
- `describeElement` reads input styles from the inline `style` object (`e.style.*`), except `boxSizing` and `direction`, which are read from `getComputedStyle`; grid template strings are passed through verbatim so that line names survive. Source: `test_helper.js`, `describeElement`.
- Three geometry records are captured per element: `unroundedLayout` from `getBoundingClientRect()` with `x`/`y` relative to the parent rectangle; `naivelyRoundedLayout` from `offsetWidth/offsetHeight/offsetLeft + parent.clientLeft`; `smartRoundedLayout` computed as `Math.round(right) - Math.round(left)` and `Math.round(x - parent.x)`. The comment states that Chrome uses a smarter rounding algorithm but does not expose its output, so the script emulates Taffy's algorithm. Source: `test_helper.js`, `describeElement`.
- Text measurement: the Ahem font is embedded as a WOFF2 data URI; the `X` glyph is 10 × 10 px; zero-width spaces are used to control min-content and max-content; `#test-root` sets `font-family: ahem; line-height: 1; font-size: 10px`. Leaf `textContent` is captured to drive measure functions. Source: `test_base_style.css`; `test_helper.js`.
- Opt-out attributes: `data-test-rounding="false"` disables rounding for a fixture (5 fixtures); `data-test-resolved-track-lists="false"` suppresses comparison of resolved grid track lists (26 fixtures), documented in the script as intended for "overlarge grids, where Taffy's MAX_GRID_TRACKS clamp intentionally differs from Chrome's track limit". Source: `test_helper.js`, `describeElement`; counts from `grep` over `test_fixtures/`.
- Expectations use `smartRoundedLayout` when rounding is enabled and `unroundedLayout` otherwise; scroll sizes are recorded only for scroll containers as `scrollWidth - naive clientWidth` floored at zero; resolved grid rows/columns are recorded from computed style. Source: `main.rs`, `generate_assertions`, lines 586-628.
- Output: one XML file per variant under `tests/xml/<family>/<name>__{border_box,content_box}_{ltr,rtl}.xml` (6064 files at this commit, equal to (1533 − 17) × 4) plus a generated `tests/xml/mod.rs` with one `#[test]` per file, gated by `#[cfg(feature = "grid")]` for grid names. Source: `main.rs`, lines 95-152.
- Comparison: the harness constructs a `TaffyTree`, enables or disables rounding from the `use-rounding` attribute, sets available space from `<viewport>`, and compares `x`, `y`, `width`, `height` with `abs() < 0.1`; scroll sizes likewise; resolved track lists compare line names exactly and track sizes with `< 0.1`, with the comment that Chrome and Taffy "format/round subpixel used sizes slightly differently". Source: `tests/xml.rs`, `impl PartialEq for OutputNode` and `track_lists_match`, lines 60-100 and 178-193.
- Freshness gate: CI job "Generated Test Freshness" runs `cargo run -p gentest` and fails if `git status --porcelain -- tests/xml` is non-empty. Source: `.github/workflows/ci.yml`, lines 287-307.
- Policy statements: "Flexbox layouts are tested by validating that layouts written in this crate perform the same as in Chrome"; generated tests must not be edited by hand. Source: `CONTRIBUTING.md`, lines 26-31 (the text still names `tests/generated`, while the current output directory is `tests/xml`).
- Changelog entries record behaviour changes made to track Chrome: content alignment "updated to match the latest spec (and Chrome 123+)" and rounding "fixed ... to follow latest Chrome". Source: `CHANGELOG.md`, lines 449 and 1137-1140.

Yoga at commit `bd8fe0d6d243cc7e0334d4cc68864a994f63beae` (2026-08-27), retrieved 2026-08-29:

- Dependencies: `selenium-webdriver ^4.16.0`, `signedsource ^2.0.0`, `minimist`; scripts `gentest` (runs `src/cli.ts`) and `gentest-validate`. Source: `gentest/package.json`. A serial predecessor, `gentest/gentest-driver.ts` (single driver, LTR/RTL by textual `start`/`end` substitution, expectations read from console logs), is still present but is not referenced by `package.json`; `gentest/gentest.js` and `gentest/gentest.rb` return 404 on the `main` branch.
- Documentation: "Many of Yoga's tests are automatically generated, using HTML fixtures ... rendered in Chrome to generate an expected layout result". Source: `README.md`, lines 19-31.
- Fixtures: 25 HTML files in `gentest/fixtures` (for example `YGAlignItemsTest.html`, `YGRoundingTest.html`, `YGIntrinsicSizeTest.html`, `YGBoxSizingTest.html`, `YGStaticPositionTest.html`). Each top-level `<div id="...">` becomes one test named by its id.
- Template: `gentest/test-template.html` loads Ahem from `gentest/fonts/Ahem.ttf`, sets `body { font: 10px/1 Ahem }`, and gives every `div, span` Yoga's defaults as CSS: `box-sizing: border-box; position: relative; display: flex; flex-direction: column; align-items: stretch; align-content: flex-start; justify-content: flex-start; flex-shrink: 0`; test roots are absolutely positioned. Source: `gentest/test-template.html`.
- Browser: a pool (default 8) of headless Chrome sessions with `--force-device-scale-factor=1 --window-position=0,0 --hide-scrollbars --headless`; the generator waits for `document.fonts.ready` before measuring. Source: `gentest/src/ChromePool.ts`; `gentest/src/cli.ts`, lines 93-104.
- Measurement: `buildLayoutTree` sets `style.direction` to `ltr` then `rtl` on each test root, walks the DOM, and records `width = Math.round(rect.right) - Math.round(rect.left)`, `height` likewise, `left = Math.round(rect.left - parentLeft)`, `top` likewise, the original `style` attribute string, `data-experiments` (space separated) and `data-disabled === 'true'`, and `innerText` for leaves. Source: `gentest/src/buildLayoutTree.ts`, lines 40-77.
- Style mapping: `CssToYoga.ts` parses the inline style string (not computed style), expands `flex: N` to `flex-grow: N; flex-shrink: 1; flex-basis: 0%`, and emits setter calls only for values that differ from Yoga defaults. Source: `gentest/src/CssToYoga.ts`, `parseStyleAttribute`, `expandShorthand`, `applyStyles`.
- Emission: each fixture yields `tests/generated/<Name>.cpp` (GoogleTest, `ASSERT_FLOAT_EQ`), `java/tests/generated/com/facebook/yoga/<Name>.java` (`assertEquals(expected, actual, 0.0f)`), and `javascript/tests/generated/<Name>.test.ts` (`expect(...).toBe(...)`); each test computes layout with `YGNodeCalculateLayout(root, YGUndefined, YGUndefined, YGDirectionLTR)` and asserts left/top/width/height for every node, then repeats for RTL. Source: `gentest/src/emitters/Emitter.ts`, `generateFixture`, lines 126-160; `tests/generated/YGAlignItemsTest.cpp`, lines 16-54.
- Known-bug handling: `data-disabled="true"` emits `GTEST_SKIP();` in C++ and `test.skip` in JavaScript; `data-experiments="Foo"` emits `YGConfigSetExperimentalFeatureEnabled(config, YGExperimentalFeatureFoo, true)`. At this commit no fixture uses either attribute. Source: `gentest/src/emitters/CppEmitter.ts`, lines 91-112; `JavascriptEmitter.ts`, lines 136-150; `grep` over `gentest/fixtures`.
- Rounding: Yoga rounds every node's absolute left/top/right/bottom to the pixel grid with `pointScaleFactor` (default 1.0) after layout, so integer expectations derived from rounded browser rectangle edges are comparable. Source: `yoga/algorithm/PixelGrid.cpp`, `roundLayoutResultsToPixelGrid`, lines 65-136; `yoga/config/Config.h`, line 80; `yoga/algorithm/CalculateLayout.cpp`, line 2938.
- Integrity: generated files carry a `@generated SignedSource<<hash>>` header; `gentest-validate.ts` verifies signatures; CI workflow `validate-tests.yml` runs `yarn gentest-validate` and `yarn gentest -h` and fails when regeneration modifies any test. Source: `gentest/scripts/gentest-validate.ts`; `.github/workflows/validate-tests.yml`, lines 23-33.

## Mechanism

Common pipeline:

```
fixture.html (structure + inline styles)
  → headless Chrome via WebDriver (Ahem font, fixed defaults, scale factor 1)
  → per-element {input styles, getBoundingClientRect geometry}
  → emitter → unit tests in the engine's language
  → engine computes layout → assert per-node x, y, width, height
```

Rounding models differ:

```
Taffy smart rounding (per element, relative to parent rect):
  width  = round(right) − round(left)
  x      = round(x_abs − parent.x_abs)
Taffy comparison: |expected − actual| < 0.1 on x, y, w, h (and scroll sizes, track sizes)

Yoga (browser side): identical edge-rounding formula
Yoga (engine side): roundLayoutResultsToPixelGrid with pointScaleFactor = 1
Yoga comparison: exact equality (ASSERT_FLOAT_EQ / toBe / assertEquals delta 0)
```

Taffy captures both unrounded and rounded layouts so that a fixture can opt out of rounding (`data-test-rounding="false"`) and be compared against unrounded floating-point Chrome values; Yoga has no unrounded mode.

Divergence handling is structural, not numeric: Taffy excludes fixtures by file-name prefix and suppresses particular assertions by attribute; Yoga skips tests or enables experimental features by attribute. Neither project stores the Chrome version used for generation in the generated artefacts; Taffy downloads the current Stable channel at generation time, and Yoga uses whatever Chrome is installed, so regenerating after a Chrome release can change expectations. Both projects rely on CI regeneration to detect such drift.

## NUIF relevance

- **Borrow**: The fixture format (HTML tree with inline styles plus browser-measured expectations under a controlled font and scale factor) is directly reusable for nuif:experiment:layout-differential; NUIF flex/grid fixtures can be lowered to the same HTML and compared against both Chrome and Taffy without new tooling.
- **Adapt**: Taffy's four-variant measurement (box-sizing × direction) is a tested baseline. Its 0.1 px value is only a safety ceiling in NUIF; the executable report derives and stores a smaller value independently for each fixture from the measured Taffy/browser delta.
- **Borrow**: The signed-artefact plus CI-regeneration pattern (Yoga) and the porcelain-status freshness gate (Taffy) are appropriate for NUIF conformance fixtures that are derived rather than authored.
- **Adapt**: NUIF resolved-layout snapshots must record the evaluation context fingerprint, including the reference browser version, since neither project pins it and expectations are known to move with Chrome releases (Taffy changelog, Chrome 123+ alignment change).
- **Adapt**: Rounding must be an explicit part of the NUIF layout family contract; Taffy's separation of unrounded and edge-rounded geometry should be mirrored so that fixtures can assert either.
- **Adapt**: Divergence flags (`x` prefix, `data-test-*`, `data-disabled`) are untyped; NUIF should replace them with the typed fidelity classes of spec/09 (evaluator bug, target semantic difference, schema loss) as required by the layout-differential experiment.
- **Reject**: Treating Chrome as the sole ground truth is not acceptable for a vendor-neutral draft specification; NUIF fixtures should record specification citations and, where browsers disagree (see nuif:research:css-flexbox-grid-algorithm-specs), state which behaviour is normative for the NUIF family.
- **Reject**: Reading input styles from the inline `style` object couples the fixtures to CSS syntax; NUIF fixtures should be authored in the NUIF layout vocabulary and lowered to CSS, not the reverse.

## NUIF executable verification

`cargo xtask gate-c` now applies this method without hand-edited generated expectations. The lock file selects Chrome for Testing 152.0.7977.64 revision 1669021 and Taffy is exactly pinned to 0.14.0. The generator revision is the NUIF source revision recorded in the JSON report. One deterministic seed produces the v0 card at 360, 768 and 1,440 px plus 24 stack/flex/Grid cases; the harness retains all three box maps and compares NUIF/Taffy, NUIF/browser and Taffy/browser.

The 2026-08-30 strict run evaluated 27 cases, 81 engine pairs and 1,536 box components. Eight Grid cases cover positive fixed and zero-minimum `fr` tracks, sparse row/column flow, explicit placement and spanning items. The complete v0, stack, flex and bounded-Grid sets passed with zero classified, blocking or unexplained divergence. Twenty-six fixtures had exact Taffy/browser agreement; one fractional Grid fixture measured 0.015594482421875 px and therefore received the fixture-local 0.02 px assertion bound. Across every case, the maximum NUIF/Taffy delta was 0.00003051757818184342 px and the maximum NUIF/browser delta was 0.015625000000056843 px. The run first exposed `fill` lowering as `auto` under non-stretch Grid alignment; explicit `justify-self`/`align-self: stretch` fixed the foreign lowering without changing the normative evaluator.

This closes the bounded explicit-Grid implementation criterion. It does not
claim the broader CSS Grid surface: intrinsic, percentage, named, repeated,
implicit, subgrid and masonry tracks remain capability-reported exclusions.
Text-dependent Grid layout is also outside this experiment; Gate D separately
pins font and shaping inputs.

## Open questions

- What is the empirical divergence between Chrome, Firefox and WebKit on the Taffy fixture corpus? The corpus is Chrome-only; running it through another browser would quantify how much "CSS-compatible" behaviour is Chrome-specific.
- Should NUIF pin a Chrome for Testing version per fixture generation and store it in the resolved snapshot's context fingerprint?
- Text fixtures rely on Ahem; NUIF text pinning (nuif:experiment:text-pinning) needs an equivalent deterministic font for layout fixtures that include shaped text.
- Taffy's 17 excluded `x` fixtures are undocumented; their content (margins with `start`/`end`, aspect-ratio stretch fills, grid fr spans) indicates areas where Taffy and Chrome disagree and could seed NUIF's divergence catalogue.
