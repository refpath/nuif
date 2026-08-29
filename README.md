# NUIF

**NUIF** is an open research and implementation project for a portable, vendor-neutral interface authoring standard.

NUIF is not a Figma clone and Figma is not its canonical model. The goal is an independent document, layout, rendering, operation, and interchange standard that editors, design systems, runtimes, source frameworks, and automation tools can implement directly or target through adapters.

The repository serves four purposes at once:

1. **Research corpus** — whitepaper-grade prior art, academic research, experiments, claims, evidence, and unresolved questions structured for machine indexing.
2. **Normative specification** — versioned modules defining the NUIF logical model, layout semantics, rendering semantics, operations, serialization, extensions, and conformance.
3. **Reference implementation** — a Rust core, headless CLI/API, renderer, codecs, and adapters that prove the specification.
4. **Reference editor** — a focused native NUIF editor used to test authoring, portability, responsiveness, components, tokens, fidelity, and automation.

## Design principles

- **Standard first.** The logical model is independent of any editor or vendor format.
- **Authored intent + resolved state.** Preserve constraints, semantics, components, tokens, provenance, and evaluation results rather than reducing designs to pixels or fixed coordinates.
- **Portability is synchronization.** Diff, patch, provenance, and reconciliation are first-class; whole-project regeneration is not the default round-trip model.
- **Small core, extensible dialects.** Vendor-specific data can survive tools that do not understand it.
- **Machine-operable by default.** Every meaningful editor capability must be available through deterministic CLI/API operations so automated and AI QA can inspect, mutate, render, replay, and verify documents without brittle GUI automation.
- **Research is data.** Every source, claim, experiment, decision, and relationship should be indexable into the Refpath research graph without requiring NLP reconstruction.
- **Falsifiable architecture.** Early prototypes are designed to expose information-loss boundaries and invalidate weak assumptions quickly.

## Status

Pre-standardization research and reference implementation bootstrap. No part of the current specification should be considered stable or normative until explicitly promoted.

## Repository map

- `docs/whitepaper/` — founding whitepaper and synthesis
- `research/` — machine-readable research corpus and source records
- `spec/` — normative and draft NUIF specification modules
- `rfcs/` — proposed changes to the standard and architecture
- `adrs/` — implementation/architecture decisions
- `schemas/` — schemas and IDLs
- `crates/` — Rust reference implementation
- `apps/editor/` — reference editor shell
- `adapters/` — external format/framework adapters
- `conformance/` — test suites, fixtures, golden outputs, and capability profiles
- `examples/` — canonical NUIF examples
- `tools/` — research/spec/indexing tooling

## License

Licensing and eventual neutral governance are still under research. Until explicit license files are added, do not assume rights beyond those granted by GitHub for viewing and forking public repositories.
