---
id: nuif:research:figma
kind: standard
status: reviewed
title: Figma public plugin/document node model
source:
  url: https://developers.figma.com/docs/plugins/api/nodes/
  authors: [Figma]
  published_at: null
  license: proprietary API documentation
retrieved_at: 2026-08-29
tags: [adapter, scene-graph, components, variables, layout]
confidence: 0.97
claims: []
relations: []
links:
  spec: []
  adr: []
  rfc: []
  code: [adapters/README.md]
  experiments: []
---
# Summary
Figma publicly exposes a hierarchical document node model through its plugin and REST surfaces, including frames, components, instances, vectors, text, layout properties and variables. Reverse-engineered `.fig` internals exist, but public APIs are the stable integration boundary.

## NUIF relevance
Treat Figma strictly as an adapter target. Public node semantics are valuable mapping evidence; undocumented file/multiplayer protocols must not become normative dependencies.
