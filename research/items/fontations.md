---
id: nuif:research:fontations
kind: implementation
status: verified
title: Fontations read-fonts and Skrifa OpenType stack
source:
  url: https://github.com/googlefonts/fontations
  authors: [Google Fonts, Fontations contributors]
  published_at: null
  license: MIT OR Apache-2.0
retrieved_at: 2026-08-30
tags: [fonts, opentype, rust, parser, outlines, fuzzing]
confidence: 0.98
claims: [nuif:claim:bounded-untrusted-input]
relations:
  - type: related_to
    target: nuif:research:opentype-font-embedding-and-portability
    note: Supplies an independent technical interpretation, not license policy.
  - type: compares_to
    target: nuif:research:ttf-parser
    note: Differential agreement avoids making one parser implementation the oracle.
links:
  spec: [spec/05-geometry-paint-text.md, spec/11-security.md]
  adr: [adrs/0003-reference-renderer.md]
  rfc: [rfcs/0010-portable-resource-package.md]
  code: [crates/nuif-text, crates/nuif-testing/src/bin/font-resources.rs]
  experiments: [nuif:experiment:text-pinning, nuif:experiment:font-resource-static-baseline]
---

# Summary

Fontations is a safe Rust family for reading, writing and interpreting OpenType
fonts. `read-fonts` is its low-level no-copy/no-allocation reader; Skrifa adds
metadata, character maps, variation information and outlines. NUIF already pins
Skrifa 0.46.2 for profile-zero outlines and now uses its independently selected
character map and metrics as the static font-resource differential oracle.

## Evidence

- The repository identifies `read-fonts` as a high-performance parser suitable
  for shaping and describes its access as allocation- and copy-free. Locator:
  repository `README.md`, “Structure”, retrieved 2026-08-30.
- Skrifa exposes metrics, codepoint-to-glyph mapping, localized strings,
  attributes, axes and TrueType/CFF/color/bitmap outline sources. Locator:
  `skrifa/README.md`, “Features”, retrieved 2026-08-30.
- Skrifa forbids unsafe code and says corrupted or malicious input should not
  panic. Fontations maintains cargo-fuzz and OSS-Fuzz integration. Locators:
  `skrifa/README.md`, “Panicking” and “Safety”; repository `README.md`,
  “Fuzzing”, retrieved 2026-08-30.

## Mechanism

The conformance executable constructs a separate Skrifa `FontRef`, reads
unscaled global metrics and variation-axis count, and builds exact Unicode
coverage ranges from Skrifa's selected character map. It compares those values
with the `nuif-font` inspection before running package and policy trials. Skrifa
does not participate in package acceptance, so a shared failure cannot become a
false cross-parser pass.

## Alternatives and decision

Fontations could replace the production parser and becomes a leading candidate
when a later profile needs variable axes, CFF/CFF2, color glyphs or font
rewriting. It is not selected for both production and oracle roles in the first
profile because agreement inside one parser family would not be differential
evidence. FreeType remains valuable as a future native third oracle and browser
stacks provide essential WOFF2 evidence, but neither is needed to define the
smallest static sfnt baseline.

## NUIF relevance

**Borrow** independent metrics, axis state and selected Unicode mappings for
cross-parser comparison; retain Skrifa for the separately versioned outline
stage.

**Adapt** parser results through NUIF limits and exact semantic ranges. A font
parser does not own package resolution, shaping or policy.

**Reject** promoting the library's broad feature surface into NUIF support
without fixtures for each declared font category.

## Open questions

- Compare NUIF and browser-selected cmaps for symbol, format 13 and variation
  sequence cases before a broader profile.
- Add FreeType or an external implementation as a third oracle only with a
  pinned build and measured sandbox boundary.
