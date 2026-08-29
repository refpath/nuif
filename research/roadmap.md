# Continuous research roadmap

The operational plan and audit findings are in `AUDIT.md`. Work advances by evidence gates, not by document volume or implementation phase names.

## Completed gates

Gate B: canonical model, operations and encodings. The executable baseline covers structural validation, anchored atomic operations, stale-base rejection, replay/inversion, text/CBOR fixpoints, negative numeric/canonical cases, opaque-byte cycles, a passing 10,000-patch seeded trial, and measured hostile-input byte, depth, node, allocation and time budgets. RFC 0009 and `cargo xtask hostile-inputs` close the final Gate B condition.

Gate C: responsive layout falsifier. `cargo xtask gate-c` compares the v0 viewport matrix and seeded stack/flex/grid cases across NUIF, Taffy 0.14.0 and pinned Chrome for Testing 152.0.7977.64. Per-fixture measured bounds, raw boxes and classifications are stored in `target/layout-differential-report.json`. The remaining grid divergences are explicit schema loss, not accepted Grid conformance.

Gate D: bounded visual and text profile. `cargo xtask gate-d` runs separate text and paint reports. Profile 0 pins Ahem/HarfRust/Unicode/Skrifa/Zeno; defines hard lines without automatic soft wrapping; fixes rectangle, ellipse, encoded-sRGB and integer-composition behavior by value; reproduces scene/PNG hashes on macOS/aarch64, Linux/aarch64 and Linux/x86_64; and keeps path/image/instance/extension semantics in property-attributed fidelity records.

## Current gate

Gate E: editor/CLI parity. The headless editor already exposes semantic accessibility nodes/actions and proves replay parity for rename and size edits. The remaining falsifier is complete v0 construction through semantic actions, with the direct fixture builder and CLI replay reaching the same canonical hash before any GUI shell can claim parity.

## Queue

1. Keep Gates B through D green with `cargo xtask gate-b`, `cargo xtask hostile-inputs`, `cargo xtask gate-c` and `cargo xtask gate-d`; commit minimized failures as fixtures and retain all machine reports as CI artifacts.
2. Design explicit Grid track and placement fields before replacing the classified profile-0 stack fallback; do not infer Grid support from the Gate C pass.
3. Finish Gate E by authoring the entire v0 fixture through editor accessibility actions and then attach the Masonry shell to the already-tested driver boundary.
4. Build one HTML/CSS retentive adapter for Gate F before expanding to design-tool or native-framework adapters.
5. Treat soft wrapping, gradients, strokes, paths, images and instance materialization as a separately versioned expanded profile; do not weaken profile-0 exactness to add them.
6. Defer collaboration profiles until canonical operations, ordering and source correspondence have passed their gates.
7. Publish the profile for independent Gate G reproduction only after the v0 round trip and its fidelity report are complete.

## Update policy

- Add evidence as a new record or source revision; use `supersedes` and `contradicts` rather than silently rewriting history.
- Record source commit, tag or specification revision where available.
- Promote `reviewed` to `verified` only after locator-level checks and executable evidence for implementation claims.
- Every experiment declares seed/input, oracle class, acceptance criteria, artifacts and implementation path before it can become `active`.
- Every completed experiment stores a machine report and the exact engine/toolchain/profile identity.
- Claims become specification requirements only through RFC review and executable conformance fixtures.
