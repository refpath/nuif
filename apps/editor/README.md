# Reference editor

The editor is an executable conformance/research instrument, not the owner of the NUIF data model. It will edit NUIF natively through the same semantic operation API available to the CLI and automated agents.

Initial scope: surfaces/infinite canvas, selection/layers, shapes and vectors, text, responsive containers/layout, components/instances/variants, tokens/themes, viewport projections, fidelity diagnostics, import/export inspection, deterministic snapshots, and operation replay.

The current leading shell architecture is Svelte 5 + TypeScript around a Rust/WASM core, with rendering evaluated separately so UI-framework convenience cannot dictate standard semantics.
