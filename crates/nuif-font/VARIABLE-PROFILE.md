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

Before returning metadata it also preflights the connected variation graph:
`gvar` version, axis/glyph counts, shared tuples, glyph-data offsets, tuple
records and explicit deltas; HVAR/MVAR item variation stores, regions, row
widths and delta-set references; MVAR record ordering; and STAT axes, values,
flags and cross-table references. Reserved raw flag bits are checked from the
original bytes because generated bitflag accessors intentionally discard them.
VVAR remains outside this candidate horizontal-text boundary.

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
then used for Skrifa advances and unhinted outlines. Every advance and
canonical 26.6 path agrees exactly with HarfBuzz's public metric and draw
callbacks; repeats are exact and the fixture's `gvar` changes the interior
outline. That fixture's HVAR store has no active variation regions and it has
no MVAR table.

`cargo xtask gate-i-font-metrics` adds a second `font-test-data` fixture whose
HVAR store changes three glyph advances across four `wght` locations. HarfRust
shaping and Skrifa metrics agree exactly with pinned HarfBuzz 14.4.0 public-API
observations at every location. The fixture deliberately uses a valid
truncated advance-index map, so the test also covers the OpenType rule that
missing trailing map entries reuse the final present entry. This is one narrow
horizontal-metric case; VVAR, side-bearing variation, `gvar` phantom-point
fallback, and a broader HVAR corpus remain untested.

`cargo xtask gate-i-font-global-metrics` adds a reproducible two-glyph Roboto
Flex subset under OFL-1.1 with exact source, license, preparation, and derived
digests. At eight complete 13-axis locations, Skrifa's MVAR-adjusted x-height,
cap height, ascent, descent, and line gap agree exactly with HarfBuzz 14.4.0's
public metric API. The `YTLC`, `YTUC`, and `opsz` cases distinguish targeted
global-metric deltas from glyph shaping. This is one MVAR store and not broad
MVAR, rights-policy, or package/runtime evidence.

`cargo xtask gate-i-font-corpus` adds independently authored Noto Sans and
Recursive subsets under OFL-1.1. Exact registry commits, upstream revisions,
font/license/source/output digests, `hb-subset` 14.4.0 commands, and retained
license texts make both fixtures reproducible and reviewable. Across eight
default/minimum/maximum/interior locations, NUIF agrees with pinned HarfBuzz
14.4.0 on 2- and 5-axis metadata and normalized coordinates, shaping, HVAR
advances, and MVAR global metrics. Seven unhinted outlines agree exactly; the
Recursive five-axis interior outline has identical topology and one control
coordinate differs by one 26.6 unit, so the cross-implementation rule permits
at most that measured 1/64-font-unit tie. The two graph shapes cover 639 `gvar`
tuples, 24,024 explicit deltas, 11 HVAR data subtables, and four MVAR data
subtables between them. Both exact assets pass candidate metadata/policy
validation before the separate typed package/runtime gate consumes them.

`cargo xtask gate-i-font-gvar-generated` independently constructs canonically
packed and checksummed sfnt values around a synthetic 300-point glyph, then
runs the production whole-font inspector. Sixteen accepted cases cover the
one-/two-byte point-count boundary, all/private/shared point lists, repeated
points, byte/word/alternating point runs, zero/byte/word/mixed delta runs,
128-point and 64-delta run maxima, private-over-shared precedence, multiple
tuples, phantom-point indices, and the maximum 32,767 packed-point count. Three
named counterexamples reject noncanonical or truncated two-byte counts. The
generated fonts are ephemeral parser inputs, not distributable or rendering
fixtures.

## Negative and security evidence

The gate rejects static, CFF2-variable, color-variable, and nonzero-face inputs,
as well as missing, unknown, out-of-range, and non-finite coordinates. Unit
tests compare twenty-one default/boundary/interior vectors with Skrifa and keep
all static-profile admission tests green.

`cargo xtask gate-i-font-security` repairs every sfnt checksum after 38 hostile
mutations, then requires the profile—not checksum handling—to reject invalid
`gvar`, HVAR, MVAR, item-variation-store, and STAT relationships. Nine `gvar`
cases exercise tuple flags/header extents, normalized shared tuples, packed
point counts/runs/bounds, tuple body sizes, packed delta counts, and rejection
of the non-OpenType 32-bit delta extension. The
gate also measures five accepted fixtures after warmup: each inspection
must allocate no more than 8 MiB, retain no more than 2 MiB, and finish within
500 ms; one early malformed graph must reject below 256 KiB allocated. Graph
limits cap 4,096 shared tuples, 65,536 tuple records, 4,194,304 explicit deltas,
32,767 variation regions, 65,535 data subtables, 65,536 delta rows, 1,048,576
region references, 256 MVAR records, and 4,096 STAT values.

These ceilings are implementation regression guards measured on one machine,
not portable timing or allocation semantics. Byte-exhaustive packed-input
enumeration, VVAR, and process-level cancellation/sandbox evidence remain open;
the declared count/run/type packing boundaries are now generated. Parser
admission alone therefore grants neither package capability authorization nor
runtime fidelity.

`cargo xtask gate-i-font-package` adds a candidate asset validator that checks
exact variable bytes, complete coordinates, names, coverage, feature bounds,
`fsType`, decoder profile, license expression, explicit embedding review, and
portability as one transaction. Twenty-one trials prove resource-only package
fixpoint, exact-byte retention across an unrelated semantic edit, declared
capability negotiation, explicit digest-pinned linked resolution, typed
admission and eleven stale/policy rejections. Omitting the exact decoder
capability rejects the package before evaluation.

## Explicit non-claims and continuation

`cargo xtask gate-i-font-runtime` now admits a typed variable asset only behind
the declared capability, retains the exact normalized coordinates in resolved
runs, matches default/interior HarfBuzz shaping and `gvar` paths, and drives
HVAR intrinsic layout plus deterministic CPU pixels with lossless item
fidelity. Cross-surface parity remains off until each applicable binding or
process adapter proves the same canonical hash, coordinate record, diagnostics
and fidelity; VVAR and other font profiles remain separate.
