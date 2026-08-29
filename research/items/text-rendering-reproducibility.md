---
id: nuif:research:text-rendering-reproducibility
kind: synthesis
status: verified
title: Text rendering reproducibility (shaping determinism, HarfBuzz test format, FreeType hinting and anti-aliasing modes, browser differences, pinning strategy)
source:
  url: https://github.com/harfbuzz/harfbuzz/tree/main/test/shape
  repository: https://github.com/harfbuzz/harfbuzz
  authors: [HarfBuzz contributors, FreeType contributors, Unicode Inc. (text-rendering-tests), Chromium and Mozilla contributors]
  published_at: "HarfBuzz 14.4.0 (2026-08-26); FreeType documentation (undated); text-rendering-tests (2016–2024)"
  license: "HarfBuzz: MIT-style (Old MIT); FreeType docs: FTL/GPLv2 dual; text-rendering-tests: Unicode License; Chromium/Firefox docs: BSD-3-Clause/MPL-2.0"
retrieved_at: 2026-08-29
tags: [text, shaping, harfbuzz, freetype, hinting, anti-aliasing, subpixel, gamma, unicode, reproducibility, conformance, testing]
confidence: 0.88
claims: [nuif:claim:authored-resolved]
relations:
  - type: extends
    target: nuif:research:harfbuzz-unicode
    note: Adds the shaping-test format, version pinning and rasterisation-variance analysis to the HarfBuzz/Unicode record.
  - type: related_to
    target: nuif:research:webrender-reftests
    note: WebRender's text reftests are pinned to platform(linux) with PNG references and subpixel-AA options.
  - type: related_to
    target: nuif:research:resvg-test-suite
    note: resvg's pinned fonts directory and generic-family mapping is a working reproducibility recipe for text fixtures.
  - type: related_to
    target: nuif:research:gpu-rendering-nondeterminism
    note: Glyph rasterisation adds hinting, gamma and subpixel positioning variance on top of general GPU variance.
  - type: related_to
    target: nuif:research:vello-testing-and-cpu-reference
    note: Vello holds hinting and emoji snapshot groups; vello_cpu glyph caching is experimental.
links:
  spec: [spec/05-geometry-paint-text.md, spec/00-conformance.md]
  adr: [adrs/0003-reference-renderer.md]
  rfc: []
  code: [crates/nuif-text, crates/nuif-render, crates/nuif-testing/src/bin/text-pinning.rs, conformance/text/harfbuzz-14.4.0-ahem.json, conformance/PLAN.md]
  experiments: [nuif:experiment:text-pinning]
---

# Summary

Text is the least reproducible part of a rendering pipeline because three independently versioned stages contribute: Unicode data and shaping (character-to-glyph mapping, positioning), glyph outline processing (hinting, interpreter version, stem darkening) and rasterisation (grayscale vs. LCD anti-aliasing, subpixel positioning, gamma and blending). HarfBuzz shows that shaping is deterministic and testable at the glyph-string level when the font bytes are pinned by hash and the shaper, font-functions backend and options are fixed; its suite stores expected output as a compact serialisation of glyph names, clusters, offsets and advances. FreeType documents that the same outline yields different bitmaps under `FT_LOAD_TARGET_*` modes, interpreter versions 35/38/40, native versus auto-hinting and stem darkening, and that correct gamma-aware blending is generally absent in desktop stacks. Browser suites respond by avoiding pixel comparisons for text or pinning them to one platform. A reproducible NUIF text path therefore pins font hashes, Unicode and shaper versions, disables hinting, uses grayscale area coverage with a defined subpixel-position quantum, and compares at three levels: glyph string, outline, raster.

## Evidence

- HarfBuzz test recording: `record-test.sh` subsets the font to the tested code points, compares `hb-shape` output of original and subset fonts, then moves the subset into `data/in-house/fonts` and names it "after its hash"; test cases go to `data/in-house/tests` and must be registered in `data/in-house/meson.build`; only open-source fonts are accepted (`test/shape/README.md`).
- Test line format: `fontfile;options;unicodes;glyphs_expected`, split on `;` (`test/shape/run-tests.py`, `fontfile, options, unicodes, glyphs_expected = line.split(";")`); directives such as `@font-funcs=ot,ft` select backends for a file; absolute font paths may carry `@<sha1>` and are skipped when the on-disk SHA-1 differs ("Different version of %s found; Expected hash %s, got %s; skipping."); `*` as expected output accepts any result; when glyph names are unavailable the comparison is redone with `--no-glyph-names` (`run-tests.py`). By default only the `ot` shaper is tested (`HB_SHAPER_LIST`) while all supported font-funcs (`ot`, `ft`) are exercised, so expectations must hold under both FreeType and OpenType font functions (`run-tests.py`, "Right now we only test the 'ot' shaper").
- Example test line: `../fonts/df768b9c257e0c9c35786c47cae15c46571d56be.ttf;;U+0633,U+064F,...;[uni06CC.fina=10+1655|uni062A.medi=9+868|...|uni0650=2@148,0+0|...]` and `../fonts/SimpArabicTest.ttf;--no-positions;U+0628,...;[daggerdbl=31|c142=30|...]` (`test/shape/data/in-house/tests/arabic-fallback-shaping.tests`); cluster-level variants such as `--cluster-level=2` (`cluster.tests`).
- Serialisation format: glyphs delimited by `[` `]`, separated by `|`; each glyph is name or index, `=cluster` unless `NO_CLUSTERS`, `@x_offset,y_offset` when either offset is non-zero, `+x_advance` and `,y_advance` when non-zero, `<x_bearing,y_bearing,width,height>` with `GLYPH_EXTENTS`; example `[uni0651=0@518,0+0|uni0628=0+1897]` (`src/hb-buffer-serialize.cc`, `hb_buffer_serialize_glyphs` documentation; harfbuzz.github.io hb-buffer reference).
- Corpus: 87 in-house test files, 94 files mirrored from Unicode's text-rendering-tests, 128 AOTS files (counts of `.tests` entries in the three `meson.build` files).
- Unicode data version: HarfBuzz's generated UCD table is built from "Unicode 17.0.0" (`src/hb-ucd-table.hh` header).
- Cluster coordinates are client data, not an implicit universal byte offset: the HarfBuzz cluster manual says each input code point receives a cluster value and that clients commonly use its code-point index; `hb_glyph_info_t.cluster` returns that value after any shaping merges. HarfRust 0.13.3 exposes the equivalent `UnicodeBuffer::add(char, u32)` and documents buffer length in Unicode code points (HarfBuzz manual `working-with-harfbuzz-clusters.html`; docs.rs `harfrust/0.13.3/harfrust/struct.UnicodeBuffer.html`).
- Version drift: HarfBuzz 14.4.0 (2026-08-26) changed outputs in ways that affect expectations: "Glyph positions and extents now saturate instead of overflowing" and "Arabic Windows-1256 fallback shaping is now enabled on all platforms" (`NEWS`). The second item removed a platform dependency in shaping output.
- Unicode text-rendering-tests: test cases are HTML snippets with rendering parameters and expected SVG; engines (FreeType+HarfBuzz+FriBidi+Raqm "FreeStack", CoreText, Allsorts, Swash, fontkit, OpenType.js, others) emit SVG; matching "is implemented by iterating over SVG paths, allowing for maximally 1 font design unit of difference" (`unicode-org/text-rendering-tests/README.md`).
- FreeType load targets: `FT_LOAD_TARGET_NORMAL` is the default gray-level hinting; `FT_LOAD_TARGET_LIGHT` snaps only vertically, keeping horizontal spacing, and "Advance widths are rounded to integer values"; `FT_LOAD_TARGET_MONO` is for monochrome; `LCD`/`LCD_V` target decimated displays; `FT_LOAD_NO_HINTING` "generally generates 'blurrier' bitmap glyphs"; `FT_LOAD_FORCE_AUTOHINT`/`FT_LOAD_NO_AUTOHINT` choose the engine; "A font's native hinters may ignore the hinting algorithm you have specified (e.g., the TrueType bytecode interpreter)"; render modes NORMAL, LIGHT, MONO, LCD, LCD_V, SDF (freetype.org, Glyph Retrieval reference).
- FreeType interpreter versions: `interpreter-version` accepts 35, 38, 40; "Version 40 corresponds to MS rasterizer v.2.1; it is roughly equivalent to the hinting provided by DirectWrite ClearType"; the v40 interpreter's approach is to "ignore all horizontal hinting instructions", whereas v35 followed the 1990s TrueType specification (freetype.org, Driver properties; Subpixel hinting article). `hinting-engine` defaults to `adobe` for `cff`, `type1` and `t1cid`; auto-hinter stem darkening is off by default (`no-stem-darkening` TRUE) (Driver properties).
- FreeType gamma and darkening: the correct approach is to "alpha blend it onto the surface in linear space and then apply gamma correction"; "No library supports linear alpha blending and gamma correction out of the box on X11"; gamma correction lightens text and stem darkening counteracts thinning; the Adobe CFF engine has darkened stems since 2013 (freetype.org, "Text rendering: general" LCD/gamma article).
- Browser policy: Chromium states page rendering "is influenced by many factors such as the host computer's graphics card and driver, the platform's text rendering system" and prefers reference tests (chromium/src `docs/testing/writing_web_tests.md`); WebRender pins text reftests with `platform(linux) == isolated-text.yaml isolated-text.png`, uses `options(disable-subpixel)` and `options(disable-aa)`, and applies large tolerances such as `fuzzy(1,3692)` with `fuzzy-if(platform(win),2,5585)` for decoration suites (`gfx/wr/wrench/reftests/text/reftest.list`); WebRender's README notes residual "differences depending on font libraries on your system" (`gfx/wr/README.md`).
- Rust renderer practice: resvg renders text fixtures with `--skip-system-fonts --use-fonts-dir tests/fonts` and fixed generic families (`crates/resvg/tests/README.md`); Vello keeps `hinting.rs` and `emoji.rs` snapshot groups (`vello_tests/tests/`), and `vello_cpu` marks glyph caching "experimental" (`sparse_strips/vello_cpu/README.md`).
- Fontations exposes unhinted glyph drawing through `OutlineGlyph::draw(DrawSettings::unhinted(...))`; Skrifa's `PathStyle::HarfBuzz` selects HarfBuzz-compatible point-stream interpretation. Zeno 0.3.3 documents 256-level anti-aliased rasterization into 8-bit alpha masks and explicit nonzero/even-odd fills (`docs.rs/skrifa/0.46.2`, `docs.rs/zeno/0.3.3`).
- NUIF spec baseline: resolved text "MAY store shaped glyph runs keyed by font hashes, shaping configuration and Unicode data version" (spec/05-geometry-paint-text.md, line 9).

## Mechanism

```text
Stage 1 - shaping (deterministic given pins)
  input:  (font bytes -> sha256), text (Unicode scalars), script, language, direction, features, cluster level,
          shaper id+version (harfbuzz 14.4.0 / harfrust x.y), unicode data version (17.0.0)
  output: [glyph=cluster@dx,dy+adv|...]                 # HarfBuzz serialisation; compare as strings
  test:   fontfile@sha1;options;U+....;[expected]        # one line per case; skip if hash mismatch

Stage 2 - outlines (deterministic given pins)
  hinting = none (FT_LOAD_NO_HINTING / no bytecode, no autohint), no stem darkening, no synthetic emboldening
  outline units: font units -> device space via exact affine; compare paths with <= 1 font-unit tolerance
  (Unicode text-rendering-tests criterion)

Stage 3 - rasterisation (deterministic only on the CPU reference path)
  anti-aliasing: grayscale area coverage (no LCD filtering, no MSAA)
  subpixel positioning: quantise glyph origin to 1/q px in x (q declared, e.g. 4) and integer y
  blending: linear-light or sRGB-space declared explicitly; gamma exponent recorded
  compare: exact per-channel on the CPU path; count-and-delta or FLIP on GPU tiers

Result record: font hashes, unicode version, shaper version, hinting=off, aa=grayscale, q, blend space, pixel ratio
```

## Executable verification

The profile-0 shaping layer pins the 22,572-byte Ahem 1.50 font at SHA-256 `f0a92cd0cc45735591c9b5b1fa8aecd5194e8dc518895ca22af94a46c23550dc`, HarfRust 0.13.3 and its Unicode 17.0.0 data. It assigns explicit Unicode-scalar indices through `UnicodeBuffer::add`, matching HarfBuzz's documented client-defined cluster contract instead of inheriting UTF-8 byte offsets from a convenience input method. The independent fixture was captured with HarfBuzz 14.4.0 `hb-shape --no-glyph-names`; eight ASCII, Unicode, LTR and RTL glyph strings match exactly.

`cargo xtask gate-d-text` repeats each shaping and outline call, rejects missing context fonts and malformed font hashes, and repeats scene/PNG generation at 360×640, 768×768 and 1440×900. It matches five independently captured `hb-vector` paths after a declared normalization that removes the redundant explicit line-to-start before contour close. The render candidate uses unhinted Skrifa 0.46.2 outlines quantized to signed 26.6 font units, Zeno 0.3.3 8-bit grayscale nonzero coverage, a fixed Ahem baseline and encoded-sRGB alpha composition.

The machine report separately classifies exact shaping, exact normalized outlines, raster equality on its recorded platform matrix and missing line-breaking semantics. The three committed scene and PNG hashes reproduce on macOS/aarch64, Linux/aarch64 and Linux/x86_64, so `cross_platform_raster_verified` is true for that matrix. This is not a claim about untested systems. The text entity remains `approximated` for absent wrapping rather than being promoted by raster equality alone.

## NUIF relevance

**Borrow**
- Adopt HarfBuzz's test-line format (font hash, options, code points, expected glyph string) for a NUIF `text-shaping` fixture class, because it makes shaping conformance font-pinned, textual and diffable without any raster.
- Adopt the Unicode text-rendering-tests criterion (outline paths within 1 font design unit) as the middle tier for glyph geometry, because it isolates outline correctness from rasterisation policy.

**Adapt**
- Pin fonts by SHA-256 of the full font file rather than HarfBuzz's SHA-1 of a subset, because NUIF documents reference whole assets and the codec already hashes them.
- Define the normative raster policy as unhinted, grayscale area coverage with a declared subpixel quantum and blend space, because FreeType documents that every one of these choices changes the bitmap and desktop stacks disagree on defaults.

**Reject**
- Do not compare text rasters across platforms with system font stacks (CoreText, DirectWrite, FreeType with native hinting), because WebRender and Chromium both show this requires per-platform references or large tolerance allowances.
- Do not allow the resolved snapshot to omit the shaper and Unicode versions, because HarfBuzz 14.4.0 changed positioning outputs and NUIF's portability reports must attribute such diffs to version drift rather than document loss.

## Open questions

- Whether a future profile should keep HarfRust 0.13.3 as its normative shaper or treat it only as a reference implementation once an independent implementation reproduces the declared glyph fixtures.
- Whether NUIF should require bidi and line-breaking algorithm versions (UAX #9, UAX #14) alongside the Unicode data version, since line-break differences are a distinct portability category in the whitepaper.
- What subpixel quantum and blend space the CPU reference path should fix; the surveyed sources document the variance but do not prescribe a value.
