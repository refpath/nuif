---
id: nuif:spec:serialization
kind: specification
status: draft
---

# 08 — Serialization and package format

Status: draft.

The NUIF logical model is encoding-independent.

Initial profiles:

- `nuif-text-0` — deterministic human-readable canonical form for fixtures/review.
- `nuif-cbor-0` — deterministic CBOR following draft-ietf-cbor-serialization §4.1 (preferred serialization) and §5.1 (bytewise-lexicographic map key order), with the narrowing rules of RFC 0005 stated by value.

The experimental `nuif-package-0` profile assigns `.nuif` to a deterministic ZIP
container. Bare encodings use `.nuif.json` and `.nuif.cbor`. Historical alpha
files that used `.nuif` for bare bytes MAY be recognized read-only through
content detection, but new `.nuif` output MUST be a package once this profile is
accepted.

`nuif-package-0` is proposed by RFC 0010. Its reference codec, cross-writer byte
fixture, package/resource identity relations, explicit resolver and hostile
archive/one-over suite are executable through `cargo xtask gate-i-package`.
This is not full Gate I: independent PNG interpretation, OpenType policy and
cross-platform/external writer evidence remain incomplete.

## Numeric and string rules (RFC 0005)

- Numeric kinds are `integer` (signed 64-bit) and `real` (binary64). Authored reals MUST be finite. Negative zero is not distinct from zero.
- In `nuif-cbor-0`, integers use major type 0 or 1 and reals use the shortest IEEE 754 floating-point width that round-trips, including integral reals. Integer and real are distinct logical values. Both real zeros use positive floating-point zero; integer zero remains distinct. Integer heads MUST be shortest; lengths MUST be definite; map keys MUST be strictly increasing in bytewise order of their complete deterministic encoding; no tags and no simple values other than `false`, `true` and `null` appear; extension and unknown-kind payloads are byte strings hashed verbatim (RFC 0008).
- Decoders used for hashing and conformance MUST reject non-canonical input rather than re-canonicalize it.
- In `nuif-text-0`, reals print as the shortest round-trip decimal in the fixed layout of RFC 0005 rule 15; `NaN` and infinities are parse errors; keys are in UTF-8 byte order; layout is not significant. Text key order is intentionally independent from CBOR encoded-key order (RFC 0008).
- Identifiers (namespaces, keys, kind names, extension names) match `[a-z0-9][a-z0-9_.:-]*`. String values are stored verbatim as valid UTF-8 and are never normalized by canonicalization.

## Hash

The canonical hash of a document is SHA-256 over its `nuif-cbor-0` bytes. The text profile has no separate hash: `hash(text) = hash(cbor(parse(text)))`. Published content identifiers carry the profile identifier. Canonical hashes MUST exclude transport-only compression differences.

Package, resource and semantic hashes are distinct:

- `document_hash` is SHA-256 of canonical `document.cbor` and covers semantic
  asset bindings;
- `resource_digest` is SHA-256 of exact resource bytes;
- `package_hash` is SHA-256 of the complete deterministic package bytes.

Package-only caches or reports may change `package_hash` without changing
`document_hash`. Replacing a resource bound to an asset changes the resource
digest and semantic document hash while preserving stable `AssetId`.

## Proposed package profile 0

The package member set is:

```text
mimetype
manifest.cbor
document.cbor
blobs/sha256/<digest-hex>
```

The first member is stored `mimetype` with exact ASCII value
`application/nuif+zip`. This media type remains provisional until registration.
`manifest.cbor` and `document.cbor` are canonical `nuif-cbor-0`.

The manifest declares package profile/version, the canonical document
descriptor, required capabilities, stable assets and every immutable resource
descriptor. A descriptor includes media type, SHA-256 digest, size, role and an
embedded or explicit linked locator. The manifest is not self-addressed.

Profile 0 uses stored ZIP members only; `mimetype` is first and other names
are bytewise sorted. Names are exact ASCII registered paths. Writers use fixed
timestamps/header attributes, no comments/extra fields/data descriptors,
encryption, directories, ZIP64 or split archives. The manual reference writer
and `zip` 8.6.0 independently reproduce the exact header fixture.

Readers MUST reject duplicate decoded names, non-ASCII/backslash/absolute/dot
paths, directories, symlinks, encryption, unsupported compression, inconsistent
headers, unknown members, undeclared blobs, missing required blobs and
size/digest mismatches. Readers MUST NOT extract package members to a filesystem.

Portable packages embed every resource required by their declared profile.
Linked resources are explicit and never fetched implicitly; a caller-supplied
resolver verifies expected size and digest before use. Credentials MUST NOT be
stored in resource locators.

## Schema versions

Every serialized record kind carries a schema version. Migrations are registered pure functions per kind; reading a record whose version is newer than the implementation knows is an error with a diagnostic, never silent loss.

Parsers MUST enforce resource limits and reject cycles where the relevant graph is specified acyclic. The experimental package limits are 80 MiB per archive, 32 MiB per resource, 64 MiB total embedded resources and 8,192 descriptors. Image interpretation and general font-policy limits remain experiment-required and MUST be accepted through later media profiles before implementations claim those capabilities.
