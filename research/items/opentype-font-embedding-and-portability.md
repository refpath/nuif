---
id: nuif:research:opentype-font-embedding-and-portability
kind: standard
status: reviewed
title: OpenType embedding permissions and reproducible font resources
source:
  url: https://learn.microsoft.com/en-us/typography/opentype/spec/os2#fstype
  authors: [Microsoft, Adobe]
  published_at: null
  license: OpenType specification terms
retrieved_at: 2026-08-30
tags: [fonts, opentype, embedding, licensing, shaping, portability]
confidence: 0.98
claims: [nuif:claim:resource-identity-separation]
relations:
  - type: extends
    target: nuif:research:text-rendering-reproducibility
    note: Adds packaging and redistribution policy to the already-pinned shaping/raster inputs.
  - type: related_to
    target: nuif:research:harfbuzz-unicode
  - type: related_to
    target: nuif:research:ttf-parser
    note: Records the retired parser and the maintenance evidence that caused its removal.
  - type: related_to
    target: nuif:research:fontations
links:
  spec: [spec/05-geometry-paint-text.md, spec/08-serialization.md, spec/11-security.md]
  adr: [adrs/0003-reference-renderer.md]
  rfc: [rfcs/0010-portable-resource-package.md, rfcs/0013-variable-truetype-resource-profile.md]
  code: [crates/nuif-text, crates/nuif-font, crates/nuif-package, crates/nuif-testing/src/bin/font-resources.rs, crates/nuif-testing/src/bin/variable-font-metadata.rs, crates/nuif-testing/src/bin/variable-font-shaping.rs, crates/nuif-testing/src/bin/variable-font-metrics.rs, crates/nuif-testing/src/bin/variable-font-global-metrics.rs, crates/nuif-testing/src/bin/variable-font-security.rs, crates/nuif-testing/src/bin/variable-font-package.rs, crates/nuif-testing/src/bin/variable-font-corpus.rs, crates/nuif-testing/src/bin/variable-font-gvar-generated.rs, conformance/font/harfbuzz-14.4.0-material-symbols-variable.json, conformance/font/harfbuzz-14.4.0-hvar-truncated-map.json, conformance/font/harfbuzz-14.4.0-roboto-flex-mvar.json, conformance/font/harfbuzz-14.4.0-noto-sans-variable.json, conformance/font/harfbuzz-14.4.0-recursive-variable.json, conformance/font/fixtures/roboto-flex-mvar-subset/PROVENANCE.md, conformance/font/fixtures/noto-sans-variable-subset/PROVENANCE.md, conformance/font/fixtures/recursive-variable-subset/PROVENANCE.md]
  experiments: [nuif:experiment:font-resource-static-baseline, nuif:experiment:variable-font-metadata-baseline, nuif:experiment:variable-font-shaping-baseline, nuif:experiment:variable-font-hvar-baseline, nuif:experiment:variable-font-mvar-baseline, nuif:experiment:variable-font-graph-security-baseline, nuif:experiment:variable-font-package-candidate, nuif:experiment:variable-font-corpus-baseline, nuif:experiment:variable-font-gvar-generated, nuif:experiment:font-resource-profile]
---

# Summary

Reproducible text requires exact font bytes, but possessing or using a font does
not automatically permit redistribution inside a portable package. OpenType's
`OS/2.fsType` flags provide machine-readable embedding signals—installable,
restricted, preview/print, editable, no-subsetting and bitmap-only—but those
flags are part of a wider license context and must not be treated as a complete
legal decision engine.

## Evidence

- OpenType 1.9.1 `OS/2`, `fsType` defines mutually exclusive usage permissions:
  installable (0), restricted (2), preview/print (4) and editable (8).
- Bit 8 forbids subsetting and bit 9 permits only embedded bitmaps. Reserved
  bits and historical version differences mean parsers must validate the table
  version and length rather than assuming one modern layout.
- Versions 0 through 2 historically permitted multiple usage bits with a
  least-restrictive interpretation, while version 3 made those bits mutually
  exclusive. The first NUIF profile deliberately rejects ambiguous historical
  combinations instead of silently selecting a permission.
- The specification says embedding-aware applications must not embed fonts
  whose permissions do not allow embedding or alter the flags, and notes that
  rights are granted by the font vendor.
- CSS Fonts Level 4 defines face selection, variation axes, feature settings and
  font fallback as separate inputs to rendered text. CSS Font Loading Level 3
  exposes document font readiness; capture before fonts settle is not a stable
  observation.

## Mechanism

A NUIF font asset points to exact bytes when policy permits and records at
least: media type, SHA-256, face or collection index, names used for matching,
variation axes, selected features, character coverage and embedding-policy
evidence. A text run still records the shaping inputs required by its profile.

The packaging policy is explicit:

- `portable`: exact bytes embedded and permitted for the declared use;
- `private_authoring`: bytes retained only in an access-controlled workspace,
  not a distributable package;
- `linked`: expected digest and resolver hint recorded, no implicit fetch;
- `substituted`: replacement bytes and item-level fidelity recorded;
- `unavailable`: metrics/evidence may be retained, but no false exactness claim.

## NUIF relevance

**Borrow** the `fsType` signal and exact OpenType face/variation metadata.

**Adapt** it into a conservative policy decision that also accepts explicit
license metadata and user/admin policy. The file-format validator reports the
facts; it does not provide legal advice.

**Reject** family name as font identity, system-font discovery for exact
profiles, silent fallback, automatic embedding from a browser's platform-font
name, and a claim that `fsType == 0` alone proves redistribution rights.

## Implemented narrow baseline

`nuif-opentype-static-single-0` accepts only one checksummed, canonically packed
TrueType-outline sfnt face. It rejects TTC, CFF/CFF2, variable, color, bitmap,
SVG and WOFF/WOFF2 sources. Package validation compares face, family names,
static axis state and exact Unicode coverage, then requires matching `fsType`,
a non-empty license expression and an explicit `approved` embedding review.

`cargo xtask gate-i-font` accepts four static TrueType fixtures, compares Skrifa
0.46.2 against a committed `hb-info` 14.4.0 metadata capture on the pinned Ahem
resource, proves package byte fixpoint and resource retention, and runs 20
malformed/unsupported, 10 policy and six portability trials. Real TTC, CFF,
variable, COLR, embedded bitmap, CBDT and sbix fixtures prove fail-closed
exclusion.

The same gate now loads a portable non-Ahem Tinos package through
`nuif-api::NuifDocument`. Digest-verified bytes automatically enter the local
evaluation context; HarfRust applies the asset's global feature map, layout uses
the exact advances, Skrifa supplies the face ascent and unhinted outlines, and
the CPU renderer produces deterministic pixels with lossless item fidelity.
The resolved run records the applied features. This closes arbitrary accepted
static-resource shaping in the reference runtime, but one fixture and one
implementation do not establish foreign-shaper or cross-platform raster
equivalence.

## Profile decomposition decision

OpenType 1.9.1 treats collections, outline technologies, variations and color
glyph descriptions as distinct structures. A collection has multiple table
directories and may share tables between faces; variation coordinates affect
glyph outlines and horizontal, vertical and global metrics through multiple
tables; color glyphs may be vector paint graphs, bitmaps or SVG documents.
HarfBuzz likewise requires variation coordinates to be configured on the font
before shaping. Therefore NUIF will not promote a single “general OpenType”
switch. The next work is separated into independently versioned capabilities:

1. `collection`: deterministic face selection and shared-table bounds, without
   changing the selected face's outline/runtime profile;
2. `variable-truetype`: exact axis definitions, normalized coordinates,
   `avar` mapping, named-instance identity and consistent `gvar`/HVAR/VVAR/MVAR
   application to outlines and metrics;
3. `cff-static` and later `cff2-variable`: separate outline decoders and
   independent malformed-input evidence;
4. `colr-vector`: bounded COLR/CPAL paint graphs and palette selection;
5. bitmap and SVG glyph sources: separate decode/sandbox profiles rather than
   fallbacks inside the vector profile;
6. WOFF2: a transport profile whose decoded sfnt identity and resource-policy
   relationship are explicit.

The OpenType font-file organization and variation-table inventory are defined
by the OpenType 1.9.1 file and variation specifications. The decomposition is a
NUIF design decision inferred from those structures, not a requirement imposed
by OpenType itself.

## Implemented variable metadata milestone

`cargo xtask gate-i-font-metadata` now gives RFC 0013 one deliberately narrower
executable result. NUIF directly checks a TrueType sfnt, `fvar` 1.0 and optional
`avar` 1.0 under explicit axis, instance and segment ceilings, then requires its
ordered metadata and final 2.14 coordinate vectors to agree with Skrifa. A
committed HarfBuzz 14.4.0 public-C-API capture independently checks four axes,
seven named instances and default/minimum/maximum plus two interior vectors;
the interior `wght` values prove that an `avar` map is actually applied.

This does not accept the resource in a package or runtime. VVAR, rendering and
cross-surface parity remain blocking. A
second gate does reproduce
seven HarfBuzz shapes including a GSUB FeatureVariations threshold and proves
one coordinate vector is reused by HarfRust and Skrifa metrics/outlines. Every
advance and canonical path now agrees with HarfBuzz's independently captured
metric and draw callbacks. The fixture has no active HVAR regions or MVAR table,
so the single-fixture result is not variable-metric or package evidence.
An independent HVAR gate now compares three nonzero location-dependent advances
across four `wght` positions with HarfBuzz and specifically exercises a valid
truncated advance-index map. It still does not cover VVAR, side bearings,
phantom-point fallback, broad HVAR behavior, or package/runtime admission.
One reproducible two-glyph Roboto Flex subset supplies the MVAR oracle. Its
OFL-1.1 text, Google Fonts commit/blob identities, source and derived digests,
and `hb-subset` 14.4.0 command are retained. Skrifa and HarfBuzz agree exactly
on five global metrics at eight locations. This does not generalize one MVAR
store into broad metric, legal-policy, or runtime evidence. A fifth gate now
walks the accepted fixtures' connected `gvar`, HVAR, MVAR, item variation store,
and STAT structures under explicit count/product bounds. It rejects 38
checksum-repaired mutations and holds six warmed allocation/time trials below
declared reference-implementation ceilings. Nine cases cover representative
packed `gvar` header/point/delta failures, exact X/Y run counts,
glyph/component-plus-phantom point bounds, and rejection of a non-OpenType
32-bit extension accepted by the upstream iterator; another checksum-repaired
case verifies explicit VVAR exclusion. Byte-exhaustive encoding enumeration,
VVAR semantics, and package/runtime admission remain open.
The package candidate gate now validates the exact asset metadata, complete
axis tuple, license expression and explicit embedding review; it preserves the
font as a resource through deterministic package fixpoint and an unrelated edit
and verifies linked bytes through an explicit digest-checking resolver. This is
not typed package admission: the dispatcher is tested to remain fail-closed so
the reference runtime cannot ignore the selected coordinates.
The broader corpus gate adds independently authored Noto Sans and Recursive
OFL-1.1 subsets with exact registry/upstream/font/license/output provenance.
Across eight 2- and 5-axis locations, NUIF matches HarfBuzz metadata,
normalization, shaping, HVAR advances and MVAR metrics exactly. Seven outlines
are exact; the Recursive interior outline has identical topology with one
control coordinate differing by one 26.6 unit, which defines the measured
cross-implementation tie bound. Both exact assets pass candidate validation,
but that result still does not enable typed package binding.
The generated `gvar` gate rebuilds 19 ephemeral, checksummed sfnt inputs around
a 300-point glyph. Sixteen valid cases cover count, point-run, delta-run,
shared/private/repeated-point, multi-tuple and maximum-count boundaries; three
malformed count cases reject by name. This closes the declared packing-boundary
matrix while preserving byte-exhaustive input enumeration as a non-claim.
The fixture's crate-level `MIT OR Apache-2.0` distribution metadata is retained,
while its embedded copyright string prevents the experiment from presenting
that fact as an automated publisher-rights determination.

## Open questions

- Which explicit license-expression vocabulary is reliable enough to augment
  `fsType` without pretending to automate legal interpretation?
- Can a portable profile subset a font only when both the license signal and
  shaping corpus permit it, while retaining an audit link to the source digest?
- How should variable-font instancing be represented when the original file may
  not be redistributed but a licensed derived instance may be?

## Sources added for the decomposition

- OpenType 1.9.1, “The OpenType Font File”:
  https://learn.microsoft.com/en-us/typography/opentype/spec/otff
- OpenType 1.9.1, “OpenType Font Variations overview”:
  https://learn.microsoft.com/en-us/typography/opentype/spec/otvaroverview
- OpenType 1.9.1, “Glyph Variations Table”:
  https://learn.microsoft.com/en-us/typography/opentype/spec/gvar
- OpenType 1.9.1, “OpenType Font Variations Common Table Formats”:
  https://learn.microsoft.com/en-us/typography/opentype/spec/otvarcommonformats
- HarfBuzz manual, “Working with OpenType Variable Fonts”:
  https://harfbuzz.github.io/fonts-and-faces-variable.html
- Google Fonts pinned Noto Sans registry metadata and upstream revision:
  https://github.com/google/fonts/blob/ade3d1533e06b2b1462ffcde8e08b129627ca360/ofl/notosans/METADATA.pb
- Google Fonts pinned Recursive registry metadata and upstream revision:
  https://github.com/google/fonts/blob/ade3d1533e06b2b1462ffcde8e08b129627ca360/ofl/recursive/METADATA.pb
- OpenType 1.9.1, “COLR — Color Table”:
  https://learn.microsoft.com/en-us/typography/opentype/spec/colr
