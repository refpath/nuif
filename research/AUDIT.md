# Research audit and corrected base plan

Audit date: 2026-08-29. Scope: the research index, 99 source records, questions, coverage map, experiments, whitepaper synthesis, accepted RFCs/ADRs, draft specification, conformance design and executable seams.

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
| CPU exactness | “CPU `f32`, tolerance 0 across operating systems” was asserted before a pinned math, font and raster pipeline existed. | Exactness is limited to the current integer-composited profile-0 raster path. Text shaping and outlines have pinned Ahem/HarfRust/Unicode/Skrifa inputs plus exact HarfBuzz goldens. Pinned Zeno grayscale scene and PNG hashes agree on the recorded macOS/aarch64, Linux/aarch64 and Linux/x86_64 matrix; missing line breaking is still approximated. |
| Resource limits | Depth 1024 and one million nodes were listed without memory/time measurements. | RFC 0009 replaces them with measured profile-0 byte, syntax, semantic, diagnostic, allocation and time bounds. Image/font/path/GPU budgets remain future-profile work. |
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

### Gate D — visual and text profile

Exit metrics:

- the CPU profile defines every supported operation by value and produces repeatable fixture bytes on the CI matrix;
- fonts, Unicode data, shaping options and raster parameters are content-addressed;
- text-layout divergence and raster divergence are reported separately;
- unsupported paints/effects create fidelity records and never disappear.

### Gate E — editor/CLI parity

Exit metrics:

- the v0 fixture can be constructed and edited through semantic editor actions without coordinate-based widget lookup;
- the editor operation log, direct API calls and CLI replay produce one canonical hash;
- snapshots include canonical document, context, layout, scene, raster and machine report;
- shell-specific screenshot failures cannot redefine model or renderer semantics.

### Gate F — one real source synchronization path

Exit metrics:

- HTML/CSS is imported/exported for a declared representable subset;
- a text, token and padding edit changes only mapped source spans plus declared formatter effects;
- comments and unmapped source regions survive;
- every mismatch has an entity/property-level fidelity entry.

### Gate G — independent reproduction

Exit only when a second implementation, built from the specification and fixtures rather than reference-code calls, parses, writes, lays out and renders the v0 profile to its declared tolerances. This remains the standardization gate.

## Thesis stop conditions

Stop or narrow the architecture if the v0 source patch routinely becomes whole-file regeneration, an ignorant implementation cannot preserve opaque bytes during neighboring edits, operation convergence requires collaboration metadata in canonical documents, tolerance tiers hide systematic semantic divergence, or a second implementation cannot reproduce the profile without reading reference code.

## Executable baseline after this audit

The repository now has a typed canonical model, structural validator, anchored atomic operations with stale-base rejection and replay/inversion, canonical text and deterministic CBOR codecs, responsive profile-0 layout, deterministic CPU rasterization, a seeded trial/ddmin/report library, an executable conformance package, a multi-command CLI and a headless editor accessibility driver with complete mutation-log replay. The Gate B long run passes 10,000 generated patches (160,000 operations), checking replay, inverse and both encodings on every patch and sampling layout/raster checks every 100 patches. The hostile-input run measures byte, syntax-depth, semantic-cardinality, elapsed-time and allocator boundaries, records its platform, and rejects every one-over case.

Gate C pins Taffy 0.14.0 and Chrome for Testing 152.0.7977.64 and runs a deterministic three-way layout report over 15 cases, 45 comparisons and 1,008 box components. The v0 card agrees exactly across NUIF, Taffy and Chrome at 360, 768 and 1,440 px after the foreign oracles exposed and drove a correction to definite-size stretch handling. Generated stack/flex cases agree within their fixture-local measured bounds. All 38 remaining pairwise divergences are classified as the same grid-schema loss; there are no blocking or unclassified divergences. Gate C is complete under its declared exit metric, but this does not implement Grid: profile 0 still lacks authored track and placement fields, and the report preserves that gap instead of hiding it behind tolerance.

Gate D's shaping, outline and pinned raster layers are executable but Gate D is not complete. Profile 0 pins the 22,572-byte Ahem 1.50 font by SHA-256, HarfRust 0.13.3 and Unicode 17.0.0; its resolved scene carries glyph IDs, Unicode-scalar clusters, advances, offsets and deduplicated unhinted outlines. Eight ASCII/Unicode LTR/RTL cases match independently captured HarfBuzz 14.4.0 strings exactly. Five Skrifa 0.46.2 signed-26.6 outline paths match normalized `hb-vector` output. Pinned Zeno 0.3.3 grayscale scene and PNG hashes reproduce byte-for-byte at three contexts on macOS/aarch64, Linux/aarch64 and Linux/x86_64. The report still classifies text semantics as approximated because line breaking/wrapping is absent. Broader paints/effects also remain open, as do full editor authoring and GUI shell work, HTML/CSS retentive synchronization, collaboration profiles and independent reproduction.
