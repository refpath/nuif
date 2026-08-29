# 05 — Geometry, paint and text

Status: draft.

Geometry follows established 2D vector mathematics: affine transforms, rectangles/rounded rectangles, ellipses, lines and Bézier paths. Path semantics SHOULD align with SVG where possible.

Paint supports solid colors, gradients, images, strokes, opacity, clipping/masks and compositing/blend modes. Color values MUST declare a color space; conversions are evaluator responsibilities.

Text stores Unicode scalar content, style runs, paragraph attributes, direction/language hints and font references. Resolved text MAY store shaped glyph runs keyed by font hashes, shaping configuration and Unicode data version.

Implementations MUST preserve source text even when resolved glyph information is present.
