---
id: nuif:spec:layout
kind: specification
status: draft
---

# 04 — Layout

Status: draft.

NUIF defines authored layout separately from resolved layout.

## Core families

`freeform`, `stack`, `flex`, `grid`, `constraint` and extension-defined `custom`.

## Shared sizing

Axes support fixed, auto/intrinsic, min-content, max-content, fit-content, percentage and fill/available sizing plus min/max clamps and aspect ratio.

For the CSS-compatible stack/flex subset, `stretch` applies only when the item's cross-axis size is `auto`; an explicit fixed, intrinsic, percentage or fit-content cross size wins. `fill` is an explicit request for the available cross size regardless of the container's alignment. This distinction is covered by the Gate C foreign-reference regression.

## Responsive rules

Conditional authored values use predicates over declared evaluation features: viewport/container dimensions, orientation, input capability, theme/media preference and named application conditions. Predicates MUST be deterministic for a supplied evaluation context.

## Resolved layout

An evaluator emits boxes/transforms plus diagnostics and the context fingerprint. Resolved geometry may be cached/serialized but is derived unless the authored family is freeform.

## Portability

Lowering to a target layout model MUST return fidelity records for every rule that is approximated, preserved-unrenderable or unsupported.

## Bounded explicit Grid

Profile 0 defines a deliberately finite Grid subset. It is a portable authored
layout primitive, not an alias for the complete CSS Grid algorithm.

A `grid` container MUST declare one or more `columns` and one or more `rows` in
its `layout.grid` value. Each track is either:

- `fixed(n)`, where `n` is a positive finite pixel length; or
- `fraction(n)`, where `n` is a positive finite flexible weight.

Profile 0 permits at most 256 tracks on either axis and 4,096 grid tracks in a
document. It has no intrinsic, percentage, auto, repeated, named, subgrid,
masonry or implicit tracks. Importers MUST report those foreign features as a
loss, approximation or preserved extension rather than silently lowering them
to this subset.

The one container `gap` is used for both axes. For one axis, let `inner` be the
non-negative content-box length after padding, `gaps` be `gap * (track_count -
1)`, `fixed` be the sum of fixed tracks, and `weight` be the sum of fractional
weights. Track sizes are:

```text
remaining = max(0, inner - gaps - fixed)
fr        = remaining / max(1, weight)
fixed(n)  = n
fraction(n) = n * fr
```

The `max(1, weight)` rule intentionally follows CSS Grid's fractional sizing
rule for a total flex factor below one. Fixed tracks can overflow the content
box. Fractional tracks never receive negative space, and a fractional weight
sum below one can leave unused space at the end of the axis.

Grid-item `column` and `row` are zero-based explicit track indices. An item MUST
provide both or neither. `column_span` and `row_span` default to one and MUST be
positive. Explicitly positioned items reserve their complete rectangular areas
before any auto placement, independent of child order. Overlap and
out-of-bounds areas are invalid.

Items with neither index are placed in child order. `auto_flow: row` scans rows
then columns; `auto_flow: column` scans columns then rows. Scanning begins at the
first cell and a cursor advances past the last placed item's span. Each item
occupies the first unoccupied rectangle at or after that cursor that fits its
spans. The cursor does not move backwards to fill earlier holes; this is the
sparse CSS `grid-auto-flow` behaviour, not `dense`. The explicit grid is
exhausted when no such rectangle exists; an evaluator MUST NOT create implicit
tracks.

An item's grid area includes the gaps crossed by its spans. `fill` consumes the
area on that axis. `auto` consumes the area when container alignment is
`stretch`, otherwise it resolves to intrinsic size. Other size intents resolve
against the grid-area length. `start`, `center` and `end` place the resulting
box on both axes; `stretch` places it at the area's start and only stretches
`auto` or `fill`. Authored freeform position is ignored for an in-flow grid
item. Descendants are evaluated within the resulting item box.

Using `layout.grid` on another layout family, using non-default grid placement
without a direct Grid parent, or using the `grid` family without valid explicit
tracks is invalid. This removes the former stack-flow fallback: a conforming
implementation either implements these semantics or reports the Grid feature
as unsupported.

## Differential context

CSS-compatible layout claims are checked against exact pinned Taffy and browser versions. The machine report MUST retain the generator source revision, browser executable/version, seed, viewport, all compared boxes, per-fixture measured tolerance and a typed classification for every value outside that tolerance. A global unexplained tolerance is not conforming evidence.
