---
id: nuif:whitepaper:foundation
kind: whitepaper
status: draft
version: 0.0.1
updated: 2026-09-06
---

# NUIF foundation

NUIF investigates authored user-interface interchange through a draft document
model, a Rust reference implementation and bounded source-adapter experiments.
The research question is whether explicit identity, correspondence and fidelity
records permit useful edits across representations without discarding unrelated
source or unknown extension data. The current evidence concerns named profiles;
it does not establish arbitrary application interchange.

## Architectural hypothesis

The model separates containment, typed relationships, authored properties and
resolved snapshots. A snapshot records an evaluation context rather than
replacing the authored document. Editable entities and assets have stable
identities; immutable resource bytes have content digests. These distinctions
are specified in [identity](../../spec/02-identity-and-properties.md),
[layout](../../spec/04-layout.md) and
[serialization](../../spec/08-serialization.md).

The proposed architecture coordinates components, layout, visual properties,
tokens, behavior, provenance and extensions. These areas have different
implementation maturity. The [adapter inventory](../../adapters/STATUS.md) and
[research coverage contract](../../research/coverage.yaml) delimit the
implemented projections and unresolved questions. The architecture is tested
through the following hypotheses:

- For a declared scalar-edit profile, re-import of synchronized source equals
  the requested document, and bytes outside the reported edits remain identical.
- Unknown extension payloads survive supported neighboring edits and canonical
  serialization without requiring the editor to interpret those payloads.
- Pinned evaluation contexts allow implementations to compare resolved output
  under explicit exact or tolerance-based criteria.

Each hypothesis requires evidence for its own domain. Success for scalar HTML
edits does not establish structural source reconciliation, behavior equivalence
or native rendering portability.

## Research method

The repository combines targeted primary-source review, executable profile
construction, generated tests, differential comparisons and failure analysis.
Research records preserve source locators, retrieval dates, interpretation and
open questions. The search process is iterative and architecture-directed; it
is not a systematic literature review with a preregistered search strategy or
an exhaustive publication census. A `reviewed` record is not an independently
verified result; [the audit policy](../../research/AUDIT.md) defines the stronger
verification conditions.

Evidence is reported at the level of the tested assertion:

| Method | Observation | Limitation |
|---|---|---|
| Reference fixtures | Expected document, operation or encoded bytes agree | Expected values can share an implementation error |
| Generated and metamorphic tests | Declared relations hold across deterministic input variations | Generator coverage does not estimate real-document prevalence |
| Differential comparisons | Named implementations agree on specified outputs | Agreement can reflect shared assumptions or upstream code |
| Repeated local execution | A pinned revision and setup reproduce the same artifact | This establishes local repeatability, not external reproduction |
| Independent external evaluation | Another team executes or implements the protocol | Required evidence remains open where no external result is recorded |

The distinction between local repetition and another team's reproduction follows
[ACM's artifact-review terminology, version 1.1](https://www.acm.org/publications/policies/artifact-review-and-badging-current),
retrieved 2026-09-06. No ACM evaluation or badge is claimed. Profile-specific
scope and conformance clauses follow the approach described in the
[W3C QA Framework, sections 2–3](https://www.w3.org/TR/2005/REC-qaframe-spec-20050817/),
retrieved 2026-09-06; this is a methodological reference, not W3C endorsement.

The verification manifest records revision, environment and artifact hashes.
Performance measurements require a workload, build mode, hardware, sampling
procedure and reference implementation. Timing on one host does not establish
a cross-platform performance bound. A valid research registry establishes
traceability, not resolution of its open experiments.

## Fidelity model

The [fidelity specification](../../spec/09-provenance-and-fidelity.md) defines
five classes: `lossless`, `representable`, `approximated`,
`preserved_unrenderable` and `unsupported`. A class applies to a stated mapping
and evidence boundary. Preserving an unknown payload does not establish that a
target renders or edits its meaning. Retaining foreign CSS bytes does not
establish visual equivalence under the browser cascade.

Screenshot reconstruction produces hypotheses about a document. Multiple
programs can render the same pixels, so screenshot-only evidence cannot identify
unique authored source. Reconstruction accuracy, confidence calibration and
edit usefulness require separate held-out evaluation, as described in the
[resource and reconstruction chapter](12-resources-capture-and-reconstruction.md).

## Threats to validity

The implementation and many fixtures share authorship. Generated inputs cover
selected structural families and boundary values, with no claim of sampling
from a deployment population. Dependency pins constrain variation but can mask
host-specific behavior. Small oracle matrices leave untested combinations of
fonts, layout and browser state. The native editor has no completed external
usability study. These limitations prevent extrapolation from passing local
gates to general beta readiness or broad interoperability.

The [developer beta contract](../BETA.md) defines the narrower workflow and the
evidence required for promotion. The [risk register](07-risk-register.md)
identifies observations that would require narrowing or revising the design.
