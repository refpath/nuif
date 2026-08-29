# Reference editor

The editor is an executable conformance/research instrument, not the owner of the NUIF data model. Its native Masonry shell and headless driver edit NUIF through the same semantic operation API available to the CLI and automated agents.

The executable profile-zero shell provides a file menu for native document import/save, PNG export, and the repository's declared SVG, HTML/CSS and DTCG profile adapters. A foreign import is bounded before parsing, presents its fidelity summary before opening as a new unsaved NUIF document, and leaves the active document untouched when parsing or confirmation fails. A foreign export writes a sibling `.report.json` fidelity record. The shell also provides page creation, layer and component browsing, identity-backed canvas selection, frame/rectangle/ellipse/path/text insertion, subtree duplication and deletion, undo/redo, evaluation-width presets, zoom, panel visibility and a command palette. The canvas opens with a document-aligned background grid, pixel rulers and explicit `px` measurement labels; grid and rulers can be toggled independently. Its inspector authors names, positions, sizing intents, stack/flex layout, gaps, four-edge padding, alignment, solid fills and pinned-font text. Multi-field Apply is one atomic transaction.

Run `cargo run --locked -p nuif-editor` to open the native editor. The same binary accepts `--headless --script <jsonl>` with either direct `command` records or entity-bound `action` records. Its accessibility surface supports selection plus name, size intent, position, layout spacing, solid fill and text edits; undo/redo patches are logged too, and every run replays the complete mutation log from the opening document and requires the final hashes to match. Document reads share the 16 MiB profile limit; scripts are capped at 8 MiB, 100,000 commands and 64 KiB per line before JSON parsing.

`cargo xtask editor-gui-trial` drives the real AccessKit nodes of the native shell without pixel coordinates, renders the complete 1280×800 Masonry tree through the CPU harness twice, and requires identical shell and document raster hashes. It emits the screenshot, canonical output, semantic-node inventory and machine-readable report under `target/editor-gui-trial/`. The full shell is specified in `UI-SPEC.md`; a Svelte 5 shell over WASM remains a later browser demonstration.

Snapshots reject zero dimensions, an edge above 4,096 pixels or more than
16,777,216 pixels before layout or raster allocation. Accessibility and
inspector numeric inputs reject non-finite values for fixed, percentage and
fit-content sizes, positions, spacing and text metrics. `cargo xtask
editor-hostile-inputs` checks those boundaries together with missing semantic
nodes, atomic multi-operation failure, empty history, redo invalidation and
complete operation-log replay; it writes
`target/editor-hostile-input-report.json`.

The draft `UI-SPEC.md` is broader than executable profile zero. Multi-selection, drag reparenting/reordering, direct manipulation handles, snapping/guides, in-editor token editing, component authoring, advanced paint/effects, arbitrary foreign formats and non-PNG rendering export remain gated on corresponding model, protocol, layout, adapter or renderer profiles. The shell does not present inert controls for those features.

Use `cargo xtask editor-package` to build and verify the native package for the host platform, or `cargo xtask editor-launch` to package and open it. macOS produces `NUIF Editor.app`, Windows produces a GUI-subsystem executable, and Linux produces a relocatable desktop application directory. Version tags produce five GitHub prerelease archives with checksums and provenance attestations. See `PACKAGING.md` and `docs/VERSIONING.md` for paths, tag rules and unsigned-alpha limitations.
