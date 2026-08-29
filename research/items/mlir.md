---
id: nuif:research:mlir
kind: standard
status: active
title: MLIR multi-level intermediate representation and dialect conversion
source:
  url: https://mlir.llvm.org/docs/LangRef/
  authors: [LLVM Project]
  published_at: null
  license: LLVM Project license
retrieved_at: 2026-08-29
tags: [ir, dialects, lowering, transformation]
confidence: 0.98
claims: [nuif:claim:multi-level-ir]
relations:
  - type: inspired_by
    target: nuif:architecture:dialects
links:
  spec: [spec/07-extensions-and-dialects.md]
  adr: []
  rfc: [rfcs/0001-multi-level-document-model.md]
  code: []
  experiments: []
---
# Summary
MLIR deliberately supports multiple abstraction levels and domain-specific dialects in one framework, with explicit legality targets and rewrite-based lowering. Its text, in-memory and compact serialized representations demonstrate that logical IR semantics need not be tied to a single storage encoding.

## Evidence
Primary references: MLIR Language Reference, Dialect Conversion documentation, and MLIR rationale. Dialect conversion separates conversion targets, rewrite patterns and type conversion.

## NUIF relevance
Borrow the ideas of dialect namespaces, explicit lowering passes, validation and partial legality. Do not copy SSA/control-flow machinery: NUIF is an authored document model, not compiler code IR.
