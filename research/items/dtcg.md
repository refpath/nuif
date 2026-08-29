---
id: nuif:research:dtcg
kind: standard
status: reviewed
title: Design Tokens Format Module 2025.10
source:
  url: https://www.w3.org/community/reports/design-tokens/CG-FINAL-format-20251028/
  authors: [W3C Design Tokens Community Group]
  published_at: 2025-10-28
  license: W3C Community Final Specification Agreement
retrieved_at: 2026-08-29
tags: [tokens, design-systems, interoperability]
confidence: 0.99
claims: []
relations: []
links:
  spec: [spec/03-components-and-composition.md]
  adr: []
  rfc: []
  code: [adapters/STATUS.md]
  experiments: []
---
# Summary

The Design Tokens Format Module 2025.10 defines JSON interchange for typed
design tokens, hierarchical groups, aliases, group extension and
vendor-specific metadata. A token is identified structurally by an object with
`$value`; its name is the containing object key.

## Evidence

- §4 assigns `application/design-tokens+json` and recommends `.tokens` or
  `.tokens.json`. A conforming file remains JSON.
  https://www.w3.org/community/reports/design-tokens/CG-FINAL-format-20251028/#file-format
  (retrieved 2026-08-29).
- §5.1 requires a name and `$value`. Names are case-sensitive, cannot begin
  with `$`, and cannot contain `{`, `}` or `.` because those characters are
  reserved by the reference syntax. §5.2 requires an unambiguous type and
  prohibits type inference from the value.
  https://www.w3.org/community/reports/design-tokens/CG-FINAL-format-20251028/#name-and-value
  (retrieved 2026-08-29).
- §5.2.3 and §6.3.2 require processors to preserve unknown `$extensions` data.
  Reverse-domain keys are recommended to reduce collisions.
  https://www.w3.org/community/reports/design-tokens/CG-FINAL-format-20251028/#extensions-0
  (retrieved 2026-08-29).
- §6 defines hierarchical groups, inherited `$type`, `$root`, `$extends`, empty
  groups and cycle/error handling. §7 defines curly-brace and JSON Pointer
  references, chained resolution and circular-reference rejection.
  https://www.w3.org/community/reports/design-tokens/CG-FINAL-format-20251028/#groups
  and https://www.w3.org/community/reports/design-tokens/CG-FINAL-format-20251028/#aliases-references
  (retrieved 2026-08-29).
- §8 defines primitive and composite types. A conforming processor must validate
  the value against the resolved type rather than preserving an untyped JSON
  value as if it were interoperable.
  https://www.w3.org/community/reports/design-tokens/CG-FINAL-format-20251028/#types
  (retrieved 2026-08-29).

## NUIF relevance

**Borrow** the media type, name grammar, explicit type system, alias resolution,
group model, cycle rejection and unknown-extension preservation rules.

**Adapt** DTCG paths to stable NUIF token identity by storing an `EntityId` in a
reverse-domain `$extensions` entry for generated files. A first bounded profile
can map boolean, string and finite number tokens exactly. It must retain the
declared DTCG type and distinguish an alias from its resolved value.

**Reject** full DTCG conformance with the current `Token { id, name, value }`
model. The model lacks declared type, description, deprecation, group identity,
group extension, alias syntax and token-local opaque extensions. These fields
require a token-model RFC or an adapter-owned retentive package before a full
round trip can be claimed.
