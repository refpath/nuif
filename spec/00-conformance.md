---
id: nuif:spec:conformance
status: draft
---

# Conformance and fidelity

A conforming implementation MUST NOT silently discard semantically relevant information. Import, lowering, migration, and export operations MUST be able to emit structured diagnostics.

Initial fidelity classes are:

- `lossless`
- `representable`
- `approximated`
- `preserved_unrenderable`
- `unsupported`

Capability profiles will identify which optional modules and extensions an implementation can evaluate, render, mutate, and preserve.

Unknown extensions MUST be preserved byte-for-byte or canonical-value-for-canonical-value when the package/encoding permits preservation and when doing so does not violate security policy.
