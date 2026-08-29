---
id: nuif:research:ipld-dag-cbor-strictness
kind: standard
status: reviewed
title: IPLD DAG-CBOR strictness rules and content identifiers as a hash-bearing CBOR profile
source:
  url: https://ipld.io/specs/codecs/dag-cbor/spec/
  authors: [IPLD project, Protocol Labs, multiformats project]
  published_at: null
  license: MIT OR Apache-2.0 (IPLD specifications repository)
retrieved_at: 2026-08-29
tags: [ipld, dag-cbor, cbor, content-addressing, cid, deterministic-encoding, floating-point, decoder-strictness]
confidence: 0.9
claims: []
relations:
  - type: related_to
    target: nuif:research:deterministic-cbor-profiles-and-numeric-canonicalization
    note: DAG-CBOR is the deployed precedent for forbidding NaN and Infinity and for opaque byte strings under a strict decoder.
  - type: extends
    target: nuif:research:content-addressed-versioning
    note: Specifies how IPFS derives content identifiers from one designated encoding and includes the codec in the identifier.
  - type: compares_to
    target: nuif:research:canonicalization-rfc8785-and-cbor-deterministic
    note: DAG-CBOR predates CDE and keeps the RFC 7049 length-first key order and fixed binary64 floats.
links:
  spec: [spec/08-serialization.md]
  adr: [adrs/0004-serialization.md]
  rfc: []
  code: [crates/nuif-codec]
  experiments: []
---

# Summary

DAG-CBOR is the IPLD codec that encodes the IPLD Data Model in CBOR for content addressing. Its specification states that "DAG-CBOR requires that there exist a single, canonical way of encoding any given set of data, and that encoded forms contain no superfluous data that may be ignored or lost in a round-trip decode/encode." The strictness section fixes seven rules: only tag 42 (CID links) is permitted and decoders must reject other tags; the RFC 8949 §4.2 rules are applied with shortest integer and length arguments, keys "sorted in (byte-wise) lexical order, including their major type 3 and length" (therefore length first), and no indefinite-length items; only major-type-7 minors 20, 21, 22, 25, 26 and 27 are usable; floats "must always encoded in 64-bit, double-precision form"; `NaN`, `Infinity` and `-Infinity` "must not be accepted as they do not appear in the IPLD Data Model"; `-0.0` "should not appear or be accepted" and zero is always `0x0000000000000000`; encoders and decoders handle a single top-level item. Decoders "should reject encoded forms not adhering to" the rules but "may relax strictness requirements by default" for historical data. Links are CIDs prefixed with the identity multibase byte `0x00` inside a byte string under tag 42. A CID is a typed content address `(content-type, content-address)` whose bytes include the codec multicodec, so identical data under two codecs yields two identifiers. The IPLD Data Model defines floats as IEEE 754 values "excluding special values such as NaN, Infinity and -Infinity" and notes that DAG-CBOR restricts integers to the signed 64-bit range.

## Evidence

- Strictness introduction: "DAG-CBOR requires that there exist a single, canonical way of encoding any given set of data, and that encoded forms contain no superfluous data that may be ignored or lost in a round-trip decode/encode." https://ipld.io/specs/codecs/dag-cbor/spec/ section "Strictness" (retrieved 2026-08-29).
- Rule 1: "Use no tags other than the CID tag (42). A valid DAG-CBOR encoder must not encode using any additional tags and a valid DAG-CBOR decoder must reject objects containing additional tags as invalid." Same section (retrieved 2026-08-29).
- Rule 2: apply "the 'Deterministically Encoded CBOR' rule suggestions defined in section 4.2 of RFC 8949"; "a valid DAG-CBOR decoder should reject encoded forms not adhering to the following rules": integer encoding "must be as short as possible", lengths of major types 2–5 as short as possible, tag 42 as short as possible, "The keys in every map must be sorted in (byte-wise) lexical order, including their major type 3 and length. Therefore, the keys are sorted by length first.", "Indefinite-length items are not supported, only definite-length items are usable." Same section (retrieved 2026-08-29).
- Rule 3: "The only usable major type 7 minor types are those for encoding Floats (minors 25, 26, 27), False (minor 20), True (minor 21) and Null (minor 22)." Rule 4: "Floating point values must always encoded in 64-bit, double-precision form, regardless of whether they can be represented as half (16) or single (32) precision." Rule 5: "IEEE 754 special values NaN, Infinity and -Infinity must not be accepted as they do not appear in the IPLD Data Model." Rule 6: "The floating point value -0.0 should not appear or be accepted", zero "always be encoded as 0x0000000000000000". Rule 7: a single top-level CBOR object. Same section (retrieved 2026-08-29).
- Decoder relaxation: "DAG-CBOR decoders may relax strictness requirements by default" to accept historical data. Same page (retrieved 2026-08-29).
- Links: "the Multibase identity prefix (0x00) is prepended to the binary form of a CID and this new byte array is encoded into CBOR as a byte-string (major type 2), and associated with CBOR tag 42"; the identity prefix "must not be omitted". Section "Links" (retrieved 2026-08-29).
- IPLD Data Model kinds: floats are "roughly what you'd expect from IEEE 754 floats, but excluding special values such as NaN, Infinity and -Infinity"; "Some codecs, such as DAG-CBOR, will assume that integers must be within the 64-bit signed range and reject anything larger"; bytes "are not considered to have any character encoding". https://ipld.io/docs/data-model/kinds/ (retrieved 2026-08-29).
- CID: `<cidv1> ::= <CIDv1-multicodec><content-type-multicodec><content-multihash>`; "A CID is a self-describing content-addressed identifier... a typed content address: a tuple of (content-type, content-address)". https://github.com/multiformats/cid (retrieved 2026-08-29).

## Mechanism

DAG-CBOR selects, for each Data Model value, one byte sequence: no tag other than 42, shortest heads, length-first key order (the RFC 7049 canonical order that RFC 8949 §4.2.3 retains only for compatibility), fixed binary64 floats, and a value set that excludes NaN, the infinities and negative zero. The content identifier is the multihash of that byte sequence prefixed by the codec code. Determinism is therefore established at the codec boundary, and cross-codec identity is not claimed: DAG-JSON of the same value has a different CID. Byte strings are Data Model `bytes`; the codec never interprets their content, so nested payloads survive strict decoding unchanged. Decoder strictness is "should reject" with an explicit allowance for lenient defaults, which the specification justifies by deployed data that predates the rules.

## NUIF relevance

**Borrow**
- The value-set restriction pattern: exclude NaN and the infinities and collapse negative zero at the data-model level, so that every remaining value has one encoding.
- The single-top-level-item rule and the "no tags except the ones the profile defines" rule for `nuif-cbor-0`.
- Opaque byte strings as the carrier for content the codec must not interpret (CID links in DAG-CBOR; extension payloads in NUIF).
- Including the encoding profile identifier in any content-addressed identifier NUIF publishes, following the CID structure, so that a `nuif-cbor-0` hash is never confused with a hash of a later profile.

**Adapt**
- NUIF geometry cannot exclude NaN from arithmetic, but it can exclude NaN from authored properties; a NUIF validator rejects NaN and infinities at property-set time, as DAG-CBOR does at decode time.
- "May relax strictness by default" is acceptable for a general reader but not for the hash path; NUIF needs two decoder modes with the strict mode mandatory for canonical hashing.

**Reject**
- Length-first key order and always-binary64 floats; RFC 8949 §4.2.1 and the current IETF drafts specify bytewise-lexicographic order and shortest float width.
- The signed 64-bit integer limit as a data-model rule; NUIF integers stay within the CBOR major type 0/1 range and do not need the extra restriction.

## Open questions

- Whether NUIF content identifiers should be multihash-encoded CIDs (codec plus hash) or a NUIF-specific tuple; interoperability with IPFS tooling is the only reason to prefer CIDs.
- Whether the "single top-level item" rule holds for `.nuif` packages, which contain several records, or whether the package manifest is the single item and records are byte strings addressed from it.
