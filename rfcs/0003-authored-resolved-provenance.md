# RFC 0003 — Authored state, resolved state and provenance

Status: proposed

Store authored values as canonical semantics; resolved values are context-keyed derived records. Provenance/correspondence records map foreign constructs to NUIF constructs for retentive synchronization.

The same entity may have multiple resolved records for different viewport/theme/font contexts.

Source adapters should patch original syntax trees using correspondence and structured edits instead of regenerating whole files.
