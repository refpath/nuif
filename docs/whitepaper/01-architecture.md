# NUIF architecture thesis

NUIF defines authored-interface semantics in draft specification modules and
tests selected projections through a reference implementation. The model,
adapter profiles and implementation choices have separate acceptance criteria.

## Core thesis

The architectural hypothesis is that explicit authored properties, typed
relationships and context-scoped evaluated results permit source edits with
declared fidelity. Flattening is treated as a lowering with a loss report when
it removes information required by subsequent edits.

The recommended architecture is a layered hybrid:

```text
Document containment tree
        │ stable IDs
        ├── component / instance graph
        ├── token / theme graph
        ├── layout constraint graph
        ├── interaction / state graph
        ├── provenance / correspondence graph
        └── asset dependency graph

Authored model ──evaluate/lower──► resolved model ──► render scene
      ▲                               │
      └──────── reconcile / lift ◄────┘
```

### Prior-art foundations

- MLIR: dialects, explicit lowering, partial legality and multiple abstraction levels.
- OpenUSD: non-destructive composition, references, layers and variants.
- glTF: small core, extension registry, used/required capabilities.
- DTCG: token interchange.
- SVG/Unicode/OpenType: geometry and text foundations.
- Retentive/symmetric lenses: synchronization with preserved source regions.

The [prior-art comparison](05-prior-art-and-competitive-map.md) and
[cross-industry synthesis](11-cross-industry-patterns.md) identify the source
records and adaptation decisions.

### Integration work

The unresolved integration work concerns correspondence between authored
properties, resolved observations and source syntax. The
[adapter inventory](../../adapters/STATUS.md) distinguishes implemented scalar
synchronization from unsupported structural changes.

## Canonical layers

| Layer | Proposed responsibility |
|---|---|
| Document | Identity, containment, semantics and accessibility |
| Component | Definitions, instances, slots, parameters, variants and overrides |
| Layout | Authored sizing and constraints |
| Visual | Geometry, paint, text and effects |
| Behavior | Interactions, state transitions, animation and data bindings |
| Resolved | Computed layout, shaped text and paint for an evaluation context |
| Provenance | Correspondence and fidelity diagnostics |
| Resource | Semantic assets, byte resources, locators and derivation records |

This decomposition includes proposed semantics beyond the implemented profiles.
[Research coverage](../../research/coverage.yaml) records their evidence status.

No lower layer is permitted to silently erase a higher-level authored construct. Lowerings that cannot represent a construct must emit fidelity records.

## Stable identity

Identity is semantic and independent of path, order and display name. Moving an entity does not change its ID. Content hashes identify immutable resources and canonical snapshots, not editable semantic entities.

## Compiler and reconstruction ports

Deterministic source adapters and probabilistic screenshot reconstruction meet
at the operation boundary:

```text
retained source + resolved host observations ─┐
                                              ├─> typed operations -> core
pixels + OCR/CV/model hypotheses ─────────────┘                    -> render/evaluate
```

Source-backed and screenshot-only inputs retain distinct evidence classes. A
model/provider is replaceable and cannot redefine the operation grammar,
validator, layout semantics, resource identity or fidelity ceilings.

## Falsifiability

The [v0 HTML experiment](../../adapters/html-css/V0-PROFILE.md) tests retention
of component identity, token bindings, layout intent and an opaque extension
through an editor/source round trip. Its named fixture passes. The broader
hypothesis remains subject to counterexamples from other documents and edits.

The resource/reconstruction extension fails if independent package writers
cannot reproduce the proposed bytes, if browser capture cannot be pinned without
secret leakage, if visual objectives reward flat screenshot copies, or if
adaptation fails to beat the untuned tool-assisted baseline on a frozen holdout.
