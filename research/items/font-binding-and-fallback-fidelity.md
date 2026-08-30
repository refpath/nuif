---
id: nuif:research:font-binding-and-fallback-fidelity
kind: synthesis
status: reviewed
title: Requested font identity, replacement resources and item-level fallback fidelity
source:
  url: https://www.w3.org/TR/css-fonts-4/
  authors: [W3C CSS Working Group]
  published_at: null
  license: W3C Document License
retrieved_at: 2026-08-30
tags: [fonts, fallback, substitution, fidelity, resources, layout]
confidence: 0.98
claims: [nuif:claim:resource-identity-separation]
relations:
  - type: extends
    target: nuif:research:opentype-font-embedding-and-portability
  - type: extends
    target: nuif:research:text-rendering-reproducibility
links:
  spec: [spec/05-geometry-paint-text.md, spec/09-provenance-and-fidelity.md]
  rfc: [rfcs/0010-portable-resource-package.md]
  code: [crates/nuif-core, crates/nuif-layout, crates/nuif-render, crates/nuif-testing/src/bin/font-resources.rs]
  experiments: [nuif:experiment:font-resource-static-baseline]
---

# Summary

A requested font, the stable asset that records its portability decision and
the bytes actually used for layout are different facts. NUIF now binds a text
item to a font asset by optional `AssetId`, retains the requested SHA-256 on the
text, and derives an effective SHA-256 from the asset. This represents exact,
substituted and unavailable outcomes without treating a family name as exact
identity or overwriting authored intent.

## Evidence

- CSS Fonts Level 4 defines `font-family` as an ordered selection input and
  notes that a family name does not identify an individual face. Its fallback
  procedure may select a different installed font and can vary between user
  agents and operating systems.
- The same specification distinguishes downloadable `@font-face` resources
  from installed fonts and selects a fallback when the intended resource is
  unavailable. This proves that requested family, selected face and resource
  availability are separate state.
- CSS Font Loading Level 3 exposes `unloaded`, `loading`, `loaded` and `error`
  states for a face rather than pretending that every family request resolved.
  Source: https://www.w3.org/TR/css-font-loading-3/.
- OpenType `OS/2.fsType` describes embedding signals for bytes; it does not
  identify which text items requested those bytes or which replacement was
  chosen. Source:
  https://learn.microsoft.com/en-us/typography/opentype/spec/os2#fstype.

## Executable decision

`TextContent.font_sha256` remains the requested exact identity.
`TextContent.font_asset` is an optional stable semantic reference:

- absent: legacy executable profile 0 uses the requested hash directly;
- exact: the referenced font asset's resource hash must equal the requested
  hash;
- substituted: the requested hash is retained and the referenced asset's
  resource hash becomes the declared effective replacement;
- unavailable: the referenced asset has no resource and remains linked to the
  affected text item.

Core validation rejects a missing asset, a non-font target, an exact binding
whose digest differs from the request, or a bound usable asset without a valid
resource digest. Resolution is pure and performs no filesystem, network or
platform-font lookup.

Layout shapes with an available declared replacement and emits item-level
`approximated` fidelity. If the substitute is absent from the evaluation
context, or if the asset is unavailable, layout and rendering emit item-level
`unsupported` fidelity and rendering emits no false text command. Legacy
unbound profile-0 text retains its typed missing-context error.

The existing HTML, React, Svelte, SVG and Penpot profiles do not encode this
binding. Their exporters therefore reject it at profile inspection instead of
silently dropping it; their importers continue to create unbound text. The
same audit found that those profiles and the scalar DTCG profile did not encode
the document asset table at all, so every such exporter now rejects any
non-empty asset table before serialization.

## Alternatives rejected

- **Family-name matching:** ambiguous across faces and platforms and expressly
  insufficient for an exact-font claim.
- **Replace the requested hash:** loses authored intent and makes it impossible
  to say what was substituted.
- **Global substitution map only:** cannot represent different decisions for
  individual text items and weakens property-level fidelity.
- **Fidelity text without a stable asset reference:** reports loss but cannot
  connect an unavailable resource, policy evidence and affected item.
- **Implicit system fallback:** makes canonical output depend on the host and
  bypasses explicit resource authority.

## Evidence boundary

`cargo xtask gate-i-font` packages and decodes substituted and unavailable
assets, verifies the binding survives, exercises layout with and without the
replacement in the context, and proves renderer command/fidelity behavior in
six blocking trials. The declared replacement is the already pinned Ahem
resource, so this establishes whole-text item semantics, not a general fallback
engine.

Cluster-level fallback, missing-glyph reporting, multiple faces per run,
variable axes, feature-dependent substitution, shaping with arbitrary packaged
font bytes and cross-platform raster equivalence remain separate work.

## NUIF relevance

The binding keeps requested design intent, resource policy and the bytes used
for evaluation as separate core facts. Thin adapters can reject or report the
unsupported field without inventing host fallback, and layout/rendering derive
the same item-level fidelity from one authoritative resolution function.
