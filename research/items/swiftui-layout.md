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
  code: [adapters/STATUS.md]
  experiments: []
---
# Summary

SwiftUI custom layout uses proposal–response measurement followed by subview
placement. A `Layout` implementation receives proxy values rather than direct
subviews and can query dimensions, spacing, priority and custom layout values.

## Evidence

- The `Layout` protocol requires
  `sizeThatFits(proposal:subviews:cache:)` and
  `placeSubviews(in:proposal:subviews:cache:)`. Optional methods provide
  alignment, spacing, axis properties and caches.
  https://developer.apple.com/documentation/swiftui/layout (retrieved
  2026-08-29).
- `LayoutSubview.sizeThatFits(_:)` accepts a proposed size. SwiftUI views choose
  a size while considering the parent proposal; the result is not a CSS-style
  fixed-width declaration.
  https://developer.apple.com/documentation/swiftui/layoutsubview/sizethatfits(_:)
  (retrieved 2026-08-29).
- `ViewThatFits` can choose the first child that fits the proposal. Custom
  layout examples share measurements between sizing and placement through a
  cache.
  https://developer.apple.com/documentation/swiftui/composing-custom-layouts-with-swiftui
  (retrieved 2026-08-29).

## NUIF relevance

**Borrow** proposal–response sizing, intrinsic probes, stack alignment, spacing
and explicit separation between sizing and placement.

**Adapt** profile-zero stacks and fixed/intrinsic/fill sizing into a generated,
profile-owned Swift subset. Compiler, SDK, operating-system version, dynamic
type, locale and font registry are evaluation-context provenance.

**Reject** arbitrary SwiftUI import as a lossless document operation. Result
builders, modifiers, state, environment values, custom `Layout` types and
platform views are executable semantics.
