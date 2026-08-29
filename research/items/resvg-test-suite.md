---
id: nuif:research:resvg-test-suite
kind: repository
status: reviewed
title: resvg regression suite, resvg-test-suite reference corpus, usvg lowering and the SVG support table
source:
  url: https://github.com/linebender/resvg
  repository: https://github.com/linebender/resvg-test-suite
  authors: [Yevhenii Reizner (RazrFalcon), Linebender contributors]
  published_at: "2026-08-02 (resvg v0.48.1)"
  license: "resvg/usvg: Apache-2.0 OR MIT; resvg-test-suite: MIT"
retrieved_at: 2026-08-29
tags: [svg, resvg, usvg, test-suite, reference-rasterization, conformance-matrix, pixel-exact, rust, testing]
confidence: 0.93
claims: [nuif:claim:authored-resolved]
relations:
  - type: related_to
    target: nuif:research:svg
    note: resvg is the most complete Rust static-SVG implementation and its suite is a de facto conformance corpus for SVG 1.1 static features.
  - type: related_to
    target: nuif:research:ssim-and-classical-image-metrics
    note: resvg demonstrates the exact-comparison policy (per-channel threshold 1, zero differing pixels).
  - type: related_to
    target: nuif:research:vello-testing-and-cpu-reference
    note: Vello's sparse-strips snapshot harness reuses resvg's diff structure (threshold, diff image, REPLACE/MAKE_REF env).
  - type: related_to
    target: nuif:research:text-rendering-reproducibility
    note: resvg pins a fonts directory and generic-family mapping and disables system fonts for reproducible text fixtures.
links:
  spec: [spec/05-geometry-paint-text.md, spec/00-conformance.md]
  adr: [adrs/0003-reference-renderer.md]
  rfc: []
  code: [conformance/PLAN.md, crates/nuif-render]
  experiments: []
---

# Summary

resvg is a Rust static-SVG renderer built on tiny-skia. Parsing and rendering are split: `usvg` lowers SVG into a resolved tree (only absolute path segments, resolved `use`, CSS, text and markers, `objectBoundingBox` converted to `userSpaceOnUse`), and `resvg` rasterises that tree. The regression suite consists of roughly 1,700 single-issue SVG files with a fixed 200 × 200 viewBox, rendered at 300 px width with pinned fonts and compared against PNGs with a per-channel threshold of 1 and zero tolerated differing pixels. A generator script emits one `#[test]` per SVG. The separately maintained `resvg-test-suite` holds the same SVGs together with manually verified reference PNGs and publishes a per-feature support table for resvg, browsers and other libraries. The README claims bit-identical output across platforms because no system libraries are used.

## Evidence

- Scope: resvg "aims to only support the static SVG subset; i.e. no `a`, `script`, `view` or `cursor` elements, no events and no animations"; SVG Tiny 1.2 "is not supported and support is also not planned"; SVG 2 support is in progress (README, "SVG support").
- Suite size: "a vast test suite that includes around 1600 tests", described as SVG-to-PNG regression tests that exclude dependency tests (README, lines 23–25). The current tree holds 1,722 SVG files under `crates/resvg/tests/tests/`: filters 398, masking 93, paint-servers 151, painting 306, shapes 133, structure 262, text 379 (GitHub tree listing, 2026-08-29); `render.rs` contains 1,716 generated tests.
- Reproducibility claim: "if you render an SVG file on x86 Windows and then render it on ARM macOS - the produced image will be identical. Each pixel would have the same value." (README, "Reproducibility").
- Naming: tests are organised as `tests/<category>/<element-or-attribute>/<case>.svg` with a sibling `.png`, for example `painting/stroke-linejoin/{arcs,bevel,miter,miter-clip,round}.svg` (tree listing).
- Authoring rules: fixed 200 × 200 viewBox template with a frame `rect`; "Each test must test only a single issue"; every element needs an `id`; unique `title` under 60 characters; line length under 100; UTF-8; `check.py` enforces these (`crates/resvg/tests/README.md`).
- Reference generation: render with `--width 300 --skip-system-fonts --use-fonts-dir 'tests/fonts' --font-family 'Noto Sans' --serif-family 'Noto Serif' --sans-serif-family 'Noto Sans' --cursive-family 'Yellowtail' --fantasy-family 'Sedgwick Ave Display' --monospace-family 'Noto Mono'`, then `oxipng -o 6 -Z`; 300 px "to test scaling" (`crates/resvg/tests/README.md`, "Render PNG").
- Two PNG sets: `resvg-test-suite/png` contains reference images ("how the SVG files should be rendered"); `resvg/tests/png` contains images rendered by resvg itself "used only for regression testing" (`crates/resvg/tests/README.md`, "resvg tests vs resvg-test-suite tests").
- Harness: `IMAGE_SIZE: u32 = 300`; a global `fontdb` loads `tests/fonts` and sets the five generic families; `MAKE_REF` regenerates references; the actual image is alpha-demultiplied before comparison; `get_diff` marks a pixel different if any of R, G, B, A differs by more than `DIFF_THRESHOLD = 1`, treats two fully transparent pixels as equal, counts size mismatches as differences, and writes a three-panel diff PNG to `tests/diffs/` (`crates/resvg/tests/integration/main.rs`).
- Generated tests: `gen-tests.py` walks `tests/**/*.svg`, derives a function name from the path and emits `#[test] fn ... { assert_eq!(render("..."), 0); }`; the `IGNORE` list excludes `filters/feMorphology/huge-radius` (CI timeout), invalid-size and non-UTF-8 structure cases, and `paint-servers/radialGradient/focal-point-correction` with the comment "Produces slightly different output on some hardware. Not a bug, just a SIMD rounding difference." (`crates/resvg/tests/gen-tests.py`).
- usvg lowering: attributes resolved (inheritance, defaults), CSS applied, basic shapes converted to paths, only absolute MoveTo/LineTo/QuadTo/CurveTo/ClosePath segments, `use` and nested `svg` resolved, invalid elements removed, relative units converted, images loaded or decoded, references resolved, `switch` resolved, text "completely resolved", markers converted into regular elements, all filters supported, recursive elements removed, `objectBoundingBox` replaced with `userSpaceOnUse` (`crates/usvg/README.md`, "Features").
- Unsupported features are enumerated (font-based SVG elements, `color-profile`, external `use`, `clip`, `color-interpolation`, `direction`, `unicode-bidi`, and others) (`docs/unsupported.md`).
- Support table: rows are SVG elements and attributes grouped by category; columns are resvg, Chrome, Firefox, Safari, Batik, Inkscape, librsvg, SVG.NET, QtSvg; legend "Passed | Failed | Crashed | ? | Undefined behavior" (linebender.org/resvg-test-suite/svg-support-table.html). Results are produced by manual comparison recorded in `results.csv` "via `tools/vdiff`" and charted by `stats.py` (`resvg-test-suite/README.md`).
- Versions: resvg and usvg 0.48.1 (crates.io, 2026-08-02).

## Mechanism

```text
Fixture:   tests/<category>/<feature>/<case>.svg     (viewBox 0 0 200 200, one issue per file)
Reference: same path with .png, rendered by resvg at width 300 with pinned fonts, oxipng-optimised
Harness (integration/main.rs):
  tree   = usvg::Tree::from_data(svg, Options { fontdb: pinned, resources_dir: fixture dir })
  size   = tree.size().scale_to_width(300); pixmap = render(tree, scale transform)
  actual = demultiply_alpha(pixmap)
  diff   = count of pixels where (not both alpha == 0) and any channel |a - b| > 1
  assert diff == 0                                     # generated by gen-tests.py
  MAKE_REF=1 -> write/overwrite reference; failures write tests/diffs/<name>.png (expected | mask | actual)
Conformance matrix:
  for each implementation impl and fixture f: result[impl][f] in {passed, failed, crashed, unknown, undefined}
  aggregate per feature row and per implementation column
```

## NUIF relevance

**Borrow**
- Adopt the single-issue fixture discipline (fixed canvas, unique title, one feature per file, category/feature/case paths) for the NUIF `render` suite, because it makes failures attributable and the corpus enumerable as a feature matrix.
- Adopt the exact-comparison harness pattern (threshold 1, zero differing pixels, three-panel diff artefact, explicit regenerate flag) for the CPU reference path, because resvg demonstrates cross-platform pixel identity with this policy.
- Adopt the support-table pattern (implementations × features with a five-state legend) as the public conformance matrix for NUIF profiles, because it communicates partial support without collapsing to a single score.

**Adapt**
- Apply the usvg idea of a lowered, fully resolved tree to NUIF's resolved-snapshot layer, because conformance fixtures should compare the resolved scene as well as the raster; NUIF must keep authored intent alongside, which usvg discards.
- Keep separate "reference" and "regression" raster sets, because NUIF needs hand-verified references for the standard and implementation-rendered snapshots for regression detection.

**Reject**
- Do not exclude fixtures silently for SIMD rounding as `gen-tests.py` does; NUIF should instead pin the reference path's arithmetic (scalar or SIMD with identical rounding) or move the fixture to the tolerance tier with a recorded reason.
- Do not adopt manual `vdiff` triage as the source of truth for the matrix, because NUIF's matrix must be produced by the automated suite.

## Open questions

- Whether resvg-test-suite's MIT-licensed SVG corpus can be reused directly as NUIF import fixtures for the SVG adapter, and how to attribute it.
- How NUIF should treat "undefined behavior" cells; resvg marks them but the NUIF spec would need an explicit undefined-behaviour category.
