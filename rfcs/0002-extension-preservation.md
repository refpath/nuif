# RFC 0002 — Opaque extension preservation

Status: proposed

Unknown extension data remains attached to its owning entity/property and MUST survive load/save/edit cycles unless the owner is deleted or the user explicitly removes it.

Documents declare used and required extensions. Unsupported required extensions block claims of faithful rendering but do not necessarily block structural editing.

This goes beyond codec unknown fields: preservation is a document-model requirement.
