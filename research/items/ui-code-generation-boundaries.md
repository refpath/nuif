---
id: nuif:research:ui-code-generation-boundaries
kind: paper
status: reviewed
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
  - type: related_to
    target: nuif:research:design2code-real-world-benchmark
  - type: related_to
    target: nuif:research:model-agnostic-screenshot-reconstruction-and-training
links:
  spec: [spec/09-provenance-and-fidelity.md, spec/14-observation-capture-and-reconstruction.md]
  adr: []
  rfc: [rfcs/0003-authored-resolved-provenance.md, rfcs/0011-observation-and-inference-provenance.md]
  code: [adapters/README.md]
  experiments: [nuif:experiment:layout-inference]
---
# Summary
DCGen divides a screenshot into smaller regions, describes and generates those
regions, then reassembles the result. Its reported improvement supports
hierarchical crops as an ablation, but it still generates a possible frontend
from pixels rather than recovering original source. Design2Code independently
shows persistent element-recall and layout errors on real webpages. Visually
reconstructing a static state also does not establish its interaction or state
model.

## Evidence

- Wan et al., DOI 10.1145/3729364 / arXiv:2406.16386, identifies omission,
  distortion and arrangement failures and reports up to a 14% visual-similarity
  improvement from divide-and-conquer generation on its studied models/data.
- DCGen segments the screenshot, generates descriptions and code for manageable
  regions and then integrates them; the reported percentage is not a NUIF
  benchmark or a guarantee on unseen models.
- Design2Code (ACL Anthology 2025.naacl-long.199) uses 484 real pages and also
  identifies element recall and layout as material weaknesses.

## Mechanism

Hierarchical segmentation preserves more local detail than a single resized
full-screen image and lets a model focus on smaller visual relationships. The
integration step must still reconcile coordinate spaces, shared styles,
hierarchy and responsive constraints. NUIF can make those joins explicit in an
observation graph and typed operations.

## NUIF relevance
Pixels are evidence, not authored truth. Hierarchical crops should be tested
against a deterministic/OCR baseline and one-shot proposal under identical
budgets. Inferred semantics, layout and behavior carry confidence/provenance and
cannot be classified as lossless without stronger source evidence.

## Open questions

- Which deterministic segmentation is stable enough across viewports to share
  proposed identities?
- Does local detail improve final typed structure or only pixel similarity?
- How should overlapping region proposals be merged without duplicate entities?
