---
id: nuif:spec:provenance-and-fidelity
kind: specification
status: draft
---

# 09 — Provenance, correspondence and fidelity

Status: draft.

`ProvenanceRecord` identifies an origin system, source artifact revision and optional source path/range/node/property identity.

`CorrespondenceRecord` maps NUIF identities/properties to one or more foreign identities/properties and can retain adapter-specific reconstruction hints.

Correspondence is optional canonical-adjacent metadata: it may be stored in a package/profile without changing the semantic document.

Every import/export/lowering returns a `FidelityReport` with item-level status: lossless, representable, approximated, preserved_unrenderable or unsupported. Diagnostics identify the entity/property and transformation pass responsible.

This mechanism is informed by symmetric and retentive lens research.

## Evidence classes

Provenance for capture/reconstruction additionally declares one evidence class:

- `authored_source`
- `resolved_source`
- `observed_pixels`
- `inferred`
- `user_confirmed`
- `derived`
- `unavailable`

The evidence class and fidelity status answer different questions. Evidence
states what was available; fidelity states what was preserved or represented.
Confidence states predicted correctness and MUST NOT promote a weaker evidence
class to a stronger fidelity claim.

`authored_source` MAY be `lossless` only under a declared adapter round-trip
profile. `resolved_source` can be exact for one pinned resolved context without
proving authored intent. `observed_pixels` and `inferred` MUST NOT be `lossless`
for authored structure, original resources, responsive rules, accessibility or
behavior. User confirmation is an additional record and does not erase the
original inference history.

A `derived` record identifies its input digests and exact deterministic or
generative transformation. Screenshot crops, traces, inpainting and upscaling
are derived resources, never recovered originals.

## Confidence and alternatives

Raw provider confidence and calibrated confidence are separate fields.
Calibrated confidence identifies a versioned calibration profile and typed
correctness event. Whole-document confidence cannot substitute for property- or
decision-level confidence.

When evidence is ambiguous, an implementation SHOULD preserve ranked
alternatives or abstain. Automatic application MAY be governed by a declared
risk/coverage policy; below-threshold values require review or remain unresolved.

## Observation identity

An observation records source artifact digest, source region/locator,
coordinate space, evaluation context, provider artifact/version, candidate
values, evidence class, confidence and privacy/retention class. Coordinate
transforms between source pixels, device pixels, viewport pixels, crop-local
pixels and NUIF units are explicit.
