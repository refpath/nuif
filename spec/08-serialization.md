# 08 — Serialization and package format

Status: draft.

The NUIF logical model is encoding-independent.

Initial profiles:

- `nuif-text-0` — deterministic human-readable canonical form for fixtures/review.
- `nuif-cbor-0` — deterministic CBOR following draft-ietf-cbor-serialization §4.1 (preferred serialization) and §5.1 (bytewise-lexicographic map key order), with the narrowing rules of RFC 0005 stated by value.

A `.nuif` package is a container with a manifest, document records, optional resolved caches/correspondence records, and content-addressed assets. Exact archive/container technology remains experimental.

## Numeric and string rules (RFC 0005)

- Numeric kinds are `integer` (signed 64-bit) and `real` (binary64). Authored reals MUST be finite. Negative zero is not distinct from zero.
- In `nuif-cbor-0`, integral reals within the signed 64-bit range and both zeros MUST be encoded as integers; non-integral reals MUST use the shortest IEEE 754 width that round-trips; integer heads MUST be shortest; lengths MUST be definite; map keys MUST be strictly increasing in bytewise order of their encoding; no tags and no simple values other than `false`, `true` and `null` appear; extension and unknown-kind payloads are byte strings hashed verbatim.
- Decoders used for hashing and conformance MUST reject non-canonical input rather than re-canonicalize it.
- In `nuif-text-0`, reals print as the shortest round-trip decimal in the fixed layout of RFC 0005 rule 15; `NaN` and infinities are parse errors; keys are in UTF-8 byte order; layout is not significant.
- Identifiers (namespaces, keys, kind names, extension names) match `[a-z0-9][a-z0-9_.:-]*`. String values are stored verbatim as valid UTF-8 and are never normalized by canonicalization.

## Hash

The canonical hash of a document is SHA-256 over its `nuif-cbor-0` bytes. The text profile has no separate hash: `hash(text) = hash(cbor(parse(text)))`. Published content identifiers carry the profile identifier. Canonical hashes MUST exclude transport-only compression differences.

## Schema versions

Every serialized record kind carries a schema version. Migrations are registered pure functions per kind; reading a record whose version is newer than the implementation knows is an error with a diagnostic, never silent loss.

Parsers MUST enforce resource limits and reject cycles where the relevant graph is specified acyclic.
