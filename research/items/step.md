---
id: nuif:research:step
kind: standard
status: reviewed
title: ISO 10303 STEP product-data exchange architecture
source:
  url: https://www.iso.org/standard/83105.html
  authors: [ISO TC 184/SC 4]
  published_at: 2024-01-01
  license: ISO
retrieved_at: 2026-08-29
tags: [interchange, lifecycle, schema, cad]
confidence: 0.96
claims: []
relations: []
links:
  spec: []
  adr: []
  rfc: []
  code: []
  experiments: []
---
# Summary
STEP is a family of product-information representation and exchange standards designed to survive exchange among heterogeneous systems across a product lifecycle, with explicit data specification methods and modular parts.

## NUIF relevance
The lesson is organizational: a durable interchange specification needs modular normative parts, conformance classes and schema discipline. Avoid STEP's complexity explosion by aggressively constraining the NUIF core and using profiles/extensions.
