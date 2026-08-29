---
id: nuif:research:taffy
kind: repository
status: active
title: Taffy Rust CSS layout engine
source:
  url: https://github.com/DioxusLabs/taffy
  authors: [Taffy contributors]
  published_at: null
  license: MIT
retrieved_at: 2026-08-29
tags: [layout, css, flexbox, grid, rust]
confidence: 0.98
claims: [nuif:claim:authored-resolved]
relations: []
links:
  spec: [spec/04-layout.md]
  adr: [adrs/0002-layout-engine.md]
  rfc: []
  code: [crates/nuif-layout]
  experiments: []
---
# Summary
Taffy is a Rust layout library implementing CSS Block, Flexbox and Grid algorithms and is embedded by Servo, Bevy, Slint and other systems. Its traitified style boundary is useful for integrating a separate authored model with a standards-derived evaluator.

## NUIF relevance
Recommended initial evaluator for CSS-compatible layout families, but not the canonical NUIF layout model. NUIF needs a superset vocabulary and explicit lowering/loss reports for unsupported CSS features and non-CSS layout families.
