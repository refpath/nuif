# Protocol, portability and synchronization

NUIF models editing through semantic operations and source correspondence.
[Operations and patches](../../spec/06-operations-and-patches.md) define the
protocol; the [adapter inventory](../../adapters/STATUS.md) limits the currently
implemented source mappings.

## Operations

The protocol operates on stable entities and semantic properties. The draft operation vocabulary includes creation, deletion, movement, property
changes, relation edits, overrides, token bindings and extension edits within
transactions. Editor gestures lower to these operations.

A drag inside a stack should usually become a reorder or layout-property edit; a drag in freeform space may become a transform edit. GUI coordinates are input data, not the protocol abstraction.

## Patch model

A patch is a deterministic ordered set of operations with base snapshot identity, optional preconditions, transaction metadata and provenance. Patches can be replayed headlessly.

Three-way merge uses stable identity first and structural matching only when identity is absent. Conflicts are typed: property, delete/edit, ordering, relationship, extension and semantic-lowering conflicts.

## Correspondence

Adapters maintain correspondence records between NUIF entities/properties and foreign constructs such as DOM nodes, CSS declarations, static component attributes or design-tool node IDs. Correspondence is separable from the canonical design so source-specific metadata can be detached when unnecessary.

## Fidelity classes

Every adapter/evaluator may report:

- `lossless` — semantics preserved and reconstructable.
- `representable` — equivalent semantics represented through different constructs.
- `approximated` — visible/behavioral approximation with known semantic loss.
- `preserved_unrenderable` — data retained opaquely but not understood/rendered.
- `unsupported` — data could not be safely preserved.

Silent degradation is a conformance failure.
