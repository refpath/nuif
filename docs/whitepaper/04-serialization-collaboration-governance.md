# Serialization, collaboration and governance

## Logical model before encoding

NUIF defines one logical model with multiple conforming encodings.

### Text form
A canonical, reviewable representation is required for examples, fixtures, diffs and Git workflows. The exact syntax remains experimental; human readability and deterministic canonicalization are required.

### Binary form
Deterministic CBOR is the initial recommendation because RFC 8949 specifies deterministic encoding without coupling NUIF to a generated-code schema system. A schema-based high-performance encoding may later become a profile after benchmarks.

The proposed package form separates manifest/document records from
content-addressed resources. RFC 0010 selects a candidate deterministic ZIP
profile with fixed `mimetype`, canonical manifest/document records and
SHA-256-addressed blobs. Bare encodings use explicit `.nuif.json` and
`.nuif.cbor` names. The proposal remains experiment-required: exact ZIP header
fixtures, two independent writers, image/font budgets and resource profiles do
not yet exist.

Semantic document, resource and package hashes have different scopes. Stable
asset identity is not content addressing. Unknown extension payloads remain
explicit typed bytes/values and must not depend on accidental codec unknown-
field behavior.

## Collaboration

Canonical documents do not require CRDT tombstones, clocks or replica metadata. A collaboration profile maps NUIF operations to an append-only/change structure and can use Automerge, Yjs or another convergent transport. Checkpoints serialize back to canonical NUIF.

This keeps offline files simple and permits multiple collaboration engines.

The executable register profile uses causal multi-value registers. The
separate existing-tree profile replays uniquely ordered moves, rejects cycles,
models deletion as profile trash and orders siblings through stable RGA-style
origins. Semantic move and deletion conflicts remain visible even when the
profile can choose a deterministic checkpoint. Automerge is presently tested
as an operation-set transport, not claimed as an implementation of the tree
algorithm.

## Governance

Early development occurs in `refpath/nuif`, but the architecture assumes eventual neutral stewardship. A plausible progression is:

1. OSS research/reference implementation under Refpath.
2. public RFC process + implementer registry.
3. independent community/working group once two independent implementations exist.
4. investigate W3C Community Group for UI/document semantics and/or Khronos-style governance if renderer/asset vendors become primary stakeholders.

Specification text, schemas and conformance tests need clear royalty-free contribution/IP terms before claiming standards-track stability.
