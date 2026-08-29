# Reference editor

The editor is an executable conformance/research instrument, not the owner of the NUIF data model. Its native Masonry shell and headless driver edit NUIF through the same semantic operation API available to the CLI and automated agents.

Initial scope: surfaces/infinite canvas, selection/layers, shapes and vectors, text, responsive containers/layout, components/instances/variants, tokens/themes, viewport projections, fidelity diagnostics, import/export inspection, deterministic snapshots, and operation replay.

Run `cargo run --locked -p nuif-editor` to open the native editor. The same binary accepts `--headless --script <jsonl>` with either direct `command` records or entity-bound `action` records. Its accessibility surface supports selection plus name, width and height edits; undo/redo patches are logged too, and every run replays the complete mutation log from the opening document and requires the final hashes to match. Document reads share the 16 MiB profile limit; scripts are capped at 8 MiB, 100,000 commands and 64 KiB per line before JSON parsing.

`cargo xtask editor-gui-trial` drives the real AccessKit nodes of the native shell without pixel coordinates, renders the complete 1280×800 Masonry tree through the CPU harness twice, and requires identical shell and document raster hashes. It emits the screenshot, canonical output, semantic-node inventory and machine-readable report under `target/editor-gui-trial/`. The full shell is specified in `UI-SPEC.md`; a Svelte 5 shell over WASM remains a later browser demonstration.

Use `cargo xtask editor-package` to build and verify the native package for the host platform, or `cargo xtask editor-launch` to package and open it. macOS produces `NUIF Editor.app`, Windows produces a GUI-subsystem executable, and Linux produces a relocatable desktop application directory. See `PACKAGING.md` for exact paths and unsigned-development-build limitations.
