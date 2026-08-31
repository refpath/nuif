# Variable TrueType metadata research profile

Status: implemented metadata/normalization and isolated shaping experiments;
not an executable package, session, layout, rendering, or fidelity capability.

Candidate identifier: `nuif-opentype-variable-truetype-single-0`

RFC 0013 decomposes variable TrueType support from collections, CFF2, color,
bitmap, SVG-glyph, and WOFF2 capabilities. This file records only the first
implementation milestone: strict axis metadata admission and deterministic
coordinate normalization.

## Implemented boundary

`inspect_opentype_variable_metadata` applies the existing 32 MiB, 256-table,
single-face TrueType sfnt checks. It requires the static baseline tables plus
`fvar`, `gvar`, and `STAT`; checks directory packing and checksums before parser
use; and rejects collections, CFF/CFF2, color, bitmap, SVG, VARC, and nonzero
face selection.

The metadata parser independently reads and bounds:

- `fvar` 1.0 headers, up to 16 ordered unique axes and 256 named instances;
- exact 16.16 minimum, default, and maximum values, flags, and name IDs;
- complete named-instance coordinate tuples and optional PostScript name IDs;
- optional `avar` 1.0 maps with at most 256 records per axis, strict input and
  monotonic output order, and required `-1`, `0`, and `1` mappings.

It compares axis and instance metadata with Skrifa. A coordinate request must
contain every axis exactly once, contain only finite in-range values, and is
converted through the NUIF-owned OpenType 16.16 normalization, `avar` remapping,
and 2.14 rounding path. The final ordered vector must agree exactly with
Skrifa or evaluation fails.

## Independent evidence

`cargo xtask gate-i-font-metadata` checks one redistributed four-axis
`font-test-data` 0.9.1 fixture. The committed HarfBuzz 14.4.0 public-C-API
capture independently records axis metadata, named-instance count, and five
normalized vectors: default, minimum, maximum, and two interior positions. The
interior `wght` results exercise non-identity `avar` remapping. The capture can
be regenerated with `tools/font/capture_harfbuzz_variable.py`; offline CI reads
the committed result and does not require a system HarfBuzz library.

The fixture is distributed by the `font-test-data` package whose package
metadata declares `MIT OR Apache-2.0`; its embedded name record says Google LLC
and “All Rights Reserved.” That combination is recorded as test-distribution
evidence, not treated as an automated legal conclusion or sufficient publisher
approval for a portable NUIF release.

`cargo xtask gate-i-font-shaping` additionally applies that exact normalized
vector to a research-only `VariableResourceTrial`. Seven pinned `hb-shape`
cases agree with HarfRust at default, minimum, maximum, two interior locations,
and immediately below/at a GSUB FeatureVariations threshold. The same vector is
then used for Skrifa advances and unhinted outlines; repeats are exact and the
fixture's `gvar` changes the interior outline. Skrifa is shared implementation
evidence here, not an independent metric or outline oracle.

## Negative and security evidence

The gate rejects static, CFF2-variable, color-variable, and nonzero-face inputs,
as well as missing, unknown, out-of-range, and non-finite coordinates. Unit
tests compare twenty-one default/boundary/interior vectors with Skrifa and keep
all static-profile admission tests green.

The experiment does not yet mutate every `fvar`/`avar` count and offset, measure
allocation/time ceilings, or inspect `gvar`, HVAR, VVAR, MVAR, and STAT internal
graphs. Parser admission is therefore not promoted to package acceptance.

## Explicit non-claims and next gate

No variable font may yet pass `validate_packaged_font`, enter the evaluation
context, participate in layout/rendering, or claim lossless fidelity under this
candidate identifier. The next executable gate must add a rights-reviewed
multi-fixture corpus, independent outline and HVAR/MVAR metric oracles, malformed
variation-graph cases, and allocation/time ceilings before the shared
package/runtime path changes.
