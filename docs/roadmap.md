# Research and implementation roadmap

The evidence gates and quantified acceptance criteria are normative for project planning in `research/AUDIT.md`; these phases describe implementation order only.

## Phase 0 — foundation (complete)
Exit: research graph schema, architectural RFCs, compilable core seams, CI and v0 falsification fixture exist.

## Phase 1 — canonical model (complete)
Implement typed properties, relations, components, tokens, extensions, deterministic IDs, validation and operation replay. Exit: structural conformance suite and canonical hash stability.

## Phase 2a — responsive layout falsifier (complete)
Implement freeform + stack/flex subset and a pinned NUIF/Taffy/Chrome context matrix. Exit: responsive-card layout agreement at three viewports, measured per-fixture bounds and classification of every generated divergence.

## Phase 2b — bounded explicit Grid (complete)
Profile 0 now defines positive fixed/`fr` tracks, sparse row/column auto-flow,
zero-based explicit placement, positive spans, no implicit tracks and bounded
resource use. The independent NUIF evaluator implements those rules directly;
Taffy and CSS are lowering targets, not hidden runtime dependencies. Gate C
exercises simple, explicit and spanning Grid cases and passes with no classified,
blocking or unexplained divergence. Intrinsic, percentage, named, repeated,
subgrid, masonry and implicit CSS tracks remain outside this bounded profile.

## Phase 3 — visual/text (complete for profile 0)
Pinned Ahem/HarfRust shaping matches HarfBuzz glyph goldens; unhinted Skrifa 0.46.2 outlines match normalized `hb-vector` goldens; hard-line layout, rectangles, ellipses, encoded-sRGB color and integer composition have normative scene/raw-RGBA baselines across macOS/aarch64, Linux/aarch64 and Linux/x86_64. PNG hashes remain deterministic artifact diagnostics but are not pixel-conformance boundaries. Path, image, instance and extension paint remain explicit unsupported/preserved fidelity rather than hidden fallbacks. Full UAX #14 soft wrapping and expanded vector paints belong to a future profile.

## Phase 4a — bare serialization/protocol (complete for profile 0)
Canonical text + deterministic CBOR plus patch/diff/query CLI. Exit: byte-stable
cycles and measured hostile-input limits. This phase does not include the
portable `.nuif` package, images or general font resources.

## Phase 4b — portable package and resources (active; container segment implemented)
RFC 0010 now has a package layer above `nuif-codec`, stable assets in the core,
explicit verified resource resolution and package-preserving CLI/editor I/O.
The manual writer and an independent ZIP writer produce identical bytes;
semantic/resource/package hashes obey distinct fixtures; hostile archives and
package/resource/count one-over cases are blocking through
`cargo xtask gate-i-package`. Existing raw `.nuif` inputs migrate read-only and
new bare forms use `.nuif.json`/`.nuif.cbor`. The executable
`nuif-png-rgba8-0` baseline independently decodes a deliberately narrow PNG
subset. The separately named `nuif-png-basic-rgba8-1` expansion now covers
non-interlaced lossless-to-RGBA8 greyscale, indexed, RGB, greyscale-alpha and
RGBA forms plus valid transparency; both retain encoded bytes and repeat
package-aware CPU image rendering through `cargo xtask gate-i-image`. Gate I
remains open for 16-bit/interlaced/colour-managed PNG and live host/GPU affine
equivalence. Package/session handoff and decoded image surfaces now have
measured sharing, total-byte and allocation ceilings. Static-font inspection
and packaged validation now have warmed allocation ceilings across every
accepted fixture. A Linux/Windows/macOS resource-gate matrix is configured;
successful hosted artifacts are still required before a cross-platform
reproduction claim. The
separate `nuif-opentype-static-single-0` baseline validates one exact static
TrueType face through package encoding/resolution, compares Skrifa results with
a pinned HarfBuzz metadata capture and rejects malformed/policy/one-over cases through
`cargo xtask gate-i-font`. TTC, CFF/CFF2, variable/color/bitmap/WOFF2 fonts,
item-level substitution/unavailability fidelity, shaping integration,
successful hosted cross-platform evidence and external implementations remain open.

## Phase 5 — editor (complete for the headless profile-0 instrument)
The entire v0 fixture is authored from an empty document through identity-addressed semantic actions. Direct generation, editor output and operation replay are byte-identical, and the editor writes canonical document, context, layout, scene, CPU raster and fidelity report artifacts. The Rust-native Masonry shell from ADR 0006 and the later Svelte/WASM demonstration are non-normative interface work and cannot redefine this headless result.

## Phase 5b — native editor research preview (complete through alpha.3)
The native shell exposes the semantic driver through identity-backed canvas selection, a file menu with canonical and declared adapter import/export routes, document-aligned background grid and pixel rulers, layer and component browsing, insertion tools, evaluation widths, zoom, inspector transactions, bounded explicit Grid authoring and source-built developer installation. Grid track, flow, atomic item position and span edits use the same validated operations as the headless and accessibility surfaces. Open packages pass their digest-verified embedded resources through the same bounded session used by CLI render/snapshot, so the narrow RGBA8 image segment renders without implicit fetching. `cargo xtask editor-gui-trial`, `cargo xtask editor-hostile-inputs` and `cargo xtask editor-install-trial` exercise the semantic, visual, adversarial and lifecycle boundaries. The broader `apps/editor/UI-SPEC.md` remains a draft; direct manipulation, multi-selection, snapping, token authoring and expanded paint are not claimed by this phase.

## Phase 5c — browser and plug-in binding (complete for `nuif-wasm-api-0`)

The byte-oriented WebAssembly module wraps `nuif-api`, canonical text/CBOR and
semantic patches without copying the model into JavaScript. A Node/native
differential checks exact edited bytes, and the direct-browser target
initializes in pinned headless Chrome. Its JavaScript, TypeScript and WASM are
packaged as a CI and tagged-release developer artifact. The module declares no
filesystem, network or host-document authority. Browser-layout execution, a
WASI CLI, npm publication and live Figma/Adobe adapters remain separate
profiles and version streams.

## Phase 5d — external agent binding (complete for `nuif-mcp-tools-0`)

The MCP process is a stateless stdio adapter over the same API and semantic
patch layer. Its four inline-text tools carry no host authority, support only
the current 2026-07-28 lifecycle, and are differentially checked against the
native CLI through a real child process. Five native release jobs package and
attest the separately versioned binary; source installation remains available
without an application store. Live compatibility with named third-party MCP
hosts, large-document resource handles and any authenticated HTTP service are
separate trials and are not claimed by this phase.

## Phase 6a — first adapters/sync falsifier (complete for bounded HTML/CSS profile 0)
`nuif-html-css-0` maps a declared container/text/finite-token subset through real DOM/CSS syntax with byte-span correspondence. Text, token and four-edge padding edits change only their six spans; comments and unmapped markup survive exactly; unsupported semantics have target/property fidelity. HTML/CSS was intentionally tested before SVG because Gate F and the architecture stop condition concern minimal source patches. This narrow profile remains independently automated even after the full-v0 follow-on; arbitrary HTML/CSS and SVG remain broader adapter work.

## Phase 6b — full-v0 HTML/CSS sync (complete)
`nuif-html-css-v0` carries the complete responsive-card model through 181 retained correspondences. The full trial applies eight local token/padding/text/responsive edits while preserving all other source bytes and opaque payloads; the editor bridge applies name and width edits through semantic actions and the public CLI, then re-imports to byte-identical canonical NUIF. Browser path rendering, instance materialization and unknown visuals remain explicit target limitations.

## Phase 6c — bounded SVG sync (complete)
`nuif-svg-0` maps a fixed surface, freeform groups, rectangles, ellipses and literal pinned-font text to SVG 2 XML. The trial applies seven identity, geometry, paint, text and accessibility edits through 45 retained correspondences, preserves unmarked XML, and rejects scripts, external resources and unsupported SVG geometry before synchronization.

## Phase 6d — bounded DTCG sync (complete)
`nuif-dtcg-scalar-0` maps flat boolean, string and number tokens to the Design Tokens Format Module 2025.10. Namespaced metadata retains NUIF document and token identity and distinguishes integer from real values; the trial applies eight edits through 21 correspondences while preserving unknown extension bytes. Groups, aliases, composite types and token-local extensions require a token-model RFC and a separate profile.

## Phase 6e — adapter inventory (complete for advertised targets)
`adapters/index.json` enumerates eleven advertised targets. The blocking adapter audit requires a primary research record, integration surface, next bounded profile and exclusion boundary for every target; executable entries additionally require a crate, profile document and routed conformance gate. Seven profiles across HTML/CSS, SVG, DTCG, Penpot, static React JSX and static Svelte are integrated. Figma, Adobe UXP, SwiftUI, Jetpack Compose and Flutter remain explicitly researched or externally bounded rather than carrying unsupported implementation claims. Svelte uses Tree-sitter only for retained spans and exact official `svelte/compiler` 5.57.0 as its foreign parse/compile oracle.

## Phase 7a — collaboration property registers (complete)
`nuif-collab-registers-0` keeps causal metadata outside canonical documents and materializes concurrent register-like semantic operations through operation-set and replica-log algorithms. Every delivery of the three-replica trial converges, and distinct concurrent values remain explicit property conflicts.

## Phase 7b — bounded existing-tree structural collaboration (complete)
`nuif-collab-tree-0` implements move, reorder, trash deletion and later rescue for identities already present in one canonical base. Unique Lamport ordering plus cycle rejection preserves one-parent/acyclic structure; RGA-style stable origins preserve deterministic sibling order without putting clocks, tombstones or position IDs in canonical NUIF. Move/move, delete/move, deleted-parent, delete/descendant-move, cycle and anchor conflicts remain explicit. Two materializers converge over all 5,040 deliveries of a fixture that includes a causal moved-position anchor, plus a 4,096-change scale trial. Pinned Automerge 3.4.1 reproduces the exact immutable operation set under three merge orders, duplicate merge and save/load; it is a foreign transport oracle, not an independent implementation of the tree algorithm. Concurrent creation, causally stable garbage collection, combined property/structure transactions and external tree-materializer reproduction remain future profiles.

## Phase 8a — mechanical independent reproduction (complete for v0 profile 0)
The standard-library-only Python implementation reads, writes, lays out and rasterizes the v0 profile without importing, invoking or linking the Rust packages. Its differential trial is exact at 360, 768 and 1,440 pixels and stays in the unified CI loop.

## Phase 8b — external reproduction and standards review
Package the schema/conformance kit and obtain reproduction by an externally authored implementation. External provenance, interoperability review, neutral governance and a published conformance profile remain prerequisites for credible standards status; the in-repository mechanical reproduction and source adapter do not establish them.

## Phase 9a — canonical research publication (complete)
`cargo xtask docs-check` compiles the repository Markdown into one machine-readable catalog. `cargo xtask docs-build` renders that catalog without a second editable documentation source. `cargo xtask docs-paper` composes the thirteen canonical whitepaper modules into a working technical manuscript and a verified PDF. Pull requests build retained artifacts, while default-branch workflow runs deploy the static site through GitHub Pages. `CITATION.cff` describes the tagged alpha.3 software release; no DOI or peer-review claim is present.

## Phase 9b — implementer draft and incubation (blocked on external evidence)
Meet the implementer-draft gate in `docs/STANDARDS-ROADMAP.md`, including a general-purpose externally maintained implementation, requirement-to-test traceability, legal review of specification and patent terms and organizational supporters. Venue selection follows the resulting scope: W3C for Web and design-tool incubation, Khronos for graphics/content-tool conformance, or OASIS for a governed document protocol. Application alpha versions do not advance this phase.

## Phase 10 — source-backed browser capture (active contract baseline)

Create a dedicated browser-capture adapter instead of expanding the retentive
Tree-sitter adapter into a runtime. Pin browser/protocol/OS/context and collect
bounded source, DOM/layout/style, downloaded-resource, font-use, accessibility
and screenshot observations. Exit: repeated normalized observations/resource
hashes reproduce; multi-viewport evidence predicts a held-out context; canvas,
video, cross-origin and local-font gaps remain explicit; secret canaries never
enter exported evidence. `cargo xtask capture-baselines` currently proves
repeatable normalization, exact resource retention, query-secret redaction,
typed proposal application and cycle rejection from fixed provider input. It
does not yet drive a live browser, predict a held-out viewport or satisfy this
phase exit.

## Phase 11 — screenshot reconstruction baseline (active contract baseline)

Implement the vendor/model-neutral observation and typed-operation boundary from
RFC 0011/specification 14. Compare deterministic OCR/CV, one-shot proposal,
observation-assisted proposal, hierarchical crops, multi-viewport ranking and a
bounded render/difference correction loop. Exit: one harness reports validity,
text, element, tree, geometry, resources, held-out layout, provenance, visual,
confidence, latency and memory/cost; flat screenshot copies fail the editable
profile; an independent evaluator reproduces the main result.

The executable baseline currently proves observation-codec fixpoints, explicit
observed/inferred evidence and omissions, typed atomic proposals, default
flat-copy rejection and deterministic loop termination. Its report names the
missing OCR/model accuracy corpus, complete metric families and independent
evaluator; those omissions keep the phase open.

## Phase 12 — calibration and conditional adaptation (calibration primitive active; adaptation blocked on Phase 11)

Calibrate decision-level confidence and establish review/abstain risk thresholds
on disjoint data. Only if a stable learnable error distribution remains, create
rights-cleared validated operation traces and compare prompt/tool changes,
retrieval, supervised tuning, LoRA, QLoRA where compatible and sequence-level
distillation. Exit: a candidate beats the untuned closed-loop baseline under the
same frozen holdout/budget without validity, calibration, privacy, licensing or
maintenance regression. Training is skipped if that gate is not met.

The current interpolation/selective-review fixture tests only the calibration
API contract. It is not evidence of calibrated risk coverage on real data.

## Early falsifiers
Stop/rethink if: semantic model requires pervasive vendor-specific exceptions; opaque extensions cannot survive common operations; source synchronization routinely requires whole-file regeneration; independent implementation cannot reproduce normative layout/visual behavior from the spec; deterministic packages do not reproduce across writers; reconstruction optimizes pixels by discarding semantics; or tuning cannot beat the untuned tool-assisted baseline fairly.
