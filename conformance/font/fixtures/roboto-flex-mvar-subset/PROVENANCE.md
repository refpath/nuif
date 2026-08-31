# Roboto Flex MVAR conformance subset

This directory contains a 22,104-byte derived test fixture, not a NUIF
portable-font acceptance sample. It retains only U+0048 (`H`) and U+0078 (`x`)
while preserving the source variable-font metadata and variation stores needed
to test MVAR global metrics.

## Pinned source

- Repository: `google/fonts`
- Commit: `ade3d1533e06b2b1462ffcde8e08b129627ca360`
- Font blob: `2a11e4cd5588a89e0047140b09c912059d1a150f`
- Font SHA-256:
  `9b523f7d82593df0107173849ebb8c817471a1df4b4fb2c3cbf40cfd810c8281`
- License blob: `f6ebd5a74b229728026776434c68deff3bec5eb0`
- License SHA-256:
  `9cbaed04b20c853f99840efe5dc96956f6f6120ed83a0ade35f9281a2b63e5d0`
- Upstream project revision recorded by Google Fonts:
  `googlefonts/Roboto-Flex@739e06dc46ebb14cddd88b9768a6c1504d4677f6`

The source font and its Google Fonts metadata declare the SIL Open Font
License 1.1. The unmodified upstream `OFL.txt` is retained beside the derived
fixture. Its header declares no Reserved Font Names. This records provenance
and redistribution evidence for the fixture; it is not a general-purpose legal
policy engine.

## Reproduction

Run `tools/font/prepare_roboto_flex_mvar.py` with the exact source font and
license from the pinned Google Fonts commit. The script requires
`hb-subset 14.4.0`, checks both source digests, applies this operation, verifies
the required variable TrueType tables, and requires the exact output digest:

```text
hb-subset SOURCE.ttf --unicodes=48,78 --name-IDs=* --name-languages=* \
  --layout-features=* --output-file=RobotoFlex-MVAR-subset.ttf
```

Derived font SHA-256:
`4fe568be6e73133adf9eb03e87d094ddd7c73f4250c61d3356b55e2ea7886ea9`.

Subsetting preserves `fvar`, `avar`, `gvar`, HVAR, MVAR, and STAT. It does not
prove that every source table is semantically unchanged; the experiment checks
the derived font directly against pinned HarfBuzz observations.
