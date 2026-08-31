---
id: nuif:rfc:0013
kind: rfc
status: proposed
---

# RFC 0013 — Variable TrueType resource profile

Status: proposed research contract. Its metadata, coordinate-normalization and
isolated shaping stages have executable evidence, but no package, layout,
rendering, fidelity, or conformance-profile claim is made.
`nuif-opentype-static-single-0` remains the only executable font-resource
profile.

## Motivation

NUIF can preserve, shape and rasterize one exact static TrueType face, but the
model already has an axis-coordinate map. Accepting a variable font by merely
allowing an `fvar` table would be unsound: one coordinate selection affects
glyph outlines, advances, global metrics and OpenType Layout feature
variations. Host-default coordinates, float rounding or partial table support
could therefore produce different layout and pixels from the same package.

This RFC defines the smallest candidate variable profile that can reuse the
existing exact-resource, policy, shaping and outline pipeline without combining
collections, CFF2, color glyphs or web-font transport into one capability.

## Prior art and evidence

- OpenType 1.9.1 defines variable-font user coordinates, 16.16 normalization,
  final 2.14 coordinates, `avar` remapping, delta interpolation and the table
  relationships among `fvar`, `gvar`, HVAR, VVAR, MVAR and STAT:
  https://learn.microsoft.com/en-us/typography/opentype/spec/otvaroverview.
- The OpenType file specification distinguishes font collections, TrueType and
  CFF/CFF2 outlines, variation tables, vector color, bitmap color and SVG glyph
  data: https://learn.microsoft.com/en-us/typography/opentype/spec/otff.
- The `fvar` table defines ordered axes, min/default/max user coordinates and
  optional named instances:
  https://learn.microsoft.com/en-us/typography/opentype/spec/fvar.
- HarfBuzz configures variation coordinates on the font before shaping and
  treats named instances like independent static faces:
  https://harfbuzz.github.io/fonts-and-faces-variable.html.
- Fontations exposes the same coordinate model to both HarfRust
  `ShaperInstance` and Skrifa `LocationRef`, which makes a shared normalized
  coordinate vector possible in the Rust reference implementation. Library
  support is an implementation option, not an interoperability oracle.

## Profile identity and composition

The candidate identifier is `nuif-opentype-variable-truetype-single-0`.

It composes with `nuif-package-0` and the existing reference text/runtime
pipeline. It does not change CPU render profile 0 or make variable fonts a
required NUIF baseline. A host advertises this capability separately.

The selected input is one variable TrueType-outline sfnt face at face index
zero. The profile rejects:

- TTC/OTC collections and nonzero face indices;
- CFF or CFF2 outlines;
- COLR/CPAL, CBDT/CBLC, sbix, EBDT/EBLC and SVG glyph sources;
- WOFF and WOFF2 containers;
- `avar` version 2, VARC and multi-axis mappings until separately specified;
- font-program execution and hinted-output conformance;
- implicit platform fonts, implicit network resolution and implicit axis
  defaults supplied by a host UI.

## Semantic asset contract

The resource digest identifies the exact original variable-font bytes. The
font asset retains the existing stable `AssetId`, exact digest, face index,
family names, Unicode coverage, global OpenType feature values and portability
policy.

`FontAsset.axes` is the complete selected user-coordinate tuple:

- it contains exactly one entry for every `fvar` axis and no unknown tag;
- keys are the case-sensitive four-byte axis tags from the font;
- each value is finite, lies within that axis's declared minimum and maximum,
  and converts to OpenType 16.16 by the specification's required conversion;
- omission is invalid for this first profile, even when the intended value is
  the font default;
- canonical map order does not define axis order; the `fvar` axis array does;
- an `fvar` named instance is only descriptive provenance. Its name or index is
  never font or instance identity; the exact byte digest plus complete
  coordinate tuple is authoritative.

The asset records:

```text
font.decoder_profile = nuif-opentype-variable-truetype-single-0
opentype.fs_type = 0xNNNN
license.expression = publisher-reviewed expression
license.embedding_review = approved
```

Future subsetting or static-instancing output is a derived resource with its
own digest, derivation record and embedding review. The profile does not infer
permission to create or redistribute a derived instance from `fsType`, a named
instance or the ability to render the source.

## Coordinate algorithm

Implementations must produce one ordered normalized coordinate vector before
shaping, metrics or outline extraction:

1. read axes in `fvar` order and validate each min/default/max relation;
2. convert selected user coordinates to signed 16.16 exactly as OpenType 1.9.1
   requires;
3. apply the two-segment min/default/max normalization in 16.16;
4. apply `avar` version 1 segment maps in axis order when present;
5. clamp and convert the final values to signed 2.14 using the OpenType rule;
6. record the ordered tag, user 16.16 value and normalized 2.14 value in the
   resolved run or its referenced evaluation record;
7. pass that same normalized vector to shaping, metric evaluation and outline
   extraction.

A stage must not independently renormalize from a lower-precision float. Any
disagreement among the parser, shaper and outline engine is a typed evaluation
failure, not approximated fidelity.

Global OpenType feature settings are applied after the variable instance is
configured so GSUB/GPOS FeatureVariations are selected at the same normalized
position. Requested feature values remain explicit in the resolved run.

## Metrics, outlines and fidelity

Intrinsic inline size uses shaped advances at the selected location. Baseline
placement uses location-adjusted ascent; vertical profiles must additionally
specify VVAR and vertical-origin behavior before claiming vertical text.
Unhinted outlines use the same normalized location. MVAR, HVAR and applicable
`gvar` phantom-point effects must not be partially ignored.

An exact binding is `lossless` only when the complete coordinate tuple,
features, shaping, metrics and outlines all use the declared profile. Missing
bytes or an unsupported variable table produce item-level `unsupported` and no
text command. A declared replacement asset follows the existing
`substituted`/`approximated` contract.

## Candidate resource limits

The first implementation experiment must measure and may reduce these proposed
ceilings before the profile can become executable:

- 32 MiB encoded font bytes and 256 top-level tables, inherited from the static
  profile;
- 16 axes and 256 named instances;
- 256 `avar` segment records per axis;
- 65,536 glyphs and 65,536 variation regions;
- 64 global feature settings;
- 65,536 codepoints per shaped text item;
- separate preflight ceilings for `gvar`, HVAR, VVAR and MVAR item/region
  counts, parser allocations, shaping allocations, outline commands and total
  evaluation time.

These are admission candidates, not accepted portable limits. The experiment
must show that one-over inputs fail before proportional allocation and that
region/tuple products cannot overflow internal arithmetic.

## Security and policy

The implementation must retain NUIF-owned sfnt range, table-order, packing and
checksum checks ahead of parser interpretation. It must validate all variation
table versions, offsets, counts, axis ordering, tuple lengths, region indices,
delta runs and arithmetic ranges. Unknown major versions or required table
relationships fail closed.

The reference path remains safe Rust and unhinted. Native rasterizer execution,
TrueType instruction conformance and arbitrary downstream font-stack safety are
outside this profile. Parser budgets do not replace process cancellation,
sandboxing or host quotas.

## Conformance experiment

The first promotion prerequisite is now automated by
`cargo xtask gate-i-font-metadata`: one bounded four-axis TrueType fixture is
decoded by the NUIF path and Skrifa, while a committed HarfBuzz 14.4.0 C-API
capture independently checks axis metadata, named-instance count, and five
final normalized vectors including non-identity `avar` results. This is a
single-fixture metadata milestone, not completion of the experiment below.
`cargo xtask gate-i-font-shaping` then delivers that exact vector to HarfRust
and reproduces seven `hb-shape` cases, including a GSUB FeatureVariations
boundary. Skrifa advances and unhinted outlines use the same vector and repeat
exactly against HarfBuzz's independently captured horizontal advances and draw
callbacks. The single fixture has no active HVAR deltas or MVAR table, so this
does not complete variable-metric conformance. A second isolated
`cargo xtask gate-i-font-metrics` fixture applies nonzero HVAR deltas to three
horizontal advances at four locations, including a valid truncated advance
index map. HarfRust and Skrifa agree exactly with pinned HarfBuzz 14.4.0
observations. MVAR, VVAR, side bearings, `gvar` phantom-point fallback, broader
HVAR coverage, and package/runtime integration remain open.

Promotion from proposed to experimental requires all of the following:

1. **Two independent metadata/normalization paths.** Compare axis definitions,
   named instances and final 2.14 vectors from the Rust implementation with a
   pinned external HarfBuzz/FreeType or browser-derived oracle.
2. **Shaping oracle.** At default, min, max, named and at least two non-named
   interior positions, compare glyph IDs, scalar-index clusters, advances and
   offsets with pinned `hb-shape` output. Include a FeatureVariations fixture.
3. **Outline oracle.** Compare normalized unhinted paths or bounded geometric
   deltas at the same locations with a second implementation.
4. **Metric coherence.** Prove layout and raster use the same location-adjusted
   advances and ascent; exercise HVAR and MVAR, and reject VVAR claims until a
   vertical profile exists.
5. **Package behavior.** Preserve exact bytes through package fixpoint and an
   unrelated semantic edit; compare embedded and explicit digest-pinned linked
   resolution; reject stale axis metadata and forbidden portability outcomes.
6. **Metamorphic checks.** Default coordinates reproduce the variable font's
   default instance, identical tuples repeat exactly, map insertion order is
   irrelevant, and changing one axis leaves all other recorded coordinates
   unchanged.
7. **Negative corpus.** Cover malformed `fvar`, `avar`, `gvar`, HVAR, VVAR,
   MVAR and STAT data; duplicate/missing/unknown axes; NaN, infinity and
   out-of-range coordinates; excessive axes/instances/regions; collections,
   CFF2, color, bitmap, SVG and WOFF2 inputs.
8. **Resource evidence.** Use only fixtures whose redistribution and test use
   are recorded. Retain exact tool versions, commands, reports and source
   revision.
9. **Cross-surface parity.** Direct API, CLI, WASM and MCP must produce the same
   canonical document hash, coordinate record, diagnostics and fidelity. Native
   raster hashes remain platform-scoped until a retained matrix passes.

The gate report must distinguish exact external comparisons, internal
metamorphic checks and implementation-specific allocation measurements.

## Compatibility and migration

Existing static font assets have an empty axis map and remain valid only for
`nuif-opentype-static-single-0`. A variable resource cannot be silently opened
under the static profile. Adding the new capability does not change canonical
bytes for existing documents.

The current `FontAsset.axes` field can encode the complete user tuple. Before
implementation, the schema and diagnostics must make profile selection
unambiguous and add a resolved normalized-coordinate record without using
policy-evidence strings as semantic storage. If that cannot be done compatibly
during alpha, the model version must migrate explicitly.

## Alternatives rejected

- **One general OpenType capability:** combines unrelated parsers, renderers,
  security budgets and fidelity claims.
- **Store only a named-instance index or name:** indices and names are
  resource-local labels, not semantic identity.
- **Allow omitted axes to use host defaults:** hides an evaluation input and
  makes model intent dependent on a font/UI implementation.
- **Store only normalized coordinates:** loses authored user-space intent and
  makes resource replacement/migration hard to explain.
- **Normalize separately in layout and rendering:** permits metric/outline
  divergence.
- **Pre-instance every variable font:** may lose variation behavior, creates a
  new derived resource and requires separate license/subsetting evidence.
- **Treat parser acceptance as raster equivalence:** ignores shaping,
  FeatureVariations, metric deltas, outline interpolation and platform raster
  behavior.

## Unresolved review questions

- Should the first profile reject all TrueType instructions at admission, or
  merely exclude hinted output while retaining instruction tables inertly?
- Which rights-cleared fixture set covers HVAR, MVAR, `avar` remapping and
  FeatureVariations without depending on one font family?
- Is a location-adjusted vertical-metrics subset worth including, or should
  VVAR remain a separate vertical-text capability?
- Which external implementation can independently retain and reproduce the
  complete conformance vectors?
