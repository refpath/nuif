---
id: nuif:research:openusd
kind: standard
status: reviewed
title: OpenUSD composition, layers, references and variants
source:
  url: https://openusd.org/release/intro.html
  authors: [Pixar, Alliance for OpenUSD]
  published_at: null
  license: Apache-2.0
retrieved_at: 2026-08-29
tags: [composition, layers, variants, scene-graph, overrides]
confidence: 0.98
claims: [nuif:claim:multi-level-ir]
relations: []
links:
  spec: [spec/03-components-and-composition.md]
  adr: []
  rfc: [rfcs/0001-multi-level-document-model.md]
  code: []
  experiments: []
---
# Summary
OpenUSD composes scene description from ordered layers and composition arcs including references, inherits, variants, payloads and specializes. The key lesson is non-destructive composition: authored opinions remain separate while a stage resolves a composed view.

## Evidence
OpenUSD terminology and composition documentation define composition arcs as operators combining layer stacks and prim specifications into resolved values.

## NUIF relevance
Adapt layer/reference/variant concepts for design-system libraries, themes, brands, responsive projections and local overrides. Avoid inheriting USD's 3D-specific namespace and asset assumptions.
