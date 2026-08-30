---
id: nuif:research:model-and-dataset-documentation
kind: synthesis
status: reviewed
title: Datasheets and model cards for reconstruction artifacts
source:
  url: https://arxiv.org/abs/1803.09010
  authors: [Timnit Gebru, Jamie Morgenstern, Briana Vecchione, Jennifer Wortman Vaughan, Hanna Wallach, Hal Daumé III, Kate Crawford, Margaret Mitchell, Simone Wu, Andrew Zaldivar, Parker Barnes, Lucy Vasserman, Ben Hutchinson, Elena Spitzer, Inioluwa Deborah Raji]
  published_at: 2018-2021
  license: research publications
retrieved_at: 2026-08-30
tags: [datasets, model-cards, documentation, governance, provenance, evaluation]
confidence: 0.97
claims: [nuif:claim:model-artifact-separation, nuif:claim:evaluation-before-adaptation]
relations:
  - type: related_to
    target: nuif:research:websight-synthetic-screenshot-html
  - type: related_to
    target: nuif:research:sequence-level-knowledge-distillation
links:
  spec: [spec/09-provenance-and-fidelity.md]
  adr: []
  rfc: []
  code: []
  experiments: [nuif:experiment:reconstruction-distillation]
---

# Summary

Datasheets for Datasets and Model Cards for Model Reporting establish a useful
minimum documentation discipline for training corpora and released models. A
reconstruction system needs both: dataset records explain why examples exist,
how they were collected and where they should not be used; model records explain
intended use, evaluated conditions, limitations and performance.

These documents improve transparency but do not establish that data collection,
training or model output is lawful, representative or safe. NUIF should combine
them with content-addressed artifact manifests, exact license review, consent
policy, leak-resistant splits and executable evaluation reports.

## Evidence

- Gebru et al., *Datasheets for Datasets*, arXiv:1803.09010 / CACM 2021,
  proposes documenting motivation, composition, collection, preprocessing,
  distribution, maintenance and recommended uses.
- Mitchell et al., *Model Cards for Model Reporting*, arXiv:1810.03993 / FAT*
  2019, proposes documenting intended uses, factors, metrics, evaluation data,
  training data and performance/limitations across relevant conditions.
- Neither publication makes documentation a substitute for measurement or
  governance; the artifacts communicate how a dataset/model was constructed
  and evaluated.

## Mechanism

Every dataset snapshot receives a content digest and datasheet. Every model or
adapter receives a content digest and model card. A training-run manifest binds
base model, processors, operation-schema version, dataset splits, code revision,
hyperparameters, seeds, hardware, evaluator and resulting artifact digests.

Private/authenticated captures are excluded from training by default. Opt-in is
recorded per source; credentials and secret-bearing observations are never
training features. Takedown and deletion procedures address retained examples
and future datasets; limitations of already-released irreversible artifacts are
stated plainly.

## NUIF relevance

**Borrow** the two complementary reporting templates.

**Adapt** them to UI-specific concerns: source/capture rights, font and image
licenses, template-family leakage, sensitive text, accessibility content,
geographic/language coverage, renderer version and operation-schema version.

**Reject** “openly reachable” as permission to train, mutable dataset names as
reproducible identity and an aggregate model score without per-condition error
and calibration reporting.

## Open questions

- What minimum evidence establishes permission for synthetic, public Web,
  contributor-provided and host-exported examples?
- How can source withdrawal be propagated to future dataset releases and
  retraining schedules?
- Which UI strata must be reported before a model can claim general rather than
  profile-specific usefulness?
