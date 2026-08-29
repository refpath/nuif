---
id: nuif:research:yoga
kind: repository
status: active
title: Yoga embeddable Flexbox layout engine
source:
  url: https://github.com/facebook/yoga
  authors: [Meta, Yoga contributors]
  published_at: null
  license: MIT
retrieved_at: 2026-08-29
tags: [layout, flexbox, native, cplusplus]
confidence: 0.98
claims: []
relations:
  - type: compares_to
    target: nuif:research:taffy
links:
  spec: [spec/04-layout.md]
  adr: [adrs/0002-layout-engine.md]
  rfc: []
  code: []
  experiments: []
---
# Summary
Yoga is an embeddable C++20 Flexbox-focused layout engine with broad language/platform bindings. It demonstrates the value of a small portable layout runtime used outside browsers.

## NUIF relevance
Yoga is a differential/reference target for Flexbox semantics and a reminder that adapter/runtime portability matters. Taffy remains preferable for the Rust reference implementation because NUIF also needs Grid and native Rust integration.
