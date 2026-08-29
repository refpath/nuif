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
| Semantic query | 128, 1,024, 4,096, 8,192 entities | Stable-ID lookup and kind scan over the authored model |
| Collaboration | 2, 32, 256, 1,024 concurrent register writers | Materialize identical conflict checkpoints through the operation-set and replica-log algorithms |
| Integrated adapters | Declared HTML/CSS, SVG and DTCG fixtures | Export, import and byte-local synchronization measured separately |

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
