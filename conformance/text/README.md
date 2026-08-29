# Profile-0 text shaping fixtures

`harfbuzz-14.4.0-ahem.json` is an independent shaping oracle captured with HarfBuzz 14.4.0 and the exact Ahem 1.50 bytes embedded by `font-test-data` 0.7.0. The font SHA-256 is `f0a92cd0cc45735591c9b5b1fa8aecd5194e8dc518895ca22af94a46c23550dc`.

The fixture compares glyph identifiers, Unicode-scalar cluster indices, offsets and advances with glyph names disabled. `cargo xtask gate-d-text` evaluates the fixture through pinned HarfRust 0.13.3/Unicode 17.0.0, repeats shaping, exercises both writing directions, checks typed font failures and writes `target/text-pinning-report.json`.

This fixture proves the declared shaping subset only. The current CPU renderer positions shaped runs but draws a deterministic glyph-ID bitmap proxy. Reports classify that stage as `approximated`; no outline or cross-platform raster exactness is claimed until Gate D defines and tests outline extraction, hinting, grayscale coverage, subpixel quantization and blending.
