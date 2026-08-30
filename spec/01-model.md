---
id: nuif:spec:model
kind: specification
status: draft
---

# Logical model

The working NUIF model is a layered hybrid rather than a universal AST.

## Identity

Every durable authored entity has a stable `EntityId`. Identity is independent from display names, containment position, serialization offsets, and vendor IDs. External correspondences are recorded separately as provenance.

## Containment

Documents contain ordered authored entities. Containment expresses lifetime/ownership and author-facing hierarchy only.

## Coordinated relations

Relationships that are not ownership belong in typed relation sets/graphs: component-instance links, token references, constraints, interactions, state transitions, dependencies, provenance, and extension-defined relations.

## Authored and resolved data

Authored data expresses intent. Resolved data is derived for a named evaluation context such as viewport, scale, font environment, capability profile, and theme. Resolved data is cacheable/reproducible output and MUST NOT overwrite authored intent.

## Extensions

The core admits namespaced extension values. An implementation may understand, preserve without understanding, approximate, or reject an extension according to declared capability and security rules.
