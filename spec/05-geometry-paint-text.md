---
id: nuif:spec:geometry-paint-text
kind: specification
status: draft
---

# 05 — Geometry, paint and text

Status: draft.

Geometry follows established 2D vector mathematics: affine transforms, rectangles/rounded rectangles, ellipses, lines and Bézier paths. Path semantics SHOULD align with SVG where possible.

Paint supports solid colors, gradients, images, strokes, opacity, clipping/masks and compositing/blend modes. Color values MUST declare a color space; conversions are evaluator responsibilities.

Text stores Unicode scalar content, style runs, paragraph attributes, direction/language hints and content-addressed font references. A font SHA-256 reference MUST contain 64 lowercase hexadecimal digits. Font size and line height MUST be finite and positive.

A conformance profile that compares resolved text MUST declare the exact font bytes and hash, shaper and Unicode-data versions, direction, language, script-selection rule, feature set, cluster level, cluster coordinate unit, positioning unit and resource limits. Resolved runs contain source text plus ordered glyph identifiers, clusters, advances and offsets; they MUST NOT depend on system font discovery. Profile 0 uses Unicode-scalar indices for cluster coordinates and unscaled font units for advances and offsets.

Shaping and rasterization are distinct conformance stages. A shaping pass does not imply raster conformance. A raster profile MUST additionally declare outline extraction, hinting, stem darkening, anti-aliasing, subpixel quantization, color/blend space and compositing rules. Until those parameters and their foreign/cross-platform trials exist, an implementation MUST classify a glyph-ID bitmap proxy as `approximated` rather than exact text rendering.

Implementations MUST preserve source text even when resolved glyph information is present.

## CPU render profile 0

Profile 0 is deliberately narrower than the complete model. Its supported visual operations are encoded-sRGB solid fills of rectangles and ellipses, plus the text subset below. Color channels are finite numbers in the inclusive range 0 through 1. The reference raster starts as opaque white RGBA8.

Logical geometry is multiplied by the target scale factor before rasterization. A rectangle covers every pixel in `floor(x)..ceil(x + width)` and `floor(y)..ceil(y + height)`, clipped to the target; it does not compute fractional edge coverage. Float color channels become bytes with `round(clamp(channel, 0, 1) × 255)`. For mask coverage `coverage`, effective source alpha is `(source_alpha × coverage + 127) / 255` using integer division. Each encoded-sRGB destination color channel becomes `(source_channel × alpha + destination_channel × (255 - alpha) + 127) / 255`; output alpha remains 255.

An ellipse is the closed four-cubic path inscribed in its bounds using control coefficient `0.551915024494`. It is rasterized with nonzero fill into an 8-bit grayscale mask by Zeno 0.3.3, crates.io checksum `6df3dc4292935e51816d896edcd52aa30bc297907c26167fec31e2b0c6a32524`, then composited by the integer rule above. `conformance/render/profile-zero-v1.json` fixes rectangle and ellipse scene/PNG hashes on the recorded platform matrix.

Profile-0 text uses Ahem 1.50, HarfRust 0.13.3 with Unicode 17.0.0, unhinted Skrifa 0.46.2 outlines in signed 26.6 font units, and Zeno 0.3.3 grayscale masks. CRLF is one hard break; CR, LF, NEL, LINE SEPARATOR and PARAGRAPH SEPARATOR are individual hard breaks. Each hard line is shaped independently. Intrinsic width is the greatest shaped line advance; intrinsic height is the number of hard lines times `line_height`. The first baseline is 800 Ahem font units below the line top, subsequent baselines differ by `line_height`, LTR starts at the left edge, RTL starts at the right edge, and output is clipped to the text box. Profile 0 performs no automatic soft wrapping. Because wrapping is not an authored property in this profile, absence of soft wrapping is exact profile behavior rather than an approximation.

Path geometry, image assets, component-instance materialization and extension-defined paint/effects are not supported by this profile. Lowering MUST emit `unsupported` or `preserved_unrenderable` fidelity with the originating entity and property pointer; it MUST NOT substitute bounds rectangles or silently omit the data. Future profiles may add these operations without changing profile-0 results.
