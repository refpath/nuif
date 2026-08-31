---
id: nuif:research:reconstruction-corpus-integrity
kind: synthesis
status: reviewed
title: Group-isolated reconstruction corpora and auditable benchmark snapshots
source:
  url: https://github.com/mlcommons/training_policies/blob/master/training_rules.adoc
  repository: https://github.com/mlcommons/training_policies
  authors: [MLCommons]
  published_at: "2026"
  license: Apache-2.0 for MLCommons policy repositories; BSD-3-Clause for scikit-learn; Hugging Face documentation terms
retrieved_at: 2026-08-31
tags: [datasets, benchmark-integrity, grouped-splits, contamination, rights, checksums]
confidence: 0.96
claims: [nuif:claim:evaluation-before-adaptation, nuif:claim:model-artifact-separation]
relations:
  - type: supports
    target: nuif:research:model-and-dataset-documentation
  - type: related_to
    target: nuif:research:websight-synthetic-screenshot-html
  - type: related_to
    target: nuif:research:design2code-real-world-benchmark
links:
  spec: [spec/14-observation-capture-and-reconstruction.md]
  adr: []
  rfc: [rfcs/0011-observation-and-inference-provenance.md]
  code: [crates/nuif-reconstruct/src/evaluation/corpus.rs, crates/nuif-testing/src/bin/reconstruction-corpus-audit.rs]
  experiments: [nuif:experiment:reconstruction-corpus-integrity]
---

# Summary

A screenshot reconstruction benchmark needs stronger isolation than a random
row split. Several screenshots can originate from one site, template,
component library, font set, resource set or synthetic generator; treating
those rows as independent lets a system memorize a family while appearing to
generalize. Exact duplicate checks alone also miss transformed or cropped
members of the same family.

The selected contract combines immutable artifact digests with declared group
identities. Every origin, template, component, font, resource, generator and
near-duplicate group is confined to one adaptation, calibration, validation or
test partition. Inputs and targets retain separate disclosure levels so a
public-input/private-target evaluation can be audited without publishing the
target. Per-example rights evidence and allowed uses are explicit.

This is an integrity mechanism, not a rights engine or duplicate detector. A
validator can prove that declared identities do not cross partitions; it
cannot prove that the declarations are complete, that a license applies, that
consent is valid or that the resulting sample represents the intended world.

## Evidence

- MLPerf Training Rules section 6.5 states that training data may not contain
  data appearing in the test set. The same policies require the reference
  partitioning in the closed division. Source:
  `training_policies/training_rules.adoc`, section 6.5, retrieved 2026-08-31.
- MLPerf Inference Rules section 7 requires a checksum-verification script and
  an unchanged dataset at the start of each run. It separately identifies
  accuracy and calibration data. Source:
  https://github.com/mlcommons/inference_policies/blob/master/inference_rules.adoc,
  section 7, retrieved 2026-08-31.
- scikit-learn's `GroupKFold` and `StratifiedGroupKFold` documentation defines
  folds with non-overlapping groups; stratification is attempted subject to
  that isolation constraint. Source:
  https://scikit-learn.org/stable/modules/cross_validation.html,
  “Cross-validation iterators for grouped data”, retrieved 2026-08-31.
- Hugging Face's Dataset Cards documentation records that a repository
  `README.md` plus YAML metadata communicates license, composition, use and
  limitations, while repository revisions provide version history. Source:
  https://huggingface.co/docs/hub/main/datasets-cards, retrieved 2026-08-31.

## Mechanism

The executable `nuif-reconstruction-corpus-manifest-0` record pins a snapshot,
dataset card and evaluator by SHA-256. Each example declares its evidence
suite, split, input and target artifacts, disclosure policy, collection class,
rights evidence, permitted uses, sensitivity review and leakage groups.

`CorpusManifest::audit` rejects:

- duplicate example or per-example artifact identities;
- any non-context artifact digest reused across different partitions;
- any declared family group reused across different partitions;
- adaptation, calibration or evaluation examples without their corresponding
  permitted use;
- screenshot-only examples carrying exact source/resource inputs;
- source-backed examples without source bytes;
- retained real examples without a withdrawal policy, and private/authenticated
  examples without explicit authorization;
- missing near-duplicate assignment, invalid digests and bounded-work excess.

The audit is derived data and can be validated against the manifest to detect
edited counts. It reports partition/suite/disclosure counts but does not expose
artifact bytes.

## NUIF relevance

**Borrow** immutable checksum verification, explicit benchmark rules, grouped
partitioning and dataset-card documentation.

**Adapt** groups to UI reconstruction: origins, templates, components, fonts,
resources, synthetic generators and near-duplicate families all matter.

**Reject** random row splitting, a hash-only contamination claim, treating a
public URL as training permission, publishing private targets merely to make a
benchmark reproducible, and claiming statistical validity from manifest
validation.

## Open questions

- Which independently reviewed detector and thresholds should assign
  transformed/cropped screenshot near-duplicate groups?
- Which strata and minimum group counts are needed for useful uncertainty
  intervals and distribution-shift reporting?
- What neutral evaluator service can hold restricted targets while publishing
  reproducible input identities, evaluator versions and signed result records?
