# ADR 0002 — Use Taffy as initial CSS-family layout evaluator

Status: accepted for prototype

Taffy provides Rust implementations of Block, Flexbox and Grid and is already embedded by multiple UI systems. NUIF will wrap it behind its own evaluator interface. Taffy types MUST NOT become canonical schema types.

Constraint/freeform/proposal-response semantics remain separate evaluators/lowerings.
