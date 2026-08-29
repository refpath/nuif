# Layout and rendering research synthesis

## Layout is not geometry

NUIF separates **authored constraints** from **resolved boxes**. Fixed x/y/width/height are valid authored values for freeform content, but they are not the universal layout representation.

The initial layout vocabulary contains families rather than one universal algorithm:

- `freeform` — transforms/anchors and explicit geometry.
- `stack` — one-dimensional flow with intrinsic sizing, distribution, alignment and gaps.
- `flex` — web-compatible flexible layout semantics.
- `grid` — track-based two-dimensional layout.
- `constraint` — relational linear constraints for editor/native-layout cases.
- `custom` — extension/dialect-defined evaluator with declared fallback/resolved geometry.

Common sizing primitives are normalized across families: fixed, intrinsic-min, intrinsic-max, fit-content, fill/available, percentage, min/max clamps, aspect ratio and content measurement.

Taffy is the recommended first evaluator for CSS-compatible block/flex/grid behavior because it implements web algorithms in Rust. SwiftUI's proposal-response model and Cassowary-style constraints demonstrate why the canonical schema must remain a superset rather than serializing Taffy's `Style` directly.

## Evaluation context

Resolved layout is keyed by an explicit context including viewport/container size, pixel ratio, locale, writing direction, font set, token/theme selection and feature/dialect capabilities. Multiple resolved snapshots may coexist as caches or conformance fixtures.

## Rendering semantics

The standard specifies the visual meaning of paths, fills, strokes, transforms, clipping, masks, gradients, compositing, images, text and supported effects. It does **not** standardize GPU command buffers or a renderer implementation.

The reference renderer uses a backend trait. Vello/wgpu is the leading interactive experiment, but conformance requires deterministic raster comparisons and must permit CPU reference rendering where GPU differences would make tests unstable.

## Text

Canonical text remains Unicode text + style runs + semantic annotations + font references. Shaping produces resolved glyph IDs, clusters, advances and offsets using pinned font data and a declared Unicode/shaping version. A glyph cache never replaces semantic text.

Portability reports must distinguish font substitution, missing glyphs, line-break differences and rasterization differences from document-model loss.
