# Static OpenType resource profile 0

Status: experimental, implemented, and not a general OpenType conformance claim.

Identifier: `nuif-opentype-static-single-0`

This profile gives a NUIF package one deterministic, bounded baseline for an
exact authoring font resource. It accepts a single-face, statically instanced
TrueType-outline sfnt and rejects other font source categories instead of
silently interpreting them differently across hosts.

## Accepted input

- sfnt signature `0x00010000` and face index `0`;
- at most 32 MiB and 256 strictly sorted, unique table records with consistent
  sfnt search fields;
- required `OS/2`, `cmap`, `glyf`, `head`, `hhea`, `hmtx`, `loca`, `maxp`, and
  `name` tables;
- aligned, in-range, contiguously packed table data with exact zero padding and
  no trailing data;
- valid per-table checksums and complete-font checksum;
- `OS/2` version 0 through 5 with one unambiguous `fsType` usage permission;
- Unicode coverage derived from mappings that resolve to glyphs;
- at most 256 family names, 65,536 coverage ranges, and 64 declared feature
  settings.

Collections, CFF/CFF2 outlines, variable fonts, color or bitmap glyph tables,
SVG glyphs, WOFF/WOFF2 containers, and unknown `OS/2` versions are outside this
profile. They require separately named profiles and conformance evidence.

## Asset binding

The font asset must exactly match the parsed face index, family names, static
axis state, and Unicode coverage. Its resource descriptor must use `font/ttf`.
The asset records:

- `font.decoder_profile = nuif-opentype-static-single-0`;
- `opentype.fs_type = 0xNNNN`, matching the exact bytes;
- a non-empty `license.expression` chosen by the publisher;
- `license.embedding_review = approved`, recording an explicit human or
  organizational decision.

The parser rejects restricted or bitmap-only embedding evidence for this
portable outline profile. The review field does not grant rights, interpret a
license, or make `OS/2.fsType` authoritative over the font's actual license.
Publishers remain responsible for redistribution and embedding permission.

## Package and resolver behavior

Embedded fonts are validated during manifest construction, package encoding,
and package decoding. Digest-pinned linked fonts in an authoring package remain
unresolved; a caller-provided resolver must return the exact bounded bytes,
after which the same profile validation runs. Package parsing never performs a
network request.

## Security limits and non-claims

The implementation contains no unsafe code and uses pinned `ttf-parser`
`0.25.1` only after independent sfnt directory, range, and checksum checks.
Resource limits are validation policy, not proof that an accepted font is safe
for every downstream native rasterizer. A renderer must preserve its own
sandbox and work budgets.

This baseline does not yet prove shaping equivalence, glyph-outline
equivalence, subsetting, variable-axis behavior, color-font behavior, browser
font decoding, layout fidelity, or licensing compliance.

## Evidence boundary

`cargo xtask gate-i-font` accepts four static TrueType fixtures from
`font-test-data` 0.9.1 and compares the exact pinned Ahem metrics, static-axis
state and Unicode coverage across `ttf-parser` and Skrifa. It rejects 20
synthetic and real cases spanning malformed/checksum-invalid sfnt data, TTC,
CFF, variable, COLR, embedded bitmap, CBDT and sbix categories. Ten metadata and
embedding-policy mutations plus six portable/private/linked/substituted/
unavailable package outcomes are blocking. The real rejected fixtures prove
that those categories fail closed; they do not specify how a future profile
will accept them.
