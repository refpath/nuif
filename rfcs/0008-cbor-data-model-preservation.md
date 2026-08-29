# RFC 0008 — Preserve numeric kinds and distinguish text/CBOR key order

Status: accepted (corrective; primary evidence in `nuif:research:cbor-data-model-and-key-order-correction`). Supersedes RFC 0005 rules 6 and 8 and the coincidence claim in rule 17. All other RFC 0005 rules remain in force.

## Motivation

RFC 0005 made integral `real` values use CBOR integer encodings and required a property schema to restore the type. The NUIF logical value model, extensible properties and unknown data do not guarantee that such a schema is available. RFC 8949 §2 also defines integer and floating-point data items as distinct. The previous rule therefore made a generic lossless decoder impossible and caused `integer(1)` and `real(1.0)` to collide.

RFC 0005 also stated that UTF-8 key order coincides with CBOR encoded-key order for NUIF identifiers. It does not: complete CBOR key encodings include a length head, so `"z"` (`61 7a`) sorts before `"aa"` (`62 61 61`) in deterministic CBOR while UTF-8 sorts `"aa"` first.

## Decision

1. `integer` and `real` remain distinct logical values in every encoding and in canonical hashes.
2. An `integer` uses CBOR major type 0 or 1 with the shortest argument.
3. A finite `real`, including an integral real, uses the shortest IEEE 754 half, single or double CBOR floating-point encoding that round-trips exactly.
4. Negative real zero has no distinct NUIF identity and encodes as positive half-precision floating-point zero (`f9 00 00`). Integer zero encodes as `00`. This replaces RFC 0005 rule 8.
5. Maps in `nuif-cbor-0` sort by bytewise lexicographic order of each key's complete deterministic CBOR encoding.
6. Objects in `nuif-text-0` sort string keys by UTF-8 byte order. Text and CBOR key order need not coincide because the canonical document hash is computed from parsed `nuif-cbor-0`, not text bytes.
7. A decoder used for conformance or hashing rejects an integral real encoded as an integer when the surrounding NUIF value discriminant says `real`; it does not infer or repair the lost kind.

## Compatibility

No published documents exist. The profile identifier remains `nuif-cbor-0` because the prior rules had no implementation or fixtures; the first executable codec implements this RFC.

## Security

Keeping numeric kinds self-describing removes schema confusion at extension boundaries. Strict duplicate-key and finite-number checks from RFC 0005 remain required.

## Conformance tests

- `integer(1)` and `real(1.0)` encode differently, decode to their original variants and have different canonical hashes.
- positive and negative real zero both encode as the same floating-point data item; integer zero remains distinct.
- a map containing `"aa"` and `"z"` emits `"z"` first in CBOR and `"aa"` first in canonical text.
- encode/decode/encode is a byte fixpoint for both profiles.

## Implementation

`nuif-codec` converts Serde values to `ciborium::Value`, recursively rejects non-finite values and tags, sorts map entries by their encoded key bytes, and then uses Ciborium's shortest-width numeric writer. Strict decoding re-encodes the retained value tree and compares bytes before deserializing the NUIF document.

## Rejected alternatives

- Restore the type only from a property schema: unavailable to generic tools and unknown extensions.
- Treat integers and integral reals as one logical type: contradicts the declared NUIF value variants and creates surprising adapter behavior.
- Give text and CBOR the same key order: either violates UTF-8 lexical text order or deterministic CBOR encoded-key order.
