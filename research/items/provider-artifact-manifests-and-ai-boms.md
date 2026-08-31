---
id: nuif:research:provider-artifact-manifests-and-ai-boms
kind: synthesis
status: reviewed
title: Content-addressed provider manifests and external AI bills of materials
source:
  url: https://spdx.github.io/spdx-spec/v3.0.1/model/AI/AI/
  repository: https://github.com/spdx/spdx-3-model
  authors: [SPDX Project, OWASP CycloneDX, Ecma International, MLflow Project, ONNX Project]
  published_at: "2025-2026"
  license: respective specification and project documentation terms
retrieved_at: 2026-08-31
tags: [providers, artifacts, ai-bom, spdx, cyclonedx, model-cards, supply-chain]
confidence: 0.97
claims: [nuif:claim:model-artifact-separation, nuif:claim:bounded-untrusted-input, nuif:claim:typed-reconstruction-loop]
relations:
  - type: extends
    target: nuif:research:model-and-dataset-documentation
  - type: related_to
    target: nuif:research:oci-resource-descriptors
  - type: related_to
    target: nuif:research:model-agnostic-screenshot-reconstruction-and-training
links:
  spec: [spec/14-observation-capture-and-reconstruction.md]
  adr: []
  rfc: [rfcs/0011-observation-and-inference-provenance.md]
  code: [crates/nuif-reconstruct/src/provider.rs, crates/nuif-reconstruct/src/lib.rs, crates/nuif-capture/src/lib.rs, crates/nuif-capture/src/live.rs, crates/nuif-testing/src/bin/reconstruction-provider-manifest.rs]
  experiments: [nuif:experiment:reconstruction-provider-manifest]
---

# Summary

Reproducible reconstruction needs more than a provider name or mutable model
version. The observation must identify the exact operational system that
produced it: implementation, weights, processor, task adapter, quantization,
prompt/tool configuration and supported wire profiles. That identity must
change when any bound artifact changes.

NUIF should not define a competing software or machine-learning bill of
materials. SPDX 3.0.1 has AI and Dataset profiles, while CycloneDX 1.7 has a
machine-learning-model component and model-card structures. The selected design
therefore uses a small canonical NUIF capability wrapper and content-addresses
an external SPDX or CycloneDX inventory. The wrapper travels with observation
evidence; the potentially larger inventory, model card and artifacts remain
separately addressable.

Runtime packaging is a separate concern. MLflow packages model metadata,
dependencies, signatures and flavors for loading; ONNX supports tensors stored
outside the model protobuf. Those patterns can carry a provider, but neither
substitutes for a provider-neutral capability and evidence identity.

## Evidence

- SPDX 3.0.1 publishes distinct AI and Dataset model profiles alongside the
  core software-bill-of-materials model. Source:
  https://spdx.github.io/spdx-spec/v3.0.1/model/AI/AI/, retrieved 2026-08-31.
- The CycloneDX 1.7 JSON schema admits `machine-learning-model` components and
  defines a model card with model parameters and quantitative analysis. Its
  specification overview identifies ECMA-424 as the formal standard. Sources:
  https://github.com/CycloneDX/specification/blob/1.7/schema/bom-1.7.schema.json
  and https://cyclonedx.org/specification/overview/, retrieved 2026-08-31.
- Ecma's ECMA-424 page describes the CycloneDX v1.7 bill-of-materials format as
  a structured inventory. Source:
  https://ecma-international.org/publications-and-standards/standards/ecma-424/,
  retrieved 2026-08-31.
- MLflow's model documentation separates the model package, environment,
  flavors and input/output signature. This is useful deployment metadata but
  is tied to the MLflow loading contract. Sources:
  https://mlflow.org/docs/latest/ml/model/index.html and
  https://mlflow.org/docs/latest/ml/model/signatures, retrieved 2026-08-31.
- ONNX external data stores tensor content outside the protobuf and requires a
  relative location; parent-directory components are disallowed. This is a
  useful large-artifact packaging rule, not a complete lineage or capability
  manifest. Source: https://onnx.ai/onnx/repo-docs/ExternalData.html,
  retrieved 2026-08-31.

## Mechanism

`nuif-reconstruction-provider-manifest-0` is bounded deterministic CBOR. It
declares provider identity and maturity, capabilities, local/remote execution,
input/output profiles and exact SHA-256 identities for one implementation plus
optional model, processor, adapter, quantization, prompt-template and
tool-configuration artifacts. The hash of the canonical bytes is the
`ProviderIdentity` stored on every observation and proposal.

An observation bundle includes the canonical manifests for every referenced
provider. Validation derives every identity again, rejects duplicates and
rejects observations or proposals whose identity does not resolve. This makes
the evidence locally auditable without embedding model weights or fetching a
network resource.

Development-only deterministic providers may omit a supply-chain inventory
when they contain no learned artifacts. Released or learned providers require
an exact SPDX 3.0.1 or CycloneDX 1.7 inventory digest. Learned artifacts also
require a model-card digest. Dataset snapshots remain governed by the separate
corpus manifest and dataset card; a future training-run record must bind those
inputs and produced provider artifacts.

## NUIF relevance

**Borrow** SPDX/CycloneDX inventory vocabularies, model cards, immutable
artifact identity and deployment-package separation.

**Adapt** them with a small NUIF wrapper that states only the capabilities and
wire profiles needed to interpret reconstruction evidence.

**Reject** mutable provider/version strings, an unresolvable manifest digest,
embedding weights in ordinary `.nuif` document resources, treating MLflow or
ONNX as the interchange standard, and assuming that a valid inventory proves
completeness, security, performance, rights or safety.

## Open questions

- Which signed statement format and transparency log should bind released
  manifests, inventories, cards and binaries after release provenance exists?
- Should a future remote-provider profile bind endpoint policy and attestation
  separately from the model/implementation manifest so operational rotation
  does not rewrite artifact identity?
- What training-run vocabulary can reuse SPDX/CycloneDX relationships while
  preserving dataset split, evaluator, seed, hardware and accepted-transition
  evidence without duplicating the corpus contract?
