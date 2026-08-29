---
id: nuif:research:structured-merge
kind: paper
status: active
title: Structured differencing and three-way merge
source:
  url: https://doi.org/10.1016/j.sysarc.2023.103011
  authors: [Mastery structured merge researchers]
  published_at: 2023-12-01
  license: publisher controlled
retrieved_at: 2026-08-29
tags: [diff, merge, ast, graph, version-control]
confidence: 0.96
claims: [nuif:claim:sync-not-regenerate]
relations: []
links:
  spec: [spec/06-operations-and-patches.md]
  adr: []
  rfc: []
  code: [crates/nuif-protocol]
  experiments: []
---
# Summary
Structured merge research shows AST-aware mappings and top-down/bottom-up strategies can reduce false conflicts compared with line merge, particularly when elements move. Recent 2026 work also argues for explicit correctness properties for structural merge.

## NUIF relevance
Stable entity identity gives NUIF an advantage over inferred AST matching. Three-way merge should operate over semantic operations and graph relationships while preserving a textual fallback for human review.
