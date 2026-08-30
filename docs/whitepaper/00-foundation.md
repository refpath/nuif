---
id: nuif:whitepaper:foundation
kind: whitepaper
status: draft
version: 0.0.1
updated: 2026-08-29
---

# NUIF foundation

NUIF investigates a portable, vendor-neutral draft specification for authored user-interface documents. The candidate model is intended to preserve meaning across editors and implementation targets rather than treating a rendered bitmap, a vendor scene graph, or source-language AST as the universal truth.

## Thesis

A useful portable interface specification must coordinate several representations instead of collapsing them into one:

1. semantic/document containment;
2. component and instance identity;
3. authored layout and responsive constraints;
4. resolved geometry at explicit evaluation contexts;
5. geometry, paint, typography, and assets;
6. design-token references and themes;
7. interaction/state and data-binding graphs;
8. source/tool provenance and correspondence;
9. extension payloads that can survive unknown intermediaries;
10. deterministic operations, diff, patch, and reconciliation.

Portable resources add a second identity boundary: editable semantic assets
retain stable IDs, while exact image/font bytes use immutable content digests.
Package paths and source URLs are locators/provenance, not identity.

NUIF therefore treats portability as a synchronization problem as much as a serialization problem.

## Architectural hypothesis

The working model is a small canonical core plus coordinated graphs and extension dialects. The containment tree answers ownership and order. Typed relationship graphs express constraints, components, tokens, interactions, provenance, dependencies, and other relationships that do not belong in a tree.

The reference implementation will preserve both authored and resolved state. Resolved state is always scoped to an evaluation context and is never allowed to silently replace authored intent.

## Fidelity model

Every adapter and transformation must classify material mappings:

- `lossless` — semantics are preserved exactly;
- `representable` — equivalent target semantics exist, even if encoded differently;
- `approximated` — a declared approximation is produced;
- `preserved_unrenderable` — data survives as an extension but the target cannot render/edit it;
- `unsupported` — data cannot currently be represented or preserved safely.

Silent loss is a conformance failure.

## Explicit non-goals

NUIF does not promise to infer the unique original source program from pixels, reproduce arbitrary JavaScript execution, make every platform text renderer bit-identical, or force every target to support every capability. The draft specification should make such boundaries inspectable and machine-readable.

Screenshot reconstruction is therefore an optional inference client, not a new
canonical truth. It may propose a validated editable hypothesis and calibrated
alternatives, but screenshot-only evidence cannot be classified as lossless
authored source.

## Reference implementation role

The Rust implementation and editor are executable research instruments and conformance references. They do not define semantics by accident; normative behavior belongs in `spec/` and must be testable independently.
