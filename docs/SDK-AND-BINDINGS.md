# SDK and language-binding boundary

`nuif-api` is the ergonomic in-process SDK over the authoritative model,
codecs, package, operation, layout and render crates. It is intentionally a
façade rather than another implementation. The CLI, reference editor,
WebAssembly module and MCP adapter translate their environment at the edge and
delegate semantic work to this layer.

```text
                         nuif-api::NuifDocument
                      load · validate · apply · export
                                   │
                 canonical codecs · package · operations
                                   │
                  model · layout · render · diagnostics
                     ┌─────────────┼─────────────┐
                  native         WASM          process
                 Rust host    browser/plugin   CLI/MCP
```

## Direct Rust use

Bare encodings are explicit; the SDK does not guess from an arbitrary byte
prefix. Package loading is a separate call because it structurally validates
the ZIP, manifest, document, resource descriptors, embedded bytes and policy
before a session is returned. Host support is negotiated after structural
decode or atomically through `load_package_with_capabilities`.

```rust
use nuif_api::{DocumentEncoding, NuifDocument};
use nuif_package::PackageMode;

let mut document = NuifDocument::load(bytes, DocumentEncoding::CanonicalText)?;
let report = document.validate()?;
let patch = document.apply_operations(transaction_id, operations)?;
let revision = document.canonical_hash()?;
let cbor = document.export(DocumentEncoding::DeterministicCbor)?;
let package = document.export_package(PackageMode::Portable)?;
```

`load_package` retains verified descriptors and shared immutable embedded bytes.
When the manifest has requirements, its session is structural and read-only:
validation, hashing, bare extraction and an unchanged same-mode package copy
remain available, while semantic mutation, undo/redo, evaluation and mode
conversion return the exact required set. Successful negotiation authorizes
those actions for that loaded session. `export_package` then replaces only the
package's semantic document and requested mode and runs the ordinary package
policy; it cannot silently fetch linked resources. Applying operations uses the
same atomic transaction, revision and undo/redo implementation as the editor.

```rust
let mut structural = NuifDocument::load_package(package_bytes)?;
let report = structural.package_capability_report(&host_capabilities);
structural.require_package_capabilities(&host_capabilities)?;

let supported = NuifDocument::load_package_with_capabilities(
    package_bytes,
    &host_capabilities,
)?;
```

The report contains required, supported-required and missing-required sets in
deterministic order. Extra host capabilities are ignored. Structural loading
is appropriate for inert inspection, preservation and an explicit bare
extraction before migration; it is not authorization to rewrite the package or
a claim that the host can evaluate every required profile.

Structurally invalid but syntactically decodable bare documents can be loaded
for diagnostics; canonical export, hashing, layout and rendering still fail
closed. Package inputs are structurally valid or rejected atomically; full
package support additionally requires explicit capability negotiation.

## Binding rule

Wrappers contain transport and ownership conversion only:

- WebAssembly accepts byte arrays, bounds JSON patch and capability-set
  transport, and delegates bare/package loading, validation, hashing,
  capability negotiation, canonical export and history to `NuifDocument`.
- MCP bounds newline-delimited protocol messages and maps stateless tool calls
  to the same API; its package snapshot tool accepts bounded base64 rather than
  ambient filesystem authority.
- The CLI owns files and stdout; the editor owns window and interaction state.
- The CLI and editor declare only
  `nuif-opentype-variable-truetype-single-0` as an optional package capability.
  They evaluate and preserve that tested resource profile; any other required
  capability remains structurally inspectable and fails closed before
  evaluation or semantic rewrite.
- The reference editor opens unsupported capability-bearing packages only for
  structural read-only inspection and exact copying; it rejects semantic edits
  at both the session and package-save boundaries.
- A host plug-in owns vendor objects, permissions and undo grouping. WASM does
  not become the Figma or Canva adapter merely because it runs in a plug-in;
  Affinity currently has no documented plug-in boundary for this project and
  uses a user-mediated interchange profile instead.

The cross-surface rule is exact: the same input and patch must produce the same
canonical hash, canonical bytes and diagnostics. For package-aware surfaces it
also requires the same deterministic archive bytes, retained resources and
missing-capability set. `cargo xtask gate-wasm` and `cargo xtask gate-mcp`
compare wrappers with native output. `cargo xtask gate-i-font-surfaces`
additionally requires a complete variable-font snapshot report—hash,
coordinates, diagnostics, fidelity, outlines and raster digest—to agree across
direct Rust, CLI, generated Node/browser WASM, live stdio MCP and a linked C
release-library consumer on POSIX. The Criterion `sdk/direct_document` group
measures direct text, CBOR and package loading plus canonical export; the
resource group separately measures variable-package authorization, an
already-authorized snapshot and the combined delivery path.

## C, C++, Swift and Kotlin decision

No stable foreign ABI is declared during the `0.0.x` semantic-API phase. The
experimental `nuif-ffi-0` crate now exposes a byte-oriented C ABI over
`NuifDocument`: opaque handles, explicit document/package values, bounded
input buffers, allocator-matched returned buffers, stable numeric error classes
and panic containment. Package load/export, exact capability negotiation and
the shared snapshot report delegate to the same SDK as WASM and MCP. It exposes
no internal model structs and grants no filesystem, network or host-product
authority. The checked draft header is `bindings/nuif_ffi.h`; `cargo xtask
gate-ffi` runs the Rust ABI tests, C11 and C++17 header-consumer checks, links
and executes the C++17 consumer, checks an exact exported-symbol baseline and a linked release-library variable-font
package/snapshot comparison under normal, AddressSanitizer and
UndefinedBehaviorSanitizer execution on POSIX.
Release workflows additionally run `cargo xtask ffi-package` on each native
matrix target. The resulting versioned archive contains the header,
experimental symbol baseline, available static/shared library artifacts,
normal/sanitized conformance reports, the cbindgen configuration, C/C++/Swift
examples, licenses and a manifest covering every payload file by SHA-256. It is
a developer package for experiments, not a promise of ABI stability or a
platform store distribution.

The committed header is generated from the Rust ABI declarations by pinned
cbindgen 0.29.4 and `bindings/cbindgen.toml`. `cargo xtask ffi-header` performs
the only authorized regeneration; `cargo xtask ffi-header --check` and
`gate-ffi` reject any source/header drift. The configuration, header and exact
experimental symbol baseline are reviewed together.

`nuif-ffi-0` has an explicit pre-stability compatibility policy. Any change to
a public declaration, numeric macro, ownership rule or exported symbol may be
breaking, but must update the Rust implementation, generated header, symbol
baseline, consumers and evidence in one commit. No downstream source or binary
compatibility is promised for its `0.0.x` archives. A future `nuif-ffi-1`
freezes existing signatures, layouts, numeric errors and ownership rules for
its major profile; additions use new symbols, while removal or reinterpretation
requires another major FFI profile. Header regeneration verifies declarations
but does not by itself prove calling convention or binary compatibility.

Rust's native ABI has no stability guarantee. A C-compatible ABI adds an unsafe
ownership boundary whose handle lifetime, buffer allocation and release, panic
containment, error representation, thread rules, symbol set and calling
convention become a compatibility promise independent of Rust source
compatibility. The draft profile is evidence for those decisions, not a stable
compatibility claim.

The promotion path is:

1. Freeze a byte-oriented `nuif-ffi-1` contract over `NuifDocument`, not over
   internal model structs.
2. Put all unsafe code in a separately reviewed `nuif-ffi` crate; keep the
   model, codec, operation and SDK crates under `unsafe_code = "forbid"`.
3. Catch panics before the ABI boundary, return stable numeric error classes
   plus owned diagnostic bytes, and provide one allocator-matched buffer-free
   function and one null-tolerant handle-free function.
4. Preserve the implemented C/C++ compile checks, exported-symbol baseline and
   linked POSIX C and C++ consumers, pinned generated header and experimental
   compatibility policy; extend semantic C++ and runtime/sanitizer evidence to
   every supported target before stabilization.
5. Generate Swift and Kotlin wrappers with a pinned UniFFI release after its
   generated ownership/checksum behavior passes native tests. Package an
   XCFramework/Swift package and Android AAR separately; UniFFI generates
   bindings but does not ship those platform artifacts.
6. Apply semantic-version and ABI-compatibility checks to the FFI profile
   independently from the editor, WASM and MCP versions.

The draft now declares single-thread-at-a-time access per handle while allowing
independent handles and returned buffers on other threads. Promotion still
requires a reviewed error-code registry, an API compatibility baseline,
semantic generated consumer fixtures in Swift/Kotlin, broader C++ semantics,
complete sanitizer evidence and release packages for their actual target
triples. The current macOS Swift smoke proves C-header import, linking,
capability JSON and allocator-matched release only. Until then,
Swift or Kotlin desktop/mobile experiments should use the WASM package where
their host embeds an appropriate runtime, or call a local CLI/process adapter;
neither path is described as a native production binding.

For Node.js the WebAssembly package remains the default because it already has
exact native parity and no native addon installation matrix. A Node-API addon
is justified only if the benchmark suite shows a material workload that WASM
cannot meet. The WebAssembly Component Model remains a possible future plug-in
ABI, not a substitute for the currently deployed browser module.

## Release boundary

The editor, CLI, WASM binding, MCP service and eventual FFI packages have
independent versions. A tag for the editor may attach tested developer
artifacts, but it does not promote `nuif-api` or promise a stable ABI. The CLI
archive is the explicit no-store process integration for automation that does
not need MCP; its package smoke report exercises real generation, validation,
canonicalization and inspection through the release binary. Publishing a crate,
npm package, Swift package, AAR or vendor plug-in requires an explicit policy
and authenticated release operation for that ecosystem.
