---
id: nuif:research:gltf-interactivity
kind: standard
status: reviewed
title: KHR_interactivity portable behavior graphs
source:
  url: https://www.khronos.org/news/press/gltf-interactivity-extension-submitted-for-ratification
  authors: [Khronos Group]
  published_at: 2026-07-16
  license: Khronos specification terms
retrieved_at: 2026-08-29
tags: [behavior, interaction, state, graph, extensibility, security]
confidence: 0.99
claims: [nuif:claim:multi-level-ir]
relations:
  - type: extends
    target: nuif:research:gltf
  - type: related_to
    target: nuif:research:behavior-portability-state-machines
    note: The executable NUIF profile adopts bounded typed state and capability lessons without copying the complete Turing-capable graph.
links:
  spec: [spec/13-semantics-accessibility-and-behavior.md]
  adr: []
  rfc: []
  code: [crates/nuif-behavior/src/lib.rs, tools/behavior-oracle/check.mjs]
  experiments: [nuif:experiment:behavior-portability]
---
# Summary
KHR_interactivity adds portable, self-contained behavior graphs to glTF. Graph nodes model events, control flow, value operations and state while companion extensions add optional capabilities. The design explicitly considers constrained execution and graceful handling of unavailable companion operations.

## Evidence

The current specification is a Release Candidate. It defines directed acyclic
graphs, strictly typed value sockets, retained custom variables, static and
dynamic implementation limits, and no-op replacement for unsupported
extension operations. Its full runtime remains Turing-complete because
operations may repeatedly activate flow, so time and execution budgets are
still necessary. The 16 July 2026 announcement says it was submitted for
ratification; that does not make it a ratified extension.

## NUIF relevance
NUIF should use a separate behavior/state graph rather than embedding arbitrary scripts into visual nodes. The core behavior profile should remain declarative, bounded and capability-aware; richer runtime logic belongs in extensions or host application code.
