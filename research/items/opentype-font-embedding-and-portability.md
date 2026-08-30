---
id: nuif:research:opentype-font-embedding-and-portability
kind: standard
status: reviewed
title: OpenType embedding permissions and reproducible font resources
source:
  url: https://learn.microsoft.com/en-us/typography/opentype/spec/os2#fstype
  authors: [Microsoft, Adobe]
  published_at: null
  license: OpenType specification terms
retrieved_at: 2026-08-30
tags: [fonts, opentype, embedding, licensing, shaping, portability]
confidence: 0.98
claims: [nuif:claim:resource-identity-separation]
relations:
  - type: extends
    target: nuif:research:text-rendering-reproducibility
    note: Adds packaging and redistribution policy to the already-pinned shaping/raster inputs.
  - type: related_to
    target: nuif:research:harfbuzz-unicode
  - type: related_to
    target: nuif:research:ttf-parser
  - type: related_to
    target: nuif:research:fontations
links:
  spec: [spec/05-geometry-paint-text.md, spec/08-serialization.md, spec/11-security.md]
  adr: [adrs/0003-reference-renderer.md]
  rfc: [rfcs/0010-portable-resource-package.md]
  code: [crates/nuif-text, crates/nuif-font, crates/nuif-package, crates/nuif-testing/src/bin/font-resources.rs]
  experiments: [nuif:experiment:font-resource-static-baseline, nuif:experiment:font-resource-profile]
---

# Summary

Reproducible text requires exact font bytes, but possessing or using a font does
not automatically permit redistribution inside a portable package. OpenType's
`OS/2.fsType` flags provide machine-readable embedding signals—installable,
restricted, preview/print, editable, no-subsetting and bitmap-only—but those
flags are part of a wider license context and must not be treated as a complete
legal decision engine.

## Evidence

- OpenType 1.9.1 `OS/2`, `fsType` defines mutually exclusive usage permissions:
  installable (0), restricted (2), preview/print (4) and editable (8).
- Bit 8 forbids subsetting and bit 9 permits only embedded bitmaps. Reserved
  bits and historical version differences mean parsers must validate the table
  version and length rather than assuming one modern layout.
- Versions 0 through 2 historically permitted multiple usage bits with a
  least-restrictive interpretation, while version 3 made those bits mutually
  exclusive. The first NUIF profile deliberately rejects ambiguous historical
  combinations instead of silently selecting a permission.
- The specification says embedding-aware applications must not embed fonts
  whose permissions do not allow embedding or alter the flags, and notes that
  rights are granted by the font vendor.
- CSS Fonts Level 4 defines face selection, variation axes, feature settings and
  font fallback as separate inputs to rendered text. CSS Font Loading Level 3
  exposes document font readiness; capture before fonts settle is not a stable
  observation.

## Mechanism

A NUIF font asset points to exact bytes when policy permits and records at
least: media type, SHA-256, face or collection index, names used for matching,
variation axes, selected features, character coverage and embedding-policy
evidence. A text run still records the shaping inputs required by its profile.

The packaging policy is explicit:

- `portable`: exact bytes embedded and permitted for the declared use;
- `private_authoring`: bytes retained only in an access-controlled workspace,
  not a distributable package;
- `linked`: expected digest and resolver hint recorded, no implicit fetch;
- `substituted`: replacement bytes and item-level fidelity recorded;
- `unavailable`: metrics/evidence may be retained, but no false exactness claim.

## NUIF relevance

**Borrow** the `fsType` signal and exact OpenType face/variation metadata.

**Adapt** it into a conservative policy decision that also accepts explicit
license metadata and user/admin policy. The file-format validator reports the
facts; it does not provide legal advice.

**Reject** family name as font identity, system-font discovery for exact
profiles, silent fallback, automatic embedding from a browser's platform-font
name, and a claim that `fsType == 0` alone proves redistribution rights.

## Implemented narrow baseline

`nuif-opentype-static-single-0` accepts only one checksummed, canonically packed
TrueType-outline sfnt face. It rejects TTC, CFF/CFF2, variable, color, bitmap,
SVG and WOFF/WOFF2 sources. Package validation compares face, family names,
static axis state and exact Unicode coverage, then requires matching `fsType`,
a non-empty license expression and an explicit `approved` embedding review.

`cargo xtask gate-i-font` compares `ttf-parser` 0.25.1 against Skrifa 0.46.2 on
the pinned Ahem resource, proves package byte fixpoint and resource retention,
and runs malformed, checksum, directory, policy and one-over trials. This is an
automated baseline, not completion of the broader font-resource experiment.

## Open questions

- Which explicit license-expression vocabulary is reliable enough to augment
  `fsType` without pretending to automate legal interpretation?
- Can a portable profile subset a font only when both the license signal and
  shaping corpus permit it, while retaining an audit link to the source digest?
- How should variable-font instancing be represented when the original file may
  not be redistributed but a licensed derived instance may be?
