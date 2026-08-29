# Profile-0 text shaping fixtures

`harfbuzz-14.4.0-ahem.json` is an independent shaping and outline oracle captured with HarfBuzz 14.4.0 and the exact Ahem 1.50 bytes embedded by `font-test-data` 0.7.0. The font SHA-256 is `f0a92cd0cc45735591c9b5b1fa8aecd5194e8dc518895ca22af94a46c23550dc`.

The fixture compares glyph identifiers, Unicode-scalar cluster indices, offsets and advances with glyph names disabled. It also compares unhinted Skrifa 0.46.2 outlines in signed 26.6 font units with five normalized `hb-vector` paths; normalization removes `hb-vector`'s redundant explicit line to the contour start before close. `cargo xtask gate-d-text` repeats both stages, exercises both writing directions, checks typed font failures and writes `target/text-pinning-report.json`.

The CPU rasterizer uses the same unhinted outlines, pinned Zeno 0.3.3 8-bit grayscale nonzero-fill masks, a fixed 800-font-unit first baseline and alpha composition over encoded sRGB channels. Three context hashes reproduce on macOS/aarch64, Linux/aarch64 and Linux/x86_64. Profile 0 shapes CR/LF/CRLF/NEL/LS/PS hard lines independently, positions baselines by the authored line height, aligns to the inline-start edge and clips without automatic soft wrapping. The report classifies this deliberately bounded text profile as lossless; full UAX #14 soft wrapping is not claimed.
