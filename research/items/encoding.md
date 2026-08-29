---
id: nuif:research:encoding
kind: standard
status: active
title: Deterministic CBOR, Protobuf unknown fields and Kiwi schema evolution
source:
  url: https://www.rfc-editor.org/rfc/rfc8949.html
  authors: [IETF, Google, Evan Wallace]
  published_at: 2020-12-01
  license: mixed open specifications
retrieved_at: 2026-08-29
tags: [serialization, deterministic-encoding, schema-evolution, unknown-fields]
confidence: 0.98
claims: [nuif:claim:opaque-preservation]
relations: []
links:
  spec: [spec/08-serialization.md]
  adr: [adrs/0004-serialization.md]
  rfc: [rfcs/0002-extension-preservation.md]
  code: [crates/nuif-codec]
  experiments: []
---
# Summary
RFC 8949 defines deterministic CBOR encoding profiles suitable for hashing and reproducible binary forms. Protobuf demonstrates mature field-number evolution and binary unknown-field preservation but warns that JSON conversion loses unknown fields. Kiwi demonstrates schema-bundled forward decoding and compact tree serialization.

## NUIF relevance
Separate logical schema from encoding. Use a canonical human-readable form for review/spec fixtures plus deterministic CBOR as the first binary/wire encoding. Unknown extensions must be represented explicitly rather than relying solely on codec-specific unknown field behavior.
