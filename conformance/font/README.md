# Font oracle captures

This directory retains external observations for bounded font experiments.
Captures are inputs to offline conformance gates; they do not make the external
tool a build dependency or turn one fixture into a general format claim.

- `harfbuzz-14.4.0-ahem.json` records static Ahem metadata from `hb-info`.
- `harfbuzz-14.4.0-material-symbols-variable.json` records variable-axis
  metadata, named-instance count, five normalized 2.14 coordinate vectors, and
  seven shapes including a GSUB FeatureVariations boundary through HarfBuzz's
  public C API and `hb-shape`. It also records horizontal advances and canonical
  26.6 paths from HarfBuzz draw callbacks at every shaping location. Regenerate it with
  `python3 tools/font/capture_harfbuzz_variable.py <font>` and compare the exact
  font digest before review.
- `harfbuzz-14.4.0-hvar-truncated-map.json` records nonzero horizontal
  advances from a valid truncated HVAR advance-index map. Regenerate it with
  `python3 tools/font/capture_harfbuzz_hvar.py <font>` and the exact fixture
  named in the capture. It does not establish MVAR, VVAR, side-bearing, or
  broad HVAR support.
- `harfbuzz-14.4.0-roboto-flex-mvar.json` records global MVAR metrics and
  shaping at eight locations in the reproducible two-glyph OFL-1.1 Roboto Flex
  subset under `fixtures/`. Regenerate it with
  `python3 tools/font/capture_harfbuzz_mvar.py <font>`. The fixture provenance,
  license, preparation command, and exact source/derived digests are retained
  beside the font.
- `cargo xtask gate-i-font-security` runs the three variable fixtures through
  structural `gvar`, HVAR, MVAR, item-variation-store, and STAT preflight. Its
  37 checksum-repaired hostile mutations—including packed point/delta runs—and
  four allocation/time trials write
  `target/variable-font-security-report.json`; the measured ceilings are
  reference-implementation regressions, not portable format limits.
- `cargo xtask gate-i-font-package` validates the proposed asset metadata and
  policy for the OFL fixture, proves resource-only embedded package fixpoint and
  exact-byte retention across an unrelated edit, and exercises explicit
  digest-pinned linked resolution. Typed package binding remains deliberately
  rejected until the runtime consumes the same normalized coordinates.

The variable fixture comes from `font-test-data` 0.9.1. Its package metadata
declares `MIT OR Apache-2.0`, while the font's embedded copyright string is
retained as a separate policy fact. Neither the capture nor parser acceptance
is a license or redistribution determination.
