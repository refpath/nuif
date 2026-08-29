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

Profile 0 does not yet define Grid track sizing or item placement fields. A `grid` family value therefore cannot claim lossless Grid layout and the reference evaluator emits its declared stack fallback. Gate C classifies the resulting foreign-reference differences as schema loss; this is not Grid conformance.

## Differential context

CSS-compatible layout claims are checked against exact pinned Taffy and browser versions. The machine report MUST retain the generator source revision, browser executable/version, seed, viewport, all compared boxes, per-fixture measured tolerance and a typed classification for every value outside that tolerance. A global unexplained tolerance is not conforming evidence.
