---
id: nuif:research:automerge-yjs
kind: repository
status: reviewed
title: Automerge and Yjs CRDT architectures
source:
  url: https://github.com/automerge/automerge
  authors: [Automerge contributors, Yjs contributors]
  published_at: null
  license: MIT / project licenses
retrieved_at: 2026-08-29
tags: [crdt, collaboration, local-first, sync]
confidence: 0.97
claims: [nuif:claim:collab-profile]
relations: []
links:
  spec: [spec/10-collaboration-profile.md]
  adr: [adrs/0005-collaboration-profile.md]
  rfc: []
  code: []
  experiments: []
---
# Summary
Automerge provides Rust-backed CRDT data structures, compact change encoding and sync protocols for local-first applications. Yjs uses a modified YATA-style sequence CRDT with state-vector-based differential synchronization.

## NUIF relevance
CRDTs are suitable for a collaboration profile and operation history, but their implementation-specific metadata should not become mandatory content of every canonical NUIF document. Saved documents must remain portable to non-collaborative implementations.
