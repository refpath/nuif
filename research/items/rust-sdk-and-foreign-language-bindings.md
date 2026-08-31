---
id: nuif:research:rust-sdk-and-foreign-language-bindings
kind: synthesis
status: verified
title: Rust SDK facade and staged C, Swift and Kotlin binding boundary
source:
  url: https://doc.rust-lang.org/reference/items/external-blocks.html
  repository: https://github.com/mozilla/uniffi-rs
  authors: [Rust project contributors, Mozilla UniFFI contributors, cbindgen contributors]
  published_at: "Rust 1.98 Reference; UniFFI 0.31.2; cbindgen 0.29.4, reviewed 2026-08-31"
  license: "Rust documentation MIT OR Apache-2.0; UniFFI MPL-2.0; cbindgen MPL-2.0"
retrieved_at: 2026-08-31
tags: [rust, sdk, ffi, c, cplusplus, swift, kotlin, uniffi, cbindgen, abi, wasm]
confidence: 0.97
claims: [nuif:claim:semantic-automation, nuif:claim:bounded-untrusted-input]
relations:
  - type: extends
    target: nuif:research:wasm-headless-execution
    note: The implemented WASM binding becomes one transport wrapper over the same package-aware SDK facade.
  - type: related_to
    target: nuif:research:wasm-component-model
    note: A component ABI remains a later plug-in option and does not replace current native-language packaging.
  - type: related_to
    target: nuif:research:cargo-workspace-xtask-and-ci-layout
    note: ABI compatibility, generated headers and native consumers need their own release gates.
links:
  spec: [spec/12-cli-api-and-automation.md, spec/11-security.md]
  adr: [adrs/0001-rust-reference-core.md, adrs/0011-sdk-and-foreign-bindings.md]
  rfc: [rfcs/0010-portable-resource-package.md]
  code: [crates/nuif-api, crates/nuif-wasm, crates/nuif-mcp, crates/nuif-ffi, bindings/nuif_ffi.h, bindings/nuif_ffi.symbols, tools/ffi/header-smoke.cpp, tools/ffi/runtime-smoke.c, conformance/benches/system_surfaces.rs]
  experiments: [nuif:experiment:wasm-cross-surface, nuif:experiment:mcp-cross-surface, nuif:experiment:variable-font-surface-parity]
---

# Summary

The safe common denominator for every integration is not a Rust struct graph or
an MCP server. It is a bounded byte-oriented SDK façade over the canonical
codecs, deterministic package, semantic operations and diagnostics. Native
Rust callers use it directly; WASM, CLI, MCP and later foreign-language
bindings translate only their transport and ownership conventions. The
experimental C layer now follows that rule for deterministic packages,
capability authorization and the common snapshot report without making an ABI
stability claim.

Rust's native ABI is explicitly unstable. `extern "C"` uses the target's C
calling convention, but a usable library must still define representations,
allocation ownership, destructors, errors, panic behavior, threads and symbols.
cbindgen generates C/C++ headers from an existing public C API; it does not
design or prove that API. UniFFI generates a shared-library FFI layer and
high-level Swift/Kotlin/Python/Ruby bindings from an object model and is used in
Firefox, but its own guide says shipping platform artifacts remains the user's
responsibility. UniFFI is production-used yet pre-1.0, and 0.31 changed its
generator command and binding checksums.

NUIF should therefore stabilize in layers. First, make
`nuif-api::NuifDocument` authoritative for explicit text/CBOR load, validation,
typed operations, canonical hashes, export and verified package/resource
retention. Make WASM delegate to it and compare surfaces. Only after the
semantic API and error classes have a compatibility baseline should a small
separate unsafe C ABI be reviewed. cbindgen is appropriate for C/C++ headers;
UniFFI is the preferred Swift/Kotlin generator. Their generated packages and
versions remain separate from the editor.

## Evidence

- The Rust Reference says the Rust ABI offers no stability guarantees and
  defines `unsafe extern "C"` as matching the dominant C compiler's ABI for the
  target. Locator: *The Rust Reference*, External blocks, “ABI”, Rust 1.98,
  retrieved 2026-08-31:
  https://doc.rust-lang.org/reference/items/external-blocks.html#abi.
- Rust 1.98 documents exported C symbols with `#[unsafe(no_mangle)] pub extern
  "C" fn` and describes foreign interfaces as inherently unsafe, normally
  wrapped by safe Rust code. Locator: standard-library `extern` keyword
  documentation, retrieved 2026-08-31:
  https://doc.rust-lang.org/stable/core/keyword.extern.html.
- The Embedded Rust Book specifies `cdylib`/`staticlib`, explicit `extern "C"`
  and generated or handwritten headers as the normal Rust-to-C/C++ path; C is
  used because neither Rust nor C++ supplies the needed cross-language stable
  ABI. Locator: *A little Rust with your C*, retrieved 2026-08-31:
  https://doc.rust-lang.org/stable/embedded-book/interoperability/rust-with-c.html.
- cbindgen generates C and C++11 headers from Rust crates that already expose a
  public C API. Its README says generation reflects Rust layout/ABI guarantees,
  while also warning that project support is ad hoc and particular constructs
  may be unsupported. Locator: cbindgen README, `master`, retrieved 2026-08-31:
  https://github.com/mozilla/cbindgen.
- UniFFI compiles Rust components into shared libraries and generates bindings
  to load them. Its first-party languages are Kotlin, Swift, Python and Ruby;
  Mozilla reports extensive Firefox mobile and desktop use. The project calls
  itself production-ready but far from 1.0. Locator: UniFFI README and user
  guide overview, retrieved 2026-08-31:
  https://github.com/mozilla/uniffi-rs and
  https://mozilla.github.io/uniffi-rs/latest/.
- The UniFFI guide explicitly says it generates bindings but does not help ship
  the Rust library to target platforms. Swift output includes a C header and
  module map around the shared library. Locator: guide overview, binding
  generation and Swift overview, retrieved 2026-08-31:
  https://mozilla.github.io/uniffi-rs/latest/bindings.html and
  https://mozilla.github.io/uniffi-rs/latest/swift/overview.html.
- UniFFI 0.31.0 removed prior generator types, changed command usage and changed
  method checksums incompatibly with 0.30-generated bindings; 0.31.2 fixed
  Kotlin ARM32 return conversion and Swift boundary defects. This supports
  pinning generator/runtime pairs and testing generated consumers rather than
  treating generated source as timeless. Locator: UniFFI `CHANGELOG.md`,
  0.31.0–0.31.2, retrieved 2026-08-31:
  https://github.com/mozilla/uniffi-rs/blob/main/CHANGELOG.md.
- Executable NUIF evidence: `nuif-api` tests load text and CBOR, apply the same
  typed transaction, compare canonical hashes, undo/redo and retain package
  metadata through a byte fixpoint. `nuif-wasm` delegates those semantics to
  the façade, and `cargo xtask gate-wasm` cross-checks native and generated
  browser/Node output. Criterion adds `sdk/direct_document` load/export
  surfaces. Locator: linked code at revision containing this record.

## Mechanism

The direct SDK owns a `Session` and an optional decoded `NuifPackage`. Bare
inputs enter through an explicit `DocumentEncoding` and remain diagnosable even
when structurally invalid. Package inputs enter through the package decoder,
then hand shared digest-verified embedded buffers to the session. Semantic
operations mutate only the session document. Package export clones the retained
package envelope, replaces its document and requested mode, and revalidates the
complete manifest/resource policy before writing bytes.

The WASM object stores this SDK object. Its remaining work is bounded JSON patch
decoding and JavaScript error translation. Stateless MCP tools load the same SDK
object per request while retaining their protocol framing and JSON Schema
boundary. Direct API tests require matching hashes across text and CBOR,
replayable operation preconditions, exact undo/redo and a package
write/read/write fixpoint after editing. The generated Node/browser packages
and live MCP subprocess remain checked against native canonical output.

The experimental foreign ABI remains one layer farther out:

```text
C / C++ / Swift / Kotlin
          │ generated wrapper and owned byte buffers
          ▼
   separately reviewed nuif-ffi
          │ NuifDocument byte records and typed error classes
          ▼
          nuif-api
```

No internal `Document`, `Entity`, Rust enum layout, allocator pointer or panic
may cross that ABI by accident. The draft requires single-thread-at-a-time
access to each opaque handle; separate handles and returned buffers are
independent. Its compiled POSIX C consumer loads and re-exports the exact
variable-font package, proves pre-authorization denial and compares the full
snapshot JSON with the CLI oracle.
The same gate compiles the header under C11 and C++17, links and executes the
C++17 smoke consumer, compares all optimized exports with a checked-in
experimental symbol set, and repeats the semantic C consumer under ASan/UBSan.
The developer archive carries those reports and hashes every
payload. Pinned cbindgen 0.29.4 now derives the opaque types and function
declarations from Rust; exact regeneration is a prerequisite of the same gate.
Profile 0 permits breaking declarations only when implementation, header,
symbol baseline, consumers and evidence change together. This is pre-stability
evidence and an explicit review policy, not an ABI promise.

## NUIF relevance

**Borrow** the compiler-style single façade, explicit C calling convention,
cbindgen header generation and UniFFI Swift/Kotlin generation. **Adapt** them to
NUIF's byte records, typed errors, bounded inputs and independent profile
versioning. **Reject** direct exposure of internal model structs, duplicated
business logic in wrappers, a hand-written Swift/Kotlin ownership layer before
UniFFI is evaluated, and any claim that generated bindings alone constitute a
shippable SDK.

## Promotion checklist

- semantic API and stable error-code registry leave `0.0.x`;
- separate `nuif-ffi` unsafe-code review and panic-containment proof;
- opaque handles plus allocator-matched bytes and destructors;
- retain the pinned cbindgen regeneration check and reviewed experimental
  compatibility policy over the implemented symbol baseline;
- target-matrix semantic C/C++ consumers and sanitizer trials, extending the
  implemented POSIX linked C++ smoke and ASan/UBSan C package trial;
- pinned UniFFI generator/runtime with Swift and Kotlin consumer tests;
- target-specific XCFramework/Swift package and AAR artifacts with manifests,
  checksums, SBOMs and attestations;
- independent versions from the editor, WASM module and MCP binary.

## Open questions

- Which semantic-API milestone is strong enough to register stable foreign
  error numbers and start ABI compatibility checks?
- Whether the first native package target should be an Apple XCFramework or an
  Android AAR; demand and a maintained live consumer should decide ordering.
- Whether a plain C ABI plus cbindgen is needed independently from UniFFI's
  generated low-level C layer, or whether C/C++ adoption can wait for a named
  host requirement.
- Whether a measured Node workload ever justifies a native Node-API addon over
  the already conforming WebAssembly package.
