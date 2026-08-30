# NUIF MCP adapter

`nuif-mcp-tools-0` is a stateless, stdio-only Model Context Protocol adapter
over the same `nuif-api`, codec and semantic-operation crates used by the CLI,
WebAssembly binding and reference editor. It is an experimental developer tool,
not a network service or a canonical NUIF protocol.

Build it from a reviewed checkout without an app store or payment gateway:

```sh
cargo install --path crates/nuif-mcp --locked
```

Point an MCP host's local stdio configuration at the resulting `nuif-mcp`
executable. The exact host configuration shape belongs to that host; the server
itself accepts MCP `2026-07-28` only and writes no non-protocol data to stdout.

The profile exposes four pure tools:

- `nuif_validate`
- `nuif_inspect`
- `nuif_canonicalize`
- `nuif_apply_patch`

Every call supplies canonical NUIF text inline and returns a value. Even
`nuif_apply_patch` mutates only a temporary in-memory session and returns a new
canonical document. The process has no filesystem, network, package-resource,
host-document, credential, roots, sampling, task or hidden document-session
authority.

Limits are 4 MiB per newline-delimited MCP message, 1 MiB per inline document,
1 MiB per patch, 1,024 transactions and 16,384 operations. Larger documents and
`.nuif` packages should use the direct library, CLI or WASM surfaces under
explicit host control.

Run the independent subprocess and native-core oracle:

```sh
cargo xtask gate-mcp
cargo xtask mcp-package
```

The gate opens with `server/discover` and no legacy initialization handshake,
requires complete metadata on every request, checks generated schemas and
annotations, compares canonicalization and patch bytes with the native CLI,
classifies malformed and stale inputs, sends one frame above the transport
limit, and records a small wire-latency sample in
`target/mcp-conformance-report.json`.

`mcp-package` repeats the live gate against an optimized binary, then creates a
host archive and sibling manifest under `target/dist/`. Tagged GitHub
prereleases build and attest those archives on Linux x86-64/Arm64, Windows
x86-64 and macOS Arm64/x86-64. The binary is independently versioned at
`0.0.1`; the editor's alpha version does not imply MCP protocol maturity.
