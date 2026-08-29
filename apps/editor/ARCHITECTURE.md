# Reference editor architecture

The editor is a client of the same semantic engine used by CLI/API tooling.

```text
Svelte 5 / TypeScript shell
        │ typed commands/events
        ▼
Rust core via WASM boundary
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
