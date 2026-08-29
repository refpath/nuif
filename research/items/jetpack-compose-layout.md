---
id: nuif:research:jetpack-compose-layout
kind: standard
status: reviewed
title: Jetpack Compose constraint and single-pass layout model
source:
  url: https://developer.android.com/develop/ui/compose/layouts/basics
  authors: [Android Developers]
  published_at: null
  license: Android Developers Content License
retrieved_at: 2026-08-29
tags: [adapter, layout, jetpack-compose, constraints, code-generation]
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

Jetpack Compose lays out a UI tree by passing constraints to children,
measuring each child, deciding parent size and then placing children. Standard
layout prohibits measuring a child more than once; intrinsic measurement and
subcomposition are separate mechanisms.

## Evidence

- The layout basics define a single pass in which parents initiate measurement,
  constraints descend, and resolved sizes and placement instructions return up
  the tree. Measurement and placement are distinct sub-phases.
  https://developer.android.com/develop/ui/compose/layouts/basics#the-layout-model
  (retrieved 2026-08-29).
- Custom layout requires measuring children, deciding size and placing children.
  Compose rejects ordinary multi-pass child measurement.
  https://developer.android.com/develop/ui/compose/layouts/custom (retrieved
  2026-08-29).
- Modifier order changes the constraint and layout nodes wrapped around a
  composable. Equivalent visible output does not imply equivalent authored
  modifier structure.
  https://developer.android.com/develop/ui/compose/layouts/constraints-modifiers
  (retrieved 2026-08-29).

## NUIF relevance

**Borrow** bounded min/max constraints, intrinsic queries, row/column alignment
and the measurement/placement separation for a lowering profile.

**Adapt** NUIF stack/flex semantics into a generated, profile-owned Kotlin DSL
subset with stable identity comments or modifiers. Compile and screenshot tests
require a pinned Android Gradle Plugin, Compose version, SDK and font set.

**Reject** arbitrary Kotlin/Compose import as a lossless document operation.
Composable execution, state, modifier order, subcomposition and platform
resources require runtime evaluation and cannot be recovered from resolved
geometry alone.
