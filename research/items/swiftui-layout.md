---
id: nuif:research:swiftui-layout
kind: standard
status: reviewed
title: SwiftUI proposal-response layout model
source:
  url: https://developer.apple.com/documentation/swiftui/layout
  authors: [Apple]
  published_at: null
  license: proprietary documentation
retrieved_at: 2026-08-29
tags: [layout, intrinsic-size, proposal-response]
confidence: 0.97
claims: [nuif:claim:authored-resolved]
relations: []
links:
  spec: [spec/04-layout.md]
  adr: []
  rfc: []
  code: []
  experiments: []
---
# Summary
SwiftUI custom layout is based on parents proposing sizes, children reporting fitting sizes, and containers placing subviews. Minimum and maximum proposals can be probed rather than assuming fixed child dimensions.

## NUIF relevance
NUIF layout must represent intrinsic/proposal-based sizing, not only CSS flex/grid fields. A portable layout layer should express sizing intent and permit multiple evaluator families with defined lowering boundaries.
