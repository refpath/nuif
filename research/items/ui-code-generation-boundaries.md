---
id: nuif:research:ui-code-generation-boundaries
kind: paper
status: active
title: Screenshot-to-code and interaction-inference limits
source:
  url: https://doi.org/10.1145/3729364
  authors: [Yuxuan Wan et al.]
  published_at: 2025-06-19
  license: ACM publication
retrieved_at: 2026-08-29
tags: [program-synthesis, screenshot-to-code, inference, fidelity, interaction]
confidence: 0.97
claims: [nuif:claim:sync-not-regenerate]
relations:
  - type: supports
    target: nuif:claim:authored-resolved
links:
  spec: [spec/09-provenance-and-fidelity.md]
  adr: []
  rfc: [rfcs/0003-authored-resolved-provenance.md]
  code: [adapters/README.md]
  experiments: [nuif:experiment:layout-inference]
---
# Summary
Recent screenshot-to-code research continues to report omission, distortion and arrangement errors even when visual similarity improves. Separate 2026 interaction-inference benchmarks show that visually reconstructing an interface does not imply recovering its behavior or state model.

## NUIF relevance
Pixels are evidence, not authored truth. NUIF importers may use program synthesis or multimodal inference, but inferred semantics, layout and behavior must carry confidence/provenance and cannot be classified as lossless without stronger source evidence.
