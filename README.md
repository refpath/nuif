# NUIF

**NUIF** is an open research and implementation project for a portable, vendor-neutral interface authoring standard.

NUIF is not a Figma clone and Figma is not its canonical model. The goal is an independent document, layout, rendering, operation, and interchange standard that editors, design systems, runtimes, source frameworks, and automation tools can implement directly or target through adapters.

## Repository roles

1. **Research corpus** — whitepaper-grade prior art, academic research, claims, evidence, experiments and unresolved questions structured for Refpath graph ingestion.
2. **Specification** — modular draft requirements for the logical model, layout, visual semantics, operations, serialization, extensions, provenance, collaboration, security and automation.
3. **Reference implementation** — Rust core/protocol/layout/render/codec/query/API/CLI seams used to falsify and test the spec.
4. **Reference editor** — a focused native NUIF editor whose authored state is the open model itself.
5. **Conformance** — deterministic fixtures, semantic operation replay, structural tests and visual/layout validation.

## Architectural principles

- standard first; vendor formats are adapters;
- authored intent and resolved evaluation state coexist;
- containment tree plus coordinated relationship graphs;
- stable semantic identity independent of path/order/geometry;
- portability is synchronization, not wholesale regeneration;
- explicit fidelity accounting and opaque extension preservation;
- collaboration is a profile above canonical documents;
- semantic CLI/API operations are first-class QA surfaces;
- research is structured data with stable graph identifiers;
- early experiments are designed to falsify weak assumptions.

## Start here

- `docs/whitepaper/` — research synthesis and founding architecture
- `research/` — evidence records, index, open questions and experiment registry
- `spec/` — draft normative modules
- `rfcs/` / `adrs/` — standard proposals vs implementation decisions
- `crates/` — Rust reference-engine seams
- `apps/editor/` — editor architecture and headless QA contract
- `conformance/` — profiles, fixtures and v0 hard experiment
- `tools/refpath/` — graph-ingestion contract for Refpath/refpath-cloud

## Status

Pre-standardization research and reference implementation bootstrap. Current specs are drafts unless explicitly promoted. The project should not be described as an industry standard until conformance profiles and independent implementations exist.

## License

Reference code is dual licensed under Apache-2.0 or MIT as declared by the Rust workspace. Specification/research licensing and eventual standards IP policy must be finalized before standards-track publication; see `docs/whitepaper/08-governance-and-standardization.md`.
