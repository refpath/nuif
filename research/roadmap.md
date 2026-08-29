# Continuous research roadmap

The operational plan and audit findings are in `AUDIT.md`. Work advances by evidence gates, not by document volume or implementation phase names.

## Current gate

Gate B: canonical model, operations and encodings. The executable baseline covers structural validation, anchored atomic operations, stale-base rejection, replay/inversion, text/CBOR fixpoints, negative numeric/canonical cases, opaque-byte cycles and a passing 10,000-patch seeded trial. The remaining Gate B work is measured hostile-input depth, node, allocation and time budgets.

## Queue

1. Keep Gate B green with `cargo xtask gate-b` (10,000 deterministic generated patches, with expensive raster checks sampled every 100 patches) and commit any minimized failures as fixtures; add measured hostile-input limits before closing the gate.
2. Run Gate C layout differential generation against pinned Taffy and browser versions; derive tolerance distributions from results.
3. Pin font assets and shaping inputs before claiming Gate D text or cross-platform raster exactness.
4. Finish Gate E by authoring the entire v0 fixture through editor accessibility actions and then attach the Masonry shell to the already-tested driver boundary.
5. Build one HTML/CSS retentive adapter for Gate F before expanding to design-tool or native-framework adapters.
6. Defer collaboration profiles until canonical operations, ordering and source correspondence have passed their gates.
7. Publish the profile for independent Gate G reproduction only after the v0 round trip and its fidelity report are complete.

## Update policy

- Add evidence as a new record or source revision; use `supersedes` and `contradicts` rather than silently rewriting history.
- Record source commit, tag or specification revision where available.
- Promote `reviewed` to `verified` only after locator-level checks and executable evidence for implementation claims.
- Every experiment declares seed/input, oracle class, acceptance criteria, artifacts and implementation path before it can become `active`.
- Every completed experiment stores a machine report and the exact engine/toolchain/profile identity.
- Claims become specification requirements only through RFC review and executable conformance fixtures.
