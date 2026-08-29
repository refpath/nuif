---
id: nuif:research:canonicalization-rfc8785-and-cbor-deterministic
kind: standard
status: reviewed
title: Canonical serialization rules - JCS (RFC 8785), CBOR deterministic encoding (RFC 8949 s4.2, CDE, dCBOR) and lessons from XML C14N
source:
  url: https://www.rfc-editor.org/rfc/rfc8949.html
  authors: [Carsten Bormann, Paul Hoffman, Anders Rundgren, Bret Jordan, Samuel Erdtman, Wolf McNally, Christopher Allen, Laurence Lundblade, W3C XML Security Working Group]
  published_at: "RFC 8949 2020-12; RFC 8785 2020-06; draft-ietf-cbor-cde-13 2025-10-14; draft-mcnally-deterministic-cbor-18 2026-08-10; XML C14N 1.1 2008-05-02; XML C14N 2.0 Note 2013-04-11"
  license: IETF Trust (RFCs and Internet-Drafts); W3C Document License
retrieved_at: 2026-08-29
tags: [canonicalization, cbor, json, jcs, dcbor, cde, deterministic-encoding, hashing, floating-point, unicode, xml-c14n]
confidence: 0.94
claims: [nuif:claim:opaque-preservation]
relations:
  - type: extends
    target: nuif:research:encoding
    note: Turns the general choice of deterministic CBOR into the specific normative rules and the float/NaN/zero decisions RFC 8949 leaves open.
  - type: extends
    target: nuif:research:content-addressed-versioning
    note: Byte-stable canonical encoding is the precondition for content-addressed snapshot identity.
  - type: related_to
    target: nuif:research:json-patch-rfc6902-and-merge-patch
    note: JCS canonical JSON and JSON Patch share the I-JSON number model that excludes NaN and Infinity.
links:
  spec: [spec/08-serialization.md]
  adr: [adrs/0004-serialization.md]
  rfc: []
  code: [crates/nuif-codec]
  experiments: []
---

# Summary

RFC 8949 §4.2.1 fixes four core requirements for deterministically encoded CBOR: shortest-form arguments for integers, lengths and tags; shortest floating-point form that preserves the value; no indefinite-length items; and map keys sorted by bytewise lexicographic order of their deterministic encodings. §4.2.2 leaves to the protocol the treatment of tags, big integers, negative zero, NaN payloads, subnormals and integral-valued floats. The CDE draft packages the core requirements as a profile (preferred serialisation, definite lengths, lexicographic map sorting) and keeps the data model intact (1.0 stays a float, -0.0 is encoded as `0xf98000`). The dCBOR draft narrows further at the application level: integral floats within 64-bit range become integers, all zeros become `0x00`, all NaNs become `0xf97e00`, only `false`/`true`/`null`/floats are permitted as simple values, text must be NFC, and decoders must reject non-conforming input. RFC 8785 (JCS) canonicalises JSON text: no whitespace, fixed string escaping, numbers serialised per ECMAScript `Number::toString` (shortest round-trip, `-0` becomes `0`, NaN/Infinity are errors), properties sorted by UTF-16 code units, UTF-8 output, under the I-JSON constraint that numbers are IEEE 754 doubles. XML C14N 1.1 and the C14N 2.0 Note document why XML canonicalisation was hard: namespace and `xml:` attribute inheritance in document subsets, XPath node-set dependence, whitespace, QNames in content, and information lost (base URIs, notations, attribute types). The normative rules NUIF should adopt are listed under Mechanism.

## Evidence

- RFC 8949 §4.1: preferred serialisation "always uses the shortest form of representing the argument"; floats use the shortest encoding that preserves the value; definite-length encoding is preferred when the length is known. https://www.rfc-editor.org/rfc/rfc8949.html#section-4.1 (retrieved 2026-08-29).
- RFC 8949 §4.2.1 Core Deterministic Encoding Requirements: integer arguments 0-23 in the initial byte, 24-255 in one byte, up to 65535 in two, up to 2^32-1 in four; floating-point in the shortest form that preserves the value (1.5 as binary16); "Indefinite-length items MUST NOT appear"; "keys in every map MUST be sorted in the bytewise lexicographic order of their deterministic encodings". §4.2.1 (retrieved 2026-08-29).
- RFC 8949 §4.2.2: protocols must decide whether a tag must be present or absent; whether integers with absolute value at or above 2^64 use tags 2/3 and whether smaller values may also use them; negative zero may be disallowed; "the protocol needs to pick a single representation, typically 0xf97e00" for NaN; subnormals may be excluded; whether 1.0 is `0x01`, `0xf93c00`, `0xfa3f800000` or `0xfb3ff0000000000000`. §4.2.2 (retrieved 2026-08-29). §4.2.3 defines the legacy length-first key order of RFC 7049. §5.6 lists three decoder behaviours for duplicate keys.
- CDE: draft-ietf-cbor-cde-13, 2025-10-14, intended Best Current Practice, updates RFC 8949; CDE = `preferred-serialization` + `definite-length-only` + `lexicographic-map-sorting` (§3); floats use the shortest of binary16/32/64 preserving the value, negative zero is encoded as `0xf98000`, integral floats remain floating-point (data model preserved), preferred serialisation applies to NaNs (§3.1.2); strictly increasing key order excludes duplicates (§3.3); application-level deterministic representation (ALDR) is separate, with dCBOR as the example (Appendix B). https://www.ietf.org/archive/id/draft-ietf-cbor-cde-13.html (retrieved 2026-08-29).
- dCBOR: draft-mcnally-deterministic-cbor-18, 2026-08-10, authors McNally, Allen, Bormann, Lundblade; definite lengths only (§2.1); preferred serialisation validated by decoders (§2.2); ordered keys (§2.3); no duplicate keys (§2.4); numeric reduction: floats with zero fractional part within [-2^63, 2^64-1] become integers, all NaNs become `0xf97e00`, 0, 0.0 and -0.0 become `0x00`, decoders reject non-reduced floats (§2.5); simple values limited to `false`, `true`, `null`, floats (§2.6); text strings must be NFC UTF-8 and decoders reject non-NFC (§2.7). https://datatracker.ietf.org/doc/draft-mcnally-deterministic-cbor/ (retrieved 2026-08-29). Draft -12 (2025-02-07) states the same numeric rules in §2.3 and rejects 65-bit negative integers (Table 4); tag 201 marks dCBOR content. https://www.ietf.org/archive/id/draft-mcnally-deterministic-cbor-12.html (retrieved 2026-08-29).
- RFC 8785 (JCS), June 2020, Informational, Independent Submission, Rundgren, Jordan, Erdtman: input must be I-JSON (RFC 7493), "JSON number data MUST be expressible as IEEE 754 double-precision values" (§3.1); no whitespace between tokens (§3.2.1); strings escape U+0000-U+001F as `\uhhhh` lowercase except `\b \t \n \f \r`, escape `\` and `"`, emit other characters as-is, error on lone surrogates (§3.2.2.2); numbers "MUST be serialized according to Section 7.1.12.1 of [ECMA-262], including the 'Note 2' enhancement", NaN and Infinity "MUST cause a compliant JCS implementation to terminate with an appropriate error" (§3.2.2.3); properties sorted by UTF-16 code units, shorter prefix first (§3.2.3); UTF-8 output (§3.2.4); Appendix B table shows both `0000000000000000` and `8000000000000000` serialise to `0`; larger precision "RECOMMENDED to represent such numbers as JSON strings". https://www.rfc-editor.org/rfc/rfc8785.html and .txt (retrieved 2026-08-29).
- XML C14N 1.1, W3C Recommendation 2008-05-02: UTF-8 without BOM, line breaks to `#xA`, attribute value normalisation, entity references replaced, CDATA converted, empty elements expanded, superfluous namespace declarations removed and sorted, attributes sorted by namespace URI then local name; 1.1 changes the inheritance of `xml:base` (URI joining) and excludes `xml:id` from inheritance for document subsets; information lost: base URIs, notations and unparsed entities, attribute types. https://www.w3.org/TR/xml-c14n11/ (retrieved 2026-08-29).
- XML C14N 2.0, W3C Working Group Note 2013-04-11, not pursued to Recommendation; §1.4 motivations: performance (C14N 1.x depends on XPath node-sets; 2.0 is a tree walk), streaming, robustness (whitespace, QNames in attributes such as `xsi:type`, optional prefix rewriting), portability of subdocuments, simplicity; §2.2 parameters `IgnoreComments`, `TrimTextNodes`, `PrefixRewrite`, `QNameAware`. https://www.w3.org/TR/xml-c14n2/ (retrieved 2026-08-29).

## Mechanism

Layering used by the IETF documents (NUIF should mirror it):

```
Layer 0  well-formed CBOR                      (RFC 8949 §3)
Layer 1  preferred serialisation               (RFC 8949 §4.1)   shortest head; shortest float
Layer 2  CDE = core deterministic requirements (RFC 8949 §4.2.1; draft-ietf-cbor-cde)
         + definite lengths only + bytewise-lexicographic keys, no duplicates
Layer 3  application-level determinism (ALDR)  (dCBOR draft; NUIF profile rules)
         numeric reduction, NaN/zero canonical forms, simple-value and string constraints
```

Key ordering (RFC 8949 §4.2.1): compare the deterministic encodings of keys as byte strings; because the initial byte carries major type and argument size, shorter integers sort before longer ones and integers sort before strings.

Float rule as adopted by dCBOR §2.5:

```
canon(x):
  if x is float and x is finite and frac(x) = 0 and -2^63 ≤ x ≤ 2^64-1: encode as integer (major 0/1)
  elif x is NaN:                                   emit f9 7e 00
  elif x = ±0.0:                                   emit 00
  else:                                            shortest of binary16/32/64 that round-trips exactly
decoders reject any float not in this form
```

JCS number rule (RFC 8785 §3.2.2.3): ECMAScript `Number::toString` produces the shortest decimal digit string that round-trips to the same double, with exponent notation thresholds fixed by ECMA-262; `-0` prints as `0`; this is the text-form counterpart of the binary shortest-form rule.

Normative rules NUIF must adopt for byte-stable canonical hashing (NUIF interpretation of the sources):

1. Canonical hash is computed over the `nuif-cbor-0` deterministic encoding, never over a text or compressed form (spec/08 "MUST exclude transport-only compression differences").
2. CDE conformance: shortest heads, definite lengths, bytewise-lexicographic key order, duplicate keys rejected.
3. Numeric model declared per property type: integer-typed values use major types 0/1 only; real-typed values use IEEE 754 binary64 semantics with dCBOR numeric reduction (integral values as integers, single NaN `0xf97e00`, all zeros as `0x00`) or, if signed zero and NaN payloads carry meaning for a property, a documented exception; subnormals preserved (no information loss) but encoded in shortest form.
4. No big-integer tags unless a property type requires them; no other tags in the canonical body.
5. Strings: valid UTF-8, no normalisation of user content (NFC normalisation changes content and is a data-model decision, see open questions), identifiers and keys restricted to a canonical repertoire.
6. Sets and relation graphs serialised in a defined total order (by entity ID bytes), sequences in document order.
7. Decoder is strict: any deviation from the canonical form is a rejection, not a re-canonicalisation, so that hash equality implies byte equality.

XML C14N lessons (source statements): canonical form must not depend on a query language over the document (C14N 2.0 §1.4.1), inherited context (namespaces, `xml:base`) must be made explicit or excluded (C14N 1.1 changes), QName-valued content must be known to the canonicaliser (C14N 2.0 §1.4.3), and some information is inevitably lost, so the canonical form must be defined as the reference form rather than a derived one.

## NUIF relevance

**Borrow**
- RFC 8949 §4.2.1 and CDE as the base profile for `nuif-cbor-0`, including bytewise-lexicographic key ordering and definite lengths.
- dCBOR §2.5 numeric reduction (integral floats to integers, single NaN, single zero) and §2.6 simple-value restriction, because they remove the remaining encoder freedom that RFC 8949 §4.2.2 enumerates.
- JCS's UTF-16 code-unit property ordering and shortest round-trip number printing for the `nuif-text-0` canonical text form so text and binary forms agree on number identity.
- The strict-decoder rule (dCBOR) so that a NUIF hash identifies exactly one byte sequence.

**Adapt**
- dCBOR's NFC requirement (§2.7) must become a data-model rule in spec/02 rather than an encoder rule: text properties either store NFC by definition or store code points verbatim, and the choice affects round-tripping of imported documents.
- JCS's -0 to 0 collapse and I-JSON double limit: NUIF property types that are integers wider than 2^53 must not be routed through the JSON/text form unquoted; the text form needs a typed integer syntax.
- C14N's "context must be explicit" lesson maps to NUIF inheritance (tokens, styles, instance overrides): the canonical document must serialise authored values only, never resolved or inherited values, and resolved caches must be excluded from the hash.

**Reject**
- Length-first map key ordering (RFC 8949 §4.2.3), which exists only for RFC 7049 compatibility.
- Preserving NaN payloads, signalling NaNs or signed zero in canonical form; RFC 8949 §4.2.2 allows it but no NUIF property semantics require it.
- XML C14N-style parameterised canonicalisation (comments, whitespace trimming, prefix rewriting); a canonical form with parameters is not a single canonical form.

## Open questions

- Whether a single NUIF numeric profile suffices, or whether layout properties (which may be authored as `1.0` versus `1`) need a type-directed rule so that authored integers and reals hash differently only when the property type distinguishes them.
- Handling of extension payloads (`SetExtension { payload: Vec<u8> }` in nuif-protocol): opaque bytes are hashed verbatim, but if an extension is itself CBOR the canonical rules should apply recursively; whether to require nested canonical CBOR for registered extensions is undecided.
- Whether CDE reaches RFC status with the -0.0 rule (`0xf98000`) unchanged, which would conflict with dCBOR's zero reduction for any NUIF profile that references CDE by name.
