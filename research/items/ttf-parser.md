---
id: nuif:research:ttf-parser
kind: implementation
status: verified
title: ttf-parser safe zero-allocation OpenType inspection
source:
  url: https://github.com/harfbuzz/ttf-parser
  authors: [ttf-parser contributors]
  published_at: "ttf-parser 0.25.1"
  license: MIT OR Apache-2.0
retrieved_at: 2026-08-30
tags: [fonts, opentype, rust, parser, safety, wasm]
confidence: 0.98
claims: [nuif:claim:bounded-untrusted-input]
relations:
  - type: related_to
    target: nuif:research:opentype-font-embedding-and-portability
    note: Parses technical font facts; it does not decide redistribution rights.
  - type: compares_to
    target: nuif:research:fontations
    note: NUIF uses a separately implemented parser as its differential oracle.
links:
  spec: [spec/05-geometry-paint-text.md, spec/11-security.md]
  adr: []
  rfc: [rfcs/0010-portable-resource-package.md]
  code: [crates/nuif-font, crates/nuif-package]
  experiments: [nuif:experiment:font-resource-static-baseline]
---

# Summary

`ttf-parser` is a safe, stateless OpenType parser with configurable table
features, no unsafe code and no heap allocation in its parsing API. NUIF pins
0.25.1 with only `std` enabled and places its own sfnt directory, byte, table,
range, packing and checksum checks in front of the higher-level face metadata.

## Evidence

- The project describes zero unsafe code, zero parser allocations, immutable
  access, checked arithmetic and bounded recursion. Locator: repository
  `README.md`, “Features” and “Safety”, retrieved 2026-08-30.
- Its default features include layout, Apple and variable-font support; each is
  independently selectable. Locator: repository `Cargo.toml`, `[features]`,
  retrieved 2026-08-30.
- The 0.25.1 crate is pinned by exact version and checksum in `Cargo.lock`.

## Mechanism

`nuif-font` rejects out-of-profile bytes using a small independent sfnt
directory reader before it constructs `ttf_parser::Face`. It then derives names,
coverage and embedding signals from immutable borrowed bytes. `nuif-package`
compares those results with the semantic `FontAsset` during embedded package
validation and after explicit linked-resource resolution.

## Alternatives and decision

FreeType and HarfBuzz are mature native choices, but both would introduce a C
ABI and a materially larger execution surface for a metadata-only package
gate. `read-fonts`/Skrifa is the strongest safe Rust alternative and supports a
broader modern table surface. Using it as the independent oracle gives more
evidence than routing both sides through one parser family.

The production profile therefore uses `ttf-parser` only for the deliberately
narrow static TrueType subset. It does not use parser acceptance as the format
boundary and does not enable unused variable/layout features.

## NUIF relevance

**Borrow** safe immutable face inspection and embedding-flag interpretation.

**Adapt** with independent directory/checksum validation, explicit NUIF limits,
exact asset comparison and policy evidence.

**Reject** treating successful parsing, a family name or `fsType` alone as
permission to distribute a font.

## Open questions

- A broad profile needs a corpus that includes TTC, CFF/CFF2, variable, color,
  bitmap, WOFF2 and historic `OS/2` edge cases.
- Fuzzing the NUIF wrapper and exercising its configured feature subset remain
  separate from upstream parser fuzzing.
