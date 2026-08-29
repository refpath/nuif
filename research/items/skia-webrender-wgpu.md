---
id: nuif:research:renderers
kind: repository
status: active
title: Skia, WebRender, Servo pipeline and wgpu rendering architectures
source:
  url: https://skia.org/docs/
  authors: [Skia, Servo, Mozilla, gfx-rs contributors]
  published_at: null
  license: BSD / MPL / MIT-APACHE project licenses
retrieved_at: 2026-08-29
tags: [renderer, display-list, gpu, webgpu, scene]
confidence: 0.98
claims: []
relations: []
links:
  spec: [spec/05-geometry-paint-text.md, spec/11-security.md]
  adr: [adrs/0003-reference-renderer.md]
  rfc: []
  code: [crates/nuif-render]
  experiments: []
---
# Summary
Skia demonstrates a broad mature 2D API across raster/GPU/PDF/SVG targets. Servo explicitly separates DOM/script, layout box/fragment trees, display-list generation and WebRender. wgpu supplies a safe cross-platform Rust API over Vulkan, Metal, D3D12 and WebGPU-class backends.

## NUIF relevance
The reference implementation should preserve a renderer-independent display/scene boundary. Interactive GPU rendering and normative conformance rendering can use different backends while sharing the same lowered scene semantics.
