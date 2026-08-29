---
id: nuif:research:tree-sitter
kind: repository
status: active
title: Tree-sitter incremental concrete syntax trees
source:
  url: https://tree-sitter.github.io/tree-sitter/
  authors: [Tree-sitter contributors]
  published_at: null
  license: MIT
retrieved_at: 2026-08-29
tags: [source-code, incremental-parsing, syntax-tree, patching]
confidence: 0.99
claims: [nuif:claim:sync-not-regenerate]
relations: []
links:
  spec: [spec/09-provenance-and-fidelity.md, spec/12-cli-api-and-automation.md]
  adr: []
  rfc: [rfcs/0003-authored-resolved-provenance.md]
  code: [adapters/README.md]
  experiments: []
---
# Summary
Tree-sitter maintains concrete syntax trees incrementally and can reuse unchanged structure after precisely described text edits. It preserves source ranges and exposes changed ranges between trees.

## NUIF relevance
Source adapters need concrete-syntax-aware minimal editing rather than AST regeneration. Tree-sitter is a strong parser substrate, but formatting/comment-preserving patch generation remains adapter-specific and may require language-native tooling for some frameworks.
