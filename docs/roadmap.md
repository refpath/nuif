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

## Phase 6 — adapters/sync
SVG first, HTML/CSS second, Penpot/Figma research adapters after the canonical model is stable. Exit: minimal source patch and explicit fidelity accounting.

## Phase 7 — collaboration
Prototype Automerge/Yjs profiles over semantic operations. Exit: deterministic canonical checkpoint after concurrent edits and surfaced semantic conflicts.

## Phase 8 — independent implementation
Publish schema/conformance kit and get a second implementation to read/write/render the v0 profile. This is the gate before claiming credible standards status.

## Early falsifiers
Stop/rethink if: semantic model requires pervasive vendor-specific exceptions; opaque extensions cannot survive common operations; source synchronization routinely requires whole-file regeneration; or independent implementation cannot reproduce normative layout/visual behavior from the spec.
