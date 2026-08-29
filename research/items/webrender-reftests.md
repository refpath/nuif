---
id: nuif:research:webrender-reftests
kind: implementation
status: reviewed
title: WebRender wrench reftest harness, reftest.list syntax and RON scene capture/replay
source:
  url: https://github.com/mozilla/gecko-dev/tree/master/gfx/wr/wrench
  repository: https://hg.mozilla.org/mozilla-central (gfx/wr)
  authors: [Mozilla WebRender contributors, Servo contributors]
  published_at: "undated source; mozilla-central master retrieved 2026-08-29"
  license: MPL-2.0
retrieved_at: 2026-08-29
tags: [reftest, webrender, wrench, fuzzy-matching, capture, replay, ron, baseline, testing]
confidence: 0.9
claims: []
relations:
  - type: extends
    target: nuif:research:renderers
    note: Adds WebRender's regression-testing and capture tooling to the architectural record.
  - type: compares_to
    target: nuif:research:skia-gold-and-gm-tests
    note: Same count-and-delta tolerance model as WPT fuzzy and Gold fuzzy, but expressed in a manifest with platform predicates.
  - type: related_to
    target: nuif:research:gpu-rendering-nondeterminism
    note: platform() and swgl predicates, OSMesa in CI and per-platform PNG references are WebRender's response to backend variance.
  - type: related_to
    target: nuif:research:text-rendering-reproducibility
    note: Text reftests are the ones most often pinned to platform(linux) with PNG references and subpixel-AA options.
links:
  spec: [spec/00-conformance.md]
  adr: [adrs/0003-reference-renderer.md]
  rfc: []
  code: [conformance/PLAN.md, crates/nuif-render]
  experiments: []
---

# Summary

`wrench` is WebRender's standalone driver. It loads YAML scene descriptions or RON captures, renders them through WebRender (GL, ANGLE or the SWGL software rasteriser), and runs a reftest suite declared in `reftests/reftest.list`. Each manifest line names an operator (`==`, `!=`, and the tile-accuracy operators `**`, `!*`), optional tolerance functions (`fuzzy(max_diff,num_diff)`, `fuzzy-range(...)`, conditional `-if` variants), platform predicates (`platform(...)`, `skip_on(...)`), render options and extra draw-statistics checks. Comparison is per-pixel on 8-bit channels: the difference of a pixel is the maximum channel delta, and the tolerance bounds how many pixels may exceed each delta bucket. Captures from Firefox (`ctrl-shift-3`) serialise the scene, frame and resources as RON files under `~/wr-capture`, which `wrench show` replays; this is the mechanism for deterministic reproduction of a browser frame outside the browser.

## Evidence

- Tool purpose: "`wrench` is a tool for debugging webrender outside of a browser engine"; headless mode "for use in continuous integration" is invoked via `./headless.py args`; reftests run with `script/headless.py reftest [path]`; failures are examined with the Firefox reftest analyzer; new tests add a scene and a reference to `reftests/` plus a line in `reftests/reftest.list` (`gfx/wr/wrench/README.md`).
- Capture: enable WebRender, "Hit ctrl-shift-3 to capture the frame. The data will be put in `~/wr-capture`", then `wrench show ~/wr-capture` (`gfx/wr/wrench/README.md`, "show").
- Capture format: `CaptureConfig { root, bits: CaptureBits, scene_id, frame_id, resource_id }` with a `ron::ser::PrettyConfig` using `enumerate_arrays(true)` and single-space indentation; scenes, frames and resources are written under `scenes/{:05}`, `frames/{:05}` and `resources/{:05}` with `.ron` extensions; external images are described by `ExternalCaptureImage { short_path, descriptor, external }` and `PlainExternalImage { data, uv }`; PNG dumps of RGBA8/R8/RG8 targets are available behind the `png` feature (`gfx/wr/webrender/src/capture.rs`).
- CI rendering: "Tests run using OSMesa to get consistent rendering across platforms. Still there may be differences depending on font libraries on your system" (`gfx/wr/README.md`, "Testing").
- Operators: `ReftestOp::Equal` → `"=="`, `NotEqual` → `"!="`, `Accurate` → `"**"` (rendering at different tile sizes must be pixel-exact), `Inaccurate` → `"!*"` (`gfx/wr/wrench/src/reftest.rs`, lines 60–67).
- Fuzzy structures: `RefTestFuzzy { max_difference: usize, num_differences: usize }` (lines 94–95); `fuzzy(` and `fuzzy-if(` parse two integers and assert that only one plain `fuzzy` is present, recommending `fuzzy-range` otherwise (lines 396–408); `fuzzy-range(` and `fuzzy-range-if(` accept a list of `<=max,*num` bucket pairs (lines 372–393).
- Comparison: pixel values are asserted to be 8-bit; the per-pixel difference is the maximum over channels (`pixel_max`); a 256-bin histogram of differences is built and a prefix sum checks that the number of pixels whose difference lies in (previous max, bucket max] does not exceed the bucket's `num_differences`, with a final check that no pixel exceeds the largest allowed difference (lines 118–205).
- Platform predicates: `platform()` yields `"swgl"` when the window is software-rendered, else `"win"`, `"linux"`, `"mac"`, `"android"` by target OS (lines 594–606); `skip_on(...)` and nested conditions such as `env(android,device)` are evaluated on the manifest (lines 649–660); `include` lines splice other manifests (lines 446–451).
- Options and extra checks: `options(...)` recognises `disable-subpixel`, `disable-aa`, `allow-mipmaps`; `force_subpixel_aa_where_possible(bool)` and `max_surface_size(usize)` are line-level settings; extra checks are `draw_calls(n)`, `alpha_targets(n)`, `color_targets(n)` (`ExtraCheck` enum; lines 414–440).
- Manifest examples (`gfx/wr/wrench/reftests/text/reftest.list`): `skip_on(android,device) fuzzy(1,3692) fuzzy-if(platform(win),2,5585) fuzzy-if(platform(swgl),3,13540) == decorations-suite.yaml decorations-suite.png`; `options(disable-aa) == ahem.yaml ahem-ref.yaml`; `platform(linux) == isolated-text.yaml isolated-text.png`; `fuzzy(1,774) platform(linux) draw_calls(3) == colors.yaml colors-subpx.png`; `platform(mac) fuzzy(195,30) == color-bitmap-shadow.yaml color-bitmap-shadow-ref.yaml`.
- Anti-aliasing manifest (`gfx/wr/wrench/reftests/aa/reftest.list`): `skip_on(android) fuzzy(1,1) fuzzy-if(platform(swgl),4,27) == rounded-rects.yaml rounded-rects-ref.png`; `fuzzy-if(env(android,device),6,792) == fractional-radii.yaml fractional-radii-ref.yaml`.
- Directory layout: reftest groups include `aa`, `backface`, `blend`, `border`, `boxshadow`, `clip`, `compositor`, `filters`, `gradient`, `image`, `mask`, `scrolling`, `snap`, `split`, `text`, `tiles`, `transforms` (`gfx/wr/wrench/reftests/`).
- Canonical home: the GitHub `servo/webrender` repository is "a downstream mirror" of `gfx/wr` in mozilla-central (`gfx/wr/README.md`).

## Mechanism

```text
reftest.list grammar (as implemented in wrench/src/reftest.rs)
  line := [predicate | fuzzy | option | check]* op test reference [# comment]
  op        := "==" | "!=" | "**" | "!*"
  fuzzy     := "fuzzy(" max_diff "," num_diff ")" | "fuzzy-if(" cond "," max_diff "," num_diff ")"
             | "fuzzy-range(" ("<=" max "," "*" num)+ ")" | "fuzzy-range-if(" cond "," ... ")"
  predicate := "platform(" name ")" | "skip_on(" cond, ... ")"
  option    := "options(" ("disable-subpixel" | "disable-aa" | "allow-mipmaps")* ")"
             | "force_subpixel_aa_where_possible(" bool ")" | "max_surface_size(" n ")"
  check     := "draw_calls(" n ")" | "alpha_targets(" n ")" | "color_targets(" n ")"
  include   := "include" path

compare(test_img, ref_img, fuzziness):
  hist[0..=255] = 0
  for each pixel: d = max over channels |a_c - b_c|; hist[d] += 1
  prefix[k] = sum(hist[0..=k]); prev_max = 0; prev_fail = prefix[0]... (pixels with d = 0 are always allowed)
  for (max_diff, num_diff) in fuzziness sorted by max_diff:
      n = prefix[min(255,max_diff)] - prev_fail
      fail if n > num_diff
      prev_fail = prefix[max_diff]; prev_max = max_diff
  fail if prefix[255] - prev_fail > 0           # pixels above the largest allowed difference

capture layout (~/wr-capture)
  scenes/00001/*.ron   frames/00001/*.ron   resources/00001/*.ron   (+ externals as texel .ron / png)
```

## NUIF relevance

**Borrow**
- Adopt a manifest-driven reftest list with `==`/`!=` operators, `fuzzy(max_diff,num_diff)` and explicit per-line rationale comments for the GPU tier, because the syntax is compact, machine-parseable and in production use on a GPU renderer.
- Adopt the `**`/`!*` idea (rendering with different internal tiling must be pixel-identical) as a metamorphic test for NUIF's renderer trait, because it catches tiling-dependent nondeterminism without a reference image.

**Adapt**
- Replace `platform(win|linux|mac|android|swgl)` with capability keys (backend, adapter class, font stack, pixel ratio) in NUIF's evaluation context, because platform names are not vendor-neutral and do not capture the actual sources of variance.
- Use NUIF's own canonical serialisation for scene captures instead of RON, because the capture must round-trip through the codec suite and remain a NUIF document rather than a renderer-internal dump.

**Reject**
- Do not accept platform-specific PNG references for the normative path (as `platform(linux) == x.yaml x.png` does), because the NUIF `render` suite requires one deterministic CPU reference output per fixture and context.
- Do not rely on OSMesa or any system GL for consistency in CI, because the WebRender README itself notes residual differences from system font libraries.

## Open questions

- Whether NUIF's conformance manifest should allow `draw_calls`-style structural checks against the renderer trait (scene statistics) as a non-image assertion.
- How captured interactive scenes (scroll, animation frames) map to NUIF fixtures, since wrench captures multiple frames per scene under `frames/{:05}`.
