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
prefix. Package loading is a separate call because it fully validates the ZIP,
manifest, document, resource descriptors, embedded bytes and policy before a
session is returned.

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

`load_package` retains verified descriptors and shared immutable embedded bytes
across edits. `export_package` replaces only the package's semantic document
and requested mode, then runs the ordinary package policy. It cannot silently
fetch linked resources. Applying operations uses the same atomic transaction,
revision and undo/redo implementation as the editor.

Structurally invalid but syntactically decodable bare documents can be loaded
for diagnostics; canonical export, hashing, layout and rendering still fail
closed. Package inputs are fully valid or rejected atomically.

## Binding rule

Wrappers contain transport and ownership conversion only:

- WebAssembly accepts byte arrays, bounds JSON patch transport and delegates
  document loading, validation, hashing, canonical export and history to
  `NuifDocument`.
- MCP bounds newline-delimited protocol messages and maps stateless tool calls
  to the same API.
- The CLI owns files and stdout; the editor owns window and interaction state.
- A host plug-in owns vendor objects, permissions and undo grouping. WASM does
  not become the Figma or Adobe adapter merely because it runs in a plug-in.

The cross-surface rule is exact: the same input and patch must produce the same
canonical hash, canonical bytes and diagnostics. `cargo xtask gate-wasm` and
`cargo xtask gate-mcp` compare the wrappers with native output; the Criterion
`sdk/direct_document` group measures direct text, CBOR and package loading plus
canonical export.

## C, C++, Swift and Kotlin decision

No stable foreign ABI is declared during the `0.0.x` semantic-API phase. Rust's
native ABI has no stability guarantee. A C-compatible ABI is possible, but it
adds an unsafe ownership boundary whose handle lifetime, buffer allocation and
release, panic containment, error representation, thread rules, symbol set and
calling convention become a compatibility promise independent of Rust source
compatibility. Generating a header does not decide those rules.

The promotion path is:

1. Freeze a byte-oriented `nuif-ffi-1` contract over `NuifDocument`, not over
   internal model structs.
2. Put all unsafe code in a separately reviewed `nuif-ffi` crate; keep the
   model, codec, operation and SDK crates under `unsafe_code = "forbid"`.
3. Catch panics before the ABI boundary, return stable numeric error classes
   plus owned diagnostic bytes, and provide one allocator-matched buffer-free
   function and one idempotent handle-free function.
4. Generate C and C++ headers with a pinned cbindgen release, diff the exported
   symbol/header surface in CI and run C consumers under sanitizers on every
   supported target.
5. Generate Swift and Kotlin wrappers with a pinned UniFFI release after its
   generated ownership/checksum behavior passes native tests. Package an
   XCFramework/Swift package and Android AAR separately; UniFFI generates
   bindings but does not ship those platform artifacts.
6. Apply semantic-version and ABI-compatibility checks to the FFI profile
   independently from the editor, WASM and MCP versions.

Promotion requires a reviewed error-code registry, a declared threading model,
an API compatibility baseline, consumer fixtures in C/Swift/Kotlin, sanitizer
evidence and release packages for their actual target triples. Until then,
Swift or Kotlin desktop/mobile experiments should use the WASM package where
their host embeds an appropriate runtime, or call a local CLI/process adapter;
neither path is described as a native production binding.

For Node.js the WebAssembly package remains the default because it already has
exact native parity and no native addon installation matrix. A Node-API addon
is justified only if the benchmark suite shows a material workload that WASM
cannot meet. The WebAssembly Component Model remains a possible future plug-in
ABI, not a substitute for the currently deployed browser module.

## Release boundary

The editor, WASM binding, MCP service and eventual FFI packages have independent
versions. A tag for the editor may attach tested developer artifacts, but it
does not promote `nuif-api` or promise a stable ABI. Publishing a crate, npm
package, Swift package, AAR or vendor plug-in requires an explicit policy and
authenticated release operation for that ecosystem.
