# Layout and rendering research synthesis

## Authored and resolved layout

NUIF separates **authored constraints** from **resolved boxes**. Fixed x/y/width/height are valid authored values for freeform content, but they are not the universal layout representation.

The proposed layout vocabulary contains several families. Implemented subsets
and refusal behavior are defined in [layout profile 0](../../spec/04-layout.md):

- `freeform` — transforms/anchors and explicit geometry.
- `stack` — one-dimensional flow with intrinsic sizing, distribution, alignment and gaps.
- `flex` — web-compatible flexible layout semantics.
- `grid` — bounded explicit fixed/`fr` tracks, spans and deterministic
  no-implicit-track placement in profile 0; broader CSS Grid features remain
  capability-reported adapter input.
- `constraint` — relational linear constraints for editor/native-layout cases.
- `custom` — extension/dialect-defined evaluator with declared fallback/resolved geometry.

Common sizing primitives are normalized across families: fixed, intrinsic-min, intrinsic-max, fit-content, fill/available, percentage, min/max clamps, aspect ratio and content measurement.

Taffy is the reference evaluator selected by
[ADR 0002](../../adrs/0002-layout-engine.md). Its types remain behind the NUIF
layout interface. The proposed vocabulary also accounts for proposal–response
layout and linear constraints; these do not acquire Taffy compatibility by
being represented in the schema.

## Evaluation context

Resolved layout is keyed by an explicit context including viewport/container size, pixel ratio, locale, writing direction, font set, token/theme selection and feature/dialect capabilities. Multiple resolved snapshots may coexist as caches or conformance fixtures.

## Rendering semantics

The draft specification defines the visual meaning of paths, fills, strokes, transforms, clipping, masks, gradients, compositing, images, text and supported effects. It does not specify GPU command buffers or a renderer implementation.

The reference renderer separates scene evaluation from rasterization. The
[rendering decision](../../adrs/0003-reference-renderer.md) defines the backend boundary;
[render conformance](../../conformance/HARNESS.md) specifies exact and tolerance
criteria. Interactive GPU output does not define normative pixel identity.

## Text

The proposed text model retains Unicode content, style runs, semantic
annotations and font references. Shaping produces resolved glyph IDs, clusters, advances and offsets using pinned font data and a declared Unicode/shaping version. A glyph cache never replaces semantic text.

Portability reports must distinguish font substitution, missing glyphs, line-break differences and rasterization differences from document-model loss.
