---
id: nuif:research:sequence-level-knowledge-distillation
kind: paper
status: reviewed
title: Sequence-level knowledge distillation
source:
  url: https://aclanthology.org/D16-1139/
  doi: 10.18653/v1/D16-1139
  authors: [Yoon Kim, Alexander M. Rush]
  published_at: 2016-11
  license: ACL Anthology publication
retrieved_at: 2026-08-30
tags: [knowledge-distillation, sequence-generation, teacher-student, training-data]
confidence: 0.98
claims: [nuif:claim:evaluation-before-adaptation, nuif:claim:model-artifact-separation]
relations:
  - type: related_to
    target: nuif:research:lora-low-rank-adaptation
links:
  spec: []
  adr: []
  rfc: []
  code: []
  experiments: [nuif:experiment:reconstruction-distillation]
---

# Summary

Sequence-level knowledge distillation trains a smaller sequence model on
teacher-generated outputs rather than only matching token distributions. In the
paper's neural machine translation setting, the approach simplified the target
distribution and enabled a smaller, faster student with limited quality loss.

For NUIF, accepted operation traces are a sequence target, but the teacher must
be an evaluated pipeline—not an unversioned model response—and every sequence
must pass validation and renderer-based checks before entering training data.

## Evidence

- EMNLP 2016 paper defines sequence-level and sequence-level interpolation
  variants of knowledge distillation for neural machine translation.
- The best studied student ran ten times faster than its teacher with limited
  task-score loss; pruning further reduced parameters. These figures are
  specific to translation and are not NUIF projections.
- Distillation transfers teacher behavior, including systematic mistakes. The
  paper does not supply domain-specific correctness filters for UI semantics.

## Mechanism

A teacher decodes target sequences for source examples. The student is trained
on those generated targets, optionally mixed with original labels. Applied to
NUIF, a “target” is a validated operation sequence plus its execution outcome,
not merely a textual document emitted by the teacher.

## NUIF relevance

**Borrow** sequence-level teacher outputs for a smaller student after the
teacher pipeline has demonstrably better held-out performance.

**Adapt** each example into a trace: input hashes/context, observations,
proposal, validation diagnostics, accepted operations, intermediate renders,
difference maps, final fidelity and exact tool/model versions.

**Reject** unfiltered self-training, distillation from private inputs without
explicit opt-in, and claims that a student is correct because it imitates a
larger model.

## Open questions

- Should the student learn complete initial transactions, single corrective
  transactions, or both as distinct tasks?
- How are multiple valid reconstructions represented without collapsing to one
  arbitrary teacher choice?
- Which teacher errors survive render filtering but damage editability or
  responsive behavior?
