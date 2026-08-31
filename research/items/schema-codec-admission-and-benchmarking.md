---
id: nuif:research:schema-codec-admission-and-benchmarking
kind: synthesis
status: reviewed
title: Admission criteria for Protobuf, FlatBuffers and Cap'n Proto as NUIF codecs
source:
  url: https://capnproto.org/encoding.html
  authors: [Cap'n Proto contributors, Google Protocol Buffers team, Google FlatBuffers team]
  published_at: living specifications retrieved 2026-08-31
  license: documentation terms vary by source
retrieved_at: 2026-08-31
tags: [serialization, canonicalization, schema-evolution, partial-access, unknown-fields, benchmarking]
confidence: 0.97
claims: [nuif:claim:opaque-preservation, nuif:claim:canonical-type-preservation, nuif:claim:bounded-untrusted-input]
relations:
  - type: extends
    target: nuif:research:encoding
    note: Replaces a name-only codec comparison with an executable admission protocol and current primary-source evidence.
  - type: supports
    target: nuif:research:unknown-schema-preservation-strategies
    note: Confirms that wire compatibility alone does not prove preservation through an editing object model.
links:
  spec: [spec/08-serialization.md, spec/11-security.md]
  adr: [adrs/0004-serialization.md]
  rfc: [rfcs/0002-extension-preservation.md, rfcs/0005-deterministic-numeric-and-string-canonicalization.md, rfcs/0008-cbor-data-model-preservation.md]
  code: [crates/nuif-testing/src/bin/codec-benchmark.rs, xtask/src/main.rs]
  experiments: [nuif:experiment:codec-benchmark]
---
# Summary

A codec benchmark is meaningful only after the candidate represents the full
NUIF logical model and survives the format's correctness obligations. A small
generated-schema example can make a schema codec look fast while omitting
extensions, version migration, exact integer/real identity, canonical hashes
and hostile-input limits. NUIF therefore admits a codec to timing only after it
passes exact semantic round trip, canonical encode/decode/encode fixpoint and
opaque-data preservation across a neighboring edit.

The implemented `nuif-text-0` and `nuif-cbor-0` profiles pass that preflight and
are measured at 8, 64, 512 and 4,096 entities. Protobuf and FlatBuffers are not
admitted today. Cap'n Proto is the preferred next schema experiment, not an
accepted NUIF profile: it is the only screened schema codec whose primary
encoding specification defines a schema-agnostic canonical form, but NUIF still
needs a complete mapping, retentive old-reader editing tests, two canonical
writers and calibrated traversal limits before timing it.

# Decision criteria

Every candidate must supply one reviewable mapping from every versioned wire
field to the semantic model, including explicit unknown kinds and extensions.
The mapping must then pass these gates in order:

1. exact semantic round trip for the complete responsive-card fixture;
2. byte-identical encode/decode/encode and canonicalizer fixpoints;
3. exact opaque entity, payload, declarations and document extensions after an
   unrelated known-property edit;
4. rejection or bounded handling of oversized, over-depth and amplification
   inputs;
5. cross-version old-reader/edit/new-reader evidence;
6. at least two implementations producing the same canonical bytes;
7. only then, size, encode, decode, canonicalize and access measurements over
   the same corpus and build profile.

Native partial access is reported separately from `decode_then_select`. The
latter is an honest measurement of today's full-document decoder followed by a
map lookup; it must not be presented as zero-copy or partial decoding.

# Candidate findings

## Protocol Buffers

The official Protobuf documentation states that deterministic serialization is
not canonical and can vary after schema, build or library changes. It identifies
unknown fields as an inherent barrier because a length-delimited unknown value
cannot be distinguished as bytes or a nested message without its schema.
Proto3 binary messages normally preserve unknown fields, but official guidance
also says JSON conversion and field-by-field reconstruction lose them. These
properties are useful for message evolution but conflict with NUIF's stable
content hash unless NUIF defines and independently implements a separate
canonicalizer. Protobuf is therefore not admitted until a complete schema,
canonicalization profile and old-reader retentive edit path exist.

Primary sources:

- https://protobuf.dev/programming-guides/serialization-not-canonical/
- https://protobuf.dev/programming-guides/proto3/#unknown-fields
- https://protobuf.dev/programming-guides/encoding/

## FlatBuffers and FlexBuffers

FlatBuffers' material advantage is direct in-buffer access without constructing
an ordinary object graph. Its schema-evolution rules allow an old reader to
ignore a newly appended table field, and its verifier and keyed vectors could
support bounded lookup. The official internals documentation deliberately
leaves table-field and object placement order undefined and explicitly permits
different binaries for the same values. Thus the default format is not a
canonical hash representation.

Ignoring a future field is sufficient when an old process forwards the original
buffer unchanged. It is not evidence that an editor which unpacks, changes a
known property and rebuilds the buffer retains that field. This loss statement
is an inference from the documented old-reader behavior and must be tested
rather than assumed. A NUIF FlatBuffers profile would need a canonical writer
and a retentive reconstruction layer, which remove part of the apparent
zero-copy simplicity.

FlexBuffers preserves the direct-access property without a schema and sorts map
keys for lookup, but its coercing accessors and absence of a specified complete
canonical NUIF representation do not improve on deterministic CBOR for the
current authoring file. It remains a possible opaque payload or cache encoding,
not a primary document candidate.

Primary sources:

- https://flatbuffers.dev/white_paper/
- https://flatbuffers.dev/evolution/
- https://flatbuffers.dev/internals/
- https://flatbuffers.dev/flexbuffers/

## Cap'n Proto

Cap'n Proto specifies a canonical, unpacked, single-segment, preorder form with
trailing default words removed. The canonicalization algorithm is
schema-agnostic. Normal encoders are explicitly not required to emit that form,
so a NUIF profile would still require canonical-writer conformance. Its pointer
model offers direct traversal, while the same specification requires pointer
validation, a traversal limit that accounts for amplification, and a nesting
limit. Those security rules align better with the existing NUIF resource model
than an undocumented implicit limit would.

The remaining risk is semantic evolution through an editor. A complete NUIF
schema must show that an older implementation can edit known data while
retaining future fields, opaque extension bytes and integer/real distinctions.
Canonical bytes must agree across at least two implementations. Until those
tests exist, reporting Cap'n Proto latency alongside complete codecs would be a
category error. It is the next experiment because it clears the canonical-form
screen, not because its performance is presumed superior.

Primary source: https://capnproto.org/encoding.html, especially
“Canonicalization” and “Security Considerations”.

# Measured baseline

`cargo xtask codec-benchmark` records the corpus seed, generator, source
revision, dirty state, Rust toolchain, OS, architecture, CPU, warmups, samples,
latency distribution, allocation counts and exact encoded hashes. The first
local release run on an Apple M5 Pro with Rust 1.98.0 found deterministic CBOR
at 40.83% of canonical-text size for 4,096 entities. At that scale, canonical
text encoded in about 17.9 ms and decoded in 11.1 ms median; deterministic CBOR
encoded in about 18.9 ms and decoded in 24.2 ms. These are one-machine
measurements, not universal rankings. The important current conclusion is that
CBOR materially reduces bytes but is not presently a decode-latency
optimization. The decoder now materializes the typed document directly and
checks canonicality by re-encoding it; the generic value tree is retained only
as an invalid-input fallback needed to classify an over-depth value before a
root-type mismatch. This preserves strictness while avoiding two generic trees
on the accepted path.

The generated `target/codec-benchmark-report.json` is the evidence source. CI
archives each run; transient measurements are intentionally not committed as
goldens. Catastrophic ceilings detect broken scaling while controlled
before/after runs, not cross-host absolute values, decide optimizations.

## NUIF relevance

- Retain canonical text as the review, fixture and Git form.
- Retain deterministic CBOR as the canonical hash and compact package record
  profile; its size win is real and its current decode cost is visible.
- Continue optimizing CBOR only behind identical conformance fixtures; the
  direct typed decoder is implemented, while a streaming canonical validator
  would require a separate hostile-input proof before replacing re-encoding.
- Prototype Cap'n Proto next only as a complete experimental mapping with a
  retentive edit bridge and bounded reader.
- Do not add Protobuf or FlatBuffers dependencies merely to publish flattering
  partial-model timings.
- A future zero-copy runtime cache may use a different, explicitly noncanonical
  compiled-scene profile without replacing the authoring interchange form.
