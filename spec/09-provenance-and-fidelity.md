# 09 — Provenance, correspondence and fidelity

Status: draft.

`ProvenanceRecord` identifies an origin system, source artifact revision and optional source path/range/node/property identity.

`CorrespondenceRecord` maps NUIF identities/properties to one or more foreign identities/properties and can retain adapter-specific reconstruction hints.

Correspondence is optional canonical-adjacent metadata: it may be stored in a package/profile without changing the semantic document.

Every import/export/lowering returns a `FidelityReport` with item-level status: lossless, representable, approximated, preserved_unrenderable or unsupported. Diagnostics identify the entity/property and transformation pass responsible.

This mechanism is informed by symmetric and retentive lens research.
