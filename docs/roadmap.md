# Research and implementation roadmap

The evidence gates and quantified acceptance criteria are normative for project planning in `research/AUDIT.md`; these phases describe implementation order only.

## Phase 0 — foundation (complete)
Exit: research graph schema, architectural RFCs, compilable core seams, CI and v0 falsification fixture exist.

## Phase 1 — canonical model (complete)
Implement typed properties, relations, components, tokens, extensions, deterministic IDs, validation and operation replay. Exit: structural conformance suite and canonical hash stability.

## Phase 2a — responsive layout falsifier (complete)
Implement freeform + stack/flex subset and a pinned NUIF/Taffy/Chrome context matrix. Exit: responsive-card layout agreement at three viewports, measured per-fixture bounds and classification of every generated divergence.

## Phase 2b — Grid schema and evaluator
Define authored track sizing, auto-flow and item placement before wiring Taffy Grid behind NUIF types. The Gate C report proves the current family-only model is insufficient and classifies its fallback as schema loss; no Grid support is claimed until those fields and foreign fixtures pass.

## Phase 3 — visual/text (complete for profile 0)
Pinned Ahem/HarfRust shaping matches HarfBuzz glyph goldens; unhinted Skrifa 0.46.2 outlines match normalized `hb-vector` goldens; hard-line layout, rectangles, ellipses, encoded-sRGB color and integer composition have normative scene/PNG baselines across macOS/aarch64, Linux/aarch64 and Linux/x86_64. Path, image, instance and extension paint remain explicit unsupported/preserved fidelity rather than hidden fallbacks. Full UAX #14 soft wrapping and expanded vector paints belong to a future profile.

## Phase 4 — serialization/protocol (complete for profile 0)
Canonical text + deterministic CBOR; package/assets; patch/diff/query CLI. Exit: byte-stable cycles and hostile-input limits.

## Phase 5 — editor (complete for the headless profile-0 instrument)
The entire v0 fixture is authored from an empty document through identity-addressed semantic actions. Direct generation, editor output and operation replay are byte-identical, and the editor writes canonical document, context, layout, scene, CPU raster and fidelity report artifacts. The Rust-native Masonry shell from ADR 0006 and the later Svelte/WASM demonstration are non-normative interface work and cannot redefine this headless result.

## Phase 5b — native editor research preview (complete for alpha.2)
The native shell exposes the semantic driver through identity-backed canvas selection, a file menu with canonical and declared adapter import/export routes, document-aligned grid and pixel rulers, layer and component browsing, insertion tools, evaluation widths, zoom, inspector transactions and source-built developer installation. `cargo xtask editor-gui-trial` and `cargo xtask editor-install-trial` exercise the semantic, visual and lifecycle boundaries. The broader `apps/editor/UI-SPEC.md` remains a draft; direct manipulation, multi-selection, snapping, token authoring and expanded paint are not claimed by this phase.

## Phase 6a — first adapters/sync falsifier (complete for bounded HTML/CSS profile 0)
`nuif-html-css-0` maps a declared container/text/finite-token subset through real DOM/CSS syntax with byte-span correspondence. Text, token and four-edge padding edits change only their six spans; comments and unmapped markup survive exactly; unsupported semantics have target/property fidelity. HTML/CSS was intentionally tested before SVG because Gate F and the architecture stop condition concern minimal source patches. This narrow profile remains independently automated even after the full-v0 follow-on; arbitrary HTML/CSS and SVG remain broader adapter work.

## Phase 6b — full-v0 HTML/CSS sync (complete)
`nuif-html-css-v0` carries the complete responsive-card model through 181 retained correspondences. The full trial applies eight local token/padding/text/responsive edits while preserving all other source bytes and opaque payloads; the editor bridge applies name and width edits through semantic actions and the public CLI, then re-imports to byte-identical canonical NUIF. Browser path rendering, instance materialization and unknown visuals remain explicit target limitations.

## Phase 6c — bounded SVG sync (complete)
`nuif-svg-0` maps a fixed surface, freeform groups, rectangles, ellipses and literal pinned-font text to SVG 2 XML. The trial applies seven identity, geometry, paint, text and accessibility edits through 45 retained correspondences, preserves unmarked XML, and rejects scripts, external resources and unsupported SVG geometry before synchronization.

## Phase 6d — bounded DTCG sync (complete)
`nuif-dtcg-scalar-0` maps flat boolean, string and number tokens to the Design Tokens Format Module 2025.10. Namespaced metadata retains NUIF document and token identity and distinguishes integer from real values; the trial applies eight edits through 21 correspondences while preserving unknown extension bytes. Groups, aliases, composite types and token-local extensions require a token-model RFC and a separate profile.

## Phase 6e — adapter inventory (complete for advertised targets)
`adapters/index.json` enumerates eleven advertised targets. The blocking adapter audit requires a primary research record, integration surface, next bounded profile and exclusion boundary for every target; executable entries additionally require a crate, profile document and routed conformance gate. Four profiles across HTML/CSS, SVG and DTCG are integrated. React, Svelte, Penpot, Figma, Adobe UXP, SwiftUI, Jetpack Compose and Flutter remain explicitly researched or externally bounded rather than carrying unsupported implementation claims.

## Phase 7a — collaboration property registers (complete)
`nuif-collab-registers-0` keeps causal metadata outside canonical documents and materializes concurrent register-like semantic operations through operation-set and replica-log algorithms. Every delivery of the three-replica trial converges, and distinct concurrent values remain explicit property conflicts.

## Phase 7b — structural collaboration (next falsifier)
Implement and verify tree move/deletion plus sibling-list semantics before enabling concurrent insert/remove/move. Exit: one-parent and acyclic invariants, explicit cycle/deletion conflicts, deterministic canonical checkpoint and reproduction through a foreign collaboration engine. Property-register convergence is not evidence for this phase.

## Phase 8a — mechanical independent reproduction (complete for v0 profile 0)
The standard-library-only Python implementation reads, writes, lays out and rasterizes the v0 profile without importing, invoking or linking the Rust packages. Its differential trial is exact at 360, 768 and 1,440 pixels and stays in the unified CI loop.

## Phase 8b — external reproduction and standards review
Package the schema/conformance kit and obtain reproduction by an externally authored implementation. External provenance, interoperability review, neutral governance and a published conformance profile remain prerequisites for credible standards status; the in-repository mechanical reproduction and source adapter do not establish them.

## Phase 9a — canonical research publication (complete)
`cargo xtask docs-check` compiles the repository Markdown into one machine-readable catalog. `cargo xtask docs-build` renders that catalog without a second editable documentation source. `cargo xtask docs-paper` composes the twelve canonical whitepaper modules into a working technical manuscript and a verified PDF. Pull requests build retained artifacts, while default-branch workflow runs deploy the static site through GitHub Pages. `CITATION.cff` describes the tagged alpha.2 software release; no DOI or peer-review claim is present.

## Phase 9b — implementer draft and incubation (blocked on external evidence)
Meet the implementer-draft gate in `docs/STANDARDS-ROADMAP.md`, including a general-purpose externally maintained implementation, requirement-to-test traceability, legal review of specification and patent terms and organizational supporters. Venue selection follows the resulting scope: W3C for Web and design-tool incubation, Khronos for graphics/content-tool conformance, or OASIS for a governed document protocol. Application alpha versions do not advance this phase.

## Early falsifiers
Stop/rethink if: semantic model requires pervasive vendor-specific exceptions; opaque extensions cannot survive common operations; source synchronization routinely requires whole-file regeneration; or independent implementation cannot reproduce normative layout/visual behavior from the spec.
