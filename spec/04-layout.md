# 04 — Layout

Status: draft.

NUIF defines authored layout separately from resolved layout.

## Core families

`freeform`, `stack`, `flex`, `grid`, `constraint` and extension-defined `custom`.

## Shared sizing

Axes support fixed, auto/intrinsic, min-content, max-content, fit-content, percentage and fill/available sizing plus min/max clamps and aspect ratio.

## Responsive rules

Conditional authored values use predicates over declared evaluation features: viewport/container dimensions, orientation, input capability, theme/media preference and named application conditions. Predicates MUST be deterministic for a supplied evaluation context.

## Resolved layout

An evaluator emits boxes/transforms plus diagnostics and the context fingerprint. Resolved geometry may be cached/serialized but is derived unless the authored family is freeform.

## Portability

Lowering to a target layout model MUST return fidelity records for every rule that is approximated, preserved-unrenderable or unsupported.
