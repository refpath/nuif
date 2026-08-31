# nuif-canva

`nuif-canva` is the credential-free, normalized boundary for the bounded Canva
Design Editing profile `nuif-canva-design-editing-0`. It is deliberately a
library adapter, not a Canva SDK client: a host integration supplies a current
page snapshot, and the adapter returns a validated `nuif-core::Document` plus
`HostAdapterReport` correspondence and fidelity evidence.

The profile currently maps one fixed-size page with an optional opaque sRGB
background, nullable page/element names, ordered groups, rectangles, ellipses
and literal text using the pinned profile font. It rejects locked content,
unsupported properties, invalid geometry, nonopaque colors and unbounded input
before a host mutation could be attempted. IDs exported by NUIF use an explicit
round-trip marker so repeated import/export does not invent new identities.

## Library

```rust
let imported = nuif_canva::import_current_page(snapshot_bytes)?;
let exported = nuif_canva::export_page(&document, "2026.1")?;
```

`adapters/canva/app` is the thin Canva Apps SDK consumer. Its live mutation
subset is narrower than this normalized mapping: an empty same-size page,
nullable page name, unnamed opaque rectangles/canonical ellipses and an
optional opaque background. Text and groups remain pure-mapper fixtures until
the public host API can prove the identity and metrics the profile requires.

The adapter does not claim a live Canva run, marketplace approval, Connect API
NUIF import/export, or a native Canva file format. Those require retained
owner/reviewer host tests and remain outside credential-free CI.

## Local evidence

```text
cargo test -p nuif-canva --offline
cargo xtask gate-canva
```

The gate writes the pure mapping report, Rust/TypeScript plan and round-trip
fixtures, `target/canva-app-shell-report.json`, an informational scaling report
and `target/nuif-canva-review-app`. The packaged app carries Canva's SDK license
and is restricted to permitted apps on the Canva Platform. Every `live_host`
field remains `not_run` until a named runtime and API version have been tested
by a human reviewer.
