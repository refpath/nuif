# v0 hard experiment — responsive card

This fixture is the first architecture falsification test.

## Required authored features

- component `Card` with nested `Button` component;
- enum variant and boolean state;
- color, spacing and radius token bindings in the profile-0 token model;
- responsive layout changing from one-column to split layout;
- intrinsic text and a vector icon;
- hover/pressed state metadata;
- one opaque `vendor.probe` extension unknown to an intermediate implementation.

## Round-trip path

`NUIF reference editor → HTML/CSS adapter → source edit synchronization → NUIF → neutral intermediate editor → NUIF`.

Figma/Penpot adapters may be inserted as additional targets, but no vendor tool is required for the core proof.

## Success criteria

- 100% stable semantic entity/component IDs on NUIF-native cycles;
- 100% token-reference preservation where target can represent tokens;
- opaque extension bytes survive unchanged;
- resolved boxes at 360/768/1440 px remain within declared tolerance after representable round trips;
- no silent fidelity downgrade;
- a padding/token/text edit patches only the corresponding source region plus unavoidable formatter changes;
- canonical encoding is byte-stable after decode/encode;
- replaying the same operation log from the same base yields the same canonical hash.

## Automated baseline

Generate the canonical fixture with `nuif fixture v0-responsive-card <output.nuif>`. Run `editor-trial.jsonl` through `nuif-editor --headless --script ... --document <output.nuif>` to exercise entity-bound selection, semantic edits, undo/redo, complete mutation-log replay and a deterministic snapshot. `nuif trial <seed> <iterations> [snapshot-interval]` drives replay, inversion and text/CBOR fixpoints on every patch, with responsive-layout/CPU-rerender checks at the requested interval. `cargo xtask gate-b` runs 10,000 patches with a raster interval of 100.

After `cargo xtask browser-install`, `cargo xtask gate-c` lowers this fixture independently to Taffy and DOM/CSS at 360 × 640, 768 × 768 and 1,440 × 900. The strict 2026-08-29 report records exact agreement for every box across all three engines. `cargo xtask gate-g` separately compares canonical text, opaque preservation, layout, decoded RGBA and fidelity with the standard-library-only Python implementation.

`cargo xtask gate-f-v0` closes the retentive source segment. The model trial retains 181 source correspondences and changes only eight spans for token, padding, text and responsive edits while preserving every other byte and the unknown payload. Its second path edits name and width through the headless semantic editor, synchronizes through the public CLI and requires the imported canonical NUIF bytes to equal the editor output. The profile preserves path and instance identity but does not claim browser path rendering or instance materialization; those limitations remain explicit fidelity rather than weakening the experiment's model round trip.
