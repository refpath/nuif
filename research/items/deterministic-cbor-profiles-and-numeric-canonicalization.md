---
id: nuif:research:deterministic-cbor-profiles-and-numeric-canonicalization
kind: standard
status: reviewed
title: Deterministic CBOR profiles (CDE, draft-ietf-cbor-serialization, dCBOR) and numeric canonicalization for a binary and a text profile with one hash
source:
  url: https://datatracker.ietf.org/doc/draft-ietf-cbor-serialization/
  authors: [Laurence Lundblade, Carsten Bormann, Wolf McNally, Christopher Allen, Anders Rundgren, Bret Jordan, Samuel Erdtman, IETF CBOR Working Group, Rust Project, Blockchain Commons, Enarx Project, Toralf Wittner, quininer]
  published_at: "draft-ietf-cbor-serialization-08 2026-07-29; draft-ietf-cbor-cde-13 2025-10-13 (parked 2025-10-19, expired 2026-04-16); draft-mcnally-deterministic-cbor-18 2026-08-10; RFC 8785 2020-06; RFC 8949 2020-12; RFC 8949 erratum 8589 verified 2025-10-01; ciborium 0.2.2 2024-01-24; minicbor 2.3.0 2026-07-23; dcbor 0.25.2 2026-03-16; cbor4ii 1.2.2 2025-11-30; cbor-edn 0.0.10 2026-03-23; serde_cbor 0.11.2 2021-08-15"
  license: IETF Trust (Internet-Drafts and RFCs); Apache-2.0 OR MIT (Rust standard library); Apache-2.0 (ciborium); BlueOak-1.0.0 (minicbor); BSD-2-Clause-Patent (dcbor); MIT (cbor4ii); MIT OR Apache-2.0 (cbor-edn, serde_cbor)
retrieved_at: 2026-08-29
tags: [cbor, dcbor, cde, deterministic-encoding, canonicalization, floating-point, ieee-754, shortest-round-trip, jcs, hashing, rust, msrv, extension-preservation, decoder-strictness]
confidence: 0.9
claims: [nuif:claim:opaque-preservation]
relations:
  - type: extends
    target: nuif:research:canonicalization-rfc8785-and-cbor-deterministic
    note: Updates the IETF status (CDE parked, draft-ietf-cbor-serialization in WGLC, dCBOR still individual), enumerates the exact normative differences, and adds Rust crate evidence and empirical float-formatting results on the pinned toolchain.
  - type: related_to
    target: nuif:research:encoding
    note: Resolves which deterministic CBOR rule set nuif-cbor-0 references and which crate implements it at MSRV 1.85.
  - type: related_to
    target: nuif:research:content-addressed-versioning
    note: Fixes the byte sequence over which the canonical hash is computed so that content-addressed snapshots are well defined.
  - type: related_to
    target: nuif:research:ipld-dag-cbor-strictness
    note: DAG-CBOR is the precedent for a hash-bearing CBOR profile that forbids NaN and Infinity and keeps byte strings opaque.
  - type: related_to
    target: nuif:research:opentimelineio
    note: OTIO equality tests normalise trailing decimal zeros in JSON, an instance of the integral-float identity problem.
links:
  spec: [spec/08-serialization.md, spec/02-identity-and-properties.md]
  adr: [adrs/0004-serialization.md]
  rfc: [rfcs/0002-extension-preservation.md]
  code: [crates/nuif-codec, crates/nuif-core]
  experiments: []
---

# Summary

As of 2026-08-29 no deterministic-CBOR profile beyond RFC 8949 §4.2 has reached RFC status. draft-ietf-cbor-cde (CDE) passed Working Group Last Call on 2025-03-06, was moved to "Parked WG Document" on 2025-10-19 after the CBOR Working Group found no consensus to continue with the document, and expired on 2026-04-16 at revision -13. The working group adopted draft-lundblade-cbor-serialization as draft-ietf-cbor-serialization on 2025-11-19; revision -08 (2026-07-29, Standards Track) entered Working Group Last Call on 2026-07-30. It defines "preferred-plus serialization" (shortest-form arguments for every major type, definite lengths only, a single NaN `0xf97e00`, no leading zeros in big numbers, mandatory subnormal support in the shortest float width) and "deterministic serialization" (preferred-plus with map entries sorted bytewise-lexicographically by the deterministic encoding of their keys). It keeps the data model: integral floats stay floats, `0.0` is `0xf90000`, and negative zero is not mentioned. draft-mcnally-deterministic-cbor (dCBOR) remains an individual submission at revision -18 (2026-08-10); its title changed from "A Deterministic CBOR Application Profile" (-13, intended Experimental) to "dCBOR: Deterministic CBOR" (-14, intended Standards Track) and its abstract now describes "a set of narrowing rules". dCBOR §2.5 reduces integral floats in [-2^63, 2^64-1] to integers, all zeros to `0x00` and all NaNs to `0xf97e00`; §2.6 permits only `false`, `true`, `null` and floats among simple values; §2.7 requires NFC text; every decoder rule is MUST reject. RFC 8949 erratum 8589 (verified 2025-10-01) adds the sign bit to NaN map-key equivalence. RFC 8785 (JCS) prints numbers with the ECMAScript `Number::toString` algorithm (shortest round-trip digits, exponent notation at magnitude ≥ 10^21 or < 10^-6, `-0` printed as `0`, NaN and Infinity are errors). Rust's `Display` for `f64` (core::num::flt2dec, shortest mode since PR #24612, 2015-05-09) produces the same digit string but a different layout: it prints `-0` for negative zero and never uses exponent notation; `{:e}` (LowerExp) yields the shortest digits in exponent form. Among Rust CBOR crates at MSRV 1.85, only `dcbor` 0.25.2 implements and enforces a complete deterministic rule set (numeric reduction, NaN reduction, NFC, key order, duplicate rejection); `ciborium` 0.2.2 narrows floats and integers to the shortest bit-preserving width but does not sort keys or check input; `minicbor` 2.3.0 and `cbor4ii` 1.2.2 write fixed-width floats and preserve iteration order; `serde_cbor` is unmaintained (RUSTSEC-2021-0127). None of CDE, draft-ietf-cbor-serialization, dCBOR or DAG-CBOR constrains the content of a byte string, so opaque extension payloads survive a strict decoder unchanged.

## Evidence

### Status of the IETF documents

- CDE datatracker history: WG -00 adopted 2023-11-27 from draft-bormann-cbor-cde; Working Group Last Call initiated 2025-03-06 with intended status Best Current Practice; -13 published 2025-10-13; "IETF WG state changed to Parked WG Document from In WG Last Call" 2025-10-19; document expired 2026-04-16; no RFC number. https://datatracker.ietf.org/doc/draft-ietf-cbor-cde/history/ (retrieved 2026-08-29).
- Interim 2025-10-15 minutes (interim-2025-cbor-18): Lundblade "I oppose publication of -cde in its current form. We should publish a document about serialization in general, not about determinism."; Hoffman "So far no consensus on continuing with CDE document, but consensus for CDE topic."; Bormann on chat: consensus that the definite-length-only constraint can be addressed in the same document as determinism. https://datatracker.ietf.org/doc/minutes-interim-2025-cbor-18-202510151400/ (retrieved 2026-08-29).
- Interim 2025-10-01 minutes (interim-2025-cbor-17): Lundblade "Decoder checking has to be optional. Normative behavior can't depend on it." and "Let's keep determinism clean about determinism, and not extend it to defense about malicious input."; Bormann "The encoder has the situation under control and shouldn't need to check." https://datatracker.ietf.org/doc/minutes-interim-2025-cbor-17-202510011400/ (retrieved 2026-08-29).
- IETF 124 minutes (2025-11-07): Lundblade "My document is std track." with "No update to 8949, align closely, eg. new serialization only differs in NaN handling."; Bormann: "We now have 3 choices in the wild" (well-known, legacy canonical, common deterministic); Leiba: technical erratum on NaN verified. https://datatracker.ietf.org/doc/minutes-124-cbor-202511071430/ (retrieved 2026-08-29).
- draft-ietf-cbor-serialization history: WG -00 approved 2025-11-19 (replaces draft-lundblade-cbor-serialization), shepherd Paul E. Hoffman; -08 2026-07-29; "IETF WG state changed to In WG Last Call from WG Document" 2026-07-30. https://datatracker.ietf.org/doc/draft-ietf-cbor-serialization/history/ (retrieved 2026-08-29).
- IETF 125 minutes (2026-03-16) discuss only draft-ietf-cbor-serialization (normative language, test vectors, "2k test vectors" from the hackathon); IETF 126 minutes (2026-07-23): the draft "is close to ready for Working Group Last Call", open items are byte-string handling inconsistencies across bundle protocol, C509 and COSE, "the usual discussion of nontrivial NaNs", and bignums. https://datatracker.ietf.org/doc/minutes-125-cbor-202603160830/ and https://datatracker.ietf.org/doc/minutes-126-cbor/ (retrieved 2026-08-29).
- dCBOR datatracker: revision -18 dated 2026-08-10, "Active Internet-Draft (individual)", no stream, IESG state "I-D Exists", no replaced-by entry, no RFC number; the document header reads "Intended status: Standards Track", "Expires: 11 February 2027", authors McNally, Allen, Bormann, Lundblade. https://datatracker.ietf.org/doc/draft-mcnally-deterministic-cbor/ and https://www.ietf.org/archive/id/draft-mcnally-deterministic-cbor-18.txt (retrieved 2026-08-29).
- dCBOR title history: -11 (2024-08-07) and -13 (2025-08-10) are titled "dCBOR: A Deterministic CBOR Application Profile" with intended status Experimental; -14 (2025-11-01) is titled "dCBOR: Deterministic CBOR" with intended status Standards Track. https://datatracker.ietf.org/doc/draft-mcnally-deterministic-cbor/11/, /13/, /14/ (retrieved 2026-08-29).
- dCBOR -18 references: normative [RFC8949], [RFC8610], [IEEE754], [UNICODE-NORM]; informative [cbor-deterministic] (the CDE draft), [cbor-dcbor], [BCRustDCBOR], [BCSwiftDCBOR], [BCTypescriptDCBOR], [GordianEnvelope]; draft-ietf-cbor-serialization is not referenced. -18 §9 (retrieved 2026-08-29).

### Normative rules

- RFC 8949 §4.2.1: shortest integer arguments, shortest float that preserves the value, no indefinite-length items, map keys sorted bytewise-lexicographically by deterministic encoding; §4.2.2 leaves tags, big integers, negative zero, NaN, subnormals and integral floats to the protocol. https://www.rfc-editor.org/rfc/rfc8949.html#section-4.2 (retrieved 2026-08-29).
- RFC 8949 erratum 8589, Verified 2025-10-01, §5.6.1: NaN values are equivalent as map keys "if they have the same significand after zero-extending both significands at the right to 64 bits, and if they both have the same sign bit." https://errata.rfc-editor.org/rfc8949 (retrieved 2026-08-29).
- draft-ietf-cbor-serialization-08 §4.1 (preferred-plus): shortest-form argument for all major types; definite-length encoding only for strings, arrays and maps; floats "MUST be encoded in the shortest of double, single or half-precision that preserves precision"; "Subnormal numbers MUST be supported in this shortest-length encoding"; "For example, 0.0 can always be reduced to half-precision so it MUST be encoded as 0xf90000"; "Encoders MUST NOT output any NaN other than the half-precision NaN 0xf9 0x7e 0x00"; a value representable in major type 0 or 1 "MUST be encoded with major type 0 or 1, never as a big number"; no leading zeros in big numbers. §5.1 (deterministic): "If a map is encoded, the items in it MUST be sorted in the bytewise lexicographic order of their deterministic encodings of the map keys." §1.3: "This document defines new serializations rather than updating those in [STD94]". §8: the CDDL control `.serial` "applies recursively through nested arrays and maps, but does not extend into byte strings". Appendix H: checking decoders are permitted, not required. Appendix I.1: wrapping CBOR in a byte string isolates encoding errors of the wrapped data. Negative zero, integral-float reduction and duplicate keys are not addressed in the retrieved text. https://www.ietf.org/archive/id/draft-ietf-cbor-serialization-08.html and .txt (retrieved 2026-08-29).
- CDE -13 §3.1.2: shortest head that preserves the value; "an encoder that is asked by an application to represent a negative floating point zero (-0.0) will generate 0xf98000"; "there is no attempt to mix integers and floating point numbers"; typical applications encode the quiet non-negative NaN as `0xf97e00`. §3.3: each key MUST be lexicographically strictly greater than the preceding key (which excludes duplicates). Appendix B: Application-level Deterministic Representation (ALDR) rules are "a concept that is separate from CDE itself"; "An early example of a separate document is the dCBOR specification", which "specifies the use of CDE together with some application-level rules, i.e., an ALDR ruleset"; ALDR rules "do not 'fork' CBOR". Appendix C.3.2: CDE-checking decoders "MUST check the input for keeping the preferred-serialization and definite-length-only encoding constraints" and "MUST NOT present to the application a decoded data item that fails one of these checks"; generic decoders are not required to check. §4: CDDL controls `.cde` and `.cdeseq` require the byte-string content to be CDE, for example `leaf = #6.24(bytes .cde any)`. https://www.ietf.org/archive/id/draft-ietf-cbor-cde-13.html and the "disentangle" editor's copy https://cbor-wg.github.io/draft-ietf-cbor-cde/disentangle/draft-ietf-cbor-cde.html (retrieved 2026-08-29).
- dCBOR -18: §2.1 definite lengths, decoders MUST reject indefinite-length items; §2.2 encoders MUST emit only preferred serialization, decoders MUST validate and reject; §2.3 bytewise-lexicographic key order, decoders MUST validate; §2.4 decoders MUST reject duplicate keys; §2.5 numeric reduction: encoders "MUST check whether floating point values to be encoded have the numerically equal value in DCBOR_INT = [-2^63, 2^64-1]" and convert them to that integer, "the three representations of a zero number in CBOR (0, 0.0, -0.0 in diagnostic notation) are all reduced to the basic integer 0", encoders "MUST reduce all encoded NaN values to the quiet NaN value having the half-width CBOR representation 0xf97e00", decoders "MUST reject any encoded floating point values that are not encoded according to the above rules"; §2.6 only `false` (0xf4), `true` (0xf5), `null` (0xf6) and floats are valid simple values, decoders MUST reject others; §2.7 encoders "MUST only emit text strings that are in NFC", decoders "MUST reject any encoded text strings that are not in NFC"; §3 tag 201 declares enclosed dCBOR "at the data model level and the encoded data item level"; §4 implementation status lists Swift, Rust and TypeScript (Blockchain Commons, BSD-2-Clause-Patent) and Ruby (Bormann, Apache-2.0, exclusion checking not implemented); §7.1 test vectors include the smallest half, single and double subnormals; §7.2 Table 4 invalid encodings include `12.0` as `f94a00` ("Can be reduced to 12"), `-2^63-1` as `3b8000000000000000` and `-2^64` as `3bffffffffffffffff` ("65-bit negative integer value"). No rule constrains byte-string content. https://www.ietf.org/archive/id/draft-mcnally-deterministic-cbor-18.txt (retrieved 2026-08-29).
- RFC 8949 §3.4.5.1 (tag 24): a contained byte string "is valid if it encodes a well-formed CBOR data item"; §3.1 major type 2 carries an arbitrary byte sequence whose length is the argument. https://www.rfc-editor.org/rfc/rfc8949.html#section-3.4.5.1 (retrieved 2026-08-29).
- RFC 8785 §1: hashing and signing "need the data to be expressed in an invariant format"; §3.2.2.3: numbers "MUST be serialized according to Section 7.1.12.1 of [ECMA-262], including the 'Note 2' enhancement", NaN and Infinity "MUST cause a compliant JCS implementation to terminate with an appropriate error"; Appendix B: `0000000000000000` and `8000000000000000` both serialise as `0`, `0000000000000001` as `5e-324`, `4340000000000000` as `9007199254740992`, `7fefffffffffffff` as `1.7976931348623157e+308`. https://www.rfc-editor.org/rfc/rfc8785.html (retrieved 2026-08-29).
- ECMAScript `Number::toString` layout as documented by MDN: "Scientific notation is used if radix is 10 and the number's magnitude (ignoring sign) is greater than or equal to 10^21 or less than 10^-6"; "Both 0 and -0 have '0' as their string representation"; the algorithm "uses the least number of significant figures necessary to distinguish the output from adjacent number values". https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Number/toString (retrieved 2026-08-29). The ECMA-262 §6.1.6.1.20 algorithm text itself was not retrieved in this session (the multipage fetch returned only the table of contents); the RFC 8785 citation of ECMA-262 §7.1.12.1 with Note 2 is the normative locator.

### Rust float formatting

- rust-lang/rust PR #24612 "New floating-to-decimal formatting routine" (lifthrasiir, merged 2015-05-09) introduced `core::num::flt2dec` with Grisu3 and a Dragon4 fallback; "all specifiers default to the shortest representation". https://github.com/rust-lang/rust/pull/24612 (retrieved 2026-08-29).
- `library/core/src/num/flt2dec/mod.rs` at tag 1.85.0, lines 13–17: the shortest mode output is "correctly rounded when parsed back", "shortest such one, i.e., there is no representation with less than n digits that is correctly rounded", and "closest to the original value"; lines 18–32 name this the shortest mode. https://raw.githubusercontent.com/rust-lang/rust/1.85.0/library/core/src/num/flt2dec/mod.rs (retrieved 2026-08-29).
- `library/core/src/fmt/float.rs` at tag 1.85.0: `float_to_decimal_display` (lines 77–88) calls `float_to_decimal_common_shortest` with `Sign::Minus` when no precision is given; the Debug implementation switches to exponent form when `(abs != 0.0 && abs < 1e-4) || abs >= 1e+16` (lines 15–18); the `floating!` macro (line 226) implements Display, Debug, LowerExp and UpperExp for f32 and f64. https://raw.githubusercontent.com/rust-lang/rust/1.85.0/library/core/src/fmt/float.rs (retrieved 2026-08-29).
- Empirical run of a 30-line probe compiled with `rustc 1.85.0 (4d91de4e4 2025-02-17)`, edition 2024, on this repository's pinned toolchain (2026-08-29):

| bits (binary64) | `{}` Display | `{:?}` Debug | `{:e}` LowerExp | JCS Appendix B / ECMAScript |
|---|---|---|---|---|
| `0000000000000000` | `0` | `0.0` | `0e0` | `0` |
| `8000000000000000` | `-0` | `-0.0` | `-0e0` | `0` |
| `3ff0000000000000` | `1` | `1.0` | `1e0` | `1` |
| `3fb999999999999a` | `0.1` | `0.1` | `1e-1` | `0.1` |
| `444b1ae4d6e2ef50` (1e21) | `1000000000000000000000` | `1e21` | `1e21` | `1e+21` |
| `4415af1d78b58c40` (1e20) | `100000000000000000000` | `1e20` | `1e20` | `100000000000000000000` |
| `3e7ad7f29abcaf48` (1e-7) | `0.0000001` | `1e-7` | `1e-7` | `1e-7` |
| `3eb0c6f7a0b5ed8d` (1e-6) | `0.000001` | `1e-6` | `1e-6` | `0.000001` |
| `0000000000000001` | `0.000…005` (326 characters) | `5e-324` | `5e-324` | `5e-324` |
| `7fefffffffffffff` | 309 digits | `1.7976931348623157e308` | `1.7976931348623157e308` | `1.7976931348623157e+308` |
| `4340000000000000` | `9007199254740992` | `9007199254740992.0` | `9.007199254740992e15` | `9007199254740992` |

  Every Display string parsed back to the identical bit pattern. `f32` values widened to `f64` print under the f64 shortest rule: `0.1f32` prints as `0.10000000149011612` after widening, and as `0.1` when formatted as `f32`. `f64::NAN`, `INFINITY` and `NEG_INFINITY` print as `NaN`, `inf`, `-inf`.

### Rust crates at MSRV 1.85

| Crate | Version (date) | Licence | MSRV or edition | Deterministic rules | Unknown tags | Decoding model |
|---|---|---|---|---|---|---|
| ciborium | 0.2.2 (2024-01-24) | Apache-2.0 | crates.io metadata 1.58; main-branch `Cargo.toml` `rust-version = "1.85"`, edition 2021 | integers shortest (`ciborium-ll/src/hdr.rs` lines 85–91); floats narrowed to f16 or f32 when the widened value is bit-identical (lines 104–118); map keys written in caller order (`ciborium/src/ser/mod.rs` lines 269–274); no canonical check on input; docs: "liberal in what we accept" | `Value::Tag(u64, Box<Value>)` retains any tag | whole `Value` tree or serde; `Value::Map` is `Vec<(Value, Value)>` preserving wire order |
| minicbor | 2.3.0 (2026-07-23) | BlueOak-1.0.0 | MSRV unspecified; edition 2024 (requires 1.85 or later) | `Encoder::f16`, `f32`, `f64` write the requested width; no sorting or canonical check documented | derive ignores unknown fields; manual `Decoder` sees every tag | non-allocating `Decoder` with `position`, `set_position`, `probe`, `skip`, `tokens` |
| cbor4ii | 1.2.2 (2025-11-30) | MIT | unspecified | `src/core/enc.rs` lines 323–339 write f32 and f64 at full width; maps in iteration order (lines 316–322); datetime, bignum and bigfloat not implemented | not documented | serde and core API |
| dcbor | 0.25.2 (2026-03-16) | BSD-2-Clause-Patent | MSRV unspecified; edition 2024; `no_std` feature; deps `half ^2.4.1` (half 2.7.1 declares MSRV 1.81), `unicode-normalization ^0.1.22`, `chrono ^0.4.28`; no serde | README enforces shortest integers and floats, key order, definite lengths, duplicate rejection, numeric reduction to [-2^63, 2^64-1], single NaN, simple-value restriction, NFC; `CBOR::try_from_data` rejects data that "violates dCBOR encoding rules" or has trailing content; `Error` variants `NonCanonicalNumeric`, `NonCanonicalString`, `MisorderedMapKey`, `DuplicateMapKey`, `UnusedData`, `InvalidSimpleValue`, `UnsupportedHeaderValue`, `Underrun` | `CBORCase::Tagged(Tag, CBOR)` holds any tag | whole reference-counted tree |
| cbor-edn | 0.0.10 (2026-03-23) | MIT OR Apache-2.0 | MSRV 1.76 | diagnostic-notation converter, not a canonical codec | n/a | n/a |
| serde_cbor | 0.11.2 (2021-08-15) | MIT/Apache-2.0 | n/a | RUSTSEC-2021-0127 (2021-11-30): unmaintained, repository archived; suggested replacements ciborium and minicbor | n/a | n/a |

Sources: https://crates.io/api/v1/crates/{ciborium,minicbor,dcbor,cbor4ii,cbor-edn,serde_cbor}; https://raw.githubusercontent.com/enarx/ciborium/main/ciborium/Cargo.toml; https://raw.githubusercontent.com/enarx/ciborium/main/ciborium-ll/src/hdr.rs; https://raw.githubusercontent.com/enarx/ciborium/main/ciborium/src/ser/mod.rs; https://docs.rs/ciborium/latest/ciborium/; https://raw.githubusercontent.com/twittner/minicbor/develop/minicbor/Cargo.toml; https://docs.rs/minicbor/latest/minicbor/encode/struct.Encoder.html; https://docs.rs/minicbor/latest/minicbor/decode/struct.Decoder.html; https://docs.rs/minicbor-derive/latest/minicbor_derive/; https://raw.githubusercontent.com/quininer/cbor4ii/master/src/core/enc.rs; https://raw.githubusercontent.com/BlockchainCommons/bc-dcbor-rust/master/Cargo.toml; https://raw.githubusercontent.com/BlockchainCommons/bc-dcbor-rust/master/README.md; https://docs.rs/dcbor/latest/dcbor/struct.CBOR.html; https://docs.rs/dcbor/latest/dcbor/enum.Error.html; https://docs.rs/dcbor/latest/dcbor/enum.CBORCase.html; https://raw.githubusercontent.com/starkat99/half-rs/main/Cargo.toml; https://rustsec.org/advisories/RUSTSEC-2021-0127.html (all retrieved 2026-08-29).

### Numeric values in authored interface documents

- nuif-core: `SizeIntent::Fixed(f64)` (`crates/nuif-core/src/lib.rs` line 54), `EntityId(pub u128)` (line 6), `Extensions(pub BTreeMap<String, Vec<u8>>)` (line 60); no `f32` and no integer-typed property exists in the current model. spec/03 lists "number" as a parameter class; spec/04 lists percentage sizing; spec/05 requires affine transforms and colour values with a declared colour space; no timestamp property is specified (grep over `spec/` and `rfcs/`, 2026-08-29).
- DTCG Format Module 2025.10: colour `components` are numbers in the range 0–1 (`[0, 0.4, 0.8]`), `dimension.value` is "a numeric value (integer or floating-point)", `number` "MUST be a JSON number value", `duration.value` is integer or floating-point; no precision constraint. https://www.w3.org/community/reports/design-tokens/CG-FINAL-format-20251028/ (retrieved 2026-08-29).
- glTF 2.0 JSON encoding: integer-typed properties "MAY be stored as decimals with a zero fractional part or by using exponent notation" and "MUST NOT contain any non-zero fractional value"; floating-point values `NaN`, `+Infinity`, `-Infinity` "MUST NOT be present"; non-integer numbers "SHOULD be written in a way that preserves original values" across a round trip. https://raw.githubusercontent.com/KhronosGroup/glTF/main/specification/2.0/Specification.adoc, section "JSON Encoding" (retrieved 2026-08-29; the GLB chunk section was beyond the retrievable length and is not cited).
- OTIO test utilities compare JSON with trailing-decimal-zero normalisation (existing record nuif:research:opentimelineio, evidence line for `test_utils.py` lines 15–31).

### Hash precedents

- IPLD CID: `<cidv1> ::= <CIDv1-multicodec><content-type-multicodec><content-multihash>`; a CID is "a tuple of (content-type, content-address)", so the same data under two codecs has two CIDs. https://github.com/multiformats/cid (retrieved 2026-08-29).
- Automerge binary format: "A change hash is the 32-byte SHA256 hash of the concatenation of the chunk type (0x01) chunk length and chunk contents fields of a change represented as a Change Chunk"; "Implementations must generate the shortest possible uLEB encodings, and should reject documents with overly long encodings." https://automerge.org/automerge-binary-format-spec/ (retrieved 2026-08-29).
- OpenUSD: `.usdc` is "losslessly, bidirectionally convertible to the .usda text format" and `usdcat --usdFormat usda|usdc` converts between them; no content hash is defined (existing record nuif:research:openusd-composition-and-crate, evidence lines for the glossary and toolset; https://openusd.org/release/toolset.html retrieved 2026-08-29).

## Mechanism

Rule matrix (source statements; "unspecified" means the retrieved text contains no rule):

| Rule | RFC 8949 §4.2.1 | CDE -13 | draft-ietf-cbor-serialization-08 deterministic | dCBOR -18 | DAG-CBOR |
|---|---|---|---|---|---|
| integer arguments | shortest | shortest | shortest, big numbers only above 64-bit range | shortest; 65-bit negatives rejected | shortest, signed 64-bit range |
| float width | shortest preserving value | shortest preserving value | shortest of double, single, half | shortest after reduction | always binary64 |
| integral float | protocol decides | stays float | stays float | integer if in [-2^63, 2^64-1] | stays float |
| -0.0 | protocol decides | `0xf98000` | unspecified | `0x00` | should not appear; encode as `0x0000000000000000` |
| NaN | protocol decides | typically `0xf97e00` | only `0xf97e00` | only `0xf97e00` | rejected |
| Infinity | allowed | allowed | allowed | allowed | rejected |
| subnormals | protocol decides | shortest form | must be supported in shortest form | test vectors include them | binary64 |
| map order | bytewise lexicographic of encoded keys | same, strictly increasing | same | same | length-first then bytewise (RFC 7049 order) |
| duplicate keys | disallowed by sorting | excluded by strict order | unspecified | decoders MUST reject | rejected |
| text normalisation | none | none | none | NFC, decoders MUST reject | none |
| simple values | any | any | any | `false`, `true`, `null`, floats | `false`, `true`, `null`, floats |
| decoder strictness | not required | checking decoders MUST reject; generic decoders exempt | checking decoders optional | MUST reject every deviation | should reject; decoders may relax by default |
| byte-string content | opaque | opaque unless `.cde` control applied | opaque; `.serial` stops at byte strings | opaque | opaque |

Digit generation and layout. Both JCS (via ECMAScript) and Rust `Display` emit the shortest decimal digit string that parses back to the same binary64 value, and both pick the closest candidate; the two algorithms differ only in layout: negative zero (`0` in JCS, `-0` in Rust) and exponent thresholds (JCS switches to `d.ddde±x` outside 10^-6 ≤ |v| < 10^21; Rust `Display` never switches, Rust `Debug` switches outside 10^-4 ≤ |v| < 10^16). A text profile can therefore specify a layout over the shortest digits without ECMAScript semantics:

```
digits, exp := shortest round-trip decimal of v as produced by `{:e}` (d[.ddd]e[-]x)
n := exp + 1                               // position of the decimal point relative to the digits
if 0 < n <= 21:  integer part = digits padded with zeros to n places, fraction = remaining digits
if -6 < n <= 0:  "0." + (-n zeros) + digits
otherwise:       d[.ddd] + "e" + sign + |n-1|
```

The `{:e}` output on rustc 1.85.0 for the probe values (`1e21`, `5e-324`, `1.7976931348623157e308`, `9.007199254740992e15`) contains the digit strings JCS Appendix B expects; the layout step adds the explicit `+` and the threshold switch.

Typed reduction. dCBOR's integral-float reduction is lossless only when the reader knows the type of the slot. For a property typed `real` the wire item `0x01` decodes to `1.0`, and for a property typed `integer` the encoder never produces a float; the reduction is then a wire-level normalisation. In an untyped slot (a generic `Value`, or an extension payload interpreted by a foreign decoder) `1` and `1.0` collapse, which is exactly the problem OTIO's trailing-zero normalisation and glTF's "integer stored as 1.0" rule work around at the JSON layer.

Negative zero. Authored input rarely contains `-0.0`; it results from arithmetic (negation of zero, products of a negative factor with zero as in a mirroring transform, rounding of small negative results). IEEE 754 equality treats it as equal to `+0.0`; NUIF geometry has no property whose meaning depends on the sign of zero. Reducing it removes a source of hash instability between an authored `0` and a computed `-0.0`.

Width. The same real number stored as `f32` and as `f64` yields the same shortest binary width under every CBOR profile (the widened `f32` value is exactly representable in binary32), but not the same shortest decimal string (`0.1` versus `0.10000000149011612`). A logical model with a single real type (binary64) keeps binary and text canonicalisation consistent.

Hash definition. Every precedent hashes one designated byte sequence (JCS text, DAG-CBOR block, Automerge change chunk) and does not claim identity across encodings; IPLD makes the codec part of the identifier. The workable definition of "the same hash from text and binary" is therefore: the hash is computed over the `nuif-cbor-0` bytes, and `nuif-text-0` is defined as a lossless surface syntax over the same value set, so that `hash(text)` is by definition `hash(cbor(parse(text)))`. This requires the value sets to coincide: binary64 reals, integers within the CBOR 64-bit range, one NaN, one zero, NFC or verbatim text chosen once.

Strictness and opaque payloads. A strict decoder (dCBOR, CDE-checking, DAG-CBOR) rejects non-canonical structure but never inspects byte-string content, so an extension payload carried as a byte string survives byte-for-byte whatever its internal format. Tag 24 is unsuitable for opaque payloads because RFC 8949 §3.4.5.1 requires the content to be well-formed CBOR. If a registered extension promises canonical CBOR content, the promise can be expressed with the `.cde`-style CDDL control and checked by the extension's own decoder, not by the container decoder.

## NUIF relevance

**Borrow**
- draft-ietf-cbor-serialization-08 §4.1 and §5.1 as the structural base of `nuif-cbor-0` (shortest arguments, definite lengths, shortest float width with subnormals, single NaN `0xf97e00`, bytewise-lexicographic key order, no big-number tags inside the 64-bit range), because it is the only deterministic-CBOR text in Working Group Last Call and it is Standards Track.
- dCBOR §2.4, §2.5 and §2.6 (duplicate rejection, integral-float reduction within [-2^63, 2^64-1], zero reduction to `0x00`, NaN reduction, simple-value restriction) as the application-level rules, stated in the NUIF specification by value rather than by reference, because dCBOR is an individual draft whose text may change.
- dCBOR's decoder rule set (MUST reject every deviation) for canonical hash inputs, so that hash equality implies byte equality.
- The `dcbor` crate (0.25.2, BSD-2-Clause-Patent, edition 2024, `no_std` capable) as the initial `nuif-cbor-0` implementation, behind the existing `nuif-codec` traits, because no other crate at MSRV 1.85 checks canonical form on input.
- Shortest round-trip digits from `core::fmt` `{:e}` plus a fixed layout for `nuif-text-0`, matching JCS digit strings without ECMAScript.
- Byte strings for opaque extension payloads; no tag 24; no content check by the container decoder.

**Adapt**
- The logical model needs exactly two numeric types, `integer` (i64/u64 within the CBOR range) and `real` (binary64); `f32` colour components and percentages are stored as binary64 in the model and narrowed on the wire by the shortest-width rule. The empirical width difference in text output is the reason.
- dCBOR's NFC rule becomes a data-model rule on text properties (spec/02), decided once for both profiles; whichever choice is made, the text profile and the binary profile apply it identically or the hashes diverge.
- The `-0` and exponent-layout differences between Rust `Display` and JCS mean `nuif-text-0` MUST NOT be specified as "print with Rust Display" and MUST NOT be specified as "print with ECMAScript"; it is specified by the digit-plus-layout rule above.
- Because negative zero reduces to `0` in the canonical form, `SizeIntent::Fixed(-0.0)` and `Fixed(0.0)` hash equally; `PartialEq` on `f64` already treats them as equal, so the in-memory model and the hash agree.

**Reject**
- CDE -13 as a normative reference: parked and expired, and its `-0.0` rule (`0xf98000`) contradicts the zero reduction chosen here.
- DAG-CBOR's length-first key order and always-binary64 floats: they are RFC 7049 legacy choices that no current IETF text recommends.
- Accept-and-recanonicalize decoding for hash inputs: it makes one hash identify several byte sequences and depends on decoder-specific behaviour.
- `serde_cbor` (unmaintained), and `ciborium`, `minicbor` or `cbor4ii` as canonical encoders without an additional canonicalisation layer (none sorts keys or checks input; two write fixed-width floats).

## Open questions

- Whether draft-ietf-cbor-serialization will add a negative-zero rule or a duplicate-key rule before publication; the NUIF profile states both explicitly so that the outcome does not change `nuif-cbor-0`.
- Whether the `dcbor` crate's 0.x API and its tracking of dCBOR revisions are stable enough for a reference implementation, or whether `nuif-codec` needs an independent canonical encoder over `ciborium-ll` with a conformance test against the dCBOR §7 test vectors.
- Whether Rust's shortest-digit tie-breaking (closest value, `flt2dec` lines 13–17) and ECMA-262 Note 2 (closest value, even digit on ties) ever differ for binary64; no counterexample was found in this session and the question is unverified.
- Whether text properties store NFC by definition (dCBOR §2.7) or code points verbatim; the choice changes round-trip fidelity of imported documents and must be made in spec/02 before either profile is frozen.
- Whether registered extensions that carry CBOR inside their payload are required to be canonical (`.cde`-style control in their CDDL) so that extension payloads remain diffable, or whether only opaque preservation is promised.
