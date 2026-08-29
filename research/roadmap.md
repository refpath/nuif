# Continuous research roadmap

The operational plan and audit findings are in `AUDIT.md`. Work advances by evidence gates, not by document volume or implementation phase names.

## Completed gates

Gate B: canonical model, operations and encodings. The executable baseline covers structural validation, anchored atomic operations, stale-base rejection, replay/inversion, text/CBOR fixpoints, negative numeric/canonical cases, opaque-byte cycles, a passing 10,000-patch seeded trial, and measured hostile-input byte, depth, node, allocation and time budgets. RFC 0009 and `cargo xtask hostile-inputs` close the final Gate B condition.

Gate C: responsive layout falsifier. `cargo xtask gate-c` compares the v0 viewport matrix and seeded stack/flex/grid cases across NUIF, Taffy 0.14.0 and pinned Chrome for Testing 152.0.7977.64. Per-fixture measured bounds, raw boxes and classifications are stored in `target/layout-differential-report.json`. The remaining grid divergences are explicit schema loss, not accepted Grid conformance.

## Current gate

Gate D: visual and text profile. The text-pinning experiment is automated: Ahem 1.50 plus HarfRust 0.13.3/Unicode 17.0.0 matches eight HarfBuzz 14.4.0 shaping strings, unhinted Skrifa 0.46.2 signed-26.6 outlines match five normalized `hb-vector` goldens, and pinned Zeno 0.3.3 8-bit grayscale hashes agree on macOS/aarch64, Linux/aarch64 and Linux/x86_64. `cargo xtask gate-d-text` records each stage separately. Gate D remains open because line breaking/wrapping is absent and broader paints/effects are incomplete.

## Queue

1. Keep Gates B and C plus the Gate D text subgate green with `cargo xtask gate-b`, `cargo xtask hostile-inputs`, `cargo xtask gate-c` and `cargo xtask gate-d-text`; commit minimized failures as fixtures and retain all machine reports as CI artifacts.
2. Design explicit Grid track and placement fields before replacing the classified profile-0 stack fallback; do not infer Grid support from the Gate C pass.
3. Add bounded line breaking/wrapping and the remaining paint operations without regressing the cross-platform outline/grayscale hashes.
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
