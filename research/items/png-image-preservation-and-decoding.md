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
  code: [crates/nuif-media, crates/nuif-render]
  experiments: [nuif:experiment:image-resource-rgba8-baseline, nuif:experiment:image-resource-profile]
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

## Resolved policy decisions

- Animation remains outside static decoder profiles; accepting APNG requires a
  separate time/frame/composition contract.
- Profiles zero and one reject conflicting or undeclared colour metadata even
  where PNG defines precedence. This makes encoded-sRGB interpretation explicit
  and prevents decoder-specific colour conversion.
- Rust `png` 0.18.1 is the implementation decoder and independently implemented
  `zune-png` 0.5.2 is the differential oracle. Both run with explicit
  allocation bounds and integrity checks.

## Executable profiles

`nuif-png-rgba8-0` answers the ambiguity question by accepting only
non-interlaced RGBA8 with no colour metadata or one valid pre-image `sRGB`
chunk. It interprets both cases as encoded sRGB, rejects every other ancillary
chunk and orientation/animation metadata, and caps encoded bytes, dimensions,
pixels, decoded bytes and chunk count. `cargo xtask gate-i-image` compares
`png` 0.18.1 with independently implemented `zune-png` 0.5.2 across every PNG
row filter, then exercises exact package retention, resource-aware lowering,
repeatable CPU rendering and hostile one-over cases.

`nuif-png-basic-rgba8-1` is a separate compatible expansion. It admits all
non-interlaced PNG colour/depth combinations that normalize to RGBA8 without
sample-precision loss: 1/2/4/8-bit greyscale and indexed colour, RGB8,
greyscale-alpha8 and RGBA8. Required palettes and valid `tRNS` transparency are
expanded exactly. Thirteen fixtures span every admitted colour/depth
combination and both colour-key and indexed transparency; both decoders
produce identical normalized RGBA bytes. The original bytes remain the asset
identity, and a profile-one RGB resource passes the same scene/raster path.

The wider profile deliberately rejects 16-bit samples rather than truncating
precision, and still rejects interlace, the complete PNG Third Edition colour
precedence model, orientation, animation and arbitrary ancillary metadata.
A real-world corpus, live host affine equivalence, GPU comparison and hosted
cross-platform image-raster reproduction remain open evidence—not implied by
decoder agreement on the generated fixtures.
