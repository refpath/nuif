---
id: nuif:adr:0001
kind: adr
status: accepted
---

# ADR 0001: Rust for the reference core

## Decision

The initial reference implementation uses Rust for the canonical in-memory model, operations protocol, validation, codecs, layout/renderer integration, headless tooling, and WASM boundary.

## Rationale

The project needs memory safety for hostile document parsing, deterministic systems-oriented behavior, strong enum/type modeling, high-performance geometry/text/GPU ecosystems, native and WebAssembly targets, fuzz/property testing, and stable FFI boundaries. Rust gives the strongest combined fit among the evaluated mainstream implementation languages.

The editor UI and source-specific adapters may use TypeScript or target-native languages where that produces a cleaner integration. Those layers must communicate through stable protocol/API boundaries and must not redefine the canonical model.
