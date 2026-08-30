---
id: nuif:research:omniparser-ui-grounding
kind: implementation
status: reviewed
title: OmniParser screen-region detection and icon captioning
source:
  url: https://www.microsoft.com/en-us/research/articles/omniparser-for-pure-vision-based-gui-agent/
  repository: https://github.com/microsoft/OmniParser
  authors: [Yadong Lu, Jianwei Yang, Yelong Shen, Ahmed Awadallah]
  published_at: 2024-08
  license: repository and model-component licenses differ by version
retrieved_at: 2026-08-30
tags: [ui-grounding, detection, icons, ocr, screen-parsing, licensing]
confidence: 0.9
claims: [nuif:claim:typed-reconstruction-loop, nuif:claim:evaluation-before-adaptation]
relations:
  - type: related_to
    target: nuif:research:screenai-ui-annotation
  - type: related_to
    target: nuif:research:ui-code-generation-boundaries
links:
  spec: [spec/09-provenance-and-fidelity.md, spec/13-semantics-accessibility-and-behavior.md]
  adr: []
  rfc: []
  code: []
  experiments: [nuif:experiment:screenshot-reconstruction-baseline]
---

# Summary

OmniParser is a compact screen-parsing pipeline for GUI agents. It combines OCR,
interactable-region detection and icon captioning to provide grounded regions
to a downstream vision-language model. This is useful as an optional
observation provider, especially for small controls, but its target is action
grounding rather than full visual reconstruction.

Dependency and weight licensing must be checked at the exact selected revision.
The current repository describes its newer `icon_detect_v3` as based on an
MIT-licensed YOLOv9 implementation, while earlier Ultralytics-based detectors
retain their original AGPL terms; caption models are described separately. A
generic NUIF pipeline cannot inherit one blanket license assumption.

## Evidence

- Microsoft Research describes two curated tasks: interactable icon detection
  and icon functional description, implemented with complementary detection and
  captioning models.
- The reported benchmarks focus on agent grounding/navigation such as
  ScreenSpot, Mind2Web, Android-in-the-Wild and WindowsAgentArena, not design
  reconstruction or resource recovery.
- The current repository README distinguishes the license provenance of the
  newer detector, earlier detectors and captioning models. Exact weights and
  revisions still require an artifact manifest before distribution.
- The parser uses OCR and icon regions; it does not recover original DOM, CSS,
  font files, image assets or responsive constraints.

## Mechanism

```text
screenshot -> OCR text boxes
           -> detector regions/interactivity
           -> caption selected icon crops
           -> deduplicate/serialize grounded screen elements
           -> downstream model
```

The detector, OCR engine and caption model can be benchmarked independently and
their observations can be passed to a model without incorporating their code or
weights into the NUIF core.

## NUIF relevance

**Borrow** modular region proposals, OCR fusion and icon crop captioning as an
optional `ObservationProvider`.

**Adapt** every output to typed boxes with model/version/license identity,
confidence and source region. Evaluate non-interactive visual elements too.

**Reject** making one implementation mandatory, treating “interactable” as a
complete UI element taxonomy, or importing any model weights without a locked
bill of materials and compatible redistribution terms.

## Open questions

- Does the provider improve final NUIF structural and visual measures over a
  strong OCR/CV baseline at acceptable latency and VRAM?
- How stable are region identifiers across nearby viewports and states?
- Can its icon captions be calibrated well enough to remain observations rather
  than accidental behavior assertions?
