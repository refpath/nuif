# Benchmark suite

NUIF separates portable release budgets from controlled-hardware statistical
measurements. `cargo xtask performance` always runs the portable release
profile, then executes every Criterion path once with Criterion's test mode.
That smoke execution detects stale fixtures, panics and dependency drift without
pretending that a shared CI runner provides stable throughput measurements.
Successful execution writes `target/criterion-smoke-report.json`; portable
latency and allocation budgets remain in `target/performance-profile-report.json`.

For controlled measurements, run:

```sh
cargo bench --locked -p nuif-conformance --bench profile_zero
cargo bench --locked -p nuif-conformance --bench system_surfaces
```

Keep the machine idle, record its CPU, operating system, power mode, toolchain
and source revision, and compare saved Criterion baselines only on equivalent
hardware. Do not turn one noisy percentage into a merge gate. A proposed
optimization must preserve the conformance gates first and then improve a
predeclared workload over repeated samples.

## Coverage

`profile_zero` measures validation, canonical text/CBOR, patch application,
local transactions, undo/redo, layout, scene construction, CPU rasterization
and complete SDK snapshots over bounded fixture scales.

`system_surfaces` measures direct SDK text/CBOR/package calls, structural and
authorized package-capability paths, register and advanced collaboration
materializers, package/resource profiles, variable-font package delivery and
every integrated adapter profile:

- HTML/CSS profile 0 and full-v0 import/export/synchronization;
- SVG, DTCG, React and Svelte import/export/synchronization;
- Penpot native/foreign package import, export and no-op/edited synchronization;
- Figma snapshot and Canva current-page import/mutation-plan generation;
- web accessibility and finite behavior projection.

The variable-font baseline separates package load plus capability authorization,
snapshotting an already-authorized document, and the end-to-end load/authorize/
snapshot path. The portable report applies catastrophic time and allocation
ceilings; Criterion supplies controlled-hardware distributions. Neither path
includes process startup or claims cross-platform raster timing equivalence.

The `collaboration/advanced_profiles` group also measures the bounded nested
creation profile 1, mixed property/structure materializer, register-prefix
compaction path and conservative structural-prefix compaction path on
prevalidated fixtures. These are diagnostic in-process costs, not throughput
claims for a networked collaboration service.

Figma, Canva and browser profiles measure pure mapping only; live host latency
belongs to separately versioned host trials. WASM, MCP and CLI process startup belong
to their cross-surface package gates, because mixing process launch with
in-process Criterion samples would obscure both costs. Affinity, SwiftUI,
Compose and Flutter have no integrated executable profile and
therefore no benchmark claim. The Affinity draft composes the already measured
SVG adapter with a separately timed human/live-host trial. Canva's integrated
pure mapping has a benchmark claim; its live host remains external evidence.

## Optimization rule

Benchmark setup constructs and validates fixtures before timing. Measured code
must consume inputs through `black_box`, and mutation/history measurements use
batched clones so state does not leak between samples. A faster result is not
accepted if it changes canonical bytes, diagnostics, fidelity, resource policy
or operation atomicity.
