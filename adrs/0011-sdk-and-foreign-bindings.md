---
id: nuif:adr:0011
kind: adr
status: accepted
---

# ADR 0011: One byte-oriented SDK precedes foreign ABI stabilization

Decision delegated to research on 2026-08-31. Evidence:
`nuif:research:rust-sdk-and-foreign-language-bindings` and
`nuif:research:wasm-headless-execution`.

## Context

The reference engine already has a headless `nuif-api` session and working CLI,
WASM, MCP and editor clients. The WASM crate nevertheless duplicated bare
document decoding, validation, canonical export, hashing and session history
composition. Adding C, Swift and Kotlin wrappers before removing that
duplication would create more places for semantics to escape the core.

Rust does not promise stability for its native ABI. A public C ABI can be
stable only after the project specifies ownership, buffers, errors, panics,
threads and symbol compatibility. Swift and Kotlin additionally need generated
language wrappers and platform packaging; binding generation alone is not a
distribution system.

## Decision

1. `nuif-api::NuifDocument` is the single package-aware, byte-oriented Rust SDK
   façade. It owns no filesystem, network, plug-in or process authority.
2. The façade accepts explicit canonical text/CBOR profiles, separately loads
   fully verified packages, applies typed semantic operations and exports bare
   encodings or deterministic packages through existing implementations.
3. WASM, MCP, CLI and editor wrappers may add transport limits and host policy,
   but must not implement document semantics independently.
4. Cross-surface conformance requires exact canonical bytes, hash, diagnostics
   and patch behavior for the shared subset.
5. No stable C ABI is declared while the semantic SDK is `0.0.x`. The eventual
   `nuif-ffi` is a small separately reviewed crate over SDK byte records, not a
   C representation of internal Rust structs.
6. C/C++ headers are generated with pinned cbindgen and rejected when stale.
   The experimental profile permits reviewed breaking changes only when Rust,
   header, symbol baseline, consumers and evidence move together; stable major
   profiles may only add symbols without another major FFI profile. Swift/Kotlin
   wrappers should prefer pinned UniFFI over handwritten per-language ownership
   glue, but only after ABI, sanitizer and native-consumer gates are defined.
7. Each foreign binding and platform package has an independent profile,
   version, compatibility report and release stream.

## Consequences

- Direct Rust and browser clients use one operation and serialization path.
- Packages and embedded resources survive SDK edits without giving wrappers
  implicit resolution authority.
- The repository avoids advertising a stable ABI whose memory or error contract
  has not been reviewed.
- Native mobile bindings remain planned, with explicit promotion evidence
  instead of placeholder exports that appear production-ready.
