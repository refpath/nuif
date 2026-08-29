---
id: nuif:research:flutter-layout
kind: standard
status: reviewed
title: Flutter box-constraint layout model
source:
  url: https://docs.flutter.dev/ui/layout/constraints
  authors: [Flutter team]
  published_at: null
  license: CC-BY-4.0 for Flutter documentation
retrieved_at: 2026-08-29
tags: [adapter, layout, flutter, constraints, code-generation]
confidence: 0.98
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

Flutter's box layout passes constraints from parent to child, returns sizes from
child to parent and assigns child positions in the parent. The standard box
layout is one pass and each render object chooses a size within its incoming
constraints.

## Evidence

- The Flutter constraint guide states the processing order as constraints down,
  sizes up and parent-assigned positions. Width and height requests can be
  constrained or ignored by ancestors.
  https://docs.flutter.dev/ui/layout/constraints (retrieved 2026-08-29).
- The guide identifies the one-pass limitation: a box can choose only within
  parent constraints, does not choose its global position and cannot determine
  geometry independently of the tree.
  https://docs.flutter.dev/ui/layout/constraints#limitations (retrieved
  2026-08-29).

## NUIF relevance

**Borrow** `BoxConstraints`-compatible min/max sizing, row/column flex and
parent-relative placement for a bounded lowering profile.

**Adapt** NUIF entities into a generated, profile-owned Dart widget subset.
Stable identity metadata and source spans are required for reconciliation.
Conformance requires a pinned Flutter engine, platform, pixel ratio and fonts.

**Reject** arbitrary Dart widget-tree import. Builders, state, inherited
widgets, custom render objects, assets and platform plugins are executable
semantics outside profile zero.
