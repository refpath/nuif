# WASM binding

The browser/editor binding will expose `nuif-api` rather than duplicating core semantics in TypeScript.

Initial contract:

- load/canonicalize a document;
- inspect/query semantic entities;
- apply protocol transactions;
- evaluate layout at an explicit context;
- build/render a scene;
- return diagnostics/fidelity reports;
- replay deterministic operation logs.

The exact binding technology (`wasm-bindgen`, WebAssembly Component Model/WIT, or another generated ABI) is intentionally not normative yet. The v0 experiment must prove a stable Rust API first.
