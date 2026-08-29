# RFC 0005 — Deterministic numeric and string canonicalization

Status: accepted (decision delegated to research on 2026-08-29; evidence in `nuif:research:deterministic-cbor-profiles-and-numeric-canonicalization`, `nuif:research:ipld-dag-cbor-strictness`, `nuif:research:canonicalization-rfc8785-and-cbor-deterministic`)

## Motivation

`spec/08-serialization.md` requires byte-stable canonical hashes for `nuif-cbor-0` and `nuif-text-0` but leaves numeric normalization, map ordering and string rules undefined. The IETF CBOR working group's Common Deterministic Encoding draft (draft-ietf-cbor-cde) was parked on 2025-10-19 for lack of consensus and expired on 2026-04-16 without an RFC; its successor, draft-ietf-cbor-serialization (revision 08, 2026-07-29, Standards Track, in working-group last call), preserves the data model and is silent on negative zero and duplicate keys. The individual draft dCBOR (revision 18, 2026-08-10) narrows the data model (integral floats become integers, all zeros become `0x00`, one NaN, NFC strings, strict decoders) and is not working-group adopted. NUIF must state its rules by value so that neither draft's future changes alter NUIF hashes.

## Prior art

RFC 8949 §4.2 (core deterministic encoding requirements and the list of decisions left to protocols); RFC 8949 erratum 8589 (NaN sign bit in key equivalence); RFC 8785 (JSON Canonicalization Scheme: ECMAScript shortest number serialization, `-0` to `0`, NaN and Infinity rejected); IPLD DAG-CBOR (strict codec: shortest floats, no NaN or Infinity, no indefinite lengths, content identifiers carry the codec identifier); Automerge's binary change format (hash over a defined byte layout); glTF JSON and GLB (same document, two containers, no cross-container hash equivalence claimed).

## Decision

### Logical numeric model

1. The logical model has two numeric kinds: `integer` (signed 64-bit) and `real` (IEEE 754 binary64). There is no 32-bit real kind in the model; adapters convert.
2. An authored `real` property MUST NOT hold NaN, positive infinity or negative infinity. Validation rejects such values at set time with a diagnostic. Resolved snapshots MAY carry non-finite values only inside diagnostics, never as resolved geometry.
3. Negative zero has no distinct identity in the logical model: `-0.0` and `+0.0` are the same value and canonicalize identically.

### `nuif-cbor-0`

4. The base profile is draft-ietf-cbor-serialization §4.1 (preferred serialization: shortest integer heads, definite lengths only, shortest float width that preserves the value including subnormals, the single NaN encoding `0xf97e00`, no bignum tags for values within the 64-bit range) and §5.1 (map keys sorted by bytewise lexicographic order of their encoded form). NUIF cites these sections by name and restates every rule it depends on below so that a change in the draft does not change NUIF.
5. Integers encode as major type 0 or 1 with the shortest argument. Values outside `[-2^63, 2^63-1]` are invalid for `integer` properties; the wire range beyond that is never produced.
6. A `real` whose value is integral and within `[-2^63, 2^63-1]` MUST be encoded as an integer (major type 0 or 1). A decoder that knows the property type restores the real. This is a wire-level narrowing, not a semantic change, because property types are declared by the schema.
7. A `real` that is not integral MUST be encoded as the shortest of half, single or double precision that round-trips the value exactly. Subnormals are preserved.
8. Both zeros MUST encode as `0x00` (integer zero).
9. NaN and infinities never occur in canonical documents (rule 2). If a future property type admits them, the encoding is `0xf97e00` for NaN and the shortest-width infinity.
10. Simple values other than `false`, `true` and `null` are not used. No tags appear in the canonical body.
11. Map keys MUST be in strictly increasing bytewise lexicographic order of their encoded form; a duplicate key is invalid.
12. Decoders used for hashing or conformance MUST be strict: any deviation from rules 4–11 is rejected with a diagnostic. Decoders MUST NOT re-canonicalize accepted input silently; a lenient import path MAY exist for foreign data and MUST report `approximated` when it rewrote bytes.
13. Extension payloads and unknown-kind payloads are CBOR byte strings and are hashed verbatim; the container decoder never inspects their content. Tag 24 (embedded CBOR) is not used because it requires well-formed content.

### `nuif-text-0`

14. The text profile is a lossless surface syntax over the same value set. Its canonical hash is defined as the hash of the `nuif-cbor-0` encoding of the parsed document: `hash(text) = hash(cbor(parse(text)))`. No separate text hash exists.
15. Reals print as the shortest decimal digit string that round-trips to the same binary64 value (Rust `core::num::flt2dec` shortest mode is a conforming implementation), laid out as plain decimal when `10^-6 <= |v| < 10^21` and otherwise as `d.ddde±x`. Integral reals print without a fraction. `-0` is never printed. `NaN`, `inf` and `-inf` are parse errors.
16. Integers print in decimal without leading zeros or a plus sign.
17. Map keys are written in UTF-8 byte order, which coincides with rule 11 for the identifier repertoire in rule 19.
18. Whitespace, comments and key quoting styles are not significant and are not preserved; the text encoder emits one fixed layout.

### Strings and identifiers

19. Namespaces, property keys, extension names and entity kind names are identifiers restricted to `[a-z0-9][a-z0-9_.:-]*`. They are compared bytewise.
20. String property values (names, text content, token names) are stored verbatim as valid UTF-8 and are never normalized by canonicalization. Rationale: retentive synchronization and minimal source patches require byte-exact preservation of authored text; Unicode normalization is idempotent but not the identity, and applying it would rewrite user text. Adapters that must compare strings semantically compare NFC forms without altering stored values. The validator emits an informational diagnostic for identifiers or token names that are not in NFC.
21. Invalid UTF-8 is a decoding error, not a repairable condition.

### Hash

22. The canonical hash of a document is SHA-256 over the `nuif-cbor-0` bytes of the document record. Published content identifiers carry the profile identifier (`nuif-cbor-0`) so that a future profile cannot collide silently.
23. Resolved snapshots and correspondence records are hashed separately under the same rules and are never part of the document hash.

## Compatibility

No documents exist yet. Decoders implementing only RFC 8949 §4.2 accept `nuif-cbor-0` output. dCBOR decoders accept it except where NUIF omits NFC normalization (rule 20); dCBOR's NFC rule applies to its own profile, not to NUIF.

## Security

Strict decoding removes canonicalization ambiguity as an attack surface (two byte sequences for one value). Extension payloads are opaque bytes with a declared size limit enforced by the parser (`spec/11-security.md`). Numeric narrowing never changes a value; a decoder that does not know a property's type treats an integer-encoded real as an integer, which is why typed decoding is required for hash-equivalence claims.

## Conformance tests

- canonicalization suite: encode, decode, encode fixpoint; hash stability across platforms; every rule 4–13 has a positive and a negative fixture (non-canonical inputs rejected by the strict decoder).
- numeric fixtures: subnormals, `2^53 ± 1`, integral reals at the `2^63` boundary, `-0.0` input canonicalizing to `0x00`, shortest-width selection for half and single precision.
- text fixtures: layout switch at `10^-6` and `10^21`; `5e-324` prints as `5e-324`; parse rejection of `NaN` and `inf`.
- dCBOR §7 test vectors for the rules NUIF shares, with the divergent cases (NFC) marked as intentionally not shared.

## Implementation

`nuif-codec` uses the `dcbor` crate (0.25.2, BSD-2-Clause-Patent, `no_std`, edition 2024) behind the `Encoder`, `Decoder` and `Canonicalizer` traits for `nuif-cbor-0`; it is the only crate at Rust 1.85 that both produces and verifies a full deterministic profile. `ciborium` narrows numbers but does not order or check keys; `minicbor` and `cbor4ii` write fixed-width floats; `serde_cbor` is unmaintained (RUSTSEC-2021-0127). The dependency is isolated behind the codec traits with an escape hatch to a small in-house encoder over `ciborium-ll` if `dcbor` diverges from these rules. Text number formatting uses the standard library's shortest-digit mode with the layout in rule 15, not `Display` (which prints `-0` and never uses exponents).

## Rejected alternatives

- Adopt dCBOR by reference: an individual draft with strict-reject semantics on NFC that would rewrite user text and can change under NUIF.
- Adopt draft-ietf-cbor-serialization by reference alone: silent on negative zero and duplicate keys; NUIF must state both.
- Preserve `-0.0` and the float/integer distinction on the wire (data-model-preserving profile): produces two encodings for values the logical model treats as equal and makes text and binary hashes diverge.
- Separate text hash: two hashes for one document invite inconsistency; the text profile is a view.
- NFC-normalize string values: violates verbatim preservation required by RFC 0003 and the minimal-patch requirement.
- Use JSON with RFC 8785 as the binary profile: no byte strings, no integer/real distinction, larger payloads.

## Unresolved

- Whether Rust's shortest-digit algorithm and ECMAScript `Number::toString` ever differ in tie-breaking (no counterexample found; irrelevant to NUIF hashes because the text profile hashes through CBOR).
- `dcbor` crate MSRV is not declared; verified to build on 1.85 in the research record only by manifest inspection, to be confirmed in CI.
