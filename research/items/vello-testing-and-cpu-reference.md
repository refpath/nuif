---
id: nuif:research:vello-testing-and-cpu-reference
kind: repository
status: reviewed
title: Vello test infrastructure (vello_tests, vello_sparse_tests) and the vello_cpu renderer as a deterministic reference path
source:
  url: https://github.com/linebender/vello/tree/main/vello_tests
  repository: https://github.com/linebender/vello
  authors: [Linebender contributors]
  published_at: "2026-08-14 (vello 0.10.0); 2026-08-07 (vello_cpu 0.2.0)"
  license: Apache-2.0 OR MIT
retrieved_at: 2026-08-29
tags: [vello, vello-cpu, sparse-strips, snapshot-test, flip, tolerance, cpu-reference, wgpu, rust, testing]
confidence: 0.9
claims: [nuif:claim:authored-resolved]
relations:
  - type: extends
    target: nuif:research:vello
    note: Adds the testing model and the CPU renderer status to the base Vello record.
  - type: implements
    target: nuif:research:flip-perceptual-difference-metric
    note: vello_tests pools FLIP error maps (nv-flip, 67 PPD) with mean thresholds of 0.01 and 0.001.
  - type: related_to
    target: nuif:research:resvg-test-suite
    note: vello_sparse_tests reuses resvg's exact-diff harness shape (per-channel threshold, diff PNGs, REPLACE env).
  - type: related_to
    target: nuif:research:gpu-rendering-nondeterminism
    note: Vello documents fast-math and precision differences between GPU and CPU paths as the reason for non-exact comparison.
  - type: related_to
    target: nuif:research:renderers
    note: Vello's two-tier model (GPU compute vs. CPU sparse strips) mirrors NUIF's renderer-trait split.
links:
  spec: [spec/05-geometry-paint-text.md, spec/00-conformance.md]
  adr: [adrs/0003-reference-renderer.md]
  rfc: []
  code: [crates/nuif-render, conformance/PLAN.md]
  experiments: []
---

# Summary

The Vello repository contains two test systems. `vello_tests` targets the compute-shader renderer (`vello`): property tests, snapshot tests that treat GPU shaders as the source of truth while also executing the CPU shader fallbacks, and GPU-versus-CPU comparison tests; all image comparisons pool a FLIP error map and assert on its mean. `vello_sparse_tests` targets the sparse-strips renderers (`vello_cpu`, `vello_hybrid`, WebGL): a proc macro expands each test into per-backend and per-SIMD-level variants compared against one reference PNG with an integer per-component tolerance (0 for the f32 CPU pipeline, 2 for the u8 pipeline, 1 for SIMD and hybrid) and an optional count of pixels allowed to deviate fully. As of August 2026, `vello_cpu` 0.2.0 is described by its README as a CPU-only renderer with broad feature support, SIMD paths for all major architectures, an optional f32 pipeline intended for test snapshots, and remaining gaps (complex filter graphs panic, experimental glyph caching); the enclosing `sparse_strips` README still marks the directory as not production-ready.

## Evidence

- Test kinds: property tests run on GPU and CPU; snapshot tests "use the GPU shaders as a source of truth, but the CPU shaders are also ran"; they have "a non-exact comparison metric, because of small differences between rendering on different platforms", including "fast math" on Apple platforms; comparison tests check that "the GPU renderer matches the reference CPU renderer" and are expected to be phased out (`vello_tests/README.md`).
- Storage: smoke snapshots live in-repository under `smoke_snapshots` and "are always required to pass"; other snapshots use git LFS as "an experiment", and tests pass on CI if LFS files fail to download because of bandwidth or storage limits (`vello_tests/README.md`, "LFS").
- Metric: `nv_flip::flip(expected, rendered, nv_flip::DEFAULT_PIXELS_PER_DEGREE)` builds a `FlipPool`; images are converted to RGB8 (alpha dropped); a size mismatch is a failure (`vello_tests/src/snapshot.rs`, lines 300–345). `DEFAULT_PIXELS_PER_DEGREE = 67.0` (`nv-flip/src/lib.rs`, line 15; crate 0.1.2).
- Thresholds: `assert_mean_less_than(0.01)` in all four smoke snapshots (`vello_tests/tests/smoke_snapshots.rs`, lines 29, 47, 75, 119) and `0.001` in a known-issue reproduction (`vello_tests/tests/known_issues.rs`, line 55); the helper asserts `value < 0.1` as a sanity bound and documents that a non-zero mean may arise "due to fast math on the GPU or different precisions" (`vello_tests/src/compare.rs`, lines 37–48).
- Controls: `VELLO_TEST_UPDATE`, `VELLO_TEST_CREATE`, `VELLO_TEST_GENERATE_ALL`, `VELLO_SKIP_LFS_SNAPSHOTS`, each accepting `all`, `cpu`, `gpu` or a test name (`vello_tests/src/snapshot.rs`, lines 90–290; `src/lib.rs`, `env_var_relates_to`); `VELLO_CI_GPU_SUPPORT=no` sets `cfg(skip_gpu_tests)` (`vello_tests/build.rs`); `VELLO_DEBUG_TEST` dumps intermediate images (`src/lib.rs`, line 88). Default anti-aliasing for tests is `AaConfig::Area` (`src/lib.rs`, line 71).
- Sparse-strips tolerance semantics: a tolerance of 0 "means that it must be an exact match"; 1 means each component may differ by at most 1; `DEFAULT_CPU_U8_TOLERANCE = 2`, `DEFAULT_SIMD_TOLERANCE = 1`, `DEFAULT_CPU_F32_TOLERANCE = 0`, `DEFAULT_HYBRID_TOLERANCE = 1`; the u8 value of 2 avoids per-test overrides for bilinear image cases (`sparse_strips/vello_dev_macros/src/lib.rs`, lines 12–23).
- Macro attributes: `cpu_u8_tolerance`, `hybrid_tolerance` (added to the defaults), `diff_pixels` ("maximum number of pixels that are allowed to completely deviate", motivated by gradient colour-stop boundaries under floating-point inaccuracy), `transparent`, `skip_cpu`, `skip_multithreaded`, `skip_hybrid`, `hybrid_only`, `hybrid_no_depth`, `no_ref`, `glyph`, `ignore_reason`; generated variants are `_cpu_u8_scalar`, `_cpu_u8_neon`, `_cpu_u8_sse42`, `_cpu_u8_avx2`, `_cpu_u8_wasm`, f32 counterparts, `_hybrid`, `_hybrid_webgl`, `_hybrid_no_depth`; one instance is flagged `is_reference` and writes the reference PNG (`sparse_strips/vello_dev_macros/src/test.rs`, lines 13–70, 80–140, 218–225, 486–580).
- Sparse-strips comparison: `check_ref` renders, encodes PNG, loads `snapshots/<test>.png`, computes `get_diff(ref, actual, threshold, diff_pixels)`; `is_pix_diff` compares R, G, B only (alpha ignored) with `abs_diff > threshold`, treats two alpha-0 pixels as equal; a test fails when the count of differing pixels exceeds `diff_pixels`; `REPLACE=1` rewrites the reference from the reference instance; references are oxipng-optimised; on wasm the snapshot bytes are inlined with `include_bytes!` (`sparse_strips/vello_sparse_tests/tests/util.rs`, lines 360–460, 549–666; `vello_dev_macros/src/test.rs`, lines 197–215).
- Sparse-strips targets: the crate tests "CPU, WGPU, WASM32 WebGL"; WebGL runs via `wasm-pack test --headless --chrome --features webgl --release` (`sparse_strips/vello_sparse_tests/README.md`).
- Architecture: sparse strips aim to run "on GPUs without compute shader support, using only fragment and vertex shaders", mitigate performance cliffs and handle low-memory conditions; crates `vello_common`, `vello_cpu` ("CPU-based renderer optimized for multithreading and SIMD"), `vello_hybrid`, `vello_sparse_shaders` (WGSL→GLSL for WebGL); the directory is "not yet suitable for production use" (`sparse_strips/README.md`).
- vello_cpu status: "a solid CPU-only 2D renderer with broad, reliable feature support" with "optimized SIMD implementations for all major architectures"; limitations: complex filter graphs panic, multi-threaded filters unsupported, glyph caching experimental, API lifecycle rough; features `u8_pipeline` (`OptimizeSpeed`) and `f32_pipeline` (`OptimizeQuality`, "espectially useful for rendering test snapshots"), `std`/`libm`, `multithreading`, `text`; MSRV 1.88 (`sparse_strips/vello_cpu/README.md`). Design is documented in a 2025 ETH master's thesis linked from the README.
- Versions: vello 0.10.0 (2026-08-14), vello_cpu/vello_hybrid/vello_common 0.2.0 (2026-08-07) (crates.io; `CHANGELOG.md`; GitHub releases `sparse-strips-v0.2.0`).
- Text: `vello_tests/tests/hinting.rs` and `emoji.rs` exist as snapshot groups (directory listing); `vello_sparse_tests/tests/glyph.rs` generates cached and uncached glyph variants (`vello_dev_macros/src/test.rs`, `glyph`).

## Mechanism

```text
vello_tests snapshot:
  img = render(scene, params{use_cpu, aa=Area})               # GPU via wgpu or CPU shader fallback
  ref = decode(smoke_snapshots/<name>.png | lfs snapshots)
  require size(img) == size(ref)
  map  = FLIP(rgb(ref), rgb(img), ppd = 67)
  pass iff mean(map) < threshold                               # 0.01 typical, 0.001 strict
vello_tests compare_gpu_cpu:
  pass iff mean(FLIP(cpu_render, gpu_render)) < threshold      # threshold < 0.1 enforced

vello_sparse_tests (#[vello_test(width, height, ...)]):
  for variant in {cpu_u8_{scalar,neon,sse42,avx2,wasm}, cpu_f32_..., hybrid, hybrid_webgl, hybrid_no_depth}:
      tol = base_tol(variant) + user_tol
      n = count(pixels p: not both alpha 0 and any c in RGB |ref_c - img_c| > tol)
      pass iff n <= diff_pixels                                # diff_pixels default 0
  reference PNG written once by the designated reference variant; REPLACE=1 regenerates
```

The CPU f32 pipeline with tolerance 0 is the only configuration in either harness that asserts bit-exact equality against a stored image; this observation is NUIF's, derived from the constants above.

## NUIF relevance

**Borrow**
- Use `vello_cpu` with `RenderMode::OptimizeQuality` (f32 pipeline, scalar or a pinned SIMD level) behind the NUIF renderer trait as the deterministic conformance path, because the crate's own harness already holds that configuration to tolerance 0.
- Reuse the tiered tolerance model (exact for CPU f32, ±1 for SIMD/hybrid, ±2 for u8, FLIP mean for wgpu) as the template for NUIF's determinism tiers, because it is derived from measured behaviour of a Rust renderer rather than assumed.

**Adapt**
- Replace `diff_pixels` escape hatches with fixture-level tier assignment and recorded reasons, because gradient boundary flips are a property of the fixture class and should be visible in the conformance report.
- Record the vello_cpu version, pipeline, SIMD level and thread count in every `render` result, because the harness shows that each of these changes tolerance.

**Reject**
- Do not treat wgpu output as a source of truth for conformance, as `vello_tests` does for its snapshots, because NUIF's ADR 0003 requires the CPU path to be normative and the GPU path to be an experiment.
- Do not depend on git LFS for reference rasters; NUIF fixtures must be small, in-repository and always required to pass, because Vello's harness deliberately passes when LFS is unavailable.

## Open questions

- Whether `vello_cpu` multithreaded rendering is bit-identical to single-threaded output in the f32 pipeline; the harness has a `skip_multithreaded` attribute but the tolerance tables do not distinguish thread counts.
- Whether the `vello_hybrid` ±1 tolerance is stable across wgpu backends (Vulkan, Metal, D3D12, WebGL), since the variants share one reference image.
- Filter support gaps in `vello_cpu` (complex filter graphs) versus the effects vocabulary NUIF intends to specify in spec/05.
