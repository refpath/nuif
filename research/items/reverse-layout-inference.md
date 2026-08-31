---
id: nuif:research:reverse-layout-inference
kind: paper
status: reviewed
title: Reverse engineering flexible GUI layouts from observations
source:
  url: https://arxiv.org/abs/2202.11523
  authors: [Yue Jiang, Wolfgang Stuerzlinger, Christof Lutteroth]
  published_at: 2022-02-23
  license: arXiv distribution
retrieved_at: 2026-08-29
tags: [layout, inference, constraints, responsive, reverse-engineering]
confidence: 0.99
claims: [nuif:claim:authored-resolved]
relations:
  - type: extends
    target: nuif:research:cassowary
links:
  spec: [spec/04-layout.md, spec/09-provenance-and-fidelity.md]
  adr: []
  rfc: [rfcs/0003-authored-resolved-provenance.md]
  code: [crates/nuif-reconstruct/src/layout_inference.rs, crates/nuif-testing/src/bin/live-browser-capture.rs]
  experiments: [nuif:experiment:layout-inference]
---
# Summary
ReverseORC demonstrates that responsive layout intent can be inferred more reliably by sampling the same UI at multiple sizes and fitting flexible constraint specifications, rather than trying to infer a layout manager from one fixed screenshot. Earlier layout-inference work similarly uses relative-position graphs and graph rewriting to recover higher-level layout structures.

## NUIF relevance
Foreign imports should preserve observations and inference confidence. When authored intent is unavailable, adapters should infer from multiple evaluation contexts where possible and label reconstructed constraints as inferred rather than pretending they are lossless source semantics.

## Executable boundary

`nuif-layout-inference-0` is a deliberately bounded geometric implementation
of that research direction. It ranks five candidate families from 360/768 px
live-browser observations without consulting the 900 px holdout, then reports
the holdout error and all alternatives. On the current fixture its selected
constraint scores 0.0626 versus 0.2918 for fixed freeform. This is useful
falsification evidence for the mechanism, not evidence that the inferred
family is the author's original program or that the result generalizes to a
corpus. The report therefore retains raw uncalibrated confidence, source
observation identities and the `inferred` evidence class.
