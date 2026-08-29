# Conformance

The profile-0 baseline is executable through the `nuif-conformance` package and xtask gates. It covers structural validation, canonicalization, parser/serializer round trips, unknown-extension preservation, operations/replay/inversion, responsive stack/flex layout, pinned shaping and hard-line text, deterministic solid rectangle/ellipse CPU rendering, measured codec/model resource limits, pinned browser/Taffy differential layout, editor-driver parity, four bounded retentive adapter profiles and machine reports. Grid track/placement semantics, paths/images/instances, expanded geometry/paint profiles, broader adapters and perceptual tiers remain planned and are not claimed as implemented.

Install the locked Chrome for Testing build once with `cargo xtask browser-install`, then run `cargo xtask gate-c`. The report at `target/layout-differential-report.json` contains the raw NUIF, Taffy and browser boxes, engine versions, source revision, fixture-local calibration and every classified divergence. Schema-loss entries are visible passing evidence, not a Grid-conformance claim; unclassified or evaluator differences fail the command.

Run `cargo xtask gate-d` for both text and paint. It writes `target/text-pinning-report.json` and `target/render-profile-report.json`; committed hashes, missing font failures, color validation and property-attributed unsupported/preserved fidelity are blocking checks.

Automated and conventional QA run the same headless tests. GUI automation is supplementary; semantic API operations are the primary test interface. `HARNESS.md` specifies the workspace layout, fixture format, determinism controls, oracles, trial loop, reducer and report schema.

Run `cargo xtask performance` for the portable release-mode smoke profile. It measures validation, both codecs, protocol apply, layout, scene lowering, CPU rasterization and end-to-end snapshots at a representative 1,024-entity scale; records median/p95 latency and warmed allocation counts; enforces deliberately broad catastrophic-regression budgets; writes `target/performance-profile-report.json`; and compiles the statistical suite. Run `cargo bench -p nuif-conformance --bench profile_zero -- --noplot` on controlled hardware for Criterion scaling comparisons from 8 through 4,096 entities. Shared CI timing is evidence, not a fine-grained cross-machine baseline. See [PERFORMANCE.md](PERFORMANCE.md) for the workload contract, comparison workflow and interpretation rules.

Run `cargo xtask gate-svg` for the bounded `nuif-svg-0` adapter. It checks
exact model round trips, seven byte-local semantic edits, preservation of the
complete unchanged-byte complement, typed hostile cases and a public-CLI
export/sync/import bridge. The machine report and synchronized sources are
written under `target/svg-sync-*`; the declared subset and its exclusions are
specified in `adapters/svg/PROFILE.md`.

Run `cargo xtask gate-dtcg` for the bounded `nuif-dtcg-scalar-0` adapter. It
checks exact scalar-token round trips, integer/real discrimination, eight
byte-local edits, root and token extension preservation, duplicate/depth/count
and source limits, and a public-CLI bridge. Reports and retained sources are
written under `target/dtcg-sync-*`; groups, aliases and composite types remain
outside the declared profile.
