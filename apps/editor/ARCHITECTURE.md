# Reference editor architecture

The editor is a client of the same semantic engine used by CLI/API tooling. The shell technology is decided in ADR 0006 (accepted): a Rust-native shell on Masonry, Vello and AccessKit; the Svelte 5 shell below is retained as the browser demonstration path. The user-interface specification is `UI-SPEC.md`.

```text
Rust shell (Masonry widgets, AccessKit tree) — or Svelte 5 shell over WASM for the browser demonstration
        │ typed commands/events
        ▼
Rust core (in-process; WASM boundary only in the browser build)
  ├── document store
  ├── protocol/transactions
  ├── layout evaluators
  ├── render-scene builder
  ├── query/diagnostics
  └── codecs
        │
        ▼
renderer backend (WebGPU/Vello experiment)
```

The UI shell may keep ephemeral selection/viewport/panel state, but authored document state is NUIF state. Canvas gestures MUST translate into semantic protocol operations before mutation.

The editor must expose a local automation endpoint or in-process API that mirrors CLI semantics. MCP may be added as an adapter, never as the canonical automation contract.
