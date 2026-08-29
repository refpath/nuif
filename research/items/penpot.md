---
id: nuif:research:penpot
kind: repository
status: active
title: Penpot open design data model and file format
source:
  url: https://help.penpot.app/technical-guide/developer/data-model/
  authors: [Penpot]
  published_at: null
  license: MPL-2.0 for implementation
retrieved_at: 2026-08-29
tags: [design-editor, svg, shapes, portability]
confidence: 0.98
claims: [nuif:claim:authored-resolved]
relations: []
links:
  spec: [spec/01-model.md, spec/05-geometry-paint-text.md]
  adr: []
  rfc: []
  code: [adapters/README.md]
  experiments: []
---
# Summary
Penpot models a design primarily as shapes corresponding to SVG nodes augmented with constraints, interactions and editor metadata. Its open `.penpot` package stores inspectable JSON plus assets. Penpot can exactly reconstruct its own SVG exports using metadata and best-effort infer arbitrary SVG.

## NUIF relevance
Strong precedent for open inspectable design documents and metadata-assisted round-trip. NUIF should not make `Shape` the universal top-level semantic abstraction; semantic entities and relationships need to exist independently of vector representation.
