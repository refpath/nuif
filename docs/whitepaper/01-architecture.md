# NUIF architecture thesis

NUIF is a specification-first authored-interface model. Its center is neither a vendor editor nor a source framework.

## Core thesis

A portable interface document must retain **intent, structure, relationships and evaluated results simultaneously**. A single flattened scene tree cannot preserve enough information for loss-minimizing round trips across editors and runtime frameworks.

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

### Borrowed foundations

- MLIR: dialects, explicit lowering, partial legality and multiple abstraction levels.
- OpenUSD: non-destructive composition, references, layers and variants.
- glTF: small core, extension registry, used/required capabilities.
- DTCG: token interchange.
- SVG/Unicode/OpenType: geometry and text foundations.
- Retentive/symmetric lenses: synchronization with preserved source regions.

### New work required

NUIF must define the missing combination: authored UI semantics + resolved state + cross-tool provenance + structural loss accounting + source patch synchronization.

## Canonical layers

1. **Document layer** — identity, containment, semantics, accessibility.
2. **Component layer** — definitions, instances, slots, parameters, variants and overrides.
3. **Layout layer** — authored sizing/layout intent independent of resolved geometry.
4. **Visual layer** — geometry, paint, text and effects.
5. **Behavior layer** — interactions, states, animation and data bindings.
6. **Resolved layer** — computed layout, shaped text, flattened paint/effect plans for a declared evaluation context.
7. **Provenance layer** — source/destination correspondence and fidelity diagnostics.
8. **Resource layer** — stable semantic assets bound to content-addressed bytes,
   package/resolver locators and derivation records.

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

The architecture fails if the v0 experiment cannot preserve a non-trivial responsive component through editor→HTML→NUIF→editor while retaining component identity, token bindings, layout intent, an opaque foreign extension and a minimal source patch after an edit.

The resource/reconstruction extension fails if independent package writers
cannot reproduce the proposed bytes, if browser capture cannot be pinned without
secret leakage, if visual objectives reward flat screenshot copies, or if
adaptation fails to beat the untuned tool-assisted baseline on a frozen holdout.
