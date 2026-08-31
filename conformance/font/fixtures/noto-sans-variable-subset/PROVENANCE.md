# Noto Sans variable conformance subset

This directory contains an 11,260-byte derived test fixture, not a NUIF
portable-font acceptance sample. It retains U+0041 (`A`), U+0048 (`H`),
U+0066 (`f`), U+0069 (`i`), U+0078 (`x`), U+00C5 (`Å`), and U+00E9 (`é`)
while preserving the variable metadata, layout tables, and variation stores
needed by conformance tests.

## Pinned source

- Registry repository: `google/fonts`
- Registry commit: `ade3d1533e06b2b1462ffcde8e08b129627ca360`
- Font blob: `75575046c015ff623a848096a15779867ba71453`
- Font SHA-256:
  `bfb7bb691513f12e734dc346c03a03f784912432d7e3fa8e56efcf906fe86b3d`
- License blob: `6843f31878c9945e1e71e1baa275546b4eefead8`
- License SHA-256:
  `cee9892f9f0cc8fe882c9e9537ee6a89621d86ee7ceaf70b02e2b2b1c25c061a`
- Upstream project revision recorded by Google Fonts:
  `notofonts/latin-greek-cyrillic@c4a321e123e4d4ff315f57f4e0adf294fe3a95be`
- Upstream release recorded by Google Fonts: `NotoSans-v2.015`

Google Fonts metadata identifies the family as Noto Sans, the designer as
Google, and the license as SIL Open Font License 1.1. The unmodified registry
`OFL.txt` is retained beside the derived fixture. Its copyright notice names
the Noto Project Authors and declares no Reserved Font Names. This is fixture
provenance and redistribution evidence, not a general-purpose legal decision.

## Reproduction

Run `tools/font/prepare_variable_font_corpus.py` with the exact Noto Sans and
Recursive source fonts and licenses from the pinned registry commit. The
script requires `hb-subset 14.4.0`, verifies every source and license digest,
and applies this operation:

```text
hb-subset SOURCE.ttf --unicodes=41,48,66,69,78,C5,E9 --name-IDs=* \
  --name-languages=* --layout-features=* \
  --output-file=NotoSans-variable-subset.ttf
```

Derived font SHA-256:
`0afd77effc877ff84fa7995a58c396c124514855f8084056846b54b8cb76f3ce`.

The script verifies that `fvar`, `avar`, `gvar`, HVAR, MVAR, and STAT survive
subsetting and caps the fixture at 16 KiB. Conformance gates test the derived
font directly; preserving a table does not by itself prove semantic equality
with the full source font.
