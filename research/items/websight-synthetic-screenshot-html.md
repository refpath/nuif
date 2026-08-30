---
id: nuif:research:websight-synthetic-screenshot-html
kind: dataset
status: reviewed
title: WebSight synthetic screenshot and HTML pairs
source:
  url: https://huggingface.co/blog/websight
  repository: https://huggingface.co/datasets/HuggingFaceM4/WebSight
  authors: [Hugo Laurençon, Léo Tronchon, Victor Sanh, Hugging Face M4]
  published_at: 2024-03-15
  license: dataset card and individual resource terms require versioned review
retrieved_at: 2026-08-30
tags: [dataset, synthetic-data, screenshot-to-code, html, pretraining, licensing]
confidence: 0.91
claims: [nuif:claim:evaluation-before-adaptation]
relations:
  - type: related_to
    target: nuif:research:pix2struct-screenshot-parsing-pretraining
  - type: compares_to
    target: nuif:research:design2code-real-world-benchmark
    note: WebSight is large synthetic training data; Design2Code is a smaller real-world evaluation set.
links:
  spec: [spec/09-provenance-and-fidelity.md]
  adr: []
  rfc: []
  code: []
  experiments: [nuif:experiment:reconstruction-distillation]
---

# Summary

WebSight provides large synthetic HTML/screenshot pairs for screenshot-to-code
training. Version 0.1 contained 823,000 synthetic pairs; version 0.2 expanded to
roughly two million and changed generation toward real images and Tailwind CSS.
Its scale makes it useful for pretraining or controlled ablations, but the
synthetic distribution and version-specific generation choices prevent it from
being the sole training or evaluation corpus for NUIF reconstruction.

## Evidence

- The official March 2024 project article describes v0.1 as 823,000 synthetic
  HTML/screenshot pairs and v0.2 as two million examples.
- The article states that v0.2 introduced real images in screenshots and
  switched the generated frontend style to Tailwind CSS.
- The paper record is arXiv:2403.09029. The authoritative dataset namespace is
  `HuggingFaceM4/WebSight`; mirrors are not equivalent provenance.
- The dataset is intended for screenshot-to-HTML fine-tuning, not recovery of
  original production sites or binary assets.

## Mechanism

Synthetic prompts and generated HTML/CSS are rendered by a browser to create
paired targets. This provides exact generated structure for each synthetic
screenshot and makes controlled perturbations possible. It also inherits the
generator's design patterns, framework choices, text distribution and resource
simplifications.

## NUIF relevance

**Borrow** the scale and exact synthetic pairing for perception pretraining and
render-loop ablations.

**Adapt** by generating a larger share of data directly from canonical NUIF so
every entity, operation, resource, layout context and provenance label is known.
Hold out entire templates, component families, fonts and visual themes.

**Reject** treating synthetic HTML as proof of real-world generalization,
training on unversioned mutable dataset snapshots, or assuming image pixels
identify the original asset bytes.

## Open questions

- What are the exact redistribution, image-source and generated-output terms of
  the pinned WebSight revision selected for any training run?
- Which visual/layout distributions are underrepresented compared with the
  project's target corpus?
- Does WebSight pretraining improve typed NUIF operations after controlling for
  model size and total training tokens?
