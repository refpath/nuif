---
id: nuif:research:cbor-data-model-and-key-order-correction
kind: standard
status: verified
title: "CBOR integer/float identity and deterministic encoded-key ordering"
source:
  url: https://www.rfc-editor.org/rfc/rfc8949.html
  authors: [Carsten Bormann, Paul Hoffman]
  published_at: "RFC 8949, December 2020"
  license: IETF Trust Legal Provisions
retrieved_at: 2026-08-29
tags: [cbor, deterministic-encoding, numeric-types, map-order, correction]
confidence: 0.99
claims: [nuif:claim:canonical-type-preservation]
relations:
  - type: contradicts
    target: nuif:research:deterministic-cbor-profiles-and-numeric-canonicalization
    note: Contradicts integral-real reduction when the NUIF logical value model keeps integer and real as distinct types.
  - type: extends
    target: nuif:research:canonicalization-rfc8785-and-cbor-deterministic
    note: Adds the cross-profile key-order counterexample and separates data-model identity from serialization width.
links:
  spec: [spec/08-serialization.md]
  adr: [adrs/0004-serialization.md]
  rfc: [rfcs/0008-cbor-data-model-preservation.md]
  code: [crates/nuif-codec/src/lib.rs]
  experiments: [nuif:experiment:roundtrip-trial-loop]
---

# Summary

RFC 8949 defines integer and floating-point values as distinct members of the CBOR basic generic data model even when their mathematical values are equal. Its preferred serialization narrows the width of a floating-point encoding but does not change a floating-point data item into an integer. NUIF's RFC 0005 instead reduced an integral `real` to a CBOR integer and relied on an external property schema to restore its type. That is not lossless for extension values, unknown properties, or generic tools and gives `integer(1)` and `real(1.0)` the same bytes despite the NUIF logical model distinguishing them.

RFC 8949 core deterministic ordering compares the complete deterministic encodings of map keys bytewise. For text keys, the encoded length byte participates in the comparison. UTF-8 lexical order therefore does not generally coincide with CBOR encoded-key order: UTF-8 sorts `"aa"` before `"z"`, while their CBOR encodings `0x62 61 61` and `0x61 7a` sort `"z"` first. RFC 0005's claim of coincidence is false.

## Evidence

- RFC 8949 §2 states that integer and floating-point values are distinct in the basic generic data model even when they have the same numeric value. Locator: heading "Data Models" and the paragraph beginning "Note that integer and floating-point values are distinct".
- RFC 8949 §4.1 defines preferred serialization of a floating-point value as the shortest floating-point width that preserves its value. It does not authorize conversion to major type 0 or 1. Locator: heading "Preferred Serialization", floating-point bullet.
- RFC 8949 §4.2.1 requires map keys to be sorted by bytewise lexicographic order of their deterministic encodings. Locator: heading "Core Deterministic Encoding Requirements".
- RFC 8949 §4.2.3 gives the encoded examples `"z" = 0x617a` and `"aa" = 0x626161`. The bytes provide the minimal counterexample for the ordering claim. Retrieved 2026-08-29.
- Executable regressions: `nuif_codec::tests::encoded_key_order_is_not_utf8_order` and `nuif_codec::tests::integer_and_integral_real_remain_distinct`.

## Mechanism

`nuif-cbor-0` preserves the NUIF numeric kind: integers use CBOR major types 0/1; reals always use a CBOR floating-point data item at the shortest exact width. Because the logical model does not distinguish negative real zero, both real zeros use positive floating-point zero, while integer zero remains the integer `0x00`. CBOR maps sort by complete encoded key bytes. `nuif-text-0` independently sorts object keys by UTF-8 bytes because its hash is defined through the parsed CBOR form rather than its textual byte order.

## NUIF relevance

The correction removes a schema-dependent decoding requirement from generic and extension values, prevents numeric type collisions in canonical hashes, and makes the text and CBOR ordering rules independently implementable. RFC 0008 supersedes only the conflicting rules of RFC 0005; its finite-number, strict-decoder, verbatim-string, opaque-byte and hash rules remain applicable.

## Open questions

No open question remains for profile 0. A future profile that intentionally unifies integers and integral reals would define a different logical value model and a different profile identifier.
