# 07 — Extensions and dialects

Status: draft.

NUIF uses a small core and namespaced extensions.

Documents declare `extensions_used` and `extensions_required`, and MAY declare a `fallback_kind` per namespace. A required extension means correct interpretation/rendering cannot be guaranteed without it.

Extension lifecycle namespaces:

- `NUIF_*` — ratified standard extension;
- `EXT_*` — multi-implementation experimental extension;
- vendor/project namespace — owner-specific extension.

## Preservation (RFC 0002, RFC 0007)

Unknown extension payloads MUST be preserved at their attachment point unless an operation explicitly removes the owning entity/property. An implementation that does not declare a namespace preserves its payloads byte-for-byte; one that declares it MAY re-encode deterministically. Payloads are opaque byte strings with a declared encoding (`Cbor` or `Octets`); a malformed payload yields a diagnostic on its owner and does not invalidate the document.

An entity of unknown kind, or of a known kind with a newer schema version than supported, loads as `Unknown` with namespace, kind name, schema version and payload retained; its core fields stay typed and editable; layout uses the declared `fallback_kind` or `Container`; rendering reports `preserved_unrenderable`.

## Validation

- namespace present but not in `extensions_used`: error;
- namespace in `extensions_used` and unsupported: information;
- namespace in `extensions_required` and unsupported: blocks faithful-rendering claims, never structural editing.

Promotion of an `EXT_*` namespace to `NUIF_*` requires a conformance fixture and a validator rule (pattern: glTF extension status ladder).

Dialects may define higher-level authored constructs and lowering rules. A dialect cannot redefine core semantics.
