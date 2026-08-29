# Serialization, collaboration and governance

## Logical model before encoding

NUIF defines one logical model with multiple conforming encodings.

### Text form
A canonical, reviewable representation is required for examples, fixtures, diffs and Git workflows. The exact syntax remains experimental; human readability and deterministic canonicalization are required.

### Binary form
Deterministic CBOR is the initial recommendation because RFC 8949 specifies deterministic encoding without coupling NUIF to a generated-code schema system. A schema-based high-performance encoding may later become a profile after benchmarks.

The package form separates manifest/document records from content-addressed assets and permits partial loading. Unknown extension payloads are explicit typed bytes/values and must not depend on accidental codec unknown-field behavior.

## Collaboration

Canonical documents do not require CRDT tombstones, clocks or replica metadata. A collaboration profile maps NUIF operations to an append-only/change structure and can use Automerge, Yjs or another convergent algorithm. Checkpoints serialize back to canonical NUIF.

This keeps offline files simple and permits multiple collaboration engines.

## Governance

Early development occurs in `refpath/nuif`, but the architecture assumes eventual neutral stewardship. A plausible progression is:

1. OSS research/reference implementation under Refpath.
2. public RFC process + implementer registry.
3. independent community/working group once two independent implementations exist.
4. investigate W3C Community Group for UI/document semantics and/or Khronos-style governance if renderer/asset vendors become primary stakeholders.

Specification text, schemas and conformance tests need clear royalty-free contribution/IP terms before claiming standards-track stability.
