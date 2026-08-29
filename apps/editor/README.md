# Reference editor

The editor is an executable conformance/research instrument, not the owner of the NUIF data model. Its implemented headless driver edits NUIF through the same semantic operation API available to the CLI and automated agents; the Masonry GUI shell remains pending.

Initial scope: surfaces/infinite canvas, selection/layers, shapes and vectors, text, responsive containers/layout, components/instances/variants, tokens/themes, viewport projections, fidelity diagnostics, import/export inspection, deterministic snapshots, and operation replay.

The headless binary accepts `--headless --script <jsonl>` with either direct `command` records or entity-bound `action` records. Its accessibility surface supports selection plus name, width and height edits; undo/redo patches are logged too, and every run replays the complete mutation log from the opening document and requires the final hashes to match. The full shell is specified in `UI-SPEC.md`; its planned stack is Rust-native Masonry, Vello and AccessKit under ADR 0006. A Svelte 5 shell over WASM remains a later browser demonstration.
