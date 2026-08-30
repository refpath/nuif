---
id: nuif:research:png-image-preservation-and-decoding
kind: standard
status: reviewed
title: PNG third-edition preservation and deterministic decode inputs
source:
  url: https://www.w3.org/TR/png-3/
  authors: [W3C PNG Working Group]
  published_at: 2025-06-24
  license: W3C Document License
retrieved_at: 2026-08-30
tags: [png, images, color, alpha, metadata, decoding, reproducibility]
confidence: 0.99
claims: [nuif:claim:resource-identity-separation]
relations:
  - type: related_to
    target: nuif:research:text-rendering-reproducibility
    note: Both require exact source bytes plus a pinned interpretation stack for reproducible rendering.
  - type: related_to
    target: nuif:research:content-addressed-versioning
links:
  spec: [spec/05-geometry-paint-text.md, spec/08-serialization.md, spec/11-security.md]
  adr: [adrs/0003-reference-renderer.md]
  rfc: [rfcs/0010-portable-resource-package.md]
  code: [crates/nuif-render]
  experiments: [nuif:experiment:image-resource-profile]
---

# Summary

PNG is a lossless encoded raster format, but displaying it is not equivalent to
copying an RGBA array. The datastream can declare color through CICP, ICC, sRGB,
or gamma/chromaticity chunks; alpha samples are linear and unassociated; Exif
and other ancillary data may affect interpretation or preservation. NUIF should
retain original bytes as the authoritative resource and treat decoded pixels or
GPU textures as derived caches tied to a pinned decoder profile.

## Evidence

- PNG Third Edition §4.3 defines four color-signalling routes and their
  precedence: CICP, ICC, sRGB, then chromaticity plus gamma.
- §6.2 states that PNG color samples are not premultiplied by alpha. Alpha is a
  linear fraction of full opacity and is not gamma-corrected.
- §11.3 defines ancillary chunks including iCCP, sRGB, cICP, eXIf, physical
  pixel dimensions and textual data. A decode-only pipeline can therefore lose
  source information even when its visible pixels are acceptable.
- §13.14 and §13.16 discuss decoder color and alpha handling. The specification
  discourages unnecessary color conversion during format conversion because
  gamut and rounding loss can accumulate.

## Mechanism

The resource descriptor identifies the original PNG datastream. An image asset
records its intrinsic dimensions and the exact decoder profile used to obtain a
canonical pixel surface. `ImagePaint` records fit, crop rectangle, transform,
sampling and opacity; none of these values are inferred from the byte path.

```text
PNG bytes (authoritative, digest-pinned)
  -> bounded parser + pinned color/orientation policy
  -> straight-alpha reference pixels
  -> declared conversion/premultiplication at scene lowering
  -> optional decoded/GPU cache keyed by source digest + decoder profile
```

## NUIF relevance

**Borrow** PNG as the first image-resource profile because it has an open,
mature specification and supports lossless storage with alpha and explicit
color metadata.

**Adapt** the format into a stricter NUIF decoder profile that fixes accepted
color chunks, orientation handling, output color space, alpha conversion,
sampling, maximum dimensions, decoded bytes and ancillary-chunk budgets.

**Reject** replacing the original resource with decoded pixels, silently
discarding color metadata, or calling a screenshot crop the original asset.
Screenshot crops are derived resources with screenshot digest and crop region
in their provenance.

## Open questions

- Which PNG animation subset, if any, belongs in the first image profile?
- Should conflicting color chunks be rejected even where PNG defines a
  precedence, to avoid surprising cross-decoder behavior?
- Which decoder implementation/version becomes the independent reference and
  how will malformed-image differential testing be bounded?
