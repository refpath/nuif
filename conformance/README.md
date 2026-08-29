# Conformance

The profile-0 baseline is executable through the `nuif-conformance` package and xtask gates. It covers structural validation, canonicalization, parser/serializer round trips, unknown-extension preservation, operations/replay/inversion, responsive stack/flex layout, deterministic solid-color CPU rendering, measured codec/model resource limits, pinned browser/Taffy differential layout, editor-driver parity and machine reports. Grid track/placement semantics, shaped text, full geometry/paint, adapters and perceptual tiers remain planned and are not claimed as implemented.

Install the locked Chrome for Testing build once with `cargo xtask browser-install`, then run `cargo xtask gate-c`. The report at `target/layout-differential-report.json` contains the raw NUIF, Taffy and browser boxes, engine versions, source revision, fixture-local calibration and every classified divergence. Schema-loss entries are visible passing evidence, not a Grid-conformance claim; unclassified or evaluator differences fail the command.

Automated and conventional QA run the same headless tests. GUI automation is supplementary; semantic API operations are the primary test interface. `HARNESS.md` specifies the workspace layout, fixture format, determinism controls, oracles, trial loop, reducer and report schema.
