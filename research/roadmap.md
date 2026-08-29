# Continuous research roadmap

The operational plan and audit findings are in `AUDIT.md`. Work advances by evidence gates, not by document volume or implementation phase names.

## Completed gates

Gate B: canonical model, operations and encodings. The executable baseline covers structural validation, anchored atomic operations, stale-base rejection, replay/inversion, text/CBOR fixpoints, negative numeric/canonical cases, opaque-byte cycles, a passing 10,000-patch seeded trial, and measured hostile-input byte, depth, node, allocation and time budgets. RFC 0009 and `cargo xtask hostile-inputs` close the final Gate B condition.

Gate C: responsive layout falsifier. `cargo xtask gate-c` compares the v0 viewport matrix and seeded stack/flex/grid cases across NUIF, Taffy 0.14.0 and pinned Chrome for Testing 152.0.7977.64. Per-fixture measured bounds, raw boxes and classifications are stored in `target/layout-differential-report.json`. The remaining grid divergences are explicit schema loss, not accepted Grid conformance.

## Current gate

Gate D: visual and text profile. The first text subgate is automated: Ahem 1.50 is pinned by SHA-256, HarfRust 0.13.3 and Unicode 17.0.0 produce resolved runs with Unicode-scalar clusters, and eight LTR/RTL cases match independently captured HarfBuzz 14.4.0 glyph strings. `cargo xtask gate-d-text` separately records exact shaping and repeatable-but-approximated raster evidence. Gate D remains open because glyph outlines, grayscale coverage, blend space, broader paints/effects and cross-platform normative raster measurements are not defined.

## Queue

1. Keep Gates B and C plus the Gate D text subgate green with `cargo xtask gate-b`, `cargo xtask hostile-inputs`, `cargo xtask gate-c` and `cargo xtask gate-d-text`; commit minimized failures as fixtures and retain all machine reports as CI artifacts.
2. Design explicit Grid track and placement fields before replacing the classified profile-0 stack fallback; do not infer Grid support from the Gate C pass.
3. Add a pinned outline extractor and unhinted grayscale CPU raster stage; calibrate it across the CI matrix before claiming Gate D raster exactness.
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
