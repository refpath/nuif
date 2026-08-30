---
id: nuif:research:vello
kind: repository
status: reviewed
title: Vello Rust 2D renderer
source:
  url: https://github.com/linebender/vello
  authors: [Linebender contributors]
  published_at: null
  license: Apache-2.0 OR MIT
retrieved_at: 2026-08-29
tags: [renderer, gpu, vector, rust, wgpu]
confidence: 0.96
claims: []
relations: []
links:
  spec: [spec/05-geometry-paint-text.md]
  adr: [adrs/0003-reference-renderer.md]
  rfc: []
  code: [crates/nuif-render]
  experiments: []
---
# Summary
Vello is a Rust GPU-compute-centric 2D renderer using wgpu. Its scene abstraction covers vector shapes, images, gradients and text-oriented drawing, while current releases still document evolving APIs and some incomplete effect/glyph areas.

## NUIF relevance
Use Vello as an implementation experiment, not a normative dependency. Maintain a renderer trait and conformance raster path so the draft specification's visual semantics remain independent of Vello's evolution.
