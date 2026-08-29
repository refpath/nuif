---
id: nuif:research:svg
kind: standard
status: reviewed
title: SVG 2 vector graphics model
source:
  url: https://www.w3.org/TR/SVG2/
  authors: [W3C SVG Working Group]
  published_at: null
  license: W3C document license
retrieved_at: 2026-08-29
tags: [vector, geometry, paint, paths, interoperability]
confidence: 0.99
claims: []
relations: []
links:
  spec: [spec/05-geometry-paint-text.md]
  adr: []
  rfc: []
  code: [adapters/README.md]
  experiments: []
---
# Summary
SVG provides mature interoperable semantics for paths, transforms, paint servers, clipping, masks, compositing-related constructs and vector geometry.

## NUIF relevance
Borrow geometry/path mathematics and compatible paint semantics wherever practical. SVG remains a rendering/document format, not NUIF's semantic component/layout model, so SVG import may require inference and metadata for exact round trips.
