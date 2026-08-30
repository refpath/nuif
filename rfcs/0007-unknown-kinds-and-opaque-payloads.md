---
id: nuif:rfc:0007
kind: rfc
status: accepted
---

# RFC 0007 — Unknown entity kinds and opaque payloads

Status: accepted (decision delegated to research on 2026-08-29; evidence in `nuif:research:unknown-schema-preservation-strategies`, `nuif:research:opentimelineio`, `nuif:research:godot-tscn-scene-format`, `nuif:research:openusd-composition-and-crate`, `nuif:research:gltf-validator-and-sample-assets`). Extends RFC 0002.

## Motivation

RFC 0002 requires unknown extension data to survive load, save and edit. It does not define how an entity whose kind is unknown is represented, which operations remain valid on it, how it lays out and renders, or how validation severities are assigned. Godot preserves unknown node classes as placeholders that record the original class and properties and write them back; OpenTimelineIO preserves unknown schemas with name, version and raw payload while still decoding nested known objects; Blender ignores and cannot re-save unknown data; OpenUSD keeps unknown `typeName` as metadata with `fallbackPrimTypes`; glTF gates by `extensionsUsed` and `extensionsRequired` with validator codes `UNDECLARED_EXTENSION` (error) and `UNSUPPORTED_EXTENSION` (information).

## Decision

### Representation

1. An entity whose kind namespace, kind name or schema version is not supported MUST be loaded as `EntityKind::Unknown`, retaining namespace, kind name, schema version and the kind-specific payload bytes. A known kind whose `schema_version` exceeds the implementation's support MUST also load as `Unknown`, never fail.
2. Core fields (identifier, name, children, `nuif`-namespace properties, relations, extensions) of an unknown entity remain typed and editable. Only the kind-specific payload is opaque.
3. Every containment slot MUST admit `Unknown`.

### Operations

4. `Remove`, `Move`, `Rename`, `SetExtension`, `RemoveExtension` and core-property operations MUST apply to unknown entities unchanged.
5. `SetUnknownPayload` is valid only when the applying implementation declares the payload's namespace; other implementations MUST reject it with a diagnostic.

### Preservation

6. An implementation that does not declare a namespace MUST preserve that namespace's payloads byte-for-byte. An implementation that declares the namespace MAY re-encode deterministically (value-for-value). RFC 0002's "byte/value-for-byte" language is read as these two cases.
7. In `nuif-cbor-0`, opaque payloads are CBOR byte strings; the declared encoding (`Cbor` or `Octets`) is a sibling field, not a CBOR tag (RFC 0005 rule 13 forbids tags in the canonical body). Hashing covers the bytes. `nuif-text-0` encodes the bytes losslessly.
8. A malformed opaque payload yields a diagnostic on the owning entity and MUST NOT invalidate the document.
9. Lowering, flattening and codec conversion MUST carry `Unknown` entities and extension payloads through; a conformance fixture asserts byte identity after an edit cycle through an implementation that declares neither the kind nor the namespace.

### Evaluation

10. Layout treats an unknown entity as the kind named by the document's `fallback_kind` declaration for its namespace, else as `Container` with its authored size intents.
11. Rendering of an unknown entity reports `PreservedUnrenderable { namespace, entity }` and draws nothing for the kind-specific payload; children render normally.

### Validation severities

12. A namespace present in the document but absent from `extensions_used`: error.
13. A namespace declared in `extensions_used` and unsupported by the implementation: information.
14. A namespace declared in `extensions_required` and unsupported: blocks claims of faithful rendering and export fidelity above `preserved_unrenderable`; MUST NOT block structural editing.

### Type changes (`nuif-core`, signatures only)

```rust
pub enum OpaqueEncoding { Cbor, Octets }
pub struct OpaquePayload { pub encoding: OpaqueEncoding, pub bytes: Vec<u8> }
pub struct UnknownKind { pub namespace: String, pub kind: String, pub schema_version: u32, pub payload: OpaquePayload }
pub enum EntityKind { /* existing variants */ Unknown(UnknownKind) }
pub struct Extensions(pub BTreeMap<String, OpaquePayload>);
pub struct ExtensionDeclarations { pub used: BTreeSet<String>, pub required: BTreeSet<String>, pub fallback_kind: BTreeMap<String, EntityKind> }
pub enum Fidelity { /* existing */ PreservedUnrenderable { namespace: String, entity: Option<EntityId> } }
```

`nuif-protocol`: `SetExtension { entity, namespace, payload: OpaquePayload }`, `RemoveExtension { entity, namespace }`, `SetUnknownPayload { entity, payload: OpaquePayload }`.

## Compatibility

Existing `Extensions(BTreeMap<String, Vec<u8>>)` gains the encoding field; no documents exist.

## Security

Payload size and count are bounded by parser limits (`spec/11`); payloads are never interpreted by implementations that do not declare the namespace, so untrusted content in them cannot reach an interpreter.

## Conformance tests

- extensions suite: decode with an ignorant implementation, apply `Rename`, `Move`, `SetExtension` on neighbours and on the unknown entity itself, encode, assert byte identity of the unknown payload and restoration of the original kind name and version by a knowing implementation.
- validation suite: fixtures for severities 12–14.
- layout suite: unknown entity with `fallback_kind` and without.

## Rejected alternatives

- Silent drop (pre-3.5 proto3 behaviour, Blender): the failure NUIF exists to prevent.
- Host-typed re-encoding of unknown properties (Godot `Variant`): changes bytes in an ignorant implementation.
- Hard failure on newer schema versions (OpenTimelineIO): prevents structural editing of otherwise valid documents.
- CBOR tag 24 for declared-CBOR payloads: tags are forbidden in the canonical body by RFC 0005, and tag 24 requires well-formed content, which rule 8 does not.

## Unresolved

- Whether `fallback_kind` may name a kind from another extension namespace (transitive fallback) is deferred until a second dialect exists.
