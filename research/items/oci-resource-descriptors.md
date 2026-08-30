---
id: nuif:research:oci-resource-descriptors
kind: standard
status: reviewed
title: OCI content descriptors and verification-before-consumption
source:
  url: https://github.com/opencontainers/image-spec/blob/main/descriptor.md
  repository: https://github.com/opencontainers/image-spec
  authors: [Open Container Initiative]
  published_at: null
  license: Apache-2.0
retrieved_at: 2026-08-30
tags: [content-addressing, descriptor, digest, media-type, size, verification]
confidence: 0.99
claims: [nuif:claim:resource-identity-separation, nuif:claim:bounded-untrusted-input]
relations:
  - type: extends
    target: nuif:research:content-addressed-versioning
    note: Adds a concrete media-type, digest and size descriptor with verification ordering.
  - type: related_to
    target: nuif:research:ipld-dag-cbor-strictness
links:
  spec: [spec/08-serialization.md, spec/11-security.md]
  adr: [adrs/0004-serialization.md]
  rfc: [rfcs/0009-profile-zero-resource-budgets.md]
  code: []
  experiments: [nuif:experiment:portable-package-resources]
---

# Summary

The OCI Image Specification uses descriptors to identify arbitrary byte
content by media type, digest and size. Optional URLs are retrieval hints, not
identity. A consumer verifies size and digest before expensive interpretation.
This is a stronger resource-reference pattern for NUIF than paths or URLs alone.

## Evidence

- OCI Image Specification `descriptor.md`, *Properties*, defines `mediaType`,
  `digest` and `size` as the required descriptor fields; `urls`, annotations and
  platform information are optional.
- *Digests* defines the digest as a content identifier calculated from the
  exact bytes. Implementations must support SHA-256 verification; its encoding
  is lowercase hexadecimal.
- *Verification* says content from untrusted sources should have its size
  checked and digest recalculated before consumption, and advises against heavy
  processing before verification.
- URLs do not replace the descriptor digest. The bytes retrieved from any
  location still have to satisfy the declared descriptor.

## Mechanism

```text
ResourceDescriptor {
  media_type: string,
  digest: "sha256:" + 64 lowercase hex digits,
  size: u64,
  locations: [package path or explicitly permitted external locator],
}

resolve location -> enforce declared/implementation size -> hash bytes
                 -> compare digest -> dispatch by media type
```

Semantic assets refer to a stable asset identity. The asset refers to one
immutable resource descriptor. Replacing its content updates that binding but
does not require replacing every semantic reference to the asset.

## NUIF relevance

**Borrow** the required descriptor triple and verification ordering. Digest
and declared size are also useful before archive expansion, image decode and
font parsing.

**Adapt** optional URLs into a resolver policy that is disabled by default.
Package paths are normalized locators only; an implementation must compare the
bytes to the descriptor regardless of where they were found.

**Reject** OCI image layering, registries, platform manifests and container
execution semantics. NUIF needs the descriptor pattern, not an OCI image.

## Open questions

- Should additional digest algorithms be syntactically preservable but
  unsupported, or rejected by the first package profile?
- Which resource metadata is semantic and hashed with the document, and which
  metadata is package-only provenance?
- Can range-addressable resources be added without weakening whole-byte digest
  verification?
