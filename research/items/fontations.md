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
    note: The former production parser was retired after RUSTSEC-2026-0192; a HarfBuzz golden now supplies external evidence.
links:
  spec: [spec/05-geometry-paint-text.md, spec/11-security.md]
  adr: [adrs/0003-reference-renderer.md]
  rfc: [rfcs/0010-portable-resource-package.md]
  code: [crates/nuif-font, crates/nuif-text, crates/nuif-testing/src/bin/font-resources.rs]
  experiments: [nuif:experiment:text-pinning, nuif:experiment:font-resource-static-baseline]
---

# Summary

Fontations is a safe Rust family for reading, writing and interpreting OpenType
fonts. `read-fonts` is its low-level no-copy/no-allocation reader; Skrifa adds
metadata, character maps, variation information and outlines. NUIF pins Skrifa
0.46.2 for both profile-zero outlines and static package-font metadata after
retiring `ttf-parser` for RUSTSEC-2026-0192. NUIF-owned sfnt validation remains
ahead of the library, and a committed HarfBuzz capture is the external metadata
oracle.

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

`nuif-font` constructs a Skrifa `FontRef` only after NUIF validates sfnt search
fields, table ordering, ranges, packing, padding and checksums. NUIF directly
reads required `head`, `maxp` and `OS/2` fields, requires metric agreement and
owns the conservative embedding-bit policy. The conformance executable compares
the resulting units, glyph count, family, table inventory and normalized
Unicode coverage with a digest-bound `hb-info` 14.4.0 capture before it runs
package and policy trials.

For an accepted static package resource, `nuif-text::ResourceFont` opens
HarfRust and Skrifa views over the same digest-checked bytes. It applies the
asset's global OpenType feature values during shaping, obtains the face ascent
from Skrifa metrics and reuses the opened face for unique glyph outlines within
each render item. The font gate exercises this path with Tinos in addition to
the independent Ahem metadata and shaping goldens.

## Alternatives and decision

Fontations replaces the unmaintained production parser because it is already
pinned for outlines, forbids unsafe code and maintains fuzzing infrastructure.
NUIF does not use Skrifa as its own independent oracle: a pinned HarfBuzz capture
provides external evidence, while direct sfnt reads catch disagreement in the
required fields. FreeType remains valuable as a future native third oracle and
browser stacks provide essential WOFF2 evidence, but neither is needed to
define the smallest static sfnt baseline.

## NUIF relevance

**Borrow** maintained metadata, character-map and outline access behind a
single exact version pin.

**Adapt** parser results through NUIF limits and exact semantic ranges. A font
parser does not own package resolution, shaping or policy.

**Reject** promoting the library's broad feature surface into NUIF support
without fixtures for each declared font category.

## Open questions

- Compare NUIF and browser-selected cmaps for symbol, format 13 and variation
  sequence cases before a broader profile.
- Add FreeType or an external implementation as a third oracle only with a
  pinned build and measured sandbox boundary.
