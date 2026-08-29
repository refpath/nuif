---
id: nuif:research:symmetric-lenses
kind: paper
status: active
title: Symmetric Lenses
source:
  url: https://doi.org/10.1145/1925844.1926428
  authors: [Martin Hofmann, Benjamin C. Pierce, Daniel Wagner]
  published_at: 2011-01-26
  license: ACM
retrieved_at: 2026-08-29
tags: [bidirectional-transformations, lenses, synchronization]
confidence: 0.99
claims: [nuif:claim:sync-not-regenerate]
relations: []
links:
  spec: [spec/09-provenance-and-fidelity.md]
  adr: []
  rfc: []
  code: []
  experiments: []
---
# Summary
Symmetric lenses model synchronization where both sides may contain information absent from the other, avoiding a permanently privileged source/view direction.

## NUIF relevance
Design files and source frameworks are peers with asymmetric capabilities. Synchronization should therefore be symmetric at the system level even when individual adapters implement directional lowering/lifting passes.
