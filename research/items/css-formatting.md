---
id: nuif:research:css-formatting
kind: standard
status: reviewed
title: CSS formatting tree, Flexbox and Grid algorithms
source:
  url: https://www.w3.org/TR/css-display-3/
  authors: [W3C CSS Working Group]
  published_at: 2026-06-01
  license: W3C permissive document license
retrieved_at: 2026-08-29
tags: [css, layout, box-tree, flexbox, grid]
confidence: 0.99
claims: [nuif:claim:authored-resolved]
relations: []
links:
  spec: [spec/04-layout.md]
  adr: [adrs/0002-layout-engine.md]
  rfc: []
  code: [crates/nuif-layout]
  experiments: []
---
# Summary
CSS separates the source element tree from an intermediary formatting box tree and then applies family-specific layout algorithms. CSS Display defines box-tree generation; Flexbox and Grid define normative algorithms whose results implementations must reproduce even if their internal algorithms differ.

## NUIF relevance
This strongly supports separating semantic containment from formatting/layout structure. NUIF can borrow exact CSS-compatible semantics where selected, but should not claim arbitrary HTML/CSS equivalence when anonymous boxes, generated content, cascade, writing modes or other web-specific fixups are absent.
