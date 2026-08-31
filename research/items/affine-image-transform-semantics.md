---
id: nuif:research:affine-image-transform-semantics
kind: standard
status: reviewed
title: Forward affine image-paint coordinates and inverse sampling
source:
  url: https://www.w3.org/TR/css-transforms-1/
  authors: [W3C CSS Working Group]
  published_at: 2019-02-14
  license: W3C Document License
retrieved_at: 2026-08-30
tags: [images, transform, sampling, figma, canvas, coordinates]
confidence: 0.98
claims: [nuif:claim:resource-identity-separation]
relations:
  - type: related_to
    target: nuif:research:png-image-preservation-and-decoding
links:
  spec: [spec/05-geometry-paint-text.md]
  rfc: [rfcs/0010-portable-resource-package.md]
  code: [crates/nuif-render]
  experiments: [nuif:experiment:image-resource-rgba8-baseline]
---

# Summary

An image transform needs a declared coordinate direction, matrix layout,
reference box, composition order and clipping rule. Six unnamed numbers are not
interoperable semantics. NUIF uses the conventional forward 2D affine matrix
layout and inverse-maps destination pixel centers during rasterization.

## Evidence

- CSS Transforms Level 1 defines the current transformation matrix as the
  mapping from local coordinates into the parent/viewport coordinate system and
  represents `matrix(a,b,c,d,e,f)` as `[a c e; b d f; 0 0 1]`.
- The HTML Canvas transform API uses the same six-value layout. Rendering under
  a current transformation therefore has the same forward-coordinate reading,
  even though a rasterizer normally evaluates it through inverse sampling.
- Figma's current plug-in `Transform` is the top two rows of an affine 3×3
  matrix, with identity `[[1,0,0],[0,1,0]]`; its `ImagePaint.imageTransform`
  controls crop positioning. This supports a direct adapter mapping for the
  matrix values, but does not by itself specify every NUIF fit/crop/sampling
  interaction. Sources: https://developers.figma.com/docs/plugins/api/Transform/
  and https://developers.figma.com/docs/widgets/api/type-ImagePaint/.

## Executable decision

After crop selection and fit calculation, `(u,v)` denotes the selected crop's
unit square and `(p,q)` denotes normalized coordinates in the untransformed
fitted rectangle. The authored matrix is forward:

```text
[p]   [a c tx] [u]
[q] = [b d ty] [v]
[1]   [0 0  1] [1]
```

The reference rasterizer clips to the entity rectangle, evaluates destination
pixel centers, applies the exact inverse matrix, rejects samples outside the
half-open crop unit square, then performs the declared nearest or fixed-weight
bilinear sample. Fit precedes this matrix. Crop selection follows inverse
mapping. The transform origin is `(0,0)`; rotation around the center is encoded
by the caller in `tx`/`ty`.

The executable bound accepts finite components with absolute value at most
1,000,000, determinant magnitude at least `1e-12`, and inverse components at
most 1,000,000. Singular or numerically unbounded authored transforms remain in
the document but lower to item-level unsupported fidelity. A manually supplied
invalid render command is rejected atomically.

Flip, clockwise rotation, translation, singular-matrix and repeatability
fixtures run through `cargo xtask gate-i-image`. This proves the reference CPU
semantics. It does not prove that a vendor host uses the same fit/crop
composition until live adapter trials compare named host versions.

## NUIF relevance

The declared direction and inverse-sampling rule make image transforms
portable core values instead of renderer-specific conventions. Adapters can
classify a target mismatch explicitly, while the reference renderer and future
foreign implementations share one falsifiable coordinate contract.
