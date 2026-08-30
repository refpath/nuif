---
id: nuif:adr:0002
kind: adr
status: accepted
---

# ADR 0002 — Use Taffy as initial CSS-family layout evaluator

Status: accepted for prototype

Taffy provides Rust implementations of Block, Flexbox and Grid and is already embedded by multiple UI systems. NUIF will wrap it behind its own evaluator interface. Taffy types MUST NOT become canonical schema types.

Constraint/freeform/proposal-response semantics remain separate evaluators/lowerings.

## Gate C verification

The first executable use is an independent test lowering, not canonical-schema coupling: `nuif-testing` pins Taffy 0.14.0 and compares it with Chrome for Testing 152.0.7977.64 and the profile-0 reference evaluator. This exposed and corrected definite cross sizes being overwritten by `stretch`. The v0 and generated stack/flex subset now agree within measured fixture-local bounds.

Grid remains intentionally unwired in the reference evaluator because the current authored model has no track-sizing or item-placement fields. The differential report classifies those differences as schema loss. A follow-up schema decision is required before a Taffy Grid lowering can be called representable.
