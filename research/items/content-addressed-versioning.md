---
id: nuif:research:content-addressed-versioning
kind: standard
status: active
title: Content-addressed Merkle DAGs for immutable resources and snapshots
source:
  url: https://docs.ipfs.tech/concepts/merkle-dag/
  authors: [IPFS Project]
  published_at: null
  license: project documentation license
retrieved_at: 2026-08-29
tags: [content-addressing, merkle-dag, assets, snapshots, versioning]
confidence: 0.98
claims: []
relations: []
links:
  spec: [spec/02-identity-and-properties.md, spec/08-serialization.md]
  adr: []
  rfc: []
  code: []
  experiments: [nuif:experiment:codec-benchmark]
---
# Summary
Merkle DAGs assign immutable nodes identifiers derived from their contents and referenced children. This provides verifiable immutable snapshots and deduplication but changes identity whenever content changes.

## NUIF relevance
Use content hashes for immutable assets, packages and canonical snapshots, but not for editable semantic entity identity. Stable entity IDs and content-addressed snapshot/resource IDs solve different problems and must remain separate.
