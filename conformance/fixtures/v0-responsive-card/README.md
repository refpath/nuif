# v0 hard experiment — responsive card

This fixture is the first architecture falsification test.

## Required authored features

- component `Card` with nested `Button` component;
- enum variant and boolean state;
- DTCG-compatible color/spacing/radius token bindings;
- responsive layout changing from one-column to split layout;
- intrinsic text and a vector icon;
- hover/pressed state metadata;
- one opaque `VENDOR_probe` extension unknown to an intermediate implementation.

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
