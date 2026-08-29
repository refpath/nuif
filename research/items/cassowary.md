---
id: nuif:research:cassowary
kind: paper
status: reviewed
title: Cassowary incremental linear constraint solving for user interfaces
source:
  url: https://constraints.cs.washington.edu/solvers/cassowary-tr.html
  authors: [Greg J. Badros, Alan Borning]
  published_at: 1998-06-01
  license: academic publication
retrieved_at: 2026-08-29
tags: [constraints, layout, solver, incremental]
confidence: 0.99
claims: [nuif:claim:authored-resolved]
relations: []
links:
  spec: [spec/04-layout.md]
  adr: []
  rfc: []
  code: [crates/nuif-layout]
  experiments: []
---
# Summary
Cassowary is an incremental dual-simplex constraint solver designed for UI equalities, inequalities and preferences. It supports relationships that are awkward to express as a single parent-owned flow algorithm.

## NUIF relevance
Constraint layout belongs as a distinct authored layout family rather than being forced into flex/grid. The core schema should model constraint identities, strengths and variables independently of one solver implementation.
