---
id: nuif:research:unknown-schema-preservation-strategies
kind: synthesis
status: reviewed
title: Preservation of unknown entity kinds and extension payloads; OTIO UnknownSchema, Godot MissingNode, USD typeName, glTF extensions, Protocol Buffers unknown fields and CBOR tag 24 compared
source:
  url: https://github.com/AcademySoftwareFoundation/OpenTimelineIO
  repository: https://github.com/KhronosGroup/glTF
  authors: [Academy Software Foundation, Godot Engine contributors, Pixar Animation Studios, The Khronos Group, Google Protocol Buffers team, Carsten Bormann, Paul Hoffman, WebAssembly Community Group, WHATWG]
  published_at: "OTIO main (v0.18.1), Godot master (PR #60597 merged 2022-05-05), OpenUSD release v26.08, glTF 2.0 specification main, protobuf.dev proto3 guide (behaviour since 3.5, 2017-11), RFC 8949 (2020-12), WebAssembly core specification (2.0 draft), HTML Living Standard; all retrieved 2026-08-29"
  license: "Apache-2.0 (OTIO, glTF-Validator); MIT (Godot, WebAssembly spec text); TOST (OpenUSD); CC-BY-4.0 (glTF specification); BSD-3-Clause (protobuf); IETF Trust (RFC 8949); CC-BY-4.0 (HTML Living Standard)"
retrieved_at: 2026-08-29
tags: [unknown-schema, opaque-preservation, extensions, forward-compatibility, canonicalization, cbor, validation-severity, capability-negotiation, fidelity, round-trip]
confidence: 0.9
claims: [nuif:claim:opaque-preservation, nuif:claim:multi-level-ir]
relations:
  - type: extends
    target: nuif:research:opentimelineio
    note: Uses the OTIO UnknownSchema record as the baseline and adds the typed-slot limitation found in the reader.
  - type: related_to
    target: nuif:research:godot-tscn-scene-format
    note: MissingNode is the precedent for an unknown entity that remains movable, renamable and deletable.
  - type: related_to
    target: nuif:research:openusd-composition-and-crate
    note: USD keeps typeName as metadata and offers fallbackPrimTypes as a declared substitute.
  - type: related_to
    target: nuif:research:gltf-validator-and-sample-assets
    note: Supplies the severity table (UNDECLARED_EXTENSION Error, UNSUPPORTED_EXTENSION Information) adopted here.
  - type: related_to
    target: nuif:research:canonicalization-rfc8785-and-cbor-deterministic
    note: Opaque payloads must hash as byte strings under the deterministic encoding rules.
  - type: related_to
    target: nuif:research:encoding
    note: Protocol Buffers unknown-field retention is the codec-level precedent that RFC 0002 says is insufficient alone.
links:
  spec: [spec/01-model.md, spec/06-operations-and-patches.md, spec/07-extensions-and-dialects.md, spec/08-serialization.md, spec/09-provenance-and-fidelity.md]
  adr: [adrs/0004-serialization.md]
  rfc: [rfcs/0002-extension-preservation.md]
  code: [crates/nuif-core, crates/nuif-protocol, crates/nuif-codec]
  experiments: []
---

# Summary

Six systems preserve data they do not understand, with three distinct preservation units. OpenTimelineIO (OTIO) and Godot preserve whole objects of unknown type: OTIO instantiates `UnknownSchema` holding the original schema name, version and dictionary, and Godot instantiates `MissingNode`/`MissingResource` recording the original class and every assigned property, writing both back under the original name on save. OpenUSD preserves unknown type names as plain metadata on a prim whose properties compose normally and lets a writer declare `fallbackPrimTypes` for older readers. glTF preserves extension objects and `extras` per property with a document-level declaration split into `extensionsUsed` (all) and `extensionsRequired` (subset); the validator grades an undeclared extension as Error and an unsupported one as Information. Protocol Buffers retain unknown fields and unknown enum values as raw wire data since 3.5 and re-emit them, but lose them on JSON conversion and field-by-field copying. RFC 8949 tag 24 wraps an embedded CBOR item as a byte string that is not decoded with its container, and §5.4 recommends that decoders pass unknown tags through with a marker rather than fail. WebAssembly custom sections and HTML's `HTMLUnknownElement` show the same pattern at the container and DOM levels: unknown content is kept, is inert for the core semantics and must not invalidate the container.

Preservation is structural (typed values re-encoded by the writer) in OTIO, Godot and USD, and byte-level only in Protocol Buffers and CBOR tag 24. No retrieved system defines both, and none states a canonicalization rule for preserved payloads. Source statements are reported in `## Evidence`; the NUIF interpretation follows in `## NUIF relevance`.

## Evidence

- OTIO `UnknownSchema` derives from `SerializableObject` and stores `original_schema_name`, `original_schema_version` and the raw dictionary; `read_from` takes the whole dictionary minus `OTIO_SCHEMA`; `write_to` re-emits every stored key under the original label; nested known objects inside the dictionary are decoded. `src/opentimelineio/unknownSchema.h` lines 12–40, `unknownSchema.cpp` lines 8–45, `tests/test_unknown_schema.py` lines 9–100 (nuif:research:opentimelineio, retrieved 2026-08-29).
- OTIO typed slots: `Reader::read(key, Retainer<T>*)` performs `dynamic_cast<T*>` and reports `ErrorStatus::TYPE_MISMATCH` "Expected object of type ...; read type ... instead" when the object is not a `T`; `Composition` children are `std::vector<Composable*>`. `src/opentimelineio/serializableObject.h` lines 162–192; `composition.cpp` lines 55–77, main, retrieved 2026-08-29. An `UnknownSchema` object in a `Composable`-typed slot therefore fails to load; this is inferred from the code and was not executed.
- OTIO version handling: an object whose version is newer than registered is rejected with `SCHEMA_VERSION_UNSUPPORTED`; older versions are upgraded through registered dictionary transforms. `typeRegistry.cpp` lines 360–420 (nuif:research:opentimelineio).
- Godot: when `ClassDB::instantiate` fails, `SceneState::instantiate` creates `MissingNode`, sets `original_class` and `recording_properties = true`; `MissingNode::_set` records any property while recording; on pack `_parse_node` writes `original_class` as the node type; signals were recorded only from PR #105449 (merged 2025-10-10). `scene/resources/packed_scene.cpp`, `scene/main/missing_node.cpp`, master; PR #60597 (merged 2022-05-05): "missing types no longer cause data loss" (nuif:research:godot-tscn-scene-format, retrieved 2026-08-29). `MissingNode` is a `Node`, so it participates in tree operations; the class reference calls it "an internal editor class intended for keeping the data of unrecognized nodes" and emits the configuration warning "This node was saved as class type '%s', which was no longer available when this scene was loaded." (Godot docs, class `MissingNode`; `missing_node.cpp`).
- Godot placeholder values are typed by the runtime `Variant` inferred from the value, and property order and formatting are regenerated by the writer; byte-identical round trips are not claimed (nuif:research:godot-tscn-scene-format, `## Mechanism` and `## Open questions`).
- OpenUSD: `UsdPrim::GetTypeName` "returns the composed type name as authored"; unknown type names compose and round-trip; `fallbackPrimTypes` lets "prims with the unrecognized type name ... be treated as having the effective schema type of the first recognized type in the list"; properties outside any schema are `custom`, "the same function as Alembic's 'userProperties'". `pxr/usd/usd/prim.h` lines 192–204, `property.h` lines 179–185, https://openusd.org/release/api/_usd__page__object_model.html "Fallback Prim Types" (nuif:research:openusd-composition-and-crate, retrieved 2026-08-29). `UsdValidationErrorType { None, Error, Warn, Info }`; `usdchecker --strict` escalates `Warn` to failure (`pxr/usdValidation/usdValidation/error.h` lines 37–42; `usdchecker.cpp` lines 217–218).
- glTF 2.0: "Any glTF object MAY have an optional `extensions` property"; "All extensions used in a glTF asset MUST be listed in the top-level `extensionsUsed` array"; "All glTF extensions required to load and/or render an asset MUST be listed in the top-level `extensionsRequired` array"; "`extensionsRequired` is a subset of `extensionsUsed`". `specification/2.0/Specification.adoc` lines 2639–2689, main, retrieved 2026-08-29. `extras` is "Application-specific data" that "SHOULD be a JSON object rather than a primitive value for best portability" (`schema/extras.schema.json`; `schema/glTFProperty.schema.json` attaches `extensions` and `extras` to every property object). Extension registry rule: "If lack of extension support prevents proper geometry loading, extension specification must state that (and such extension must be mentioned in `extensionsRequired`)". `extensions/README.md` line 182.
- glTF-Validator `ISSUES.md`: `UNDECLARED_EXTENSION` Error "Extension is not declared in extensionsUsed."; `UNSUPPORTED_EXTENSION` Information "Cannot validate an extension as it is not supported by the validator"; `UNUSED_EXTENSION_REQUIRED` Error; `UNEXPECTED_EXTENSION_OBJECT` Error "Unexpected location for this extension."; `NON_REQUIRED_EXTENSION` Error "Extension '%1' cannot be optional."; `UNEXPECTED_PROPERTY` Warning; `EXTRA_PROPERTY` Information; `UNKNOWN_ASSET_MAJOR_VERSION` Error; `UNKNOWN_ASSET_MINOR_VERSION` Warning. https://github.com/KhronosGroup/glTF-Validator/blob/main/ISSUES.md, retrieved 2026-08-29 (line numbers in nuif:research:gltf-validator-and-sample-assets).
- Protocol Buffers: "Proto3 messages preserve unknown fields and include them during parsing and in the serialized output, which matches proto2 behavior"; before 3.5 proto3 dropped unknown fields; "unrecognized enum values will be preserved in the message" and "will still be serialized with the message"; unknown fields are lost on JSON serialization and field-by-field copying, so "message-oriented APIs, such as CopyFrom() and MergeFrom()" are recommended. https://protobuf.dev/programming-guides/proto3/ "Unknown Fields" and enum sections, retrieved 2026-08-29.
- RFC 8949 §3.4.5.1: "Tag number 24 (CBOR data item) can be used to tag the embedded byte string as a single data item encoded in CBOR format. Contained items that aren't byte strings are invalid. A contained byte string is valid if it encodes a well-formed CBOR data item; validity checking of the decoded CBOR item is not required for tag validity". §5.4: for an unrecognised tag or simple value a decoder "can report an error (and not return data). Note that treating this case as an error can cause ossification and is thus not encouraged" or "can emit the unknown item ... and then give the application an indication that the decoder did not recognize that tag number"; the latter "provides forward compatibility". §7.1: implementations "can choose to process just the enclosed tag content or, preferably, to process the tag as an unknown tag number wrapping the tag content". §4.2.1 lists the core deterministic encoding requirements (preferred serialization, minimal argument lengths, no indefinite lengths, bytewise-lexicographic map key order). https://www.rfc-editor.org/rfc/rfc8949.txt lines 1189–1199, 1726–1760, 2130–2134, 1382–1443, retrieved 2026-08-29.
- WebAssembly: custom sections "are intended to be used for debugging information or third-party extensions, and are ignored by the WebAssembly semantics"; they consist of "a name further identifying the custom section, followed by an uninterpreted sequence of bytes"; "If an implementation interprets the data of a custom section, then errors in that data, or the placement of the section, must not invalidate the module." https://webassembly.github.io/spec/core/binary/modules.html "Custom Section", retrieved 2026-08-29.
- HTML: element interface lookup ends "If name is a valid custom element name, then return HTMLElement. Return HTMLUnknownElement."; `HTMLUnknownElement` is an `HTMLElement` without `[HTMLConstructor]`. https://html.spec.whatwg.org/multipage/dom.html, retrieved 2026-08-29.
- NUIF current state: `Extensions(pub BTreeMap<String, Vec<u8>>)` on `Document` and `Entity`; `EntityKind` has no unknown variant; `Fidelity::PreservedUnrenderable { extension }`; `Operation::SetExtension { entity, namespace, payload: Vec<u8> }`. `crates/nuif-core/src/lib.rs`, `crates/nuif-protocol/src/lib.rs` at commit af8d5cb. spec/07 requires preservation "byte/value-for-byte at their attachment point"; RFC 0002 requires survival across "load/save/edit cycles unless the owner is deleted" and states that unsupported required extensions "block claims of faithful rendering but do not necessarily block structural editing".

## Mechanism

Whole-object placeholders (OTIO, Godot). The reader dispatches on a type label; when the registry lookup fails it constructs a placeholder that implements the same interface as a known object (OTIO `SerializableObject`, Godot `Node`), records the label and every field, and answers the writer's field enumeration with the recorded pairs so that the writer emits the original label. The placeholder is inert: no migrations run, no behaviour executes, and the editor warns. Two limits follow from the design. First, the placeholder is accepted only where the containing slot's type admits the placeholder's base class; OTIO's typed `Retainer<T>` slots reject it with `TYPE_MISMATCH`, so unknown objects survive only in untyped maps such as `metadata`. Second, values are re-typed through the host value model (OTIO `AnyDictionary`, Godot `Variant`) and re-serialized by the host writer, so the round trip is value-preserving, not byte-preserving.

Type as metadata (USD). The type name is a field like any other; every property is stored regardless of schema membership; unknown types therefore need no placeholder because nothing is dispatched on the type at load time. Behaviour attached to the type (schema fallbacks, validators) is absent for unknown names, and `fallbackPrimTypes` is an author-declared substitution list evaluated by older readers. Preservation is complete for anything expressible in USD's value model and undefined for foreign encodings.

Attachment-point blobs with declarations (glTF, Protocol Buffers, CBOR, WebAssembly). Unknown content is attached to the object it describes (`extensions.<name>`, unknown field numbers, tag 24 byte strings, named custom sections) and the container's core semantics ignore it. glTF adds a document-level contract: every extension present must be declared, and the required subset gates loading; the validator makes the declaration, not the understanding, the pass/fail criterion. Protocol Buffers shows the failure mode of codec-only retention: any path that reconstructs the message field by field (JSON, per-field copy) drops the unknowns, which is the gap RFC 0002 names when it says preservation "goes beyond codec unknown fields".

Canonicalization of preserved data. RFC 8949 deterministic encoding constrains the encoder of a data item; a payload the implementation does not decode cannot be re-encoded and must be treated as a byte string, which the deterministic rules cover (minimal length argument, no indefinite length). Tag 24 is the standard marker that a byte string is itself CBOR without requiring the container decoder to decode it. Hashing the byte string yields a stable canonical hash regardless of whether the embedded item is itself deterministically encoded; the embedded item's own canonical form is the responsibility of the implementation that declares the namespace.

Severity and negotiation. glTF distinguishes undeclared (Error, the document is malformed), declared and unsupported (Information, the document is valid and the feature is degraded) and required and unsupported (loading fails by specification). USD offers `--strict` to promote warnings. Both make severity a property of the declaration state rather than of the implementation's coverage.

## NUIF relevance

**Borrow**
- The OTIO/Godot placeholder shape: an `EntityKind::Unknown` variant carrying namespace, kind name, schema version and the kind-specific payload, implementing the same containment interface as known kinds so that move, rename, delete and reparent apply unchanged (Godot `MissingNode` is a `Node`).
- USD's separation of core fields from type: NUIF core properties (`nuif` namespace), name, children, relations and extensions are typed and editable on an unknown entity; only the kind-specific payload is opaque.
- glTF's declaration contract and severity mapping: unknown namespace declared in `extensions_used` is Information; present but undeclared is Error; declared in `extensions_required` and unsupported is a fidelity block, not a load failure (RFC 0002).
- RFC 8949 tag 24 as the encoding of opaque payloads in `nuif-cbor-0`, and the §5.4 rule that decoders pass unknown items through with a marker instead of failing.
- WebAssembly's rule that errors inside uninterpreted custom data must not invalidate the container: a malformed opaque payload is a diagnostic on the owning entity, not a document rejection.
- Protocol Buffers' warning that field-by-field reconstruction loses unknowns: NUIF lowering, flattening and codec conversion passes must carry `Unknown` and `Extensions` through explicitly, and a conformance fixture must assert this.

**Adapt**
- OTIO's typed-slot limitation must not be replicated: every containment slot in NUIF admits `EntityKind::Unknown`; relation endpoints typed to a specific kind treat an unknown target as `Fidelity::PreservedUnrenderable`, not as a load error.
- OTIO rejects newer schema versions; NUIF should instead demote a known kind with a newer `schema_version` than the implementation supports to `Unknown` with the same payload, so forward-compatible editing remains possible (the open question in nuif:research:opentimelineio).
- Godot's value-preserving round trip becomes byte-preserving in NUIF for uninterpreted payloads, and value-preserving (re-encoded deterministically) for payloads whose namespace the implementation declares; RFC 0002's "byte/value-for-byte" should be split into these two normative cases.
- `fallbackPrimTypes` becomes a per-namespace `fallback_kind` declaration in the extension registry (for example, a vendor chart kind falls back to `Container`), so layout treats the unknown entity as its fallback kind with authored size intents; without a declaration the fallback is `Container` with `SizeIntent::Auto`.
- glTF `extras` (schema-less, per object) maps to a reserved `nuif.extras` extension namespace rather than a separate field, keeping one attachment mechanism.

**Reject**
- Silent drop of unknown kinds or of payloads that fail the implementation's own decoder (pre-3.5 proto3 behaviour; OTIO `TYPE_MISMATCH` on typed slots).
- Re-typing opaque payloads through the host value model (Godot `Variant` inference), because the canonical hash would then depend on the host's re-encoding.
- Hard failure on a newer schema version (OTIO `SCHEMA_VERSION_UNSUPPORTED`) for kinds whose core fields still parse.
- Environment-variable or ambient selection of fallback behaviour; capability declarations are explicit inputs to load and validate.

## Open questions

- Whether `nuif-text-0` should render an opaque CBOR payload as a byte string only, or additionally as non-normative CBOR diagnostic notation in a comment; the second form aids review but must not participate in hashing or parsing.
- Whether an implementation that declares a namespace may rewrite that namespace's payload on every save (canonical re-encoding) or only when the value changed; the first churns diffs for documents authored by a different implementation of the same namespace.
- How relation operations typed to a kind (for example, `Instance { component }`) behave when the target is `Unknown`; the fidelity mapping above is a proposal without a fixture.
- The OTIO typed-slot behaviour was inferred from `serializableObject.h` and `composition.cpp` and not confirmed by executing a fixture with an unknown schema inside a track.
- Godot's byte-level round-trip stability for `MissingNode` remains unverified (nuif:research:godot-tscn-scene-format).
