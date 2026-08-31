# nuif-canva

`nuif-canva` is the credential-free, normalized boundary for the bounded Canva
Design Editing profile `nuif-canva-design-editing-0`. It is deliberately a
library adapter, not a Canva SDK client: a host integration supplies a current
page snapshot, and the adapter returns a validated `nuif-core::Document` plus
`HostAdapterReport` correspondence and fidelity evidence.

The profile currently maps one fixed-size page with ordered groups, rectangles,
ellipses and literal text using the pinned profile font. It rejects locked
content, unsupported properties, invalid geometry and unbounded input before a
host mutation could be attempted. IDs exported by NUIF use an explicit
round-trip marker so repeated import/export does not invent new identities.

## Library

```rust
let imported = nuif_canva::import_current_page(snapshot_bytes)?;
let exported = nuif_canva::export_page(&document, "2026.1")?;
```

The adapter does not claim a live Canva app, marketplace approval, Connect API
NUIF import/export, or a native Canva file format. Those require owner-authored
host tests and remain outside CI.

## Local evidence

```text
cargo test -p nuif-canva --offline
cargo xtask gate-canva
```

The gate writes `target/canva-current-page-report.json`. Its `live_host` field
is intentionally `not_run` until a named Canva runtime and API version have
been tested by a human reviewer.
