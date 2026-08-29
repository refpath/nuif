---
id: nuif:research:skia-gold-and-gm-tests
kind: implementation
status: reviewed
title: Skia GM tests, DM, Skia Gold triage and fuzzy matching; browser reftests (WPT, Chromium, Firefox)
source:
  url: https://skia.org/docs/dev/testing/skiagold/
  repository: https://github.com/google/skia-buildbot
  authors: [Skia contributors, web-platform-tests contributors, Chromium contributors, Mozilla contributors]
  published_at: "undated documentation; sources retrieved 2026-08-29"
  license: "Skia and skia-buildbot: BSD-3-Clause; web-platform-tests docs: BSD-3-Clause; Chromium docs: BSD-3-Clause; Firefox docs: MPL-2.0"
retrieved_at: 2026-08-29
tags: [golden-master, baseline, triage, reftest, fuzzy-matching, fuzzing, skia, gold, wpt, testing]
confidence: 0.9
claims: []
relations:
  - type: extends
    target: nuif:research:renderers
    note: Adds the testing and baseline-management side of Skia to the architectural record.
  - type: compares_to
    target: nuif:research:webrender-reftests
    note: WebRender's reftest.list fuzzy(max_diff,num_diff) is the same count-and-delta policy as WPT fuzzy and Gold's fuzzy matcher.
  - type: related_to
    target: nuif:research:ssim-and-classical-image-metrics
    note: Gold and WPT use channel deltas and pixel counts, not perceptual indices.
  - type: related_to
    target: nuif:research:gpu-rendering-nondeterminism
    note: Gold keys baselines by OS, GPU and backend because outputs legitimately differ per configuration.
links:
  spec: [spec/00-conformance.md]
  adr: [adrs/0003-reference-renderer.md]
  rfc: []
  code: [conformance/PLAN.md, crates/nuif-render]
  experiments: []
---

# Summary

Skia tests rendering with GM ("golden master") programs executed by the DM driver across configurations ("sinks") such as the software raster backend and GPU backends. Images are not compared against files in the repository; they are uploaded to Skia Gold, a service that stores expectations per test and per key set (OS, architecture, backend) and lets humans triage each new digest as positive, negative or untriaged. Gold supports several non-exact matchers configured through optional keys (`image_matching_algorithm` = `fuzzy`, `sobel`, `sample_area`, `positive_if_only_image`). Fuzzing is separate: libFuzzer targets in the `fuzz` binary are run by OSS-Fuzz. Browser engines use reftests instead of stored images: a test page and a reference page must render identically under the same engine, with an optional fuzzy allowance expressed as a maximum per-channel difference and a maximum number of differing pixels.

## Evidence

- Gold purpose: "Gold is a web application that compares the images produced by our bots against known baseline images" (skia.org, Skia Gold page). The page lists positive ("the diff is considered acceptable"), negative ("requires a fix") and untriaged states, and states that Gold processes more than 500,000 images per commit across OS, architecture and backends including CPU, OpenGL and Vulkan.
- Multiple positives: the client manual lists "Multiple correct (or 'positive') images for a single test" and pre-submit pass/fail plus post-submit triage modes (skia-buildbot `golden/docs/README.md`, "What is Gold?").
- Keys: `goldctl imgtest init --keys-file ./keys.json` carries key-value pairs "describing how these inputs got drawn", such as OS and GPU; `goldctl imgtest add --test-name ... --png-file ...` uploads; `--passfail` enables presubmit gating (`golden/docs/README.md`, "Using Gold").
- Matching algorithms: `image_matching_algorithm` selects `exact`, `fuzzy`, `positive_if_only_image`, `sample_area` or `sobel`; parameters are `fuzzy_max_different_pixels`, `fuzzy_pixel_delta_threshold`, `fuzzy_pixel_per_channel_delta_threshold`, `fuzzy_ignored_border_thickness`, `sobel_edge_threshold`, `sample_area_width`, `sample_area_max_different_pixels_per_area`, `sample_area_channel_delta_threshold` (`gold-client/go/imgmatching/constants.go`).
- Fuzzy semantics: images must be equal in size; the number of differing pixels must not exceed `MaxDifferentPixels`; if `PixelDeltaThreshold > 0` no pixel may have dR + dG + dB + dA above it (range 0–1020), else no pixel may have max(dR, dG, dB, dA) above `PixelPerChannelDeltaThreshold` (0–255); a border of `IgnoredBorderThickness` rows/columns is skipped; `MaxDifferentPixels = 0` degenerates to exact matching (`gold-client/go/imgmatching/fuzzy/fuzzy.go`, type comment and `Match`).
- Parameters are parsed from optional keys with validation and range checks (`gold-client/go/imgmatching/factory.go`, `MakeMatcher`, `getAndValidateIntParameter`).
- GM API: `DrawResult { kOk, kFail, kSkip }`; `getGoldKeys()` returns `name` and `source_type = "gm"`; `DEF_SIMPLE_GM(NAME, CANVAS, W, H)` (Skia `gm/gm.h`).
- DM usage: `--src` accepts `tests gm image skp`; `--config 8888` draws "using the software backend into a 32-bit RGBA bitmap" and `gl` uses the Ganesh OpenGL backend; `-w` writes results, `-r` reads a baseline directory, `--match` filters, `--nogpu`/`--nocpu` restrict work; DM emits `dm.json` with checksums of raw pixels (skia.org, Testing page). A GM is added under `gm/`, registered in `gn/gm.gni`, built with `ninja -C out/Debug dm` and run with `out/Debug/dm --match newgmtest` (skia.org, Writing Skia Tests).
- Fuzzing: fuzzers use the libFuzzer entry point `LLVMFuzzerTestOneInput`; reproduction is `out/ASAN/fuzz -t api -n RasterN32Canvas -b testcase`; OSS-Fuzz "rebuilds Skia and certain fuzzers and then runs said fuzzers" with configuration in `oss-fuzz/projects/skia` (skia.org, Fuzzing page).
- WPT reftests: reftests are "made up of the test and one or more other pages ('references')" with assertions on whether they render identically; `<link rel=match href=...>` passes if the pages render "pixel-for-pixel identically within an 800x600 window", `rel=mismatch` passes if they differ; with several references, at least one match must match and all mismatches must mismatch (web-platform-tests.org, Writing reftests).
- WPT fuzzy syntax: `<meta name=fuzzy content="maxDifference=15;totalPixels=300">`, shorthand `<meta name=fuzzy content="15;300">`, ranges `maxDifference=10-15;totalPixels=200-300`, per-reference prefix `option1-ref.html:10-15;200-300`; `maxDifference` is "a maximum difference in the per-channel color value for any pixel", `totalPixels` "a number of total pixels that may be different"; unprefixed values apply to references without a specific value (same page, "Fuzzy Matching").
- Chromium policy: pixel tests are "less robust" because rendering "is influenced by many factors such as the host computer's graphics card and driver, the platform's text rendering system"; reference pages are named `foo-expected.html` or `foo-expected-mismatch.*`; "You should only write a pixel test if you cannot use a reference test" (chromium/src `docs/testing/writing_web_tests.md`).
- Firefox manifest: `==` passes if renderings are the same, `!=` if different; `fuzzy(minDiff-maxDiff,minPixelCount-maxPixelCount)` passes if per-pixel value differences and the count of differing pixels fall in the given inclusive ranges; `fuzzy-if(condition,...)`, `fails-if`, `skip-if`, `random`, `pref()` and `asserts(count)` annotate conditions (firefox-source-docs, Reftest manifest).

## Mechanism

```text
Gold fuzzy matcher (gold-client/go/imgmatching/fuzzy/fuzzy.go)
  require size(expected) == size(actual)
  n_diff = 0; max_delta = 0
  for each pixel outside the ignored border:
      if p1 != p2: n_diff += 1
      delta = per_channel ? max(|dR|,|dG|,|dB|,|dA|) : |dR|+|dG|+|dB|+|dA|
      max_delta = max(max_delta, delta)
  pass iff n_diff <= MaxDifferentPixels and max_delta <= threshold

Gold data model
  digest = hash(png bytes); trace = (test name, keys...) ; expectation[trace][digest] in {positive, negative, untriaged}
  new digest -> untriaged -> human triage (or matcher auto-approval against the most recent positive)

Reftest (WPT / Firefox)
  render(test), render(ref) with the same engine, same window (800x600)
  match:    pass iff max per-channel |test - ref| <= maxDifference and count(differing pixels) <= totalPixels
  mismatch: pass iff images differ
```

Baselines in Gold are keyed by configuration rather than shared, so a single test may have distinct positives for `8888`, `gl` and `vk`. This is the source's design; the consequence for NUIF is interpretation.

## NUIF relevance

**Borrow**
- Adopt the reftest form for layout and paint semantics wherever a fixture can be expressed as two NUIF documents that must resolve to the same raster, because it removes stored images and platform baselines from the normative suite.
- Adopt the three-state triage vocabulary (positive, negative, untriaged) and configuration keys (OS, backend, adapter, font stack) for the non-normative GPU tier, because it is proven at browser scale and separates "different" from "wrong".

**Adapt**
- Encode WPT-style `maxDifference;totalPixels` as fixture metadata in the NUIF conformance manifest, with values required to be justified per fixture, because the browser suites show that ad hoc tolerances accumulate without rationale.
- Replace Gold's remote service with in-repository digests plus a small triage file for the reference implementation, because NUIF's suite must run offline and be vendor-neutral.

**Reject**
- Do not use per-platform positive baselines for the normative CPU reference path, because Chromium's own guidance treats platform-specific expected images as a maintenance burden to be avoided.
- Do not adopt `positive_if_only_image` (auto-approve when a test has a single image), because it converts an untriaged result into a passing baseline without human or metric review.

## Open questions

- Whether NUIF should require a reference-document form for every render fixture, or permit stored rasters only for the CPU path.
- How to express `fuzzy-if(condition)`-style conditional tolerance in a vendor-neutral manifest without encoding browser-specific platform names.
- Which fuzz targets (codec, path geometry, layout) map onto NUIF's `security` suite; Skia's fuzz taxonomy (api, image decoders, skp, path ops) is a starting list.
