# Conformance

The profile-0 baseline is executable through the `nuif-conformance` package, `nuif trial` and `cargo xtask hostile-inputs`. It covers structural validation, canonicalization, parser/serializer round trips, unknown-extension preservation, operations/replay/inversion, responsive stack layout, deterministic solid-color CPU rendering, measured codec/model resource limits, editor-driver parity and machine reports. Browser/Taffy differential layout, shaped text, full geometry/paint, adapters and perceptual tiers remain planned and are not claimed as implemented.

Automated and conventional QA run the same headless tests. GUI automation is supplementary; semantic API operations are the primary test interface. `HARNESS.md` specifies the workspace layout, fixture format, determinism controls, oracles, trial loop, reducer and report schema.
