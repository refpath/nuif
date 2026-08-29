# Reference editor

The editor is an executable conformance/research instrument, not the owner of the NUIF data model. It will edit NUIF natively through the same semantic operation API available to the CLI and automated agents.

Initial scope: surfaces/infinite canvas, selection/layers, shapes and vectors, text, responsive containers/layout, components/instances/variants, tokens/themes, viewport projections, fidelity diagnostics, import/export inspection, deterministic snapshots, and operation replay.

The shell is specified in `UI-SPEC.md` (regions, tools, property sections, bindings, automation surface) and its implementation stack in `adrs/0006-rust-native-editor.md` (accepted): Rust-native on Masonry, Vello and AccessKit. A Svelte 5 shell over the WASM bindings remains a later browser demonstration. Rendering is evaluated separately so UI-framework convenience cannot dictate standard semantics.
