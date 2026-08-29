---
id: nuif:research:differential-testing
kind: synthesis
status: reviewed
title: Differential testing (McKeeman, Csmith) and browser-referenced layout oracles (Taffy and Yoga gentest, R2Z2, X-PERT, WPT)
source:
  url: https://users.cs.utah.edu/~regehr/papers/pldi11-preprint.pdf
  doi: 10.1145/1993498.1993532
  repository: https://github.com/DioxusLabs/taffy
  authors: [William M. McKeeman, Xuejun Yang, Yang Chen, Eric Eide, John Regehr, Taffy contributors, Yoga contributors, Suhwan Song, Ali Mesbah, Shauvik Roy Choudhary]
  published_at: "2011-06-04"
  license: McKeeman DTJ article Digital Equipment Corporation; Csmith paper ACM copyrighted; Taffy MIT; Yoga MIT; Playwright Apache-2.0
retrieved_at: 2026-08-29
tags: [testing, differential-testing, oracle, layout, browser, taffy, yoga, playwright, tolerance, csmith]
confidence: 0.92
claims: [nuif:claim:authored-resolved, nuif:claim:semantic-automation]
relations:
  - type: extends
    target: nuif:research:taffy
    note: Documents how Taffy derives its fixture tests from Chrome and the 0.1 tolerance it applies.
  - type: extends
    target: nuif:research:yoga
    note: Documents Yoga's Selenium-driven gentest and its exact-equality assertions.
  - type: related_to
    target: nuif:research:metamorphic-testing-graphics
    note: Self-consistency oracles are metamorphic relations; differential oracles need an alternative implementation.
  - type: depends_on
    target: nuif:research:fuzzing-structured-inputs
    note: Csmith shows that the generator must produce inputs with a single defined meaning for differential comparison to be sound.
  - type: depends_on
    target: nuif:research:delta-debugging-and-test-case-reduction
    note: McKeeman and Csmith both require reduction before a divergence is reportable.
  - type: related_to
    target: nuif:research:css-formatting
    note: Divergence classification depends on which CSS formatting semantics NUIF families claim to match.
  - type: related_to
    target: nuif:research:reverse-layout-inference
    note: Browser boxes are the ground truth for both differential layout tests and layout inference.
  - type: related_to
    target: nuif:research:taffy-and-yoga-browser-generated-tests
    note: Companion record on the browser-generated fixture pipelines; this record adds the oracle and divergence taxonomy.
links:
  spec: [spec/00-conformance.md, spec/04-layout.md, spec/09-provenance-and-fidelity.md, spec/12-cli-api-and-automation.md]
  adr: []
  rfc: [rfcs/0004-headless-qa-contract.md]
  code: [crates/nuif-layout, crates/nuif-render, crates/nuif-cli]
  experiments: [conformance/PLAN.md, conformance/fixtures/v0-responsive-card/README.md]
---

# Summary

Differential testing feeds one input to several comparable implementations and treats disagreement, crashes or hangs as bug candidates (McKeeman 1998). Csmith (Yang et al., PLDI 2011) is the canonical generator for this oracle: every generated program has a single defined meaning, the observable is a checksum, and voting across compilers identifies the minority; 325 compiler bugs were reported. Layout engines use the same oracle with a browser as the reference implementation: Taffy's gentest drives headless Chrome through WebDriver, reads `getBoundingClientRect`, writes XML fixtures and compares with a 0.1 px tolerance; Yoga's gentest uses Selenium and emits exact-equality C++, Java and TypeScript tests. Browser-to-browser work (Mesbah and Prasad 2011, X-PERT 2013, R2Z2 2022) supplies divergence classifications and filters for benign differences. WPT reftests define the fuzzy-match syntax for rendered images.

For NUIF the browser is an alternative implementation for the CSS-compatible subset of the `flex`, `grid` and `stack` families; the NUIF evaluator (initially Taffy) is the system under test; canonical hashes and metamorphic relations give self-consistency oracles where no browser semantics exist.

## Evidence

- Definition: "If a single test is fed to several comparable programs ... and one program gives a different result, a bug may have been exposed"; differential testing trades "many computer cycles instead of human effort". McKeeman, "Differential Testing for Software", Digital Technical Journal 10(1), 1998, pp. 100–107, Abstract and p. 101 (PDF https://www.cs.tufts.edu/comp/150FP/archive/bill-mckeeman/DifferentailTesting.pdf, retrieved 2026-08-29).
- Test quality levels for C: ASCII characters, tokens, syntactically correct, type-correct, statically conforming, dynamically conforming, model-conforming; results become interesting from level 4. Same paper, pp. 102–103.
- Outcome classification: results are filed as crash, loop, abend (some but not all terminate abnormally) and diff (all complete, outputs differ); tests where a comparison compiler crashes are discarded. Same paper, p. 105.
- Reduction applies 23 heuristic transformations to a fixpoint, often requiring more than 10,000 compilations. Same paper, p. 105.
- Csmith: randomised differential testing "has the advantage that no oracle for test results is needed"; with three or more implementations "a tester can use voting to heuristically determine which implementations are wrong". Yang, Chen, Eide, Regehr, PLDI 2011, DOI 10.1145/1993498.1993532, §2.1 and Fig. 2 (preprint https://users.cs.utah.edu/~regehr/papers/pldi11-preprint.pdf, retrieved 2026-08-29).
- Design goal: every program "must be well formed and have a single meaning according to the C standard"; the observable is a checksum of non-pointer globals; C99's 191 undefined and 52 unspecified behaviours are avoided structurally or by checks; implementation-defined behaviour is allowed, so comparison is valid within an equivalence class of compilers only. Same paper, §2.2, §2.6.
- Bug classes: compile-time crash, wrong-code (wrong result, crash, wrong termination), and "silent wrong-code error" without any warning. 325 bugs reported to 11 teams (79 GCC, 202 LLVM); no interesting split vote was ever observed. Same paper, §2.6, §3.1–3.2.
- Delta-debugging variants for C "introduce undefined behavior" and produce small but useless programs, so validity checkers are needed during reduction. Same paper, §3.7.
- Taffy gentest (`main`, retrieved 2026-08-29): `scripts/gentest/Cargo.toml` depends on `fantoccini = "0.22.0"` (WebDriver) and a local `getchrome` crate; `scripts/gentest/src/main.rs` launches Chrome with `--headless --no-sandbox --disable-gpu`, loads each `test_fixtures/**/*.html` via `file://`, calls `client.execute("return getTestData()")`, and writes XML to `tests/xml/`. CONTRIBUTING.md: layouts are tested "by validating that layouts written in this crate perform the same as in Chrome"; `just gentest` downloads matching Chrome for Testing and ChromeDriver builds; fixtures starting with `x` are disabled. Note: CONTRIBUTING still mentions `tests/generated`, but the current emitter writes XML.
- Taffy fixture conventions: root element `id="test-root"`; `scripts/gentest/test_base_style.css` embeds Ahem as a data URI, sets `#test-root { font-family: ahem; line-height: 1; font-size: 10px; }`, `box-sizing: border-box`, fixed 15 px scrollbars; `test_helper.js` reads `getBoundingClientRect()` relative to the parent, offers unrounded and "smartRounded" (`Math.round(right) - Math.round(left)`) values controlled by `data-test-rounding`, and emits four trees (border-box/content-box × ltr/rtl). Files retrieved 2026-08-29.
- Taffy comparison: `tests/xml.rs` implements `PartialEq` for output nodes with `(expected - actual).abs() < 0.1` on x, y, width, height, scroll dimensions and grid tracks; the `use-rounding` attribute toggles `enable_rounding`/`disable_rounding`; `tests/xml/flex/` holds about 2,656 files such as `absolute_layout_width_height_start_top__border_box_ltr.xml` with `<viewport width="max-content" height="max-content"/>`. Retrieved 2026-08-29.
- Yoga gentest (`main`, retrieved 2026-08-29): `gentest/gentest-driver.ts` uses `selenium-webdriver` with `--force-device-scale-factor=1 --window-position=0,0 --hide-scrollbars` (`ChromePool.ts` adds `--headless`), loads `test-template.html` (Ahem via `@font-face`, `font: 10px/1 Ahem`, every element `display: flex; flex-direction: column; align-items: stretch`), and reads results from console lines prefixed `gentest-log:`. `src/buildLayoutTree.ts` uses `getBoundingClientRect()` rounded as `Math.round(right) - Math.round(left)`. Emitters write `tests/generated/*.cpp` (`ASSERT_FLOAT_EQ`), `java/tests/generated/**/*.java` (`assertEquals(..., 0.0f)`) and `javascript/tests/generated/*.test.ts` (`toBe`): exact equality. `gentest/gentest.js` and `gentest/README.md` no longer exist.
- Playwright: `locator.boundingBox()` returns `{x, y, width, height}` relative to the main-frame viewport or null if not visible; `page.evaluate()` returns the function result including `-0`, `NaN` and infinities; `browser.newContext()` sets `deviceScaleFactor` (default 1), `viewport` (default 1280×720), `reducedMotion`, `colorScheme`, `locale`, `timezoneId`; screenshots wait for `document.fonts.ready` unless `PW_TEST_SCREENSHOT_NO_FONTS_READY` is set (`packages/playwright-core/src/server/screenshotter.ts`, `main`). https://playwright.dev/docs/api/class-locator, class-page, class-browser, retrieved 2026-08-29.
- Playwright `toHaveScreenshot`: `threshold` 0.2 (YIQ perceived colour difference), `maxDiffPixels`, `maxDiffPixelRatio`, `animations: "disabled"`, `caret: "hide"`, `scale: "css"`, `mask`; comparison uses pixelmatch. https://playwright.dev/docs/api/class-pageassertions#page-assertions-to-have-screenshot-1 and https://playwright.dev/docs/test-snapshots, retrieved 2026-08-29.
- R2Z2: cross-version differential fuzzing of Chrome with a Domato-derived grammar; screenshots compared by 4,096-bit pHash Hamming distance with threshold 140; bisection finds the culprit commit; an interoperability oracle treats Firefox agreement as correctness (bug only when old Chrome equals Firefox and new Chrome differs); a non-feature-update oracle excludes commits that add WPT tests; stage analysis compares DOM, style, layout ("same size and location" per node) and paint records. 22,629 candidates yielded 13 confirmed regressions, 11 new. Song et al., ICSE 2022, DOI 10.1145/3510003.3510044, §4–§6 (PDF https://lifeasageek.github.io/papers/suhwan-r2z2.pdf, retrieved 2026-08-29).
- Mesbah and Prasad: cross-browser oracle at trace level (state-graph isomorphism) then screen level (DOM diff via XMLUnit ignoring case, whitespace, attribute order, text values, plus configurable ignore patterns); screen-level false positives ranged from 12% to 37%. ICSE 2011, §4.3, §5, §6 (PDF https://www.cs.columbia.edu/~junfeng/12fa-e6121/papers/browser-compat.pdf, retrieved 2026-08-29).
- X-PERT classifies cross-browser issues as structure, content (text, visual) and behaviour; structure is compared with an alignment graph of contains and sibling relations (left-align, above, leftOf) because users notice relative position rather than absolute size; visual content uses χ² colour histograms on leaf elements; 98 true issues at 76% precision. Roy Choudhary, Prasad, Orso, ICSE 2013, §IV, §VI, §VII (PDF http://shauvik.com/public/pubs/roychoudhary13icse_cr.pdf, retrieved 2026-08-29).
- WPT reftests pass only if test and reference render "pixel-for-pixel identically within a 800x600 window"; fuzzy syntax `<meta name=fuzzy content="maxDifference=15;totalPixels=300">`, ranges `10-15;200-300` inclusive, per-reference prefix `option1-ref.html:...`; screenshots are taken after load, web fonts and pending paints. https://web-platform-tests.org/writing-tests/reftests.html, retrieved 2026-08-29.

## Mechanism

```
differential_run(fixture, context, impls, policy):
    outcomes = { impl: run(impl, fixture, context) for impl in impls }   # boxes or image + status
    if any(o.status in {crash, timeout}): return classify_abnormal(outcomes)
    ref = policy.reference or majority(outcomes)                        # Csmith voting
    for impl, o in outcomes:
        d = compare(o, ref, policy.tolerance)                            # per-box abs diff or perceptual
        record(impl, d.kind, d.max_delta, d.entities)
    return divergences filtered by policy.known_gaps                     # feature-gap filter
```

Oracle classes (attributed):

1. Reference implementation: a browser is ground truth for CSS semantics (Taffy, Yoga gentest; X-PERT uses one browser as reference).
2. Alternative implementation, N-version: several engines with no designated truth (McKeeman; Mesbah and Prasad; R2Z2 change detector across versions).
3. Majority vote: with three or more implementations the minority is suspect (Csmith §2.1).
4. Interoperability consensus: agreement of independent engines is taken as correct; a change that breaks agreement is a regression (R2Z2 §4.3.1).
5. Self-consistency: the same engine must agree with itself across equivalent encodings (Taffy's border-box/content-box × ltr/rtl variants; WPT reftests; metamorphic relations in nuif:research:metamorphic-testing-graphics).

Tolerance policy (attributed):

- Boxes: absolute difference below 0.1 px after optional rounding (Taffy `tests/xml.rs`); exact after `Math.round` (Yoga). Sub-pixel rounding must be part of the fixture contract (`data-test-rounding`, `use-rounding`).
- Images: per-channel `maxDifference` and `totalPixels` budgets (WPT), YIQ or OKLab threshold plus `maxDiffPixels` (Playwright, pixelmatch), pHash distance for coarse triage (R2Z2).
- Structure: relative alignment relations instead of absolute coordinates when the target is a different engine with its own rounding (X-PERT).

Divergence classification (attributed):

- Abnormal: crash, hang, abend in some implementations (McKeeman; Csmith compile-time crash and timeouts).
- Structural mismatch: tree or alignment-graph differences (Mesbah; X-PERT; R2Z2 DOM stage).
- Numeric within tolerance: accepted and recorded with the observed maximum delta.
- Numeric beyond tolerance: bug candidate, reduced before reporting (McKeeman p. 105; Csmith §3.7).
- Feature gap: the reference implements semantics the system under test does not claim, or vice versa; filtered by known-gap lists (R2Z2 non-feature oracle; Csmith equivalence classes; Mesbah ignore patterns; Taffy `x`-prefixed fixtures).
- Silent wrong output: plausible boxes with no diagnostic; the analogue of Csmith's silent wrong-code error and the class NUIF's fidelity reports must make impossible.

## NUIF relevance

**Borrow**

- The Taffy gentest pipeline (WebDriver-driven Chrome, Ahem, `#test-root`, four box-model/direction variants, XML fixtures, 0.1 px tolerance) as the template for NUIF's browser-referenced `layout` fixtures; Playwright can replace fantoccini with the same `getBoundingClientRect` extraction and `document.fonts.ready` wait.
- Csmith's rule that generated inputs must have a single defined meaning: the NUIF document generator must avoid authored constructs whose lowering to CSS is `approximated` or `unsupported`, or the comparison is not sound (Csmith §2.2; `spec/04-layout.md` fidelity records).
- McKeeman's outcome buckets (crash, loop, abend, diff) and Csmith's silent-wrong-output class as the top level of the divergence taxonomy in the machine-readable report.
- WPT fuzzy syntax as the declared per-fixture tolerance format for render comparisons.

**Adapt**

- The browser is a reference only for CSS-compatible families; for `freeform`, `constraint` and `custom` families the oracle must be self-consistency or a second NUIF implementation, so the report must record the oracle class per fixture.
- Round-trip differential testing compares NUIF → HTML/CSS export → browser boxes against NUIF resolved boxes, which tests the adapter and the evaluator jointly; disagreement must be attributed to a pipeline stage as R2Z2 does (DOM, style, layout, paint).
- Rounding: NUIF should compare unrounded boxes with an epsilon and treat device-pixel snapping as a separate, declared step rather than adopting Yoga's exact-after-rounding policy.

**Reject**

- Majority voting across engines is not applicable while only one NUIF evaluator exists; voting becomes relevant once a second independent implementation exists (roadmap governance item).
- Yoga's zero-tolerance assertions, because NUIF resolved geometry is f64 and browsers expose rounded layout.

## Open questions

- Which Chrome version should be pinned as reference, and how are reference-side regressions distinguished from NUIF regressions without a second browser (R2Z2's interoperability oracle suggests adding Firefox or WebKit)?
- How should the feature-gap filter be derived automatically from the `FidelityReport` of the export so that only `lossless` or `representable` entities are compared?
- Is the 0.1 px tolerance adequate for percentage and `fit-content` sizing at 1440 px viewports, or should tolerance scale with box size?
