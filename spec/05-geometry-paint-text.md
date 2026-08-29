# 05 — Geometry, paint and text

Status: draft.

Geometry follows established 2D vector mathematics: affine transforms, rectangles/rounded rectangles, ellipses, lines and Bézier paths. Path semantics SHOULD align with SVG where possible.

Paint supports solid colors, gradients, images, strokes, opacity, clipping/masks and compositing/blend modes. Color values MUST declare a color space; conversions are evaluator responsibilities.

Text stores Unicode scalar content, style runs, paragraph attributes, direction/language hints and content-addressed font references. A font SHA-256 reference MUST contain 64 lowercase hexadecimal digits. Font size and line height MUST be finite and positive.

A conformance profile that compares resolved text MUST declare the exact font bytes and hash, shaper and Unicode-data versions, direction, language, script-selection rule, feature set, cluster level, cluster coordinate unit, positioning unit and resource limits. Resolved runs contain source text plus ordered glyph identifiers, clusters, advances and offsets; they MUST NOT depend on system font discovery. Profile 0 uses Unicode-scalar indices for cluster coordinates and unscaled font units for advances and offsets.

Shaping and rasterization are distinct conformance stages. A shaping pass does not imply raster conformance. A raster profile MUST additionally declare outline extraction, hinting, stem darkening, anti-aliasing, subpixel quantization, color/blend space and compositing rules. Until those parameters and their foreign/cross-platform trials exist, an implementation MUST classify a glyph-ID bitmap proxy as `approximated` rather than exact text rendering.

Implementations MUST preserve source text even when resolved glyph information is present.
