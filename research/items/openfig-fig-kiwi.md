---
id: nuif:research:openfig-fig-kiwi
kind: repository
status: reviewed
title: OpenFig reverse engineering of the Figma .fig Kiwi format
source:
  url: https://github.com/OpenFig-org/openfig-core/blob/main/docs/research.md
  authors: [OpenFig contributors]
  published_at: 2026-03-16
  license: repository license
retrieved_at: 2026-08-29
tags: [figma, kiwi, binary, adapter, reverse-engineering]
confidence: 0.96
claims: []
relations:
  - type: extends
    target: nuif:research:figma
links:
  spec: []
  adr: []
  rfc: []
  code: [adapters/README.md]
  experiments: []
---
# Summary
OpenFig documents `.fig` as a ZIP containing a Kiwi-serialized `canvas.fig` whose schema is embedded and evolves with Figma versions. Its tooling can parse and encode documents and package `.fig` archives. Independent Grida research similarly extracts current Kiwi schemas.

## NUIF relevance
This makes high-fidelity Figma adapters technically feasible, but the embedded schema is vendor-controlled and mutable. Reverse-engineered internals are compatibility evidence only, never a dependency or canonical semantic source for NUIF.
