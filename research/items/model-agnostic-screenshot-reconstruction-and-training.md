---
id: nuif:research:model-agnostic-screenshot-reconstruction-and-training
kind: synthesis
status: reviewed
title: Model-agnostic screenshot reconstruction, evaluation and adaptation plan
source:
  url: https://aclanthology.org/2025.naacl-long.199/
  authors: [NUIF contributors]
  published_at: 2026-08-30
  license: repository license; cited artifacts retain their own terms
retrieved_at: 2026-08-30
tags: [synthesis, screenshot-reconstruction, vision-language, operations, evaluation, distillation, lora, governance]
confidence: 0.92
claims: [nuif:claim:source-inference-separation, nuif:claim:typed-reconstruction-loop, nuif:claim:evaluation-before-adaptation, nuif:claim:calibrated-inference, nuif:claim:model-artifact-separation]
relations:
  - type: depends_on
    target: nuif:research:design2code-real-world-benchmark
  - type: depends_on
    target: nuif:research:pix2struct-screenshot-parsing-pretraining
  - type: depends_on
    target: nuif:research:screenai-ui-annotation
  - type: depends_on
    target: nuif:research:reverse-layout-inference
  - type: depends_on
    target: nuif:research:inferui-and-layout-synthesis
  - type: inspired_by
    target: nuif:research:visrefiner-render-difference-learning
  - type: related_to
    target: nuif:research:lora-low-rank-adaptation
  - type: related_to
    target: nuif:research:qlora-quantized-adaptation
  - type: related_to
    target: nuif:research:sequence-level-knowledge-distillation
  - type: related_to
    target: nuif:research:model-and-dataset-documentation
  - type: related_to
    target: nuif:research:confidence-calibration-and-selective-prediction
  - type: related_to
    target: nuif:research:resource-packaging-and-source-capture-synthesis
links:
  spec: [spec/04-layout.md, spec/05-geometry-paint-text.md, spec/09-provenance-and-fidelity.md, spec/11-security.md, spec/13-semantics-accessibility-and-behavior.md, spec/14-observation-capture-and-reconstruction.md]
  adr: []
  rfc: [rfcs/0003-authored-resolved-provenance.md, rfcs/0004-headless-qa-contract.md, rfcs/0011-observation-and-inference-provenance.md]
  code: [crates/nuif-api, crates/nuif-protocol, crates/nuif-render, crates/nuif-testing]
  experiments: [nuif:experiment:screenshot-reconstruction-baseline, nuif:experiment:reconstruction-closed-loop, nuif:experiment:inference-confidence-calibration, nuif:experiment:reconstruction-distillation]
---

# Summary

Screenshot-to-NUIF is an inverse problem, not ordinary file parsing. A static
image is compatible with many different scene graphs, layout programs, fonts,
resources, responsive rules, accessibility structures and behaviors. The
correct product is therefore a reconstruction pipeline that emits one validated
editable hypothesis, alternative hypotheses and calibrated evidence—not a
claim that it recovered the unavailable authored source.

The recommended system combines deterministic computer vision/OCR, optional UI
grounding and a replaceable vision-language reasoner with NUIF's typed
operations, validator, layout solver and renderer. The reasoner proposes; the
core decides whether a transaction is valid; the renderer produces measurable
outcomes; a bounded correction loop improves the proposal. No model provider,
training framework or base checkpoint appears in the NUIF specification.

Training is deliberately later than evaluation. First implement an untuned
baseline and closed loop, freeze a leak-resistant benchmark, and determine
which errors remain. Only then compare prompting/tool use, supervised tuning,
low-rank adaptation, quantized low-rank adaptation and sequence-level
distillation under the same data and evaluator.

## Evidence synthesis

### What existing work supports

- Design2Code's 484 real pages demonstrate persistent element-recall and layout
  errors in screenshot-conditioned frontend generation. This supports explicit
  element/geometry metrics and a real-world holdout.
- Pix2Struct shows that screenshot-to-structured-markup pretraining combines
  useful OCR, language and layout signals. It does not recover original source.
- ScreenAI shows value in an intermediate screen annotation containing element
  type, location, OCR and icon/image descriptions.
- OmniParser shows that a compact detector/OCR/caption pipeline can improve GUI
  grounding, while also demonstrating that component and weight licenses must
  be resolved per revision.
- ReverseORC and InferUI show why multiple viewports and held-out contexts are
  needed to distinguish layout hypotheses and assess generalization.
- DCGen reports improvements from hierarchical screenshot segmentation. This
  supports multi-scale crops and region-specific proposals rather than one
  downsampled full-screen prompt.
- VisRefiner is early evidence for learning from target/render differences and
  corrective edits. Its preprint status requires reproduction before adoption.
- LoRA and QLoRA reduce adaptation resource costs in their studied settings;
  neither supplies domain correctness or an evaluation method.
- Sequence-level distillation supports learning a smaller sequence generator
  from teacher outputs. Validated NUIF transactions are a safer target than raw
  teacher text.
- Calibration and selective-prediction research shows why raw confidence is not
  enough and why explicit abstention should be evaluated through risk/coverage.
- Dataset/model documentation research supplies reporting structure but does
  not replace rights review, privacy controls or executable tests.

### What the evidence does not support

The reviewed evidence does not justify any of these claims:

- one screenshot identifies the original layout or behavior;
- current general-purpose vision models can reconstruct arbitrary interfaces
  with extreme precision without deterministic tools and iteration;
- a high perceptual-similarity score implies an editable or accessible result;
- an image crop recovers the original image asset;
- visual font matching identifies exact font bytes or embedding permission;
- low-rank or quantized fine-tuning is automatically more accurate;
- a synthetic screenshot/HTML corpus is representative of real authored tools;
- one closed model, one open model or one UI detector should become normative;
- a research alpha label certifies screenshot reconstruction quality.

## Problem contract

The public task receives a set of `EvidenceInput` values and returns a
`ReconstructionResult`:

```text
EvidenceInput
  screenshots[]: bytes + viewport + DPR + crop/state/time metadata
  optional observations[]: OCR, regions, accessibility, source capture
  optional known resources[]: digest-pinned images/fonts
  requested profile + budgets

ReconstructionResult
  validated document or no-result
  accepted operation log
  immutable resources + derived-resource records
  fidelity report
  decision-level provenance and calibrated confidence
  retained alternatives/abstentions
  evaluation report and exact pipeline identity
```

Source-backed browser observations and screenshot-only inputs use this same
result type but carry different evidence classes. A source-backed field can be
`lossless` only within a declared adapter subset and only if its source bytes or
stable host semantics support the claim. A screenshot-inferred field cannot be
`lossless`; its best possible classification is `representable` or
`approximated` with inference provenance.

## Architecture

```text
Evidence normalization
  screenshots, contexts, optional source observations, known resources
              |
              v
Replaceable observation providers
  OCR/baselines | regions/edges/colors | UI grounding | repetition/assets
              |
              v
Typed ObservationGraph
  coordinates, candidates, confidence, evidence regions, provider versions
              |
              v
Replaceable proposal engine
  hierarchy + semantic kinds + layout hypotheses + typed NUIF operations
              |
              v
Core transaction boundary
  schema validation -> operation validation -> apply -> document validation
              |
              v
Deterministic layout and render
  declared contexts -> scene -> reference pixels + diagnostics
              |
              v
Difference and property evaluators
  text | elements | tree | geometry | resources | visual | accessibility
              |
              v
Bounded corrective-operation loop
  accept only improvements satisfying invariants and non-regression policy
```

The core, renderer and operation grammar are authoritative. Observation
providers and proposal models are ports with capability manifests. A provider
can be replaced without changing the file format or operation semantics.

## Observation graph

Every observation has:

- stable run-local identity;
- provider kind, artifact digest and version;
- source screenshot digest, coordinate space and region;
- predicted type/value with alternatives;
- raw and calibrated confidence;
- relationships such as contains, aligns, repeats, overlaps and possible-parent;
- evidence class: observed-source, observed-pixels, inferred, user-confirmed;
- privacy classification and retention policy.

OCR stores polygons/baselines and Unicode candidates separately from inferred
font/style. Region providers store visible boundaries rather than semantic
objects. UI grounding labels remain proposals. Repetition detection can suggest
components or stack/grid structure but cannot assert them without evaluation.

Coordinate normalization records viewport pixels, device pixels, crop origin,
page scale and NUIF logical units. No provider is allowed to mix coordinate
spaces implicitly.

## Proposal interface and typed operations

The proposal engine never mutates core structs directly. It emits a bounded
transaction using the same semantic operations as CLI, editor and adapters.
The first reconstruction grammar should cover:

- create entity with temporary/proposed identity;
- set semantic kind, name and accessibility evidence;
- establish parent/sibling anchors;
- set text and text-style candidates;
- bind image or font asset candidates;
- set geometry/paint;
- set layout family and typed constraints;
- attach inference provenance and alternatives;
- replace a prior inferred value through an explicit corrective operation.

Arbitrary code, scripts, extension blobs and raw unbounded JSON are excluded
from model output. Unsupported properties produce an abstention or explicit
opaque observation; they do not bypass the validator.

The initial proposal and every correction is atomic. A stale expected revision,
invalid graph, resource mismatch or budget excess rejects the complete
transaction and becomes feedback. The engine cannot gradually corrupt a valid
document while searching.

## Deterministic and learned responsibilities

Deterministic code should own:

- image decoding, coordinate conversion and basic color/edge statistics;
- OCR-provider invocation contract and candidate normalization;
- validation, canonicalization and operation application;
- layout solving, scene lowering and reference rendering;
- raw/perceptual difference maps and property-level metrics;
- resource hashing, package validation and provenance storage;
- loop limits, acceptance policy and reporting.

Learned or heuristic providers may own:

- text detection/recognition candidates;
- region and icon/image classification;
- grouping, hierarchy and semantic-label proposals;
- layout-family/constraint ranking;
- initial transaction generation;
- selection of corrective operations from structured differences.

This boundary keeps measurable rules out of model weights and prevents a model
change from redefining conformance.

## Multi-scale and multi-context reconstruction

Small text and controls are lost when an entire high-resolution screenshot is
reduced to a model's fixed input. The baseline therefore supplies:

1. a full-screen overview with normalized coordinates;
2. deterministic hierarchical regions;
3. overlapping high-resolution tiles with shared coordinate transforms;
4. focused crops for uncertain text/icons;
5. multiple viewports or interaction states when available.

Duplicate observations across tiles are merged by geometry and content
evidence, retaining disagreements. Multiple viewport screenshots share proposed
semantic identities; candidate layout programs are ranked by how well they
predict held-out contexts, not only by fit to the input viewport.

## Resources and provenance

Screenshot-only reconstruction can recover only visible samples. It handles
resources as follows:

- an exact known image/font resource is bound only when supplied bytes match a
  declared digest or source-backed capture provides the bytes;
- a crop from the screenshot is a derived image whose provenance includes
  screenshot digest, crop polygon, scale and any alpha/masking procedure;
- vector tracing is a derived approximation with algorithm/version and visual
  error, not the original vector;
- generated inpainting or upscaling is never canonical source recovery and
  requires an explicit accepted transformation policy;
- font appearance yields ranked candidates or substitution, never original
  font identity or redistribution permission;
- inaccessible resources remain unavailable with item-level fidelity.

A degenerate “one image covering the page” output is a valid screenshot asset
but fails the editable-reconstruction profile unless the user explicitly asks
for a flat image document.

## Closed-loop correction

For each candidate document and evaluation context:

1. validate and apply the proposed transaction;
2. render through the pinned reference path;
3. compare target and result structurally and visually;
4. localize differences to observations, entities and properties where possible;
5. propose one bounded correction transaction;
6. accept it only if validity remains true, the declared objective improves and
   no protected metric regresses beyond tolerance;
7. stop on success, no improvement, repeated state, budget or iteration limit.

The objective is a vector, not one scalar. Selection can use a lexicographic or
Pareto policy: document validity and required content first, then text and
structural errors, then geometry/resources/accessibility, then perceptual
appearance, then simplicity/editability. Every scalarization is recorded.

Loop state is content-addressed so repeated documents are detected. Tool calls,
renders and difference maps are cached by input and tool identity. Parallel
candidate evaluation is allowed; acceptance order is deterministic.

## Benchmark design

Source-backed capture and screenshot reconstruction have separate suites.

### Synthetic exact suite

Generate canonical NUIF documents across the supported profile, render them at
several contexts and retain exact model/operation/resource labels. Include
adversarial near-duplicates that look similar but require different hierarchy
or layout. Exact expected data permits:

- document/operation validity rate;
- entity precision, recall and F1;
- parent/sibling and tree-edit distance;
- property accuracy by kind;
- geometry error and intersection-over-union;
- text character/word error, region recall and baseline error;
- exact resource-digest recall where bytes are supplied;
- held-out viewport layout error;
- calibrated confidence and abstention quality;
- reference-pixel, FLIP, SSIM and pinned LPIPS diagnostics;
- latency, peak RAM/VRAM, iteration count and cost.

### Real screenshot suite

Use licensed, consented or otherwise documented inputs and human-reviewed target
annotations. Do not pretend the original author's full intent is known. Score
visible elements/text/geometry, edit-task success, responsive observations when
captured, accessibility evidence, resource provenance honesty and human visual
ranking. Preserve ambiguity rather than forcing one gold structure where
several reconstructions are equally supported.

### Source-backed suite

For browser or host captures, compare preserved source/resource bytes,
correspondence, resolved observations and held-out contexts. Source-backed
results are not mixed into screenshot-only accuracy without a label because the
available evidence is materially different.

### Splits and leakage controls

Split by originating project/domain and also by template, component family,
font family, resource family and generator seed. Near-duplicate screenshot or
DOM/resource hashes cannot cross splits. Benchmark pages never become
distillation examples. A public test set may expose inputs but keeps enough
private or rotating evaluation to detect overfitting.

## Metric policy

No single image metric is a correctness oracle:

- raw pixel difference catches exact raster changes but overreacts to benign
  platform text differences;
- FLIP models perceptual visibility under declared display parameters;
- SSIM is a classical structural diagnostic with known limitations;
- LPIPS adds learned perceptual features but pins a model and can be gamed;
- OCR/text metrics catch glyph-content errors hidden by aggregate appearance;
- geometry and tree metrics catch editable-structure errors;
- resource/provenance metrics detect false source-recovery claims;
- task edits and held-out viewport renders measure usability of the result.

Visual metrics are computed over the whole image and property-local masks. A
large correct background must not hide missing small controls. Scores are
reported with distributions and confidence intervals, not only one mean.

## Baseline and ablation ladder

The experiment order is:

1. deterministic segmentation/color/edge/repetition plus OCR, no VLM;
2. one-shot vision-language proposal from full screenshot;
3. proposal with normalized observation graph;
4. hierarchical/multi-scale crops;
5. multi-viewport layout ranking;
6. deterministic render-difference correction loop;
7. best available evaluated teacher pipeline;
8. tuned student or task adapter;
9. distilled student using only validated accepted traces.

Each stage uses identical held-out inputs and budgets. An added component stays
only if it improves a predeclared metric without unacceptable regressions in
validity, calibration, latency, memory, licensing or maintainability.

## Training-data construction

Do not train from raw model transcripts. Store a versioned `ReconstructionTrace`:

- input screenshot/resource hashes and contexts;
- observation graph and exact provider artifacts;
- proposal model/base/processor/tool versions;
- initial transaction and validator diagnostics;
- intermediate documents, renders and localized differences;
- corrective transactions, acceptance/rejection reason and objective vector;
- final canonical document/package hashes and fidelity report;
- human confirmations/corrections where present;
- rights, privacy, retention and split-group metadata.

Training targets are derived only from validated accepted transitions. Rejected
proposals can support preference/error classification if their retention is
permitted, but are never silently treated as desired output.

Data sources, in priority order:

1. synthetic canonical NUIF renders with perfect labels and controlled
   perturbations;
2. project-owned or permissively licensed source-backed pages with exact
   resources and multi-context observations;
3. contributor-provided examples under explicit training consent;
4. separately licensed public datasets after revision-level review.

Authenticated/private captures default to no training and no telemetry.
Secret scanning and redaction happen before any retained training record, and
redaction itself is recorded as a transformation.

## Adaptation and distillation decision

Training is justified only when all conditions hold:

- the untuned closed-loop baseline is reproducible;
- a frozen holdout and error taxonomy exist;
- repeated errors are plausibly learnable rather than missing core semantics;
- enough rights-cleared, high-quality traces exist;
- an adapted model has a clear deployment target and maintenance owner;
- success and rollback thresholds are predeclared.

Compare these options in order:

1. prompt/tool/schema changes;
2. retrieval or few-shot examples;
3. supervised full or partial tuning where feasible;
4. LoRA with rank/module ablation;
5. QLoRA only when memory pressure justifies it for the selected architecture;
6. sequence-level distillation into a smaller student after a stronger teacher
   and accepted-trace corpus exist.

The teacher is the best evaluated pipeline under a declared budget, not a brand
name. A teacher may combine deterministic tools and a model. A student is
replaceable and is evaluated from scratch on the same frozen holdout. Model
weights, adapters, processors and dataset snapshots are versioned artifacts
outside `nuif-core` and outside `.nuif` documents.

## Confidence and review policy

Raw provider confidence is calibrated on a disjoint calibration split for each
decision type. Reports include reliability and risk/coverage curves under
normal and shifted conditions. Automatic application requires both:

- the operation is valid and within the profile; and
- calibrated expected risk is below the profile threshold.

Otherwise the system retains alternatives, produces an explicit abstention or
asks for review. User confirmation becomes provenance; it is not retroactively
described as model certainty.

## Security and privacy

- All image, package, observation, operation, entity, text and iteration sizes
  are bounded before the corresponding expensive stage.
- Model outputs are untrusted inputs parsed by a strict operation decoder.
- Generated scripts, URLs and external resource requests are inert; no implicit
  network fetch or code execution occurs.
- Tool and renderer processes may be isolated with time, memory and GPU limits.
- Screenshot text may contain credentials, personal data or proprietary
  content; retention is opt-in and purpose-limited.
- Prompt injection embedded in a screenshot is visual content, never an
  instruction that can override the operation schema or tool policy.
- Training and inference dependencies carry a locked artifact/license bill of
  materials; one component's license is not generalized to a whole pipeline.

## Deployment boundaries

The reconstruction engine is an optional service or library beside the core:

```text
nuif-core / operations / renderer / evaluator    deterministic authority
nuif-reconstruct                                 orchestration and ports
observation providers                            replaceable local/remote tools
proposal provider                                replaceable model service/runtime
model artifacts                                  separately versioned and licensed
```

Local inference can keep screenshots private; remote inference requires an
explicit data-transfer policy. Browser/WASM and constrained-device consumers
use the resulting validated package and do not embed the training stack.

## NUIF relevance

This plan keeps probabilistic reconstruction outside the normative core while
making its outputs testable through the same operation, validation, rendering,
resource and fidelity contracts used by deterministic adapters. It supplies a
research path for screenshot import without weakening the distinction between
authored facts, source-backed observations and inferred hypotheses. Model
choice, adaptation method and deployment runtime remain replaceable; only the
typed evidence and result contracts are candidates for specification.

## Promotion gates

Screenshot reconstruction remains experimental until:

1. deterministic baseline, one-shot model and closed-loop variants run from one
   command and emit complete reports;
2. the frozen synthetic and real suites have documented rights and leak-resistant
   splits;
3. every automatic result is valid, provenance-complete and confidence-calibrated;
4. visual improvements do not come from flattening editable structure;
5. an independent evaluator reproduces the main reported results;
6. at least one real editing workflow demonstrates that reconstructed semantics
   are more useful than a flat screenshot;
7. resource, privacy, security and license audits pass for the selected runtime.

No alpha tag on the editor or core advances these gates. A future model artifact
may have its own experimental version, model card and benchmark report.

## Falsifiers

Narrow or stop the approach if:

- a flat screenshot or overfit absolute-position tree consistently beats the
  editable model under the chosen objective;
- held-out viewport performance does not improve over freeform layout;
- the correction loop cycles or improves appearance while degrading protected
  semantic metrics;
- confidence cannot be calibrated enough to support useful automatic coverage;
- tuned students fail to beat the untuned tool-augmented baseline after fair
  cost controls;
- rights-cleared data is insufficient for the claimed deployment domain;
- model/provider churn makes results irreproducible without freezing unsafe or
  unmaintainable dependencies.

## Open questions

- What is the smallest observation taxonomy that improves reconstruction across
  Web, desktop and mobile screenshots without becoming a second design schema?
- Should the first operation decoder use a constrained grammar, tool calls or a
  two-stage typed AST, and which has the lowest invalid/repair rate?
- How should equivalent but structurally different valid reconstructions be
  represented in training and evaluation?
- Which held-out edit tasks best measure real authoring usefulness?
- Can responsive rules be inferred robustly from two or three viewports, or is
  source-backed evidence required for practical accuracy?
- Which confidence events support safe automatic application and which should
  remain suggestions by design?
- What performance tier makes local reconstruction practical on developer
  hardware without choosing a normative model?
