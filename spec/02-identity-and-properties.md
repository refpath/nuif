# 02 — Identity and properties

Status: draft.

## Entity identity

Every authored entity MUST have a stable 128-bit-or-greater identifier. Identity MUST NOT depend on name, path, child index, geometry or serialized byte offset. Implementations MAY use UUIDv7/UUIDv4-compatible identifiers; the normative requirement is uniqueness and stability, not one generation algorithm.

Immutable assets MAY additionally use cryptographic content IDs.

## Property model

A property is addressed by `(entity_id, namespace, key)`. Core properties use the `nuif` namespace. Standard extensions use registered namespaces and vendor extensions use registered vendor identifiers.

Properties distinguish:

- authored value;
- token/expression binding when present;
- resolved value for an evaluation context;
- provenance/correspondence metadata.

Resolved values MUST NOT overwrite authored values.
