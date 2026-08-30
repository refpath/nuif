---
id: nuif:research:ocr-text-detection-and-recognition
kind: synthesis
status: reviewed
title: OCR detection and recognition as separate reconstruction observations
source:
  url: https://openaccess.thecvf.com/content/CVPR2021/html/Singh_TextOCR_Towards_Large-Scale_End-to-End_Reasoning_for_Arbitrary-Shaped_Scene_Text_CVPR_2021_paper.html
  authors: [Amanpreet Singh, Guan Pang, Mandy Toh, Jing Huang, Wojciech Galuba, Tal Hassner, Minghao Li, Tengchao Lv, Jingye Chen, Lei Cui, Yijuan Lu, Dinei Florencio, Cha Zhang, Zhoujun Li, Furu Wei]
  published_at: 2021-2023
  license: CVPR and AAAI publications; dataset/model terms are separate
retrieved_at: 2026-08-30
tags: [ocr, text-detection, text-recognition, baselines, ui, evaluation]
confidence: 0.95
claims: [nuif:claim:typed-reconstruction-loop, nuif:claim:evaluation-before-adaptation]
relations:
  - type: related_to
    target: nuif:research:screenai-ui-annotation
  - type: related_to
    target: nuif:research:text-rendering-reproducibility
links:
  spec: [spec/05-geometry-paint-text.md, spec/09-provenance-and-fidelity.md]
  adr: []
  rfc: []
  code: []
  experiments: [nuif:experiment:screenshot-reconstruction-baseline]
---

# Summary

Screenshot reconstruction needs both text localization and transcription.
TextOCR provides dense polygon/word annotations for detection and recognition;
TrOCR demonstrates a transformer encoder-decoder for cropped text recognition.
Neither is specifically a UI typography recovery system. The useful pattern is
to keep OCR as a replaceable observation provider with its own benchmarks,
coordinates, confidence and language coverage.

## Evidence

- TextOCR, CVPR 2021, reports 28,134 real images and 903,069 non-empty annotated
  words, with polygons for arbitrary-shaped text and explicit annotation/audit
  procedures.
- TextOCR focuses mainly on scene text; its distribution, shapes and language
  policy do not match desktop/mobile UI text exactly.
- TrOCR, AAAI 2023, DOI 10.1609/AAAI.V37I11.26538, uses pretrained image and
  text transformers for text recognition and explicitly leaves text detection
  to a separate stage.
- Both model families can hallucinate or normalize text. Exact NUIF content
  requires comparing recognized text to pixels/source evidence and retaining
  uncertainty rather than silently correcting it with a language prior.

## Mechanism

```text
screenshot -> text detector -> polygons/baselines/crop transforms
           -> recognizer -> Unicode candidates + confidence
           -> grouping/reading order -> text observations
           -> typography and layout inference (separate task)
```

Character error rate, word error rate, region precision/recall and baseline
geometry are measured independently. Font family, size, weight, line height and
letter spacing are not OCR outputs unless a separate estimator supplies them.

## NUIF relevance

**Borrow** dense text-region evaluation and separable detection/recognition.

**Adapt** evaluation to UI-scale text, multiple scripts, antialiasing modes,
icons mixed with glyphs, truncation, clipping and overlapping regions. Preserve
multiple candidates when confidence is low.

**Reject** one OCR implementation as normative, language-model autocorrection
as source truth and inferred font identity from glyph appearance alone.

## Open questions

- Which open, redistributable OCR candidates provide the best UI text accuracy
  per latency/VRAM on the frozen benchmark?
- How should ligatures, icon fonts, emoji and variable-font glyphs be classified?
- Can line baselines and advances be estimated accurately enough to improve text
  fitting before the exact font resource is known?
