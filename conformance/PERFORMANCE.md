# Profile-zero performance methodology

NUIF uses two complementary performance gates. The portable smoke profile is a
release binary that runs on developer machines and shared CI, records latency
and allocation evidence, and rejects only catastrophic regressions. The
Criterion suite is the statistical comparison tool for controlled hardware.
Results from different machines, operating-system states or Rust toolchains are
not treated as comparable baselines.

## Workloads

| Group | Scales | Work measured |
| --- | --- | --- |
| Model validation | 8, 128, 1,024, 4,096 entities | Complete profile-zero structural and resource validation |
| Canonical text and deterministic CBOR | 8, 128, 1,024 entities | Encode and decode independently |
| Protocol/session | 8, 128, 1,024, 4,096 entities | Clone a document and apply one rename transaction; local sessions cover cold and revision-cached edits plus undo/redo |
| Layout | 8, 128, 1,024, 4,096 entities | Evaluate a flat mixed shape/text document |
| Scene lowering | 8, 128, 1,024 entities | Lower evaluated entities to deterministic render commands |
| CPU raster and API snapshot | 360x640, 768x640, 1,440x900 | Raster an interactive card fixture; snapshot includes hash, layout, scene and raster |
| Embedded image and font resources | RGBA8 image plus pinned static TrueType font | Inspect and decode the media profiles; encode and decode exact-resource packages; lower a resolved image scene and raster it at 256x256 |
| Semantic query | 128, 1,024, 4,096, 8,192 entities | Stable-ID lookup and kind scan over the authored model |
| Collaboration | 2, 32, 256, 1,024 concurrent register writers | Materialize identical conflict checkpoints through the operation-set and replica-log algorithms |
| Integrated adapters | Both declared HTML/CSS profiles, SVG, DTCG, Penpot, static React JSX, static Svelte and normalized Figma snapshot fixtures | Every declared import/export direction is measured separately; retentive profiles also measure synchronization, Penpot measures official-foreign import and byte-exact no-op synchronization, and Figma measures pure mutation-plan construction and snapshot import |

`nuif_testing::performance_fixture` is deterministic, valid, bounded by the
profile-zero 8,192-entity resource limit and uses the repository-pinned font.
Every sixteenth child is text in mixed workloads. Fixture construction is
outside timed sections. Protocol clone cost remains deliberately included
because atomic application currently requires an isolated candidate document.

## Commands

Run the portable release-mode smoke gate and compile every statistical
benchmark:

```sh
cargo xtask performance
```

Run the complete Criterion suites without plotting:

```sh
cargo bench -p nuif-conformance --bench profile_zero -- --noplot
cargo bench -p nuif-conformance --bench system_surfaces -- --noplot
```

On an otherwise idle controlled machine, save a baseline before a change and
compare against it afterward:

```sh
cargo bench -p nuif-conformance --bench profile_zero -- --save-baseline before --noplot
cargo bench -p nuif-conformance --bench profile_zero -- --baseline before --noplot
cargo bench -p nuif-conformance --bench system_surfaces -- --save-baseline before --noplot
cargo bench -p nuif-conformance --bench system_surfaces -- --baseline before --noplot
```

Criterion accepts a filter after `--`, such as `-- codec --noplot`. Use
`--profile-time 15` to collect a longer, non-statistical profiling workload for
an external profiler. Keep the same power mode, foreground load, toolchain and
build inputs for both sides of a comparison.

## Local calibration

The first `system_surfaces` calibration ran on macOS/aarch64, Apple M5 Pro,
rustc 1.98.0, with 20 Criterion samples, one second of warm-up and two seconds
of measurement per case on 2026-08-30. These values establish workload
plausibility and one optimization comparison; they are not portable release
budgets.

- Stable-ID lookup at 8,192 entities measured 18.5–19.2 ns; a complete kind
  scan measured 19.2–19.5 µs.
- HTML/CSS, SVG and DTCG synchronization measured 226.7–228.0 µs,
  189.7–190.5 µs and 55.5–55.8 µs respectively for their declared fixtures.
- Penpot native export measured 108.3–109.2 µs, native import 79.3–80.2 µs,
  official-library import 98.3–99.1 µs, a two-scalar synchronized
  rebuild/re-import 93.9–94.6 µs, and byte-exact no-op synchronization
  2.85–2.87 µs. These figures use the 7,855-byte native and 5,439-byte foreign
  fixtures rather than a large production design.
- The matching allocation-instrumented smoke run now covers 48 cases. The nine
  resource-path cases add PNG structure/decode, static-font inspection, image
  and font package encode/decode, resolved scene lowering and a 256x256 image
  raster. On the same machine, font inspection measured a 0.67 ms median and
  about 1.12 MiB allocated; font package encode/decode measured 1.05/0.89 ms
  and about 1.41/1.38 MiB. The 96-byte image fixture is deliberately a boundary
  smoke case rather than a throughput claim; Criterion owns controlled
  comparisons and larger media corpus work remains explicit.
- Static React JSX export/import/synchronization measured 0.078/0.077/0.235 ms
  median for the 778-byte declared fixture, allocating about 71/67/244 KiB per
  invocation and retaining zero. These are parser-boundary calibration values,
  not React runtime or browser performance claims.
- Static Svelte export/import/synchronization measured 0.053/0.052/0.166 ms
  median for the 761-byte declared fixture, allocating about 68/64/236 KiB per
  invocation and retaining zero. Official compiler execution is intentionally
  outside these production-parser timings and remains a conformance-oracle
  cost, not an application-runtime benchmark.
- HTML/CSS v0 and the normalized Figma snapshot profile are included in both
  performance layers. The smoke report derives the complete integrated-profile
  inventory from `adapters/index.json` and fails if its benchmark inventory
  drifts, so a newly integrated adapter cannot silently omit performance
  evidence. Figma timing covers only the deterministic mapper; plug-in-host
  responsiveness remains a named live-host boundary.
- Penpot
  native export and edited synchronization allocated 291 KiB and 246 KiB per
  invocation; native and foreign imports allocated 215 KiB and 665 KiB, and the
  no-op path allocated 35 KiB. Every adapter case retained zero bytes after the
  measured invocation.
- An initial all-Deflate native export was 4,688 bytes and allocated about
  4.04 MiB; edited synchronization allocated about 3.99 MiB. Storing native
  JSON members below 4 KiB increased this small package by 3,167 bytes while
  reducing those allocations by about 93% and the two Criterion times by about
  49% and 52%. Imported compression methods remain retentive, so foreign-package
  behaviour did not change. The threshold is a profile workload decision, not a
  general claim that ZIP storage is preferable to Deflate.
- The 1,024-writer operation-set checkpoint measured 4.08–4.15 ms before
  replacing an all-pairs causal-maximality search with per-replica maximum
  observed vector contexts. The algorithmically independent replica-log
  frontier remained separate. A same-process saved-baseline comparison measured
  2.51–2.81 ms afterward and Criterion classified the change as a 36.1–40.3%
  reduction in time (`p < 0.05`). Exact checkpoint equality, 5,040 delivery
  permutations and both materializers remain Gate H postconditions.

## Interpretation

- Treat the portable budgets as availability limits, not optimization targets.
  Its JSON artifact is useful for trend inspection and allocation diagnosis,
  but shared-runner timing noise is expected.
- Require a repeatable Criterion change on the same machine before claiming a
  small speedup or regression. Inspect both time and throughput across scales;
  one small fixture can hide an algorithmic regression.
- Preserve canonical bytes, hashes, diagnostics, fidelity records and operation
  atomicity while optimizing. Performance never relaxes conformance.
- Record allocation changes alongside latency. A time win that creates
  unbounded intermediate data is not accepted.
- Add a workload only when it has a stable fixture and a clear user-visible
  operation. Avoid benchmarks that measure fixture setup or debug assertions.

The command design follows the official [Cargo benchmark
documentation](https://doc.rust-lang.org/cargo/commands/cargo-bench.html) and
Criterion's [command-line](https://bheisler.github.io/criterion.rs/book/user_guide/command_line_options.html)
and [configuration](https://bheisler.github.io/criterion.rs/book/user_guide/advanced_configuration.html)
guidance. `iai-callgrind` remains a possible Linux-only instruction-count layer,
but is not a portable gate because it requires Valgrind; Criterion and the
allocation-instrumented smoke profile cover the current cross-platform need.
