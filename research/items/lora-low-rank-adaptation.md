---
id: nuif:research:lora-low-rank-adaptation
kind: paper
status: reviewed
title: LoRA low-rank parameter adaptation
source:
  url: https://arxiv.org/abs/2106.09685
  authors: [Edward J. Hu, Yelong Shen, Phillip Wallis, Zeyuan Allen-Zhu, Yuanzhi Li, Shean Wang, Lu Wang, Weizhu Chen]
  published_at: 2021-06-17
  license: research publication; code and base-model licenses are separate
retrieved_at: 2026-08-30
tags: [lora, parameter-efficient-finetuning, adapters, training, model-artifacts]
confidence: 0.98
claims: [nuif:claim:evaluation-before-adaptation, nuif:claim:model-artifact-separation]
relations:
  - type: related_to
    target: nuif:research:qlora-quantized-adaptation
links:
  spec: []
  adr: []
  rfc: []
  code: []
  experiments: [nuif:experiment:reconstruction-distillation]
---

# Summary

LoRA freezes pretrained weights and learns low-rank update matrices in selected
layers. It can reduce trainable parameter count and keep task adapters separate
from the base model. That makes it a plausible packaging and experimentation
technique after NUIF has task-specific data and evaluation.

LoRA is not a reconstruction architecture, dataset-quality method or accuracy
guarantee. A low-rank adapter can efficiently learn the wrong target just as a
full fine-tune can.

## Evidence

- arXiv:2106.09685 and the ICLR 2022 OpenReview paper define a frozen base
  matrix with a trainable low-rank decomposition added to its update.
- The paper reports large reductions in trainable parameters and avoids extra
  inference latency from a separate serial adapter path when weights are merged.
- Results are model/task specific. They do not establish that every vision
  encoder/decoder, multimodal projector or operation grammar has a low-rank
  task update.

## Mechanism

For a frozen weight matrix `W`, LoRA learns `BA` with rank `r` and uses
`W + scale * BA` during the forward pass. The adapter artifact therefore depends
on the exact base-model identity, target modules, rank, scaling and training
configuration.

## NUIF relevance

**Borrow** separable, content-addressed task adapters and controlled rank/module
ablations.

**Adapt** the artifact manifest to pin base model, tokenizer/image processor,
operation-schema version, renderer/evaluator version, dataset revision and
license/provenance. Never store adapters in a `.nuif` document.

**Reject** “use LoRA” as a research conclusion before an untuned baseline and a
frozen evaluation suite exist.

## Open questions

- Which modules need adaptation for screen grounding versus operation decoding?
- Does a small adapter preserve general visual/OCR capability better than a
  full fine-tune on narrow synthetic layouts?
- Can one adapter cover both initial synthesis and corrective operations without
  negative transfer?
