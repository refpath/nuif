---
id: nuif:research:materialx-and-shader-graph-tests
kind: implementation
status: reviewed
title: MaterialX shader generation and render test suite across backends
source:
  url: https://github.com/AcademySoftwareFoundation/MaterialX/blob/main/source/MaterialXTest/README.md
  repository: https://github.com/AcademySoftwareFoundation/MaterialX
  authors: [Academy Software Foundation, MaterialX contributors]
  published_at: "v1.39.5 (2026-05-22)"
  license: Apache-2.0
retrieved_at: 2026-08-29
tags: [materialx, shader-generation, render-tests, cross-backend, differential-testing, test-suite, image-comparison]
confidence: 0.88
claims: [nuif:claim:multi-level-ir, nuif:claim:semantic-automation]
relations:
  - type: extends
    target: nuif:research:materialx
    note: Documents how the standard is tested rather than what it specifies.
  - type: compares_to
    target: nuif:research:hydra-render-delegate
    note: Hydra compares against golden images with thresholds; MaterialX compares backends against each other with RMS reports.
  - type: related_to
    target: nuif:research:renderers
    note: Multiple code generators for one graph is the shader analogue of multiple render backends for one document.
links:
  spec: [spec/00-conformance.md, spec/05-geometry-paint-text.md, spec/12-cli-api-and-automation.md]
  adr: [adrs/0003-reference-renderer.md]
  rfc: [rfcs/0004-headless-qa-contract.md]
  code: [crates/nuif-render]
  experiments: [conformance/PLAN.md]
---

# Summary

MaterialX validates its node graph standard by generating shader code for each renderable element in a document corpus (`resources/Materials/TestSuite`, organized by library: `stdlib`, `pbrlib`, `nprlib`, `bxdf`) with every registered code generator (`genglsl`, `genosl`, `genoslnetwork`, `genmdl`, `genessl`, `genmsl`, `genslang`) and then compiling and, where a backend is available, rendering the generated code. The `MaterialXTest` executable (Catch-based, driven by `ctest` or test tags such as `[genglsl]`, `[renderglsl]`, `[renderosl]`, `[rendermsl]`) reads `_options.mtlx`, a MaterialX document that parametrizes targets, light rigs, render size, geometry, IBL paths, render test paths and exclusions. Outputs are generated source, per-language logs (generation, implementation coverage, document validation, profiling, render) and rendered images.

There are no reference images and no automated pixel tolerance gate in the core render tests: pass/fail is compile-and-render success plus implementation-count checks. Cross-backend agreement is assessed by a separate report generator (`python/MaterialXTest/tests_to_html.py`) that lays out images from two or three targets side by side and, if Pillow is installed, computes per-pair RMS difference images with an optional RMS filter; CI on the extended macOS build produces an HTML/PDF "MaterialX_RenderComparison" artifact comparing `msl` and `osl`. Viewer and graph-editor screen captures are also produced in CI as artifacts without comparison.

## Evidence

- Test categories and tags: core (`Document.cpp`, `Element.cpp`, ...), I/O (`XmlIo.cpp`), shader generation (`[genshader]`, `[genglsl]`, `[genosl]`, `[genmdl]`, `[genmsl]`), render (`[rendercore]`, `[renderglsl]`, `[renderosl]`, `[rendermsl]`) — `source/MaterialXTest/README.md` sections 1–3.3 (main, retrieved 2026-08-29).
- Render setup enabled by `MATERIALX_TEST_RENDER`; OSL requires `MATERIALX_OSL_BINARY_OSLC`, `MATERIALX_OSL_BINARY_TESTRENDER`, `MATERIALX_OSL_INCLUDE_PATH`; MDL requires `MATERIALX_MDL_SDK_DIR`; MSL 2.0+ on macOS — same README, "Per-Language Render Setup".
- Output logs `gen<language>_<target>_generatetest.txt`, `_implementation_check.txt`, `_render_doc_validation_log.txt`, `_render_profiling_log.txt`, `_render_log.txt`; the render log references per-material error files — same README, "Test Outputs".
- `_options.mtlx` `TestSuiteOptions` nodedef: `overrideFiles`, `lightFiles` (`light_rig_test_2.mtlx`), `targets` (`genglsl,genosl,genoslnetwork,genmdl,genessl,genmsl,genslang`), `checkImplCount`, `shaderInterfaces` (1 reduced, 2 complete, 3 both), `renderSize` (512x512), `dumpUniformsAndAttributes`, `dumpGeneratedCode`, `renderGeometry` (`sphere.obj`), `enableDirectLighting`, `enableIndirectLighting`, `radianceIBLPath`, `irradianceIBLPath`, `renderTestPaths`, `renderTestExcludeFiles`, `outputDirectory`, `enableTracing` (Perfetto) — `resources/Materials/TestSuite/_options.mtlx` (main).
- Test suite corpus layout by library and element category (e.g. `stdlib/math/{math,math_operators,transform,trig,vector_math}.mtlx`), `Geometry`, `Images`, `Utilities` (testrender utilities and light configuration); "each file is parsed to determine renderable elements" and code is "compiled, and/or rendered" — `resources/Materials/TestSuite/README.md`.
- `ShaderRenderTester` base class with `validate(optionsFilePath)`, `runTest(TestSuiteOptions)`, `loadOptions`, virtual `saveImage` defaulting to `false`; `RenderProfileResult { elementsTested, success }`; `TestRunLogger`, `TestRunProfiler`, `TestRunTracer` — `source/MaterialXTest/MaterialXRender/RenderUtil.h` lines 42–301.
- Render test subdirectories per backend: `MaterialXRenderGlsl`, `MaterialXRenderOsl`, `MaterialXRenderMsl`, `MaterialXRenderMdl`, `MaterialXRenderSlang`; generators `MaterialXGenGlsl`, `MaterialXGenOsl`, `MaterialXGenMdl`, `MaterialXGenMsl`, `MaterialXGenSlang` — `source/MaterialXTest/` listing (retrieved 2026-08-29).
- CI: `ctest --output-on-failure`, Python tests (`MaterialXTest/main.py`, `genshader.py`, `mxspec.py compare` against specification markdown, `mxformat.py --upgrade`), shader validation via `generateshader.py --target msl --validator "xcrun metal ..."`, render captures with `MaterialXView --captureFilename` and `MaterialXGraphEditor --captureFilename` uploaded as artifacts — `.github/workflows/main.yml` lines 258–350.
- Render comparison report step (extended macOS build with OSL): `tests_to_html.py -i1 Materials -l1 msl -l2 osl -d --order-from _options.mtlx -o MaterialX_RenderComparison.html`, optionally printed to PDF with headless Chrome — `.github/workflows/main.yml` lines 351–370.
- `tests_to_html.py`: "Install pillow via pip to enable image differencing and statistics"; `computeDiff` uses `ImageChops.difference` and returns `sum(diffStat.rms) / (3.0 * 255.0)`; options `-l1/-l2/-l3` target languages, `-d/--diff`, `-e/--error` "Filter out results with RMS less than this", `-t` timestamps — `python/MaterialXTest/tests_to_html.py` lines 9–81, 191–202.
- `mxspec.py compare` checks that specification markdown node tables and `*_defs.mtlx` agree for stdlib, pbrlib and nprlib — `.github/workflows/main.yml` lines 267–269.
- Latest release v1.39.5 (2026-05-22), license Apache-2.0 (GitHub releases API and repository, retrieved 2026-08-29).

## Mechanism

The suite is a differential test over code generators with a shared oracle of "compiles and renders". For each document under the configured paths, MaterialX enumerates renderable elements (materials, node graph outputs), instantiates each generator named in `targets`, generates source under one or more shader-interface modes, and records whether every node used has an implementation for that target (`checkImplCount`). Where a backend toolchain exists, the source is compiled (GLSL via OpenGL, OSL via `oslc` and `testrender`, MDL via the SDK, MSL via Metal, Slang) and rendered into a fixed camera/geometry/light configuration to a fixed size; the image is written next to the source material or into `outputDirectory`. Failures are compile or render errors, missing implementations, or document validation errors; timings are logged for profiling. Options live in a MaterialX document, so the test harness is configured in the format under test.

Agreement across backends is reviewed, not asserted. `tests_to_html.py` walks the output tree, pairs images by material and target language, computes normalized RMS of the absolute difference when Pillow is available, renders difference images, and emits an HTML table ordered by the `renderTestPaths` list; `-e` filters pairs below an RMS threshold to surface only divergent materials. CI publishes this report as an artifact for human inspection. Separately, `mxspec.py` performs a structural check that specification tables and library definitions do not drift, which is the only automated oracle relating the normative text to the implementation.

## NUIF relevance

**Borrow**
- Configure the conformance harness with a NUIF document (the `_options.mtlx` pattern) so evaluation contexts, backends and fixture paths are expressed in the format under test.
- Run every fixture through every lowering/backend pair and log implementation coverage per construct (`checkImplCount`) so unsupported constructs are enumerated rather than discovered.
- Generate a cross-backend comparison report with normalized RMS and difference images, ordered by fixture list and filterable by threshold, as a review artifact distinct from pass/fail gates.
- Check normative text against machine-readable definitions (`mxspec.py compare`) to keep spec tables and schema files synchronized.
- Emit per-backend structured logs (generation, coverage, validation, profiling, render) with per-fixture error files referenced from the summary log.

**Adapt**
- MaterialX has no golden images; NUIF must combine MaterialX's cross-backend differential report with Hydra-style thresholded golden comparisons for the reference renderer, since UI rendering has normative pixel expectations for geometry and text placement.
- RMS over the whole image is insensitive to small localized errors typical of text and hairline rendering; NUIF should add region- or glyph-level metrics.
- Human-reviewed HTML reports are adequate for shading research but not for a headless QA contract; NUIF reports must be machine-readable first with HTML as a projection.

**Reject**
- Treating "compiles and renders" as the pass criterion for a render suite; NUIF's render suite must assert declared tolerances.
- Optional dependency (Pillow) gating whether comparison statistics exist; NUIF comparison must be a required part of the reference implementation.
- Platform-specific backends (Metal, OSL toolchain) as prerequisites; NUIF's CPU reference backend must run everywhere.

## Open questions

- Whether a normalized RMS threshold has any defensible meaning for UI fixtures, or whether per-element structural comparisons should replace image metrics entirely except for the final rasterization suite.
- How to keep a cross-backend comparison meaningful when backends legitimately differ (font hinting, anti-aliasing) without masking real defects.
- Whether NUIF should adopt a "future updates" style option (`applyFutureUpdates`-like) to test fixtures under pending dialect versions before ratification.
