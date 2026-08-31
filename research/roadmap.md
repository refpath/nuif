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

Web accessibility projection: `nuif-web-accessibility-0` lowers a bounded
ten-role, Boolean-state and five-relationship subset to inert native HTML/ARIA.
The exact Playwright 1.62.1 Chromium, Firefox and WebKit engines expose all eleven
fixture entities with matching computed role/name/state and identical bounded
ARIA snapshots on the recorded macOS/arm64 run. Host versions and differences
remain separate from semantic loss; native APIs and interaction behavior are
not claimed.

Behavior portability sidecar: `nuif-behavior-state-machine-0` runs one bounded
stable-identity program through independent Rust and Node interpreters. Full
and required-only capability traces agree over five events; required capability
absence rejects before execution and optional effects follow a recorded no-op.
`nuif-behavior-package-resource-0` now binds the same canonical-CBOR program to
the delivered document through one inert content-addressed package resource and
an independently inspected deterministic ZIP. It remains outside the canonical
semantic `Document` and excludes timers, internal events, numeric computation
and host UI execution.

Web behavior projection: `nuif-web-behavior-0` composes the bounded sidecar and
accessibility projection into enabled native-button activation, HTML `hidden`
visibility and one polite status announcement. A delimiter-safe generated
runtime is admitted by its exact CSP SHA-256 hash. Separate pointer and
Enter/Space keyboard sequences match the Rust reference transition/state/effect
sequence in pinned Playwright Chromium,
Firefox and WebKit; focus, control-state mutation, assistive-technology speech,
native UI and arbitrary authored scripts remain outside the profile.

Gate G: bounded mechanically independent reproduction. The standard-library-only Python implementation has no Rust/NUIF package dependency and exactly reproduces v0 canonical text, opaque preservation, 24 boxes, three decoded RGBA buffers and five fidelity records. External authorship and a general-purpose second implementation remain standards-publication work.

Gate H: bounded metadata-free collaboration checkpoint. Two algorithmically distinct in-repository materializers converge for every delivery of a conflict-bearing property-register history; conflicts remain explicit and canonical NUIF contains no replica state. The gate now also covers bounded concurrent creation, nested and arbitrary created anchors, mixed property/structure materialization, complete-history compaction and register-only causal prefix collection with a resumable checkpoint base.

Publication infrastructure: `docs/catalog.json` selects canonical Markdown
documents without copying their bodies. The bounded `xtask` compiler validates
metadata and repository links, generates navigation and status indexes, builds
a searchable mdBook site and composes a 13-module working manuscript. The Pages
workflow retains pull-request artifacts and restricts deployment permission to
its deployment job. This infrastructure publishes evidence; it does not
promote evidence status.

## Current falsifiers

The active codec decision gate measures canonical text and deterministic CBOR
at 8, 64, 512 and 4,096 entities after exact semantic, canonical and opaque-edit
preflight. It records native partial-load support separately from full decode
followed by selection. Protobuf and FlatBuffers are not admitted because their
documented default forms do not meet canonical and retentive-editing
requirements. Cap'n Proto is the next candidate, conditional on a complete
mapping, bounded cross-version edit trial and two canonical writers; no
schema-codec timing claim exists yet.

`nuif:experiment:v0-responsive-card`, the bounded collaboration
property-register checkpoint, existing-tree structural checkpoint and
concurrent-creation checkpoint and complete-history causal-compaction
checkpoint are complete under their declared acceptance.
The bounded nested-creation extension is also complete under its six-delivery
causal-parent acceptance.
Structural move/reorder/delete/rescue preserves one-parent/acyclic invariants
and stable sibling origins across all 5,040 deliveries, while Automerge
reproduces operation transport. The creation profile preserves base sibling
order and reports duplicate IDs explicitly across all 24 deliveries. The
compaction falsifier now covers exact-frontier complete-history collection,
and the register-only causal-prefix falsifier covers a causally closed dropped
prefix, retained suffix replay and typed refusal of unsafe concurrent pruning.
The nested-creation falsifier covers causal parent chains; its separately
versioned arbitrary-anchor extension is complete under a 24-delivery causal
sibling-chain acceptance. The next collaboration falsifiers are structural
partial garbage collection and a foreign materializer of the tree algorithm
itself.
The separately versioned mixed property/structure profile is complete under
its 24-delivery causal operation-set acceptance; creation, deletion and
multi-operation-dot boundaries remain explicit.

The package segment of `nuif:experiment:portable-package-resources` is active:
the manual writer agrees byte-for-byte with an independent ZIP writer, identity
relations and explicit resolution are exercised, and 15 hostile/one-over cases
produce `target/package-resources-report.json`. RFC 0010 remains proposed and
Gate I remains open. `cargo xtask gate-i-image` now provides a narrow
`nuif-png-rgba8-0` cross-decoder, exact-resource and repeatable CPU-render
baseline. The separate `nuif-png-basic-rgba8-1` profile covers the
non-interlaced colour/depth forms that normalize to RGBA8 without sample loss;
16-bit/interlaced/colour-managed PNG, live host/GPU affine equivalence and cross-platform
image reproduction remain excluded. `cargo xtask gate-i-font`
adds a deliberately narrow static TrueType external-oracle/package/policy baseline;
TTC, CFF, variable/color/bitmap/WOFF2 acceptance, cluster fallback and arbitrary
packaged-font shaping remain separate. Whole-text substituted/unavailable
bindings now have automated package, layout and rendering outcomes.
Package/session handoff now proves shared immutable bytes, and image scenes
deduplicate decoded surfaces under a preflighted 64 MiB total plus measured
allocation ceilings. Static-font inspection and packaged validation have their
own warmed allocation ceilings. Cross-platform writer reproduction remains an
independent requirement. Browser capture precedes
screenshot reconstruction because it provides stronger source-backed fixtures
and exposes which information is truly unavailable from pixels.

The automated capture/reconstruction contract baseline now produces
`target/capture-reconstruction-report.json`. It checks repeatable provider-input
normalization, exact browser resource retention, credential-query redaction,
honest screenshot omissions, typed proposal application, flat-copy rejection,
codec fixpoints, calibration interpolation and finite correction-loop stops.
`cargo xtask gate-j-live` separately drives Chrome for Testing 152.0.7977.64
through bounded loopback CDP. Four isolated runs retain the exact declared
response set, platform-font use, accessibility and PNG evidence; carry the
pinned runtime context into observations; reproduce the repeated 360 px
capture exactly; exclude five exercised secret canaries; and use 360/768 px
geometry to beat the one-view baseline at held-out 900 px. This closes the
local live-fixture segment. The same gate now emits a separate bounded layout
inference report: selection uses only 360/768 px training observations, retains
all row/column stack, Grid, constraint and freeform alternatives with raw
confidence and provenance, and evaluates the selected constraint only
afterward at 900 px. The observed 0.0626 versus 0.2918 normalized error is one
falsifiable fixture result, not calibrated confidence or general accuracy. The
typed confidence evaluator now has a deterministic smoke report over normal and
font-shifted holdouts, but this does not close the broader browser, screenshot,
closed-loop or calibration experiments: no cross-browser/OS capture corpus, opaque-frame
coverage, reconstruction accuracy corpus, independent evaluator or trained
artifact is claimed. Adaptation/distillation remains conditional on evidence
from that loop rather than a standing implementation commitment.

## Queue

1. Keep Gates B through H green with `cargo xtask all` and the separate nightly `cargo xtask fuzz-smoke`; reduce fuzz failures before committing them as named fixtures and retain all machine reports as CI artifacts.
2. Implement the full Cap'n Proto candidate mapping only behind the codec admission preflight; compare it after canonical-writer, old-reader retention and hostile traversal tests pass. Keep the optimized typed CBOR decoder behind identical canonical-byte and hostile-input checks; investigate a streaming canonical validator only if profiling still justifies its added parser surface.
3. Extend the bounded collaboration profiles only where a versioned causal
base can be proven: the register-only prefix profile is complete, while
structural anchor rebasing, concurrent stable-versus-retained conflicts and
frontier inference require new falsifiers. Obtain a foreign tree materializer
rather than treating the completed Automerge transport oracle as one.
4. Keep the implemented fixed/`fr`, sparse-flow, explicit-placement Grid subset
   exact; intrinsic, percentage, named, repeated, implicit, subgrid and masonry
   tracks require a separately versioned schema and foreign-oracle matrix.
5. Keep the tested Masonry shell attached only through the editor driver boundary; extend the implemented one-transaction freeform move, eight-handle freeform resize, trailing managed resize, Shift-proportional corner gesture and resolved-axis Stack/Flex reorder only when a tested semantic operation exists. Cross-parent/tree drag and Grid/Constraint reorder remain separate design work.
6. Treat soft wrapping, gradients, strokes, paths, images and instance materialization as a separately versioned expanded profile; do not weaken profile-0 exactness to add them.
7. Keep `cargo xtask gate-i-package` green across CLI/editor/package changes and add a recorded cross-platform/external writer before accepting the wire profile.
8. Extend the narrow PNG and static TrueType baselines only through new declared fixtures, add cross-platform media reproduction, and complete the broader OpenType format/policy matrix with calibrated allocation/time budgets; do not expand profile 0 by fallback.
9. Extend the passing local live-browser segment to cross-OS reproduction, opaque/cross-origin cases, matched-style/source correlation and licensed real pages before defining a portable browser-capture profile; keep WebDriver BiDi as the standards-track transport watch path.
10. Freeze the reconstruction corpus and evaluator, then compare deterministic OCR/CV, one-shot, observation-assisted, hierarchical and corrective-loop baselines through the existing typed boundary.
11. Train or distill only if the frozen evaluation demonstrates a learnable gap and rights-cleared validated traces exist.
12. Maintain the credential-free Penpot package profile under its shared ZIP resource-limit, foreign-producer and unknown-member-retention gate; defer the compact representation until upstream stability and a second fixture.
13. Run the bounded Figma profile in a named live host; run the Affinity SVG
    bridge as a retained user-mediated foreign-runtime trial; and implement the
    pure stable-API Canva current-page mapper before building its review shell.
    Retain host reports and never infer live behavior, marketplace approval or
    native NUIF support from API documentation.
14. Keep `nuif-api::NuifDocument` as the single direct SDK façade and require
    semantic-API promotion, stable errors/ownership, sanitizer-backed native
    consumers and real platform packages before declaring C, Swift or Kotlin
    binding profiles.
15. Package the conformance kit for externally authored reproduction; do not treat the in-repository Python path or Rust adapters as external interoperability evidence.
16. Keep standards-development work behind the implementer-draft and external-support gates in `docs/STANDARDS-ROADMAP.md`.

## Update policy

- Add evidence as a new record or source revision; use `supersedes` and `contradicts` rather than silently rewriting history.
- Record source commit, tag or specification revision where available.
- Promote `reviewed` to `verified` only after locator-level checks and executable evidence for implementation claims.
- Every experiment declares seed/input, oracle class, acceptance criteria, artifacts and implementation path before it can become `active`.
- Every completed experiment stores a machine report and the exact engine/toolchain/profile identity.
- Claims become specification requirements only through RFC review and executable conformance fixtures.
