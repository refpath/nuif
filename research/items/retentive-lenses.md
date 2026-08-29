---
id: nuif:research:retentive-lenses
kind: paper
status: active
title: Retentive Lenses
source:
  url: https://arxiv.org/abs/2001.02031
  authors: [Zirun Zhu, Zhixuan Yang, Hsiang-Shang Ko, Zhenjiang Hu]
  published_at: 2020-01-07
  license: arXiv distribution
retrieved_at: 2026-08-29
tags: [bidirectional-transformations, lenses, provenance, synchronization]
confidence: 0.99
claims: [nuif:claim:sync-not-regenerate]
relations: []
links:
  spec: [spec/09-provenance-and-fidelity.md]
  adr: []
  rfc: [rfcs/0003-authored-resolved-provenance.md]
  code: [crates/nuif-protocol]
  experiments: []
---
# Summary
Retentive lenses strengthen ordinary lens laws by requiring unchanged regions of a view to preserve corresponding source regions when other parts change. The work demonstrates tree transformation and resugaring use cases where provenance/correspondence enables minimal retention.

## NUIF relevance
This is a direct theoretical foundation for design↔source synchronization. NUIF adapters should maintain correspondence maps and source provenance so an edit to one property does not regenerate unrelated source regions.
