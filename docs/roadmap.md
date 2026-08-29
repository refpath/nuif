# Research and implementation roadmap

The evidence gates and quantified acceptance criteria are normative for project planning in `research/AUDIT.md`; these phases describe implementation order only.

## Phase 0 — foundation (complete)
Exit: research graph schema, architectural RFCs, compilable core seams, CI and v0 falsification fixture exist.

## Phase 1 — canonical model (active)
Implement typed properties, relations, components, tokens, extensions, deterministic IDs, validation and operation replay. Exit: structural conformance suite and canonical hash stability.

## Phase 2 — layout
Implement freeform + stack model and Taffy-backed flex/grid evaluator; add context matrices and loss reports. Exit: responsive-card layout conformance at three viewports.

## Phase 3 — visual/text
Implement vector scene lowering, color/paint basics and HarfBuzz-backed shaping experiment with pinned fonts. Exit: deterministic reference snapshots for the fixture.

## Phase 4 — serialization/protocol
Canonical text + deterministic CBOR; package/assets; patch/diff/query CLI. Exit: byte-stable cycles and hostile-input limits.

## Phase 5 — editor
Headless `EditorDriver` and accessibility action contract first; Rust-native Masonry shell from ADR 0006 second; Svelte/WASM remains a later browser demonstration. Exit: the entire fixture can be authored through semantic UI actions while operation replay produces the same document.

## Phase 6 — adapters/sync
SVG first, HTML/CSS second, Penpot/Figma research adapters after the canonical model is stable. Exit: minimal source patch and explicit fidelity accounting.

## Phase 7 — collaboration
Prototype Automerge/Yjs profiles over semantic operations. Exit: deterministic canonical checkpoint after concurrent edits and surfaced semantic conflicts.

## Phase 8 — independent implementation
Publish schema/conformance kit and get a second implementation to read/write/render the v0 profile. This is the gate before claiming credible standards status.

## Early falsifiers
Stop/rethink if: semantic model requires pervasive vendor-specific exceptions; opaque extensions cannot survive common operations; source synchronization routinely requires whole-file regeneration; or independent implementation cannot reproduce normative layout/visual behavior from the spec.
