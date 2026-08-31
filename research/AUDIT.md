# Research audit and corrected base plan

Audit date: 2026-08-29. Inventory synchronized: 2026-08-31. Current scope: the
research index, 144 substantive source records plus the record template,
questions, coverage map, experiments, whitepaper synthesis, accepted RFCs and
ADRs, draft specification, conformance design and executable seams. The current
record states are 118 reviewed, 26 verified and 1 seed template.

## Outcome

The architectural thesis remains worth testing: stable identity, authored and resolved state, typed operations, explicit fidelity and opaque preservation form a coherent portability model. The corpus has unusually broad prior-art coverage and the newer records generally carry precise primary-source locators. It did not, however, justify calling the plan executable or the evidence verified. Before this audit, 97 records were `reviewed`, none was `verified`, every registered experiment was only `planned`, the Rust workspace had zero tests, and every substantive CLI command returned `not_implemented`.

The research base is solid enough to continue only under the gates below. It is not evidence that the full NUIF thesis is true, and it is not a standards-readiness claim.

## Material findings

| Area | Finding | Disposition |
|---|---|---|
| Evidence state | `reviewed` and `verified` were effectively conflated in synthesis. Many early records summarize reputable sources but do not have an `Evidence` section with claim-level locators. | The corpus now states the distinction explicitly. Only source claims checked at their locator and backed by a regression may be `verified`. |
| Research graph | Topic aliases such as `wgpu`, `harfbuzz`, `automerge`, `yjs`, `cbor`, `protobuf` and `kiwi` did not resolve to record files. The validator ignored topic entries, questions and most experiment links. | Topic entries now resolve to actual records. Validator coverage is expanded; unresolved graph identifiers fail CI. |
| Canonical values | RFC 0005 collapsed integral reals into integers despite the logical model distinguishing the kinds, and incorrectly equated UTF-8 key order with CBOR encoded-key order. | RFC 0008 corrects both points. Codec regressions prove numeric-kind separation and the `"z"`/`"aa"` order counterexample. |
| Namespace grammar | `NUIF_*`, `EXT_*` and `VENDOR_probe` contradicted the lowercase identifier grammar in RFC 0005. | Extension lifecycle names are `nuif.*`, `ext.*` and collision-resistant lowercase vendor namespaces; the v0 probe is `vendor.probe`. |
| Accepted decisions versus code | RFCs 0006 and 0007 required anchors, unknown kinds and typed opaque payloads, while the code still used integer indices and byte vectors. | Stable anchors, typed explicit `Unknown` wrappers, opaque bytes, atomic apply, stale-base checks, replay and inverses are implemented. Automatic conversion of arbitrary future wire discriminants and namespace-registry authorization for `SetUnknownPayload` are not; RFC 0007 is therefore only partially implemented. |
| Oracle independence | The harness listed the reference implementation as oracle for several properties without separating self-consistency from independent correctness. | Reports identify oracle class. Codec, replay and inverse tests are metamorphic. Gate C now supplies Taffy and pinned Chrome foreign references for the declared CSS-compatible layout subset; adapters remain experimental until a foreign round trip is wired in. |
| Layout thresholds | A global `< 0.1 px` browser threshold and fixed visual thresholds were copied from prior systems without a NUIF calibration dataset. | Gate C now stores a bound per fixture: the measured Taffy/browser maximum rounded upward to 0.01 px and capped at 0.1 px. Exact foreign agreement retains a zero bound. These empirical bounds apply only to the pinned browser/platform report and are not normative across platforms. |
| CPU exactness | “CPU `f32`, tolerance 0 across operating systems” was asserted before a pinned math, font and raster pipeline existed. | Exactness is limited to the declared CPU profile 0. It pins color, coverage, composition, font, shaping, hard-line layout, outlines and grayscale masks; rectangle, ellipse and text hashes agree on the recorded macOS/aarch64, Linux/aarch64 and Linux/x86_64 matrix. Untested platforms and future visual operations are not covered. |
| Resource limits | Depth 1024 and one million nodes were listed without memory/time measurements. | RFC 0009 replaces them with measured profile-0 byte, syntax, semantic, diagnostic, allocation and time bounds. Orthogonal image and static-font profiles now publish measured ceilings; broader media, path and GPU budgets remain future-profile work. |
| Package/assets claim | The roadmap called package/assets complete although `.nuif` fixtures are bare canonical documents and profile 0 rejects images. | Phase 4 is split. RFC 0010 is proposed; package, image and font profiles require independent experiments before acceptance. |
| Capture versus inference | Static source synchronization, browser observation and screenshot reconstruction were described under one broad inference front. | RFC 0011/specification 14 define separate evidence classes, fidelity ceilings, typed-operation boundaries and planned capture/reconstruction/calibration gates. |
| Training proposal | Distillation and low-rank adaptation had no frozen evaluator, rights-cleared trace contract or untuned baseline. | Training is conditional Gate L work. Baseline, closed-loop, calibration and artifact/data governance precede any model adaptation. |
| Editor stack | ADR 0006 selects an unreleased Masonry revision with acknowledged API churn. The choice is plausible but not yet verified in this repository. | The stable boundary is the headless `EditorDriver` and accessibility action contract. Masonry is a replaceable shell client and cannot change document semantics. |
| Scope and adoption | The prior plan attempted model, layout, rendering, text, source synchronization, adapters, collaboration and a full editor before proving the hard round trip. | Work is gated by the v0 falsifier. No collaboration or broad GUI expansion precedes codec, responsive layout, opaque preservation and one minimal source patch. |

## Evidence confidence rules

- `seed`: discovered, not fully reviewed.
- `reviewed`: source identity and broad relevance checked; individual source-derived claims may still need locator verification.
- `verified`: every material source-derived claim has a primary locator, conflicts are recorded, and any implementation claim has a reproducible check or fixture.
- `superseded`: retained for history and linked to its replacement.
- `rejected`: evaluated and excluded, with the reason retained.

Confidence is not a substitute for status. A `0.99` reviewed record is not verified merely because its source is authoritative.

## Gated base research plan

### Gate A — evidence integrity

Exit only when the research validator resolves every record, claim, topic, question, experiment and artifact link; every accepted RFC has primary evidence; contradictions use `contradicts`/`supersedes`; and claims described as verified have locator-level evidence. Source-health checks are periodic and non-normative because network availability must not make conformance nondeterministic.

### Gate B — canonical model, operations and encodings (complete)

Exit metrics:

- structural validation covers identity, reachability, parent uniqueness, cycles, relation endpoints, token references, version handling, finite numbers and extension declarations;
- seeded operation replay and inverse restoration run for at least 10,000 generated patches with the failing seed recorded;
- canonical text and CBOR reach byte fixpoints; integer/real kinds, negative real zero, map order and opaque bytes have positive and negative fixtures;
- hostile inputs are rejected under measured byte, depth, node and time budgets.

### Gate C — responsive layout falsifier (complete)

Exit metrics:

- the v0 card resolves at 360, 768 and 1440 px with stable identity and declared responsive direction changes;
- a generated CSS-compatible subset is compared with a pinned browser and Taffy; every divergence is classified as schema loss, evaluator defect, target difference or implementation-defined behavior;
- tolerances are derived from the measured corpus and are stored per fixture, never as an unexplained global constant.

### Gate D — visual and text profile (complete for profile 0)

Exit metrics:

- the CPU profile defines every supported operation by value and produces repeatable fixture bytes on the CI matrix;
- fonts, Unicode data, shaping options and raster parameters are content-addressed;
- text-layout divergence and raster divergence are reported separately;
- unsupported paints/effects create fidelity records and never disappear.

### Gate E — editor/CLI parity (complete for profile 0)

Exit metrics:

- the v0 fixture can be constructed and edited through semantic editor actions without coordinate-based widget lookup;
- the editor operation log, direct API calls and CLI replay produce one canonical hash;
- snapshots include canonical document, context, layout, scene, raster and machine report;
- shell-specific screenshot failures cannot redefine model or renderer semantics.

Evidence: `conformance/fixtures/v0-responsive-card/editor-authoring.jsonl` starts from `Document::empty`, sets the document's extension declarations and three tokens, and inserts all eight entities by author identity and semantic anchors. `cargo xtask editor-trial` requires exact bytes and the same canonical hash from direct fixture generation, editor state and replayed operation log. It also validates both the neighboring-edit output and the fully authored output, then writes `target/editor-authoring-report.json` plus a snapshot directory containing `input.nuif`, context, layout, scene, CPU PNG and a fidelity report. CI runs that one entry point and uploads both artifacts. Every entity is asserted present in the accessibility tree; no pointer coordinate is used.

### Gate F — one real source synchronization path (complete; full-v0 follow-on complete)

Exit metrics:

- HTML/CSS is imported/exported for a declared representable subset;
- a text, token and padding edit changes only mapped source spans plus declared formatter effects;
- comments and unmapped source regions survive;
- every mismatch has an entity/property-level fidelity entry.

Evidence: `nuif-html-css-0` uses pinned Tree-sitter HTML and CSS grammars, records 25 scalar correspondences and exactly re-imports its container/text/finite-token profile. `cargo xtask gate-f` changes one token value, four padding edges and escaped text, then asserts that exactly those six spans changed and every other source byte remained identical. Inserted HTML/CSS comments and an unmapped `<aside>` survive. A second synchronization produces identical source and edit records. Unsupported fill, stale-span and one-over-size trials return their named typed failures; unsupported changes include entity identity and JSON pointer. This independently retains the original narrow Gate F boundary.

The follow-on `nuif-html-css-v0` profile carries the complete eight-entity responsive-card model through 181 source correspondences. `cargo xtask gate-f-v0` changes a token, four padding edges, escaped text and one responsive rule through exactly eight spans; requires exact re-import, repeat-identical output, byte identity outside those spans and exact unknown-payload survival; and rejects unsupported tokens, structural edits, stale spans, inconsistent derived media CSS and one-over input. Its editor bridge then changes card name and width through semantic headless actions, applies exactly two source edits through the CLI, re-imports through the CLI and requires canonical NUIF byte identity with the editor output. Path and instance identities are losslessly stored but their missing browser geometry/materialization remains `unsupported`; unknown kinds and extensions are `preserved_unrenderable`. This completes `nuif:experiment:v0-responsive-card` under its declared model/source acceptance, not arbitrary HTML/CSS or browser visual equivalence.

### Gate G — independent reproduction (complete for v0 profile 0)

Exit only when a second implementation, built from the specification and fixtures rather than reference-package calls, parses, writes, lays out and renders the v0 profile to its declared tolerances.

Evidence: `implementations/python/nuif_profile0.py` uses only the Python standard library and does not import, invoke or link a Rust/NUIF package. `cargo xtask gate-g` gives the implementations the same canonical fixture, generates reference observations at 360 × 640, 768 × 768 and 1,440 × 900, and requires the independent path to reproduce canonical text bytes, unknown opaque payload preservation after a neighbouring edit, all eight boxes, decoded RGBA and five fidelity records exactly. The report contains three matching RGBA SHA-256 values and zero layout delta in every context. Duplicate-key and deliberately corrupted layout/raster trials prove the negative path. This closes the mechanical Gate G metric; it is not external authorship, a general-purpose second implementation, neutral governance or standards publication.

### Gate H — metadata-free collaboration checkpoints (complete for bounded register and existing-tree profiles)

Exit metrics:

- replica clocks, causal context and conflict candidates remain outside canonical NUIF documents;
- independently structured operation-set and replica-log materializers converge for every delivery of the same valid change set;
- concurrent semantic property conflicts remain explicit and property-attributed;
- incomplete causal history, identifier reuse, unsupported profile expansion and invalid materialization fail closed;
- existing-tree move/delete delivery preserves one-parent/acyclic structure and explicit structural conflicts;
- a pinned foreign engine convergently transports the exact structural operation set without being treated as the tree oracle.

Evidence: `nuif-collab-registers-0` maps register-like NUIF semantic operations to causal multi-value registers. One implementation computes pairwise maximal changes from an operation set; the other incrementally maintains causal frontiers in per-replica logs. `cargo xtask gate-h` exhausts all 5,040 deliveries of a seven-change/three-replica history, compares both materializers and multiple merge orders, repeats duplicate delivery, requires two explicit property conflicts and proves canonical NUIF text contains no replica/context/conflict metadata.

`nuif-collab-tree-0` separately handles existing-identity moves, reorders and trash deletion with unique Lamport order, cycle rejection and RGA-style stable sibling origins. The gate exhausts all 5,040 deliveries of a seven-replica conflict/stable-anchor fixture, checks two materializers, join/idempotence, every required structural conflict class and 4,096 moves across 4,097 entities. Pinned `@automerge/automerge` 3.4.1 reproduces the exact immutable operation set across merge orders and save/load. That foreign result proves transport, not independent tree semantics. Concurrent creation, causal-stability garbage collection, combined structural/property transactions and an externally authored tree materializer remain open.

### Gate I — portable package and resources (container and narrow media segments active)

Exit only when:

- two independently implemented writers produce identical `nuif-package-0`
  bytes from one normative document/resource fixture;
- document, asset, resource and package identity changes obey RFC 0010 fixtures;
- package read/write reaches a byte fixpoint and no implicit resource fetch occurs;
- duplicate, traversal, symlink, directory, encryption, split, compression,
  missing/extra member, size and digest failures are atomic and typed;
- archive/member/descriptor/image/font boundary and one-over cases pass measured
  time/allocation limits;
- PNG interpretation and font policy/shaping inputs reproduce through
  independent implementations for their declared subsets.

Current evidence: stable `AssetId`/`ResourceDigest` semantics, deterministic
stored ZIP packages, exact manual/independent-writer bytes, package fixpoint,
separate document/resource/package identities, explicit resolver authority and
15 hostile/archive/one-over cases run through `cargo xtask gate-i-package`.
The CLI and editor write real packages and preserve embedded resources. The
`nuif-png-rgba8-0` segment additionally agrees across `png` and `zune-png` on
12 filter/colour-marker fixtures. The separate
`nuif-png-basic-rgba8-1` profile adds thirteen fixtures spanning every admitted
greyscale/indexed/RGB/greyscale-alpha/RGBA type and transparency form. Together
they retain exact encoded bytes, repeat resource-aware CPU rasterization and
reject 20 unsupported/hostile cases via `cargo xtask gate-i-image`. Gate I does
not yet pass: 16-bit/interlaced/colour-managed PNG, live host/GPU affine equivalence,
cross-platform image reproduction, and a cross-platform/external writer remain required. The separate
`nuif-opentype-static-single-0` segment compares exact Ahem metrics, family,
tables and Unicode coverage between Skrifa and a pinned HarfBuzz metadata
capture, accepts four static TrueType fixtures, preserves the font through
package fixpoint, requires explicit license/review evidence and rejects 20
synthetic/real malformed or out-of-profile cases plus 10 policy cases through
`cargo xtask gate-i-font`.
Six additional trials distinguish package-level portable/private/linked/
substituted/unavailable outcomes. TTC, CFF/CFF2, variable/color/bitmap/WOFF2
acceptance, cluster-level fallback, arbitrary packaged-font shaping and
cross-platform font reproduction remain required. Six item-level trials now
separate requested, substituted and unavailable text/font identities through
layout and rendering. Four accepted-font
inspections and packaged validation now carry warmed 4 MiB allocated/2 MiB
retained regression ceilings. Package-to-session handoff shares an 8 MiB buffer
under a 1 MiB allocation ceiling, and 1,024 image instances retain one 1 MiB
surface under the 64 MiB preflighted scene total. Both media segments are separate
from CPU render profile 0 and do not establish general images or packaged-font
rendering.

### Gate J — source-backed browser capture (local live segment automated)

Exit only when:

- a pinned browser/protocol/OS/context produces repeat-equivalent normalized DOM,
  layout, style, resource, font-use, accessibility and screenshot observations;
- downloaded source-resource bodies retain exact size/digest identity;
- multiple input viewports predict a held-out responsive context better than a
  one-screenshot/freeform baseline for the declared subset;
- cross-origin, local-font, canvas, video, worklet and behavior gaps are explicit;
- cookies, authorization, credentials, storage and secret canaries never enter
  exported evidence;
- captured scripts/resources remain inert in every package reader.

This gate creates a new runtime adapter; it does not enlarge the existing
Tree-sitter source-synchronization profile by implication.

Current evidence: `cargo xtask capture-baselines` repeats fixed browser-provider
input through `nuif-capture`, requires identical normalized output/package
bytes, exact image-resource digest and body retention, absence of a query-token
canary from observations/proposals/packages, typed proposal application and
cyclic-parent rejection. `cargo xtask gate-j-live` then launches exact Chrome
for Testing 152.0.7977.64 through bounded loopback CDP, accepting four declared
contexts with at most three recorded fresh-profile attempts each. It records
browser/protocol/OS/viewport/locale/timezone/media/motion/settling/freeze
context, exactly retains the declared HTML/CSS/PNG/font/probe bodies, observes
actual custom-font and accessibility results, reproduces the repeated 360 px
capture/normalization/screenshot bytes, and excludes five query, cookie,
storage, authorization and header canaries after proving they were exercised.
Geometry fitted to 360/768 px beats copying the 360 px freeform geometry at the
held-out 900 px fixture. This automates the local live segment. Gate J remains
open for cross-OS/browser reproduction, opaque/cross-origin behavior,
matched-style/source correspondence, canvas/video bounded frames and licensed
real-page evidence.

### Gate K — screenshot reconstruction and calibrated abstention (contract baseline active)

Exit only when:

- deterministic OCR/CV, one-shot, observation-assisted, hierarchical-crop,
  multi-context and corrective-loop baselines run through one frozen harness;
- every outcome is a validated document/transaction or explicit no-result;
- source-backed and screenshot-only cases remain separate evidence classes;
- reports include text, elements, tree, properties, geometry, resources,
  held-out contexts, provenance/fidelity, accessibility, visual diagnostics,
  confidence, latency, RAM/VRAM, iterations and cost;
- editable reconstruction rejects a flat screenshot cover as success;
- confidence is calibrated per decision type on disjoint data and review/abstain
  thresholds reproduce their declared risk/coverage;
- an independent evaluator reproduces the principal held-out result and one
  real editing task benefits from the reconstructed semantics.

No editor prerelease or visually selected demo can substitute for this gate.

Current evidence: the same automated report repeats strict fixed-PNG analysis,
round-trips observation bytes, distinguishes observed pixels from inference,
records four unavailable evidence categories, applies typed proposals, rejects
screenshot-derived flat-copy assets by default and exercises improved,
repeated-state, no-proposal, provider-call and memory-budget loop stops. A two-
point interpolation/selective-review fixture verifies the calibration API. No
OCR engine is evaluated, no real or held-out reconstruction corpus is scored,
the required metric families are incomplete and no independent evaluator has
reproduced a result; Gate K remains open.

### Gate L — conditional adaptation and distillation (blocked on Gate K)

This gate is skipped unless Gate K reveals repeatable learnable errors and a
rights-cleared validated trace corpus exists. If opened, exit only when:

- the dataset has digest-pinned lineage, consent/rights/privacy/retention policy,
  leak-resistant splits and a datasheet;
- every base model, processor, task adapter and run has a digest-pinned manifest
  and model card;
- prompt/tool, retrieval, supervised, LoRA, QLoRA where architecture-compatible
  and sequence-distillation candidates are compared on identical frozen data,
  budgets and evaluator versions;
- the selected candidate improves predeclared quality or efficiency without an
  unacceptable validity, calibration, privacy, license or maintenance regression;
- rollback to the untuned baseline remains possible.

LoRA/QLoRA/distillation are methods tested inside the gate, not predetermined
architecture or evidence that the gate should exist.

## Thesis stop conditions

Stop or narrow the architecture if the v0 source patch routinely becomes whole-file regeneration, an ignorant implementation cannot preserve opaque bytes during neighboring edits, operation convergence requires collaboration metadata in canonical documents, tolerance tiers hide systematic semantic divergence, or a second implementation cannot reproduce the profile without reading reference code. Narrow the proposed resource/reconstruction path if independent package writers cannot agree, browser capture cannot exclude secrets reproducibly, correction loops improve pixels by deleting semantics, confidence cannot achieve useful risk/coverage, or tuned systems fail to beat the untuned tool-assisted baseline fairly.

## Executable baseline after this audit

The repository now has a typed canonical model, structural validator, anchored atomic operations with stale-base rejection and replay/inversion, canonical text and deterministic CBOR codecs, responsive profile-0 layout, deterministic CPU rasterization, a seeded trial/ddmin/report library, validity-preserving subtree/scalar and choice-stream reducers, atomic regression-fixture emission, an executable conformance package, a multi-command CLI and a headless editor accessibility driver with complete mutation-log replay. The Gate B long run passes 10,000 generated patches (160,000 operations), checking replay, inverse and both encodings on every patch and sampling layout/raster checks every 100 patches. The hostile-input run measures byte, syntax-depth, semantic-cardinality, elapsed-time and allocator boundaries, records its platform, and rejects every one-over case. `cargo xtask reduction-profile` reduces the responsive fixture to the valid three-entity trigger path, records candidate counts and transformations, emits a canonical fixture and confirms non-overwrite behavior; failed seeded trials with report paths use this same mechanism with their recorded viewport and snapshot decision.

The active codec decision gate adds a separate four-scale release benchmark for
size, encode, decode, canonicalize, allocation and decode-then-select behavior.
Both implemented codecs must first pass semantic, canonical and opaque-data
preservation through a neighboring edit. The first Apple M5 Pro run places CBOR
near 41% of text size at 4,096 entities but shows the current generic CBOR
decoder slower than canonical text. Protobuf and FlatBuffers remain outside the
timing table because no complete NUIF mapping satisfies canonical and retentive
editing requirements; Cap'n Proto is the next conditional candidate.

Gate C pins Taffy 0.14.0 and Chrome for Testing 152.0.7977.64 and runs a deterministic three-way layout report over 27 cases, 81 comparisons and 1,536 box components. The v0 card agrees exactly across NUIF, Taffy and Chrome at 360, 768 and 1,440 px. Eight bounded-Grid cases exercise fixed/`fr` tracks, sparse row/column flow, explicit placement and spans in addition to generated stack/flex cases. All cases pass with zero classified, blocking or unexplained divergence; 26 fixtures have exact Taffy/browser agreement and one fractional Grid fixture uses its measured 0.02 px bound. The foreign oracles exposed both the earlier definite-size stretch defect and a Grid `fill` lowering defect, and both remain regression-covered. Gate C now claims the bounded explicit-Grid profile, not intrinsic, percentage, named, repeated, implicit, subgrid or masonry tracks.

Gate D is complete for the deliberately narrow CPU profile 0. It pins the 22,572-byte Ahem 1.50 font by SHA-256, HarfRust 0.13.3, Unicode 17.0.0, unhinted Skrifa 0.46.2 outlines and Zeno 0.3.3 grayscale masks. Eight ASCII/Unicode LTR/RTL runs match HarfBuzz 14.4.0 and five signed-26.6 paths match normalized `hb-vector`. Hard breaks, line-height placement, intrinsic shaped width, inline-start alignment and no automatic soft wrapping are executable lossless semantics. Encoded-sRGB solid rectangles, four-cubic ellipses and integer source-over composition are defined by value and have exact scene/raw-RGBA fixtures. PNG hashes are non-blocking encoder diagnostics. The text and paint reports reproduce on macOS/aarch64, Linux/aarch64 and Linux/x86_64. Paths, images, component instances and extension-defined visuals are not misrepresented as supported: their fidelity records retain document/entity identity and property pointers. Expanded render profiles remain future work.

Gate E is complete for the headless profile-0 editor instrument. Twelve semantic document/token/entity actions construct the full fixture from empty state. Direct generation, editor output and protocol replay converge to exact canonical text bytes and hash `nuif-cbor-0:sha256:540363fe916a3a1926fecbcbd27fd0280666e3cbbb115e561d38f3b7f322a3d6`. The 768×640 snapshot contains eight layout boxes, three supported render commands, five explicit fidelity entries, an exact CPU PNG and hashes for both RGBA and PNG bytes. `cargo xtask editor-trial` runs the old neighboring-edit/undo/redo trial and this complete-authoring trial, validates both outputs, emits the machine report and snapshot bundle, and is the CI entry point. The native research-preview shell is also exercised through AccessKit and deterministic CPU screenshots; it remains a client and cannot alter model, layout or renderer conformance. Expanded UI profiles and external reproduction remain open.

Gate F remains complete for `nuif-html-css-0`, a deliberately bounded two-entity source profile: 25 lossless correspondences and six local edits retain every unmapped byte. Its full-v0 follow-on is also complete for model preservation. `nuif-html-css-v0` maps DOM containment, every responsive-card field, real size/layout/fill CSS, responsive media CSS and opaque metadata; its 181-correspondence trial and two-edit editor/CLI bridge both re-import exactly. Tree-sitter validates HTML and CSS under a 1 MiB bound, derived CSS drift fails closed, and target visual limitations are never described as lossless browser behavior. Arbitrary HTML/CSS, collaboration profiles and expanded path/instance rendering remain open.

Gate G is complete for the bounded v0 profile. The Python implementation independently validates and canonicalizes the full fixture, preserves the opaque `vendor.probe` payload across an unrelated edit, implements the profile-0 layout algorithm and rasterizes the fixture's solid rectangles and pinned Ahem text. All 24 context/entity boxes and all three decoded RGBA buffers match exactly, with the fidelity list also byte-for-value equivalent. Its unsupported visual scope remains explicit, and an external implementation/reviewer is still required before a standards-readiness claim.

Gate H is complete for property registers and the bounded existing-tree structural profile. The property operation-set and replica-log materializers converge to hash `nuif-cbor-0:sha256:29f24d0cb9613b7a6adaf1f57760031d12271c0eb06084e3807115ef869941ab` across all 5,040 deliveries and tested merge orders. Concurrent values remain explicit, causal overwrites select only maximal values and the opaque entity stays exact. The structural operation-set and replica-log materializers separately converge over every delivery of move/reorder/delete/rescue conflicts while preserving one parent, acyclicity and stable sibling origins; a 4,096-change scale trial is bounded, and Automerge reproduces immutable operation transport. Checkpoints contain no collaboration metadata. Concurrent creation, causal garbage collection, combined property/structure transactions and a foreign tree materializer remain required before a general collaboration-profile claim.

RFCs 0010 and 0011 plus specification 14 remain research-aligned proposals.
The executable baseline now includes the deterministic package writer,
asset/resource model, bounded provider-input browser/screenshot contracts and
the pinned local live-browser segment described above. It still has no general
image/font resource profile, cross-browser/OS capture corpus, reconstruction
accuracy corpus, independent reconstruction evaluator or trained artifact. The
editor version `0.1.0-alpha.3` identifies the developer application and must not
be cited as maturity evidence for those open proposals.
