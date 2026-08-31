---
id: nuif:research:resource-packaging-and-source-capture-synthesis
kind: synthesis
status: reviewed
title: Resource-aware NUIF packaging and source-backed capture synthesis
source:
  url: https://github.com/refpath/nuif/tree/main/research/items
  authors: [NUIF contributors]
  published_at: 2026-08-30
  license: repository license
retrieved_at: 2026-08-30
tags: [synthesis, package, resources, images, fonts, browser-capture, provenance]
confidence: 0.94
claims: [nuif:claim:resource-identity-separation, nuif:claim:source-inference-separation]
relations:
  - type: inspired_by
    target: nuif:research:epub-ocf-package-container
  - type: inspired_by
    target: nuif:research:oci-resource-descriptors
  - type: depends_on
    target: nuif:research:png-image-preservation-and-decoding
  - type: depends_on
    target: nuif:research:opentype-font-embedding-and-portability
  - type: depends_on
    target: nuif:research:chromium-source-backed-ui-capture
links:
  spec: [spec/05-geometry-paint-text.md, spec/08-serialization.md, spec/09-provenance-and-fidelity.md, spec/11-security.md]
  adr: [adrs/0004-serialization.md]
  rfc: [rfcs/0003-authored-resolved-provenance.md, rfcs/0009-profile-zero-resource-budgets.md, rfcs/0010-portable-resource-package.md, rfcs/0011-observation-and-inference-provenance.md]
  code: [crates/nuif-core, crates/nuif-codec, crates/nuif-package, crates/nuif-media, crates/nuif-font, adapters/html-css/PROFILE.md]
  experiments: [nuif:experiment:portable-package-resources, nuif:experiment:image-resource-rgba8-baseline, nuif:experiment:image-resource-profile, nuif:experiment:font-resource-static-baseline, nuif:experiment:font-resource-profile, nuif:experiment:browser-source-capture]
---

# Summary

NUIF needs one resource model shared by direct authoring, adapters, browser
capture and later screenshot reconstruction. The semantic document names stable
assets; immutable descriptors identify exact bytes; package locators explain
where those bytes are carried; provenance explains where they came from. No one
of these identifiers can substitute for the others.

The recommended alpha path is a deterministic ZIP package with a fixed first
`mimetype` member, canonical CBOR manifest and document, and SHA-256-addressed
blobs. Bare canonical encodings remain available as `.nuif.json` and
`.nuif.cbor`; existing raw `.nuif` inputs are legacy read-only detection during
the alpha migration. This is a research conclusion pending the package RFC and
executable cross-writer fixtures, not a completed profile claim.

Source-backed browser capture and screenshot-only reconstruction must remain
separate products of the same import pipeline. Browser capture can preserve
source bytes, downloaded resources and resolved observations. A screenshot can
only preserve its own pixels and infer a possible editable structure.

## Evidence

- EPUB OCF demonstrates an interoperable ZIP subset, early media-type member,
  manifest discipline and explicit remote-resource boundary.
- OCI descriptors demonstrate that media type, digest and size should be
  checked before expensive content interpretation; URLs are retrieval hints.
- PNG Third Edition demonstrates why encoded source bytes, color metadata and
  straight-alpha semantics must survive independently of decoded caches.
- OpenType `fsType` demonstrates that technical ability to embed a font is not
  sufficient; an exporter must preserve and apply redistribution policy.
- CDP demonstrates that DOM, layout, style, network resources, fonts,
  accessibility and screenshots are distinct observations obtainable under a
  pinned browser execution context.
- Existing NUIF Penpot tests prove that a restricted ZIP reader can reject
  traversal, duplicates, symlinks, encryption, unsupported compression and
  expansion-limit attacks without filesystem extraction. They do not prove the
  proposed NUIF package layout or image/font budgets.
- The executable package profile now proves exact bytes from two ZIP writers
  and passes a shared-buffer allocation trial: an 8 MiB resource retains the
  same pointer across package, cloned handle map and session under 1 MiB of
  allocator traffic and retained bookkeeping.
- Image scene lowering now interns one decoded surface per digest/profile,
  preflights a 64 MiB decoded total and keeps 1,024 uses of one 512×512 image to
  one 1 MiB surface under measured release-build ceilings.

## Mechanism

The proposed model has three layers:

```text
Semantic asset
  AssetId (stable under byte replacement)
  kind + intrinsic semantics + policy
             |
             v
Immutable resource descriptor
  media_type + sha256 digest + byte size
             |
             v
Package/resolver locator
  embedded blob path | explicit linked locator + expected digest

Provenance independently records source URL/path/node/range, capture context,
derivation, license evidence and confidence.
```

Resource roles are `source`, `authoring`, `derived` and `cache`. Source and
authoring resources can affect fidelity and document semantics. Derived
resources retain their transformation record. Caches never affect the semantic
document hash and may be deleted or regenerated.

The package has two hashes:

- semantic document hash: SHA-256 of canonical `nuif-cbor-0` document bytes;
- package hash: SHA-256 of the complete deterministic package bytes.

The first remains stable when nonsemantic caches or container metadata change.
The second proves the exact delivered artifact. A manifest binds every
semantically required resource descriptor and role.

## NUIF relevance

This boundary lets every host use one core while choosing a suitable shell:
Rust API, C ABI, WASM, CLI, editor or process protocol. Those surfaces do not
reimplement resource rules. Browser capture is a new adapter because it owns a
pinned runtime and observation protocol; the existing source adapter remains a
retentive static compiler path.

Images preserve encoded originals. Fonts preserve exact bytes only where policy
allows. Screenshot-derived crops, reconstructed vectors and generated assets
are derived approximations with evidence and confidence, never disguised as
captured originals. External resolution is opt-in and always digest-checked.

## Remaining questions

- Can an externally authored writer reproduce the already exact in-repository
  two-writer package bytes on every supported host?
- Do the measured package/image allocation ceilings reproduce on hosted Linux,
  Windows and macOS runners, and what corresponding ceiling is appropriate for
  the font pipeline? The variable-font runtime now has an exact same-revision
  artifact aggregator; the other resource reports still need equivalence rules
  before they can join that aggregate.
- Which resource substitutions are allowed by each portability profile, and
  when must a missing resource make validation fail?
- How should an adapter preserve an inaccessible local font: metrics only,
  outlines, a linked descriptor, or an unavailable fidelity item?
