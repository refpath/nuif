# Continuous research roadmap

The operational plan and audit findings are in `AUDIT.md`. Work advances by evidence gates, not by document volume or implementation phase names.

## Completed gates

Gate B: canonical model, operations and encodings. The executable baseline covers structural validation, anchored atomic operations, stale-base rejection, replay/inversion, text/CBOR fixpoints, negative numeric/canonical cases, opaque-byte cycles, a passing 10,000-patch seeded trial, and measured hostile-input byte, depth, node, allocation and time budgets. RFC 0009 and `cargo xtask hostile-inputs` close the final Gate B condition.

Gate C: responsive layout falsifier. `cargo xtask gate-c` compares the v0 viewport matrix and seeded stack/flex/grid cases across NUIF, Taffy 0.14.0 and pinned Chrome for Testing 152.0.7977.64. Per-fixture measured bounds, raw boxes and classifications are stored in `target/layout-differential-report.json`. The remaining grid divergences are explicit schema loss, not accepted Grid conformance.

Gate D: bounded visual and text profile. `cargo xtask gate-d` runs separate text and paint reports. Profile 0 pins Ahem/HarfRust/Unicode/Skrifa/Zeno; defines hard lines without automatic soft wrapping; fixes rectangle, ellipse, encoded-sRGB and integer-composition behavior by value; reproduces scene/raw-RGBA hashes on macOS/aarch64, Linux/aarch64 and Linux/x86_64; reports PNG encoding separately; and keeps path/image/instance/extension semantics in property-attributed fidelity records.

Gate E: editor/CLI parity. Twelve semantic actions author the complete v0 fixture from an empty document. Direct generation, editor output and operation replay are byte-identical, while the archived snapshot carries canonical input, context, layout, scene, CPU raster and fidelity.

Gate F: bounded retentive HTML/CSS synchronization. The declared container/text/token subset reparses exactly; six mapped edits change only their scalar spans; comments and unmapped markup survive; unsupported properties remain typed and attributed.

Full-v0 source follow-on: `nuif-html-css-v0` retains 181 model correspondences for the complete responsive card. Eight token/padding/text/responsive edits and the two-edit semantic editor/CLI bridge both re-import exactly, while path, instance and unknown target limitations remain explicit.

Gate G: bounded mechanically independent reproduction. The standard-library-only Python implementation has no Rust/NUIF package dependency and exactly reproduces v0 canonical text, opaque preservation, 24 boxes, three decoded RGBA buffers and five fidelity records. External authorship and a general-purpose second implementation remain standards-publication work.

Gate H: bounded metadata-free collaboration checkpoint. Two algorithmically distinct in-repository materializers converge for every delivery of a conflict-bearing property-register history; conflicts remain explicit and canonical NUIF contains no replica state.

Publication infrastructure: `docs/catalog.json` currently selects 231 canonical Markdown documents without copying their bodies. The bounded `xtask` compiler validates metadata and repository links, generates navigation and status indexes, builds a searchable mdBook site and composes a 13-module working manuscript. The Pages workflow retains pull-request artifacts and restricts deployment permission to its deployment job. This infrastructure publishes evidence; it does not promote evidence status.

## Current falsifiers

`nuif:experiment:v0-responsive-card` and the bounded collaboration property-register checkpoint are complete under their declared acceptance. The next collaboration falsifier is structural: concurrent insert/remove/move must preserve one-parent/acyclic invariants, sibling ordering and deletion intent through an explicit tree/list algorithm, then reproduce its checkpoint through a foreign engine. Register convergence does not imply any of those properties.

The next resource falsifier is `nuif:experiment:portable-package-resources`:
two writers must agree on bytes before RFC 0010 can become accepted. PNG and
font profiles remain independent gates. Browser capture precedes screenshot
reconstruction because it provides stronger source-backed fixtures and exposes
which information is truly unavailable from pixels.

The reconstruction work begins with untuned baselines and a frozen evaluator.
Adaptation/distillation is conditional on evidence from that loop rather than a
standing implementation commitment.

## Queue

1. Keep Gates B through H green with `cargo xtask all`; commit minimized failures as fixtures and retain all machine reports as CI artifacts.
2. Extend collaboration from property registers to a proved tree move/list profile with explicit cycle, tombstone and sibling-order behavior; compare a foreign engine before broad claims.
3. Design explicit Grid track and placement fields before replacing the classified profile-0 stack fallback; do not infer Grid support from the Gate C pass.
4. Attach any Masonry shell to the already-tested editor driver boundary; keep shell behavior outside model/layout/render conformance.
5. Treat soft wrapping, gradients, strokes, paths, images and instance materialization as a separately versioned expanded profile; do not weaken profile-0 exactness to add them.
6. Implement the RFC 0010 package/resource experiment before changing `.nuif` writers; retain historical raw `.nuif` input as read-only alpha compatibility.
7. Calibrate PNG and OpenType parser/decoder/policy budgets independently; do not expand profile 0 by fallback.
8. Build pinned source-backed browser capture as a new adapter with secret-canary, multi-viewport and unavailable-evidence tests.
9. Freeze the reconstruction benchmark, then compare deterministic OCR/CV, one-shot, observation-assisted, hierarchical and corrective-loop baselines.
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
