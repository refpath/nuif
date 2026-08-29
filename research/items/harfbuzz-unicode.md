---
id: nuif:research:harfbuzz-unicode
kind: standard
status: active
title: HarfBuzz shaping and Unicode text algorithms
source:
  url: https://harfbuzz.github.io/shaping-and-shape-plans.html
  authors: [HarfBuzz contributors, Unicode Consortium]
  published_at: null
  license: mixed open specifications
retrieved_at: 2026-08-29
tags: [text, shaping, unicode, fonts, line-breaking]
confidence: 0.99
claims: []
relations: []
links:
  spec: [spec/05-geometry-paint-text.md]
  adr: []
  rfc: []
  code: []
  experiments: []
---
# Summary
HarfBuzz maps Unicode code points to ordered glyph IDs, clusters, advances and offsets based on font data and shaping rules. Unicode UAX #9/#14/#29 define bidi, line-break and segmentation foundations, but actual line choice and font rendering remain higher-level concerns.

## NUIF relevance
Canonical documents store semantic text, runs, font references and layout intent. Shaped glyph runs are resolved/cache data tied to evaluator context and font hashes; they must not replace source text.
