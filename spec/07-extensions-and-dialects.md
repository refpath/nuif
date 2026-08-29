# 07 — Extensions and dialects

Status: draft.

NUIF uses a small core and namespaced extensions.

Documents declare `extensions_used` and `extensions_required`. A required extension means correct interpretation/rendering cannot be guaranteed without it.

Extension lifecycle namespaces:

- `NUIF_*` — ratified standard extension;
- `EXT_*` — multi-implementation experimental extension;
- vendor/project namespace — owner-specific extension.

Unknown extensions MUST be preserved byte/value-for-byte at their attachment point unless an operation explicitly removes the owning entity/property. An implementation unable to interpret an extension reports `preserved_unrenderable`.

Dialects may define higher-level authored constructs and lowering rules. A dialect cannot redefine core semantics.
