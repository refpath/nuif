---
id: nuif:research:confidence-calibration-and-selective-prediction
kind: synthesis
status: reviewed
title: Confidence calibration, risk coverage and explicit abstention
source:
  url: https://proceedings.mlr.press/v70/guo17a.html
  authors: [Chuan Guo, Geoff Pleiss, Yu Sun, Kilian Q. Weinberger, Yonatan Geifman, Ran El-Yaniv]
  published_at: 2017-2019
  license: PMLR publications
retrieved_at: 2026-08-30
tags: [calibration, confidence, selective-prediction, abstention, risk, evaluation]
confidence: 0.97
claims: [nuif:claim:calibrated-inference, nuif:claim:source-inference-separation]
relations:
  - type: related_to
    target: nuif:research:reverse-layout-inference
  - type: related_to
    target: nuif:research:ui-code-generation-boundaries
links:
  spec: [spec/09-provenance-and-fidelity.md]
  adr: []
  rfc: [rfcs/0003-authored-resolved-provenance.md]
  code: []
  experiments: [nuif:experiment:inference-confidence-calibration]
---

# Summary

Inference confidence is useful only when it predicts observed correctness under
a declared condition. Guo et al. show that modern neural-network confidence can
be poorly calibrated and evaluate post-hoc calibration, including temperature
scaling. SelectiveNet studies an explicit reject option and risk/coverage
trade-off. Together they support calibrated confidence and abstention instead
of forcing every screenshot region into a confident semantic claim.

## Evidence

- Guo et al., ICML 2017, defines calibration as predicted probability matching
  empirical correctness likelihood, documents miscalibration in studied modern
  networks, and reports temperature scaling as a strong simple baseline on its
  classification datasets.
- Geifman and El-Yaniv, *SelectiveNet*, ICML 2019,
  https://proceedings.mlr.press/v97/geifman19a.html, trains prediction and
  rejection jointly and evaluates risk as coverage changes.
- Both studies concern classification/regression. Applying their procedures to
  structured UI reconstruction requires task-specific correctness events and
  cannot reuse their thresholds directly.

## Mechanism

NUIF confidence is attached to individual observations and inferred decisions,
not only a whole document. Calibration sets map raw scores to empirical success
for events such as correct text, region class, parent, layout family, resource
match or operation acceptance. A policy can abstain, retain alternatives or
request review when expected risk exceeds a profile limit.

## NUIF relevance

**Borrow** reliability diagrams, expected calibration error as one diagnostic,
proper scoring where applicable and risk/coverage curves.

**Adapt** correctness to multiple structured outcomes. A candidate can be
visually close but structurally wrong, so confidence is typed by decision and
evaluated on frozen, shifted and out-of-distribution subsets.

**Reject** raw model likelihood as portable confidence, one global threshold
for every property and forced guesses where evidence is absent.

## Open questions

- Which structured correctness events have enough validation examples for
  stable calibration?
- How should confidence compose when a parent/layout/resource decision depends
  on several uncertain observations?
- What risk/coverage target is acceptable for automatic application versus a
  suggestion shown for human review?
