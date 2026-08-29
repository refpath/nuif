---
id: nuif:research:accessibility-semantics
kind: standard
status: active
title: WAI-ARIA and accessibility API mappings
source:
  url: https://www.w3.org/TR/wai-aria-1.3/
  authors: [W3C Accessible Rich Internet Applications Working Group]
  published_at: 2026-06-01
  license: W3C document license
retrieved_at: 2026-08-29
tags: [accessibility, semantics, roles, states, properties, mappings]
confidence: 0.99
claims: []
relations:
  - type: compares_to
    target: nuif:research:open-ui
links:
  spec: [spec/13-semantics-accessibility-and-behavior.md]
  adr: []
  rfc: []
  code: []
  experiments: []
---
# Summary
WAI-ARIA defines portable authored accessibility semantics as roles, states and properties, while the Core Accessibility API Mappings specify how host-language semantics are exposed to platform accessibility APIs. This separation is directly relevant to a vendor-neutral authored UI model.

## NUIF relevance
NUIF should carry semantic role/name/state/relationship intent independently of visual geometry, then let platform adapters lower that intent to ARIA, native accessibility APIs or framework semantics. NUIF must not claim that platform accessibility trees are identical; adapter conformance should report semantic equivalence and unsupported platform behavior explicitly.
