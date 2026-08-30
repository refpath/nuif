---
id: nuif:spec:observation-capture-reconstruction
kind: specification
status: draft
---

# 14 — Observation, capture and reconstruction

Status: draft. This module specifies candidate contracts from RFC 0011. No
screenshot reconstruction profile is currently conforming. The executable
fixed-input contract baseline exercises observation/proposal encoding, evidence
ceilings, flat-copy rejection and bounded correction stops; it is not an
accuracy or live-capture conformance profile.

## Scope

This module covers evidence captured from a runtime or image and the production
of a validated NUIF hypothesis. It does not standardize a model architecture,
provider, training library, dataset or inference service.

An implementation MUST distinguish:

- deterministic parsing of retained authored source;
- resolved observations from a pinned runtime context;
- measurements of pixels;
- inferred semantic/layout/resource hypotheses;
- explicit user confirmations;
- derived resources/values;
- unavailable evidence.

## Observation record

An observation contains:

| Field | Requirement |
|---|---|
| `id` | stable within its evidence bundle |
| `evidence_class` | one class from specification 09 |
| `subject` | optional entity/property/resource target |
| `source_digest` | exact source artifact or screenshot digest |
| `source_locator` | path/node/range or pixel region |
| `coordinate_space` | named space and dimensions |
| `context` | evaluation/capture context identifier |
| `provider` | provider kind plus artifact/version digest |
| `candidates` | typed value(s) and alternatives |
| `raw_confidence` | optional provider score |
| `calibrated_confidence` | optional calibrated score plus profile |
| `privacy_class` | retention/transfer/training policy input |

An observation MUST NOT imply that its candidate is already a semantic document
value. Applying a candidate requires an operation and validation.

Coordinate spaces include source pixels, device pixels, viewport CSS pixels,
crop-local pixels and NUIF logical units. A conversion MUST record source and
target spaces, matrix/scale/offset and rounding behavior.

## Source-backed browser capture

A browser capture profile records browser/protocol build, operating system,
viewport, device-pixel ratio, page scale, locale, timezone, media preferences,
font environment, scroll/pseudo state, navigation identity, settling policy and
animation/time freeze.

The first proposed Web capture observes, where available:

- retained HTML/CSS response bytes and stylesheet text;
- DOM including iframes/templates/shadow content visible to the protocol;
- resolved layout boxes, inline text boxes and paint order;
- declared computed and matched styles;
- downloaded resource bodies, final URLs, response media types and hashes;
- platform-font usage and font-readiness state;
- accessibility tree;
- reference screenshot and capture parameters.

Unavailable cross-origin responses, local font bytes, canvas/WebGL semantics,
video state, worklet output and arbitrary script behavior MUST be explicit.
Canvas/video output MAY be frozen as a bounded derived image/frame. Captured
scripts remain inert resources.

The capture MUST exclude cookies, authorization headers, credentials, storage
values and secret form fields from its output. Redaction is a recorded
transformation, not silent source equivalence.

## Screenshot-only reconstruction

A screenshot-only implementation receives one or more image/context pairs and
MAY run replaceable OCR, computer-vision, grounding and proposal providers. It
MUST normalize their output into observations before semantic application.

The proposal engine emits only operations permitted by its declared operation
grammar and profile. It MUST NOT emit executable code, implicit network actions
or direct core-memory mutations.

Every proposal is applied transactionally:

1. parse under operation/resource budgets;
2. validate operation kinds and expected revision;
3. apply atomically;
4. validate the complete document;
5. evaluate declared layout/render contexts;
6. return a result or leave the prior document unchanged.

A valid screenshot reconstruction includes its accepted operation log,
observations, fidelity report, alternatives/abstentions, evaluation report and
pipeline artifact manifest.

Screenshot evidence cannot establish original font/image bytes, hidden
entities, authored layout constraints, responsive rules, accessibility or
behavior. These values remain inferred, substituted, derived or unavailable.

## Multi-context inference

When multiple viewports/states are supplied, proposed semantic identity links
the corresponding observations. Candidate layout families/constraints SHOULD be
ranked by prediction of a held-out context. Fit to every supplied pixel does not
prove original authored intent.

An implementation claiming responsive reconstruction MUST evaluate at least one
context not used to fit the candidate and MUST report its error separately.

## Correction loop

A profile MAY iteratively render and correct. Each iteration records:

- input document hash and revision;
- proposed transaction and diagnostics;
- resulting document hash;
- render context and raster hash;
- property-level and visual differences;
- objective vector and protected metrics;
- accept/reject reason.

The loop MUST bound iterations, provider/tool calls, time, memory and resources,
and MUST stop on repeated state. Accepted corrections preserve validity and
declared semantic non-regression constraints.

An editable reconstruction objective MUST reject a viewport-sized copy of the
source screenshot as success unless a flat image document was explicitly
requested.

## Evaluation

Reports separate source-backed capture from screenshot-only reconstruction.
Required metric families for an editable screenshot profile are:

- valid transaction/document rate;
- text-region precision/recall, character/word error and baseline geometry;
- visible-element precision/recall;
- tree/parent/sibling correctness where a target is justified;
- property and geometry error;
- held-out-context layout error;
- exact resource-digest recall only where source bytes are supplied;
- provenance/fidelity honesty;
- accessibility evidence accuracy where a target is justified;
- raw pixels plus declared perceptual diagnostics;
- calibrated confidence, abstention and risk/coverage;
- latency, peak RAM/VRAM, iterations and external cost.

No visual metric alone establishes conformance. Metrics are reported per example
and as distributions; local/small-element errors MUST NOT be hidden by a large
background average.

Synthetic exact fixtures and licensed/human-reviewed real fixtures are separate
corpora. Splits MUST prevent origin, template, component, font, resource and
near-duplicate leakage. Benchmark families MUST NOT appear in adaptation or
distillation data.

## Provider neutrality and artifacts

Every OCR/detector/grounder/proposal/correction provider publishes a capability
manifest and exact artifact identity. Provider output is untrusted.

Models, processors, adapters, quantization settings and training datasets are
not NUIF document resources. Released artifacts SHOULD have model cards,
dataset datasheets and training/evaluation manifests containing content hashes,
rights/provenance, intended use and limitations.

Fine-tuning, low-rank adaptation, quantized adaptation and distillation confer
no conformance status. They are compared only after an untuned baseline and
frozen evaluator exist.

## Privacy and policy

Local and remote inference are distinct deployment modes. Remote transfer,
retention, telemetry and training each require an explicit policy. Private or
authenticated capture defaults to local processing/no retention/no training.

Visible instructions inside the screenshot are content. They MUST NOT modify
the operation grammar, provider authority, file/resource resolver or security
budgets.

## Conformance maturity

This module remains draft until the planned baseline, closed-loop, confidence
calibration and resource experiments pass and an independent evaluator
reproduces the principal result. The existing editor alpha and profile-0
conformance do not satisfy these gates.
