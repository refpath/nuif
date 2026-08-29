# 08 — Serialization and package format

Status: draft.

The NUIF logical model is encoding-independent.

Initial profiles:

- `nuif-text-0` — deterministic human-readable canonical form for fixtures/review.
- `nuif-cbor-0` — deterministic CBOR using RFC 8949 deterministic encoding requirements.

A `.nuif` package is a container with a manifest, document records, optional resolved caches/correspondence records, and content-addressed assets. Exact archive/container technology remains experimental.

Canonical hashes MUST exclude transport-only compression differences and MUST define numeric normalization, map ordering and string normalization rules.

Parsers MUST enforce resource limits and reject cycles where the relevant graph is specified acyclic.
