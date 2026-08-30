# Continuous research roadmap

The operational plan and audit findings are in `AUDIT.md`. Work advances by evidence gates, not by document volume or implementation phase names.

## Completed gates

Gate B: canonical model, operations and encodings. The executable baseline covers structural validation, anchored atomic operations, stale-base rejection, replay/inversion, text/CBOR fixpoints, negative numeric/canonical cases, opaque-byte cycles, a passing 10,000-patch seeded trial, and measured hostile-input byte, depth, node, allocation and time budgets. RFC 0009 and `cargo xtask hostile-inputs` close the final Gate B condition.

Gate C: responsive and bounded-Grid layout falsifier. `cargo xtask gate-c`
compares the v0 viewport matrix and 24 seeded stack/flex/Grid cases across the
independent NUIF evaluator, Taffy 0.14.0 and pinned Chrome for Testing
152.0.7977.64. Per-fixture measured bounds, raw boxes and classifications are
stored in `target/layout-differential-report.json`. Fixed/`fr` tracks, sparse
row/column flow, explicit placement and spans pass with no classified, blocking
or unexplained divergence; broader CSS Grid remains outside profile 0.

Gate D: bounded visual and text profile. `cargo xtask gate-d` runs separate text and paint reports. Profile 0 pins Ahem/HarfRust/Unicode/Skrifa/Zeno; defines hard lines without automatic soft wrapping; fixes rectangle, ellipse, encoded-sRGB and integer-composition behavior by value; reproduces scene/raw-RGBA hashes on macOS/aarch64, Linux/aarch64 and Linux/x86_64; reports PNG encoding separately; and keeps path/image/instance/extension semantics in property-attributed fidelity records.

Gate E: editor/CLI parity. Twelve semantic actions author the complete v0 fixture from an empty document. Direct generation, editor output and operation replay are byte-identical, while the archived snapshot carries canonical input, context, layout, scene, CPU raster and fidelity.

Gate F: bounded retentive HTML/CSS synchronization. The declared container/text/token subset reparses exactly; six mapped edits change only their scalar spans; comments and unmapped markup survive; unsupported properties remain typed and attributed.

Full-v0 source follow-on: `nuif-html-css-v0` retains 181 model correspondences for the complete responsive card. Eight token/padding/text/responsive edits and the two-edit semantic editor/CLI bridge both re-import exactly, while path, instance and unknown target limitations remain explicit.

Gate G: bounded mechanically independent reproduction. The standard-library-only Python implementation has no Rust/NUIF package dependency and exactly reproduces v0 canonical text, opaque preservation, 24 boxes, three decoded RGBA buffers and five fidelity records. External authorship and a general-purpose second implementation remain standards-publication work.

Gate H: bounded metadata-free collaboration checkpoint. Two algorithmically distinct in-repository materializers converge for every delivery of a conflict-bearing property-register history; conflicts remain explicit and canonical NUIF contains no replica state.

Publication infrastructure: `docs/catalog.json` currently selects 235 canonical Markdown documents without copying their bodies. The bounded `xtask` compiler validates metadata and repository links, generates navigation and status indexes, builds a searchable mdBook site and composes a 13-module working manuscript. The Pages workflow retains pull-request artifacts and restricts deployment permission to its deployment job. This infrastructure publishes evidence; it does not promote evidence status.

## Current falsifiers

`nuif:experiment:v0-responsive-card` and the bounded collaboration property-register checkpoint are complete under their declared acceptance. The next collaboration falsifier is structural: concurrent insert/remove/move must preserve one-parent/acyclic invariants, sibling ordering and deletion intent through an explicit tree/list algorithm, then reproduce its checkpoint through a foreign engine. Register convergence does not imply any of those properties.

The package segment of `nuif:experiment:portable-package-resources` is active:
the manual writer agrees byte-for-byte with an independent ZIP writer, identity
relations and explicit resolution are exercised, and 15 hostile/one-over cases
produce `target/package-resources-report.json`. RFC 0010 remains proposed and
Gate I remains open. `cargo xtask gate-i-image` now provides a narrow
`nuif-png-rgba8-0` cross-decoder, exact-resource and repeatable CPU-render
baseline. The separate `nuif-png-basic-rgba8-1` profile covers the
non-interlaced colour/depth forms that normalize to RGBA8 without sample loss;
16-bit/interlaced/colour-managed PNG, arbitrary transforms and cross-platform
image reproduction remain excluded. `cargo xtask gate-i-font`
adds a deliberately narrow static TrueType external-oracle/package/policy baseline;
TTC, CFF, variable/color/bitmap/WOFF2 acceptance and complete item-level
portability fidelity remain separate beyond the automated package-state outcomes.
Cross-platform writer reproduction and total-resource allocation evidence are
also independent requirements. Browser capture precedes
screenshot reconstruction because it provides stronger source-backed fixtures
and exposes which information is truly unavailable from pixels.

The automated capture/reconstruction contract baseline now produces
`target/capture-reconstruction-report.json`. It checks repeatable provider-input
normalization, exact browser resource retention, credential-query redaction,
honest screenshot omissions, typed proposal application, flat-copy rejection,
codec fixpoints, calibration interpolation and finite correction-loop stops.
This starts but does not complete the broader browser, screenshot, closed-loop
or calibration experiments: no live browser, accuracy corpus, held-out
responsive prediction, independent evaluator or trained artifact is claimed.
Adaptation/distillation remains conditional on evidence from that loop rather
than a standing implementation commitment.

## Queue

1. Keep Gates B through H green with `cargo xtask all` and the separate nightly `cargo xtask fuzz-smoke`; reduce fuzz failures before committing them as named fixtures and retain all machine reports as CI artifacts.
2. Extend the executable existing-tree collaboration profile to concurrent creation, causal-stability garbage collection and combined property/structure transactions; obtain a foreign tree materializer rather than treating the completed Automerge transport oracle as one.
3. Design explicit Grid track and placement fields before replacing the classified profile-0 stack fallback; do not infer Grid support from the Gate C pass.
4. Keep the tested Masonry shell attached only through the editor driver boundary; complete direct manipulation without moving semantic rules into shell code.
5. Treat soft wrapping, gradients, strokes, paths, images and instance materialization as a separately versioned expanded profile; do not weaken profile-0 exactness to add them.
6. Keep `cargo xtask gate-i-package` green across CLI/editor/package changes and add a recorded cross-platform/external writer before accepting the wire profile.
7. Extend the narrow PNG and static TrueType baselines only through new declared fixtures, add cross-platform media reproduction, and complete the broader OpenType format/policy matrix with calibrated allocation/time budgets; do not expand profile 0 by fallback.
8. Extend the fixed-input capture contract into a pinned live-browser adapter with header/body secret canaries, multi-viewport observations and explicit unavailable evidence.
9. Freeze the reconstruction corpus and evaluator, then compare deterministic OCR/CV, one-shot, observation-assisted, hierarchical and corrective-loop baselines through the existing typed boundary.
10. Train or distill only if the frozen evaluation demonstrates a learnable gap and rights-cleared validated traces exist.
11. Maintain the credential-free Penpot package profile under its shared ZIP resource-limit, foreign-producer and unknown-member-retention gate; defer the compact representation until upstream stability and a second fixture.
12. Run the bounded Figma and Adobe profiles in named live host versions; retain host reports and do not infer live behavior from API documentation.
13. Package the conformance kit for externally authored reproduction; do not treat the in-repository Python path or Rust adapters as external interoperability evidence.
14. Keep standards-development work behind the implementer-draft and external-support gates in `docs/STANDARDS-ROADMAP.md`.

## Update policy

- Add evidence as a new record or source revision; use `supersedes` and `contradicts` rather than silently rewriting history.
- Record source commit, tag or specification revision where available.
- Promote `reviewed` to `verified` only after locator-level checks and executable evidence for implementation claims.
- Every experiment declares seed/input, oracle class, acceptance criteria, artifacts and implementation path before it can become `active`.
- Every completed experiment stores a machine report and the exact engine/toolchain/profile identity.
- Claims become specification requirements only through RFC review and executable conformance fixtures.
