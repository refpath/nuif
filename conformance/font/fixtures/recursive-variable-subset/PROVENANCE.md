# Recursive variable conformance subset

This directory contains a 69,204-byte derived test fixture, not a NUIF
portable-font acceptance sample. It retains U+0041 (`A`), U+0048 (`H`),
U+0066 (`f`), U+0069 (`i`), U+0078 (`x`), U+00C5 (`Å`), and U+00E9 (`é`)
while preserving the variable metadata, layout tables, and variation stores
needed by conformance tests.

## Pinned source

- Registry repository: `google/fonts`
- Registry commit: `ade3d1533e06b2b1462ffcde8e08b129627ca360`
- Font blob: `367e2df5ad567b3dcd958d11a712de9fe7d3bac1`
- Font SHA-256:
  `653221ca467f4732fe6856ac493f6c409e9f56a7674abe36b2364acc89796f7c`
- License blob: `d4361a99fe5b5091c2a7596ee55818da51373a3d`
- License SHA-256:
  `f9f539cf7549bd417159dbdb9c400943a5b60a7366c2c6fbde9f095173d82479`
- Upstream project revision recorded by Google Fonts:
  `arrowtype/recursive@071fc21f217781110d67e8d0bf5021f31cbdcb85`

Google Fonts metadata identifies the family as Recursive, the designer as
Arrow Type, and the license as SIL Open Font License 1.1. The unmodified
registry `OFL.txt` is retained beside the derived fixture. Its copyright notice
names the Recursive Project Authors and declares no Reserved Font Names. This
is fixture provenance and redistribution evidence, not a general-purpose legal
decision.

## Reproduction

Run `tools/font/prepare_variable_font_corpus.py` with the exact Noto Sans and
Recursive source fonts and licenses from the pinned registry commit. The
script requires `hb-subset 14.4.0`, verifies every source and license digest,
and applies this operation:

```text
hb-subset SOURCE.ttf --unicodes=41,48,66,69,78,C5,E9 --name-IDs=* \
  --name-languages=* --layout-features=* \
  --output-file=Recursive-variable-subset.ttf
```

Derived font SHA-256:
`11fca6aeeaa73644a2174d2608cab7eb5d9828f5d88a7feca2c299415f3fa604`.

The script verifies that `fvar`, `avar`, `gvar`, HVAR, MVAR, and STAT survive
subsetting and caps the fixture at 72 KiB. Conformance gates test the derived
font directly; preserving a table does not by itself prove semantic equality
with the full source font.
