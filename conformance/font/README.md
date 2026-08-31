# Font oracle captures

This directory retains external observations for bounded font experiments.
Captures are inputs to offline conformance gates; they do not make the external
tool a build dependency or turn one fixture into a general format claim.

- `harfbuzz-14.4.0-ahem.json` records static Ahem metadata from `hb-info`.
- `harfbuzz-14.4.0-material-symbols-variable.json` records variable-axis
  metadata, named-instance count, five normalized 2.14 coordinate vectors, and
  seven shapes including a GSUB FeatureVariations boundary through HarfBuzz's
  public C API and `hb-shape`. Regenerate it with
  `python3 tools/font/capture_harfbuzz_variable.py <font>` and compare the exact
  font digest before review.

The variable fixture comes from `font-test-data` 0.9.1. Its package metadata
declares `MIT OR Apache-2.0`, while the font's embedded copyright string is
retained as a separate policy fact. Neither the capture nor parser acceptance
is a license or redistribution determination.
