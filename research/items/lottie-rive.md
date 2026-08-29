---
id: nuif:research:lottie-rive
kind: standard
status: active
title: Lottie and Rive portable animation/runtime models
source:
  url: https://lottiefiles.github.io/lottie-spec/
  authors: [Lottie Animation Community, Rive]
  published_at: null
  license: mixed project licenses
retrieved_at: 2026-08-29
tags: [animation, vector, state-machine, binary-format]
confidence: 0.97
claims: []
relations: []
links:
  spec: [spec/05-geometry-paint-text.md]
  adr: []
  rfc: []
  code: []
  experiments: []
---
# Summary
Lottie specifies a JSON-based animated-vector document with extensible additional data. Rive serializes artboards, shapes, animation and state machines into a compact binary runtime format designed for forward evolution.

## NUIF relevance
Animation/state-machine semantics should be modular rather than mixed into base geometry. NUIF should define stable behavior graph concepts and allow richer animation dialects to lower into them; it should not duplicate either runtime format wholesale.
