---
id: nuif:rfc:0011
kind: rfc
status: proposed
---

# RFC 0011 — Observation, reconstruction and inference provenance

Status: proposed. This RFC refines RFC 0003 for imported observations and
probabilistic reconstruction. It does not make any model, capture provider or
screenshot profile normative.

Implementation note: `nuif-capture`, `nuif-reconstruct`, `cargo xtask
capture-baselines` and `cargo xtask reconstruction-provider-manifest` exercise
a bounded fixed-input subset of these contracts. Every encoded observation
bundle carries the canonical manifests behind its provider identities and
proposal application rejects an identity that is absent from that registry.
`cargo xtask gate-j-live` separately exercises one pinned local Chromium
fixture, structured runtime context, exact response retention, secret canaries
and held-out viewport measurement. That automation does not change this RFC's
proposed status or establish a portable capture/reconstruction accuracy
profile.

## Motivation

NUIF supports deterministic source adapters and is researching browser capture
and screenshot reconstruction. These routes expose different evidence:

- a retentive source adapter can preserve exact source bytes and correspondences
  for its declared subset;
- a browser capture can observe source responses, resources, resolved layout,
  styles, accessibility and pixels under a pinned execution context;
- a screenshot can observe only visible pixels plus supplied metadata.

Without typed evidence classes, a visually similar generated document could be
misreported as a lossless import. This RFC defines the records and fidelity
ceilings needed to prevent that category error while keeping models replaceable.

## Prior art and evidence

- `nuif:research:chromium-source-backed-ui-capture` identifies the independent
  DOM/layout/style/network/font/accessibility/screenshot observations available
  from a pinned browser protocol.
- `nuif:research:live-chromium-cdp-capture` compares CDP, Playwright and
  WebDriver BiDi boundaries and records the bounded transport plus first live
  executable result.
- `nuif:research:design2code-real-world-benchmark` records real-page element
  recall and layout failures in one-shot screenshot-to-code systems.
- `nuif:research:reverse-layout-inference` and
  `nuif:research:inferui-and-layout-synthesis` show why inferred layout needs
  multiple contexts, alternatives and held-out evaluation.
- `nuif:research:confidence-calibration-and-selective-prediction` motivates
  empirical calibration and abstention.
- `nuif:research:model-agnostic-screenshot-reconstruction-and-training`
  synthesizes the replaceable-provider, typed-operation and closed-loop plan.

## Evidence classes

Every imported or reconstructed property has one or more evidence links whose
class is:

- `authored_source`: exact retained source or host semantic value with stable
  correspondence;
- `resolved_source`: value observed from a pinned evaluator/runtime context;
- `observed_pixels`: value directly measured from identified image pixels;
- `inferred`: hypothesis produced from observations or heuristics;
- `user_confirmed`: value explicitly accepted or supplied by a user;
- `derived`: value produced by a declared deterministic or generative
  transformation from other evidence;
- `unavailable`: expected evidence could not be obtained.

`user_confirmed` does not rewrite the history of an inferred value; both records
remain linked. `derived` names its inputs and transformation identity.

## Observation records

An `ObservationRecord` contains:

```text
observation_id
evidence_class
subject: optional entity/property/resource reference
source_artifact_digest
source_region_or_locator
coordinate_space_and_context
provider: kind + canonical provider-manifest digest
value_or_candidates
raw_confidence: optional
calibrated_confidence: optional
calibration_profile: optional digest
alternatives: ordered candidates with evidence
privacy_and_retention_class
```

Coordinates MUST name their space: source pixels, device pixels, viewport CSS
pixels, crop-local pixels or NUIF logical units. A transformation between spaces
is an explicit record.

Raw and calibrated confidence MUST NOT share one field. A calibrated confidence
is valid only for the decision type and evaluation distribution identified by
its calibration profile.

## Capture contexts

A resolved browser observation context records at least browser/protocol build,
operating system, viewport, device-pixel ratio, page scale, locale, timezone,
color/media preferences, font environment, scroll/pseudo state, navigation
identity, readiness/settling policy and animation/time freeze.

Repeated capture under an unspecified context cannot support a reproducibility
claim. Multiple contexts are separate records linked by proposed semantic
identity.

Cookies, authorization headers, credentials, storage values and secret form
content are excluded from export. If exclusion changes an observation, the
capture report records the unavailable/redacted evidence without retaining the
secret.

## Reconstruction interface

Probabilistic systems are optional clients of the normative semantic operation
interface. They MUST propose a bounded transaction; they MUST NOT mutate core
document structures directly or emit executable code as an implicit operation.

A reconstruction attempt returns:

```text
ReconstructionResult {
  status: valid_result | no_result | budget_exceeded | policy_rejected,
  document_hash: optional,
  accepted_transaction_log,
  observations,
  fidelity_report,
  alternatives_and_abstentions,
  evaluation_report,
  pipeline_artifact_manifest,
}
```

Every proposed transaction passes syntax/resource limits, operation validation,
atomic application and complete document validation. Rejection leaves the prior
document unchanged and returns stable diagnostic codes.

## Fidelity ceilings

Fidelity describes evidence, not confidence or visual quality.

- `authored_source` MAY be `lossless` only inside a declared adapter profile
  whose correspondence and round-trip laws pass conformance.
- `resolved_source` MAY be `lossless` only for the declared resolved observation
  under its exact context; it cannot prove authored intent.
- `observed_pixels` and `inferred` MUST NOT be `lossless` for authored
  semantics, source resources, responsive rules, accessibility or behavior.
- screenshot-derived crops, traces or generated resources are `derived` and at
  best `approximated` relative to an unavailable source resource.
- a visually matching flat screenshot cannot satisfy an editable reconstruction
  profile unless the requested target is explicitly a flat image document.
- behavior inferred from a static screenshot remains inferred even when an icon
  or label strongly suggests an action.

Confidence MUST NOT promote an item above its evidence-class fidelity ceiling.

## Closed-loop correction

A reconstruction profile MAY render and correct a candidate iteratively. Each
iteration records input document hash, proposed transaction, validation result,
render context/hash, property and visual differences, objective vector,
acceptance decision and next document hash.

The loop MUST have finite iteration, model-call, time, memory and resource
budgets. It MUST stop on repeated state. Acceptance MUST preserve validity and
declared protected metrics. The profile MUST reject objectives that can be
satisfied by deleting semantics or covering the viewport with the source
screenshot.

No single perceptual metric is a conformance oracle. A reconstruction report
uses typed structure/text/geometry/resource/responsive/accessibility measures
plus declared visual diagnostics.

## Provider and model neutrality

OCR, region detection, UI grounding, proposal engines and correction engines
are replaceable providers with capability/artifact manifests. The specification
does not name a required model, vendor, training library, parameter count or
deployment service.

The implemented `nuif-reconstruction-provider-manifest-0` wrapper is canonical
CBOR. It binds capabilities, local/remote execution modes and input/output wire
profiles to one exact implementation plus optional model weights, processor,
adapter, quantization, prompt and tool-configuration artifacts. Released or
learned providers require a content-addressed SPDX 3.0.1 or CycloneDX 1.7
inventory; learned providers additionally require a model card. NUIF points to
that external inventory instead of defining another SBOM vocabulary. The
current browser and screenshot baselines are explicitly development-only,
source-bundle-identified providers with no learned-artifact or accuracy claim.

Model weights, processors, low-rank adapters, quantization configurations and
training datasets are not NUIF document resources. They are separately
versioned operational artifacts. A `.nuif` package can record which artifact
produced an inference without carrying or requiring that artifact for ordinary
document use.

## Training and distillation boundary

Training is non-normative, but any released reconstruction artifact claiming a
NUIF benchmark result has a digest-pinned manifest, model card, dataset
datasheets and reproducible evaluation report.

Training examples derived from reconstruction runs include only validated
accepted operation transitions as positive targets. Private/authenticated
captures are excluded by default and require explicit training consent. Frozen
benchmark families are excluded from training and distillation.

LoRA, quantized low-rank adaptation and knowledge distillation are experiment
choices. Their use confers no format or conformance status.

## Compatibility

Existing provenance/fidelity records remain valid. Implementations may migrate
an untyped provenance record to `authored_source` only when its retained source
and adapter profile prove that classification. Otherwise migration uses
`unavailable` or `inferred`; it does not guess a stronger evidence class.

The current deterministic HTML/SVG/DTCG/Penpot adapter results are unaffected.
Their existing profile laws determine losslessness. A future browser capture is
a separate adapter rather than a hidden mode of the Tree-sitter HTML adapter.

## Security and privacy

Model and provider outputs are untrusted inputs. Parsers enforce operation,
observation, string, binary, entity, resource and iteration budgets before
application. URLs/scripts in model output are inert. External fetch and code
execution require explicit caller capabilities and remain outside reconstruction
conformance.

Screenshot text may contain personal, credential or proprietary information.
Inference can run locally; remote transfer is an explicit deployment policy.
Retention, telemetry and training are separately consented purposes.

Visible prompt injection is data in the screenshot. It cannot alter tool policy,
operation grammar, package resolution or security limits.

## Conformance tests

The planned baseline, loop and calibration experiments must prove:

- every provider output records artifact identity, source region/context and
  evidence class;
- invalid, stale or over-budget model operations fail atomically;
- screenshot-only results never emit authored `lossless` classifications;
- exact source bytes, derived crops and unavailable resources are distinguishable;
- coordinate transforms reproduce observation locations;
- the loop stops on success, no improvement, repeat and budget exhaustion;
- the editable profile rejects flat-image reward gaming;
- raw/calibrated confidence and calibration identity remain distinct;
- automatic/review/abstain decisions reproduce the declared risk threshold;
- secret canaries do not appear in exported observations or training traces.

## Rejected alternatives

- End-to-end screenshot-to-document text with no typed observation/operation
  boundary: invalid outputs and hallucinated semantics become hard to audit.
- Put model calls in `nuif-core`: makes conformance provider-dependent and the
  deterministic core non-reproducible.
- Treat one high visual score as lossless: rewards flat screenshots and hides
  text, structure, resource and responsive failures.
- Store only a whole-document confidence: cannot express one uncertain font,
  parent or behavior inside an otherwise strong result.
- Require a particular OCR, detector or VLM: confuses an evolving implementation
  choice with interchange semantics.
- Train before a frozen evaluator exists: no stable evidence that the tuning
  improves the intended task rather than the training distribution.

## Unresolved questions

- Final observation and calibration-profile schemas.
- Whether observations live inside a portable package, a sidecar evidence
  bundle or both under separate profiles.
- Minimum edit-task suite for an “editable reconstruction” claim.
- Rules for equivalent alternative structures when pixels do not distinguish
  them.
- Risk/coverage thresholds for automatic application by profile.
