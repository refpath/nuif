# Serialization, collaboration and governance

## Logical model before encoding

NUIF defines one logical model with multiple conforming encodings.

### Text form
A canonical, reviewable representation is implemented for examples, fixtures,
diffs and Git workflows. `nuif-text-0` fixes number formatting, UTF-8 key order,
layout and strict decode/canonicalize behavior; later text profiles may evolve
only through explicit versioning.

### Binary form
Deterministic CBOR is the profile-0 binary form because the NUIF profile closes
the choices left by RFC 8949 without coupling the logical model to generated
code. The executable codec gate finds it near 41% of canonical-text size at
4,096 entities on an Apple M5 Pro run, while its typed decode
path is slower than text. That result supports CBOR as a compact canonical form,
not as a universal latency winner.

A candidate is timed only after complete-model round trip, canonical fixpoint
and unknown-data preservation through a neighboring edit. Protobuf does not
specify canonical binary output. FlatBuffers deliberately permits different
byte layouts and old readers ignore new fields, so a rebuilding editor needs a
separate retention strategy. Cap'n Proto specifies a schema-agnostic canonical
form and is the preferred next experiment, but it still needs a complete NUIF
mapping, bounded old-reader edit trial and two agreeing canonical writers.
Compiled zero-copy runtime caches remain separate, explicitly noncanonical
profiles rather than replacements for authoring interchange.

The experimental package form separates manifest/document records from
content-addressed resources. RFC 0010 selects a candidate deterministic ZIP
profile with fixed `mimetype`, canonical manifest/document records and
SHA-256-addressed blobs. Bare encodings use explicit `.nuif.json` and
`.nuif.cbor` names. Exact ZIP header fixtures, two independent local writers and
bounded image/font segments now exist. Cross-platform and externally authored
writer evidence remains required before package-profile acceptance.

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
