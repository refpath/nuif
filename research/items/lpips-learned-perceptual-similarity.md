---
id: nuif:research:lpips-learned-perceptual-similarity
kind: paper
status: reviewed
title: LPIPS learned perceptual image similarity
source:
  url: https://openaccess.thecvf.com/content_cvpr_2018/html/Zhang_The_Unreasonable_Effectiveness_CVPR_2018_paper.html
  authors: [Richard Zhang, Phillip Isola, Alexei A. Efros, Eli Shechtman, Oliver Wang]
  published_at: 2018-06
  license: CVPR publication; implementation and weights have separate terms
retrieved_at: 2026-08-30
tags: [lpips, perceptual-metric, image-difference, evaluation, learned-features]
confidence: 0.97
claims: [nuif:claim:evaluation-before-adaptation]
relations:
  - type: compares_to
    target: nuif:research:ssim-and-classical-image-metrics
  - type: related_to
    target: nuif:research:flip-perceptual-difference-metric
links:
  spec: [spec/00-conformance.md]
  adr: [adrs/0003-reference-renderer.md]
  rfc: []
  code: []
  experiments: [nuif:experiment:reconstruction-closed-loop]
---

# Summary

LPIPS compares normalized deep feature activations and was calibrated/evaluated
against human perceptual judgments on image distortions. It complements raw
pixel and classical structural metrics, but it is model- and weight-dependent
and does not measure UI structure, text correctness or editability.

## Evidence

- CVPR 2018 defines distance as a spatial average of weighted squared distances
  between normalized feature activations across network layers.
- The paper evaluates linear calibration, full tuning and training-from-scratch
  variants on perceptual judgments; metric identity includes the backbone and
  weights, not merely the label “LPIPS.”
- The study concerns image-patch perceptual similarity. It does not establish a
  threshold for UI reconstruction or resistance to metric gaming.

## Mechanism

Two images are passed through the same fixed feature network. Per-layer
activations are channel-normalized, optionally channel-weighted, compared and
spatially averaged. A reproducible report must pin preprocessing, resolution,
backbone, weights, library version and reduction.

## NUIF relevance

**Borrow** LPIPS as one non-normative visual diagnostic in a metric ensemble.

**Adapt** thresholds only after correlation with human UI judgments and
property-level errors is measured. Report it beside raw pixel difference, FLIP,
SSIM, text/geometry/structure/resource metrics.

**Reject** LPIPS as the sole reward or correctness boundary; a full-page
screenshot embedded as one image could score well while containing no editable
semantics.

## Open questions

- Which backbone and preprocessing correlate best with UI differences after
  controlling for text antialiasing?
- How susceptible is the selected metric to adversarial or degenerate
  reconstructions in the operation-search loop?
- Does it add ranking value beyond FLIP and property-level measures?
