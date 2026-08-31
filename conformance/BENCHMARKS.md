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
authorized package-capability paths, entity queries, both collaboration
materializers, package/resource profiles and every integrated adapter profile:

- HTML/CSS profile 0 and full-v0 import/export/synchronization;
- SVG, DTCG, React and Svelte import/export/synchronization;
- Penpot native/foreign package import, export and no-op/edited synchronization;
- Figma snapshot import and mutation-plan generation;
- web accessibility and finite behavior projection.

Figma and browser profiles measure pure mapping only; live host latency belongs
to a separately versioned host trial. WASM, MCP and CLI process startup belong
to their cross-surface package gates, because mixing process launch with
in-process Criterion samples would obscure both costs. Adobe, SwiftUI, Compose
and Flutter have no integrated executable profile and therefore no benchmark
claim.

## Optimization rule

Benchmark setup constructs and validates fixtures before timing. Measured code
must consume inputs through `black_box`, and mutation/history measurements use
batched clones so state does not leak between samples. A faster result is not
accepted if it changes canonical bytes, diagnostics, fidelity, resource policy
or operation atomicity.
