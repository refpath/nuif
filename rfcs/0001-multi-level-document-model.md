# RFC 0001 — Multi-level document model

Status: proposed

## Decision
Adopt a containment tree with stable identity plus coordinated relationship graphs and explicit authored→resolved evaluation layers.

## Why
A single shape tree conflates semantics, layout, rendering and relationships. MLIR demonstrates multiple abstraction levels; OpenUSD demonstrates graph-like composition over a scene namespace.

## Rejected
- pure AST: too source-language-specific;
- pure scene graph: loses authored semantics;
- pure ECS: efficient implementation technique but weak interchange semantics;
- event log as canonical state: burdens simple offline documents;
- relational tables as interchange syntax: poor human authoring and containment ergonomics.
