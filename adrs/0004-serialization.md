# ADR 0004 — Encoding strategy

Status: provisional

Define serialization independently of the logical model. Prototype a canonical text form and deterministic CBOR binary form. Benchmark schema-based alternatives before ratification.

Opaque extensions are explicit NUIF values/bytes so preservation does not depend on a codec's unknown-field implementation.
