# Implementation language and runtime choice

## Decision: Rust reference core

Rust is the strongest default for the reference implementation because the project simultaneously requires untrusted binary parsing, graph/document transforms, geometry, text shaping, native/WASM embedding, GPU access, fuzzing and stable C-compatible boundaries.

### Alternatives

- **C++** has the deepest graphics ecosystem and mature Skia/Yoga integration, but expands memory-safety risk in parsers/plugins and makes a browser/WASM-safe reference core less attractive.
- **Zig** offers excellent systems control and C interoperability but has a smaller mature graphics/text/schema ecosystem and less API stability for a standards reference implementation.
- **Go** is strong for services/tooling but weaker for low-level rendering/WASM/native GUI integration and deterministic allocation-sensitive engines.
- **TypeScript** is ideal at web/editor adapter boundaries but unsuitable as the only renderer/codec/reference-core implementation.

## Stack

- Rust: document model, operations, layout abstraction, codec, renderer scene, conformance, WASM bindings.
- Taffy: initial CSS-compatible evaluator behind NUIF types.
- Vello/wgpu: interactive renderer experiment behind a NUIF renderer trait.
- HarfBuzz-compatible shaping: text experiment with pinned font inputs.
- Masonry + AccessKit: reference editor shell (ADR 0006, accepted; toolchain 1.98.0, MSRV 1.96); Svelte 5 + TypeScript for the later browser demonstration over the WASM bindings.
- Tree-sitter/language-native parsers: source adapters where concrete syntax retention is required.

Adapters MAY be written in the ecosystem-native language; conformance is against behavior/protocol, not implementation language.
