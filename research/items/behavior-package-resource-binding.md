---
id: nuif:research:behavior-package-resource-binding
kind: synthesis
status: verified
title: Content-addressed behavior attachment without canonical-schema coupling
source:
  url: https://github.com/opencontainers/image-spec/blob/main/descriptor.md
  authors: [Open Container Initiative, W3C Publishing Maintenance Working Group, Khronos 3D Formats Working Group]
  published_at: null
  license: Apache-2.0, W3C Document License and Khronos specification terms
retrieved_at: 2026-08-31
tags: [behavior, package, resource, content-addressing, capability, security, interoperability]
confidence: 0.99
claims: [nuif:claim:resource-identity-separation, nuif:claim:bounded-untrusted-input, nuif:claim:multi-level-ir]
relations:
  - type: extends
    target: nuif:research:behavior-portability-state-machines
    note: The finite sidecar gains a tested transport without becoming part of the canonical semantic Document.
  - type: related_to
    target: nuif:research:oci-resource-descriptors
    note: The attachment reuses the media-type, size and digest descriptor pattern.
  - type: related_to
    target: nuif:research:epub-ocf-package-container
    note: A declared container resource remains distinct from the application that elects to process it.
links:
  spec: [spec/08-serialization.md, spec/11-security.md, spec/13-semantics-accessibility-and-behavior.md]
  adr: []
  rfc: [rfcs/0010-portable-resource-package.md, rfcs/0012-behavior-package-resource.md]
  code: [crates/nuif-package/src/lib.rs, crates/nuif-api/src/lib.rs, crates/nuif-behavior/src/lib.rs, crates/nuif-testing/src/bin/behavior-package.rs, apps/editor/src/lib.rs, apps/editor/src/bin/editor-hostile-inputs.rs, tools/behavior-oracle/package_check.py, xtask/src/main.rs]
  experiments: [nuif:experiment:behavior-package-resource]
---

# Summary

The first behavior wire experiment should be one canonical CBOR resource in
the existing content-addressed package, not a new field in the canonical
`Document` and not a new ZIP member family. `nuif-package-0` already binds each
resource's exact bytes, size, media type and role into the same manifest as the
canonical document descriptor. Reusing that mechanism gives deterministic
delivery, digest verification, old-reader preservation and package-level
binding without freezing the semantic model.

The selected `nuif-behavior-package-resource-0` profile admits exactly one
embedded `source` resource with provisional media type
`application/nuif-behavior+cbor`. Its bytes are canonical `nuif-cbor-0` for a
validated `nuif-behavior-state-machine-0` program. The package manifest also
declares that behavior profile as required. Generic package decoding verifies
and preserves the bytes but does not interpret or execute them; an explicit
behavior API checks cardinality, descriptor policy, canonical bytes and every
entity reference against the package document.

## Evidence

- OCI content descriptors require a media type, digest and raw byte size,
  recommend embedding descriptors in other formats for secure content
  reference, and require consumers to verify size and digest before heavy
  processing. Its image layout stores content at a digest-derived blob path.
  This supports NUIF's existing resource descriptor and cheap-verification
  order rather than a behavior-specific locator system. Locators:
  https://github.com/opencontainers/image-spec/blob/main/descriptor.md and
  https://github.com/opencontainers/image-spec/blob/main/image-layout.md,
  retrieved 2026-08-31.
- EPUB 3.3 requires publication resources to be declared in the package
  manifest and normally transported in one OCF ZIP container. EPUB Reading
  Systems 3.3 explicitly distinguishes applications that merely extract or
  validate a container from full reading systems, which may ignore rendering
  requirements. This is a useful precedent for verified package access not
  implying execution. Locators: https://www.w3.org/TR/epub-33/#sec-manifest-elem
  and https://www.w3.org/TR/epub-rs-33/#sec-ocf,
  retrieved 2026-08-31.
- KHR_interactivity keeps portable behavior in an extension graph rather than
  making it arbitrary script embedded in visual nodes. Its conformance assets
  are separately executed by engines that implement the extension. NUIF uses
  a smaller finite machine, but preserves the same separation between stored
  graph data and an implementing runtime. Locator:
  https://raw.githubusercontent.com/KhronosGroup/glTF/refs/heads/main/extensions/2.0/Khronos/KHR_interactivity/Specification.adoc,
  retrieved 2026-08-31.
- RFC 0010 already restricts package members to `mimetype`, `manifest.cbor`,
  `document.cbor` and digest-addressed blobs. Behavior fits the registered blob
  path and resource manifest, so adding a special `behavior/` member or a
  second package profile would duplicate identity and validation machinery.

## Mechanism

Attachment is an explicit operation:

```text
validate(program, package.document)
encode canonical CBOR
add embedded source resource
declare nuif-behavior-state-machine-0 required
encode deterministic package
```

Opening the package has two distinct levels:

```text
NuifPackage::decode       ZIP, manifest, size, digest and document validation
require_capabilities      explicit complete package/host support negotiation
attached_behavior         cardinality, role, canonical CBOR and entity binding
BehaviorRuntime::new      caller-supplied effect-capability authorization
```

There is no automatic transition from one level to the next. In particular,
resource presence never grants script, filesystem, network or host mutation
authority.

This separation also constrains generic authoring. An editor that cannot
interpret a required capability cannot know whether an opaque resource binds
entity references, a document hash or other semantic preconditions. Preserving
that resource while changing the document would therefore manufacture an
unvalidated pairing. The reference editor uses structural read-only mode:
inspection and exact copying are allowed, but its shared driver and save
boundary reject semantic changes with the exact missing requirement set.

The behavior digest identifies only the behavior bytes. It does not claim to
be a standalone document-specific identity. The deterministic package
manifest contains both the document descriptor and behavior descriptor, and
the package hash binds that pair. Transplanting the same behavior bytes into a
different package produces a different package hash and the explicit behavior
loader still revalidates all entity references.

## NUIF relevance

This closes the first delivery gap without collapsing NUIF's layers. The
semantic document remains readable by implementations that do not implement
behavior, the package remains the authority for exact resource delivery, and a
behavior/runtime adapter remains the authority for execution semantics. A
future standards decision can therefore compare real multi-host evidence
before deciding whether behavior belongs in a required semantic profile.

## Executable evidence

`cargo xtask gate-behavior-package` generates a deterministic `.nuif` fixture,
checks canonical encoding, document/package hash separation, exact round trip,
capability/resource agreement, exact generic host-capability negotiation and
hostile mismatch cases, then invokes an
independent Python standard-library ZIP reader. The foreign reader checks the
exact archive hash, member ordering and metadata, CRC reads, media marker and
content-addressed behavior bytes. It intentionally does not decode CBOR; the
Rust side checks canonical CBOR and semantic references, so the report does not
misstate container agreement as a second behavior implementation.

## Rejected alternatives

- Add behavior to `Document` now: would turn one successful trace experiment
  into a canonical-schema commitment before native and presentation adapters.
- Add a dedicated ZIP member: would require a new path/profile while bypassing
  the existing resource descriptor, digest and limit machinery.
- Store JSON for browser convenience: would introduce a second canonical
  encoding and numeric/string rules; host adapters can generate escaped JSON
  from the validated in-memory program.
- Permit linked behavior in the portable profile: would make offline behavior
  depend on resolver authority and retrieval availability.
- Infer the attachment from any CBOR resource: would make discovery ambiguous;
  the exact provisional media type and required capability are both checked.

## Open questions

- Whether enough independent native/presentation adapters will justify moving
  a future behavior profile into canonical semantics.
- Media-type registration and final naming remain standards-track work; the
  current identifier is explicitly provisional and must not be advertised as
  IANA-registered.
