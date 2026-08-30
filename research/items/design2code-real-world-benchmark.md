---
id: nuif:research:design2code-real-world-benchmark
kind: paper
status: reviewed
title: Design2Code real-world screenshot-to-code benchmark
source:
  url: https://aclanthology.org/2025.naacl-long.199/
  doi: 10.18653/v1/2025.naacl-long.199
  authors: [Chenglei Si, Yanzhe Zhang, Ryan Li, Zhengyuan Yang, Ruibo Liu, Diyi Yang]
  published_at: 2025-04
  license: ACL Anthology publication
retrieved_at: 2026-08-30
tags: [screenshot-to-code, benchmark, webpages, visual-fidelity, layout, evaluation]
confidence: 0.98
claims: [nuif:claim:source-inference-separation, nuif:claim:evaluation-before-adaptation]
relations:
  - type: supports
    target: nuif:research:ui-code-generation-boundaries
links:
  spec: [spec/09-provenance-and-fidelity.md, spec/14-observation-capture-and-reconstruction.md]
  adr: []
  rfc: [rfcs/0003-authored-resolved-provenance.md, rfcs/0011-observation-and-inference-provenance.md]
  code: []
  experiments: [nuif:experiment:screenshot-reconstruction-baseline, nuif:experiment:reconstruction-closed-loop]
---

# Summary

Design2Code evaluates screenshot-to-frontend-code generation on 484 manually
curated real webpages. Its fine-grained analysis reports that contemporary
multimodal models especially miss visible elements and produce incorrect
layouts. The benchmark is useful evidence that screenshot reconstruction is
not solved by a single high-level similarity score or a one-shot prompt.

It is not a NUIF conformance corpus: its output is HTML/CSS, its exact source
and licensing conditions need to be reviewed before fixture reuse, and matching
one viewport cannot establish authored structure, responsiveness or behavior.

## Evidence

- ACL Anthology paper, abstract and §2: 484 diverse real-world webpages are
  manually curated as test cases for screenshot-conditioned code generation.
- §3 defines automatic evaluation over rendered output and complements it with
  human evaluation to validate system ranking.
- The paper's fine-grained results identify element recall and layout generation
  as major failure categories even when aggregate visual similarity improves.
- The official repository adds an 80-example hard subset and publishes the
  benchmark/evaluation implementation: https://github.com/NoviScl/Design2Code.

## Mechanism

The task provides one webpage screenshot to a multimodal model, executes the
generated frontend, renders the result and compares it with the target using
aggregate and element-level measures. Human evaluations provide a check on
metric ranking. This is an end-to-end code-generation evaluation, not recovery
of the original source program.

## NUIF relevance

**Borrow** a held-out real-page benchmark, element-level recall, layout
breakdowns and human validation of metric ranking.

**Adapt** output validation to typed NUIF operations and a deterministic
renderer. Add document validity, tree structure, text, geometry, resources,
responsive held-out viewports, accessibility, provenance and confidence.

**Reject** one screenshot/one viewport as proof of exact reconstruction,
aggregate screenshot similarity as the sole reward, and source-code similarity
as a semantic NUIF oracle.

## Open questions

- Which Design2Code assets can be redistributed as NUIF test fixtures under
  documented terms rather than only evaluated in place?
- How strongly do its metric rankings correlate with editable structure and
  held-out responsive behavior?
- Which failures remain after deterministic OCR, region proposals and a
  render-difference correction loop are supplied to the same model?
