---
id: nuif:research:svg
kind: standard
status: reviewed
title: SVG 2 vector graphics model
source:
  url: https://www.w3.org/TR/SVG2/
  authors: [W3C SVG Working Group]
  published_at: null
  license: W3C document license
retrieved_at: 2026-08-29
tags: [vector, geometry, paint, paths, interoperability]
confidence: 0.99
claims: []
relations: []
links:
  spec: [spec/05-geometry-paint-text.md]
  adr: []
  rfc: []
  code: [adapters/README.md, adapters/STATUS.md]
  experiments: []
---
# Summary

SVG 2 defines an XML vocabulary for nested graphics elements, coordinate
systems, geometry, paint, text, reuse and accessibility metadata. SVG element
identity uses the XML `id` attribute. The `g` element groups descendants without
introducing NUIF component or layout semantics.

## Evidence

- SVG 2 §5.1–5.2 defines an SVG document fragment and the `svg` element. The
  `viewBox` and viewport establish coordinate-system mappings; they are not
  equivalent to an authored responsive layout rule.
  https://www.w3.org/TR/SVG2/struct.html#NewDocument and
  https://www.w3.org/TR/SVG2/coords.html#ViewBoxAttribute (retrieved
  2026-08-29).
- SVG 2 §10 defines `rect`, `circle`, `ellipse`, `line`, `polyline` and `polygon`
  as basic shapes. A `rect` is axis-aligned in the current user coordinate
  system; an `ellipse` is defined by `cx`, `cy`, `rx` and `ry`.
  https://www.w3.org/TR/SVG2/shapes.html (retrieved 2026-08-29).
- SVG 2 §12 defines text layout through `text`, `tspan` and text positioning
  attributes. SVG text permits per-character positioning, text paths and
  shaping behavior beyond NUIF profile zero.
  https://www.w3.org/TR/SVG2/text.html (retrieved 2026-08-29).
- SVG 2 §13 makes fill and stroke presentation properties available to shape
  and text elements. CSS cascading and inheritance apply, so a computed paint
  cannot be attributed to one source span without cascade analysis.
  https://www.w3.org/TR/SVG2/painting.html and
  https://www.w3.org/TR/SVG2/styling.html (retrieved 2026-08-29).
- SVG 2 §16 permits WAI-ARIA attributes and the `role` attribute on SVG
  elements. This surface can carry NUIF role and accessible-name
  correspondences, subject to the SVG Accessibility API Mappings.
  https://www.w3.org/TR/SVG2/struct.html#WAIARIAAttributes (retrieved
  2026-08-29).

## NUIF relevance

**Borrow** the basic-shape geometry, sRGB presentation attributes, containment
order, XML identity and accessibility attributes for a bounded vector profile.

**Adapt** `svg`, `g`, `rect`, `ellipse` and `text` into NUIF surface, container,
shape and text entities. A generated profile requires explicit `data-nuif-*`
metadata for document identity, stable entity identity, pinned font identity
and authored sizing intent. Static numeric attributes can retain byte-span
correspondence.

**Reject** a claim that arbitrary SVG is a lossless semantic import. Paths,
transforms, CSS cascade, paint servers, clipping, masks, filters, animation,
scripts, external resources, per-character text positioning and `<use>`
instancing require separate profiles. Unknown XML can be retained as source but
cannot be classified as lossless NUIF semantics without a declared extension.
