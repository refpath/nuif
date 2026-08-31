# Diagnostic code registry

This is the canonical public registry for structured profile-zero diagnostics
emitted through `nuif_core::Diagnostic` and for failure codes retained by the
reference conformance trial. Automation must branch on `code`, not on the
human-readable message. Messages may gain detail without changing a code.

The severity below is the default for the owning profile. A code is never
repurposed; a materially different condition receives a new code. Adding a
code is compatible within an alpha profile, while removing a code or changing
its meaning requires a profile-version decision. `entity`, `pointer` and
`fidelity` remain the attribution fields defined by the diagnostic record.

Transport and command failures such as malformed CLI arguments, unavailable
package capabilities and WASM input limits are error classes rather than
document diagnostics. Their promotion boundary is tracked separately in
[SDK and language-binding boundary](SDK-AND-BINDINGS.md).

`cargo xtask diagnostic-audit` compares this bytewise-sorted table with every
code literal owned by model validation, layout evaluation and the conformance
trial. It writes `target/diagnostic-registry-report.json` and fails when a code
is undocumented, duplicated, stale, malformed or out of order.

| Code | Default severity | Category | Producer | Stable meaning |
| --- | --- | --- | --- | --- |
| `ASSET_RESOURCE_REQUIRED` | error | asset | model validation | A portable or substituted asset lacks an exact resource digest. |
| `ASSET_UNAVAILABLE_HAS_RESOURCE` | error | asset | model validation | An unavailable asset incorrectly binds resource bytes. |
| `COLOR_CHANNEL_OUT_OF_RANGE` | error | paint | model validation | An encoded-sRGB fill channel is outside the inclusive zero-to-one range. |
| `EXTENSION_FALLBACK_NOT_USED` | error | extension | model validation | A fallback is declared for a namespace absent from `extensions_used`. |
| `EXTENSION_NAMESPACE_INVALID` | error | extension | model validation | An extension namespace is not a valid lowercase NUIF identifier. |
| `EXTENSION_REQUIRED_NOT_USED` | error | extension | model validation | A required namespace is absent from `extensions_used`. |
| `EXTENSION_REQUIRED_UNSUPPORTED` | warning | extension | model validation | The implementation preserves but does not interpret a required namespace. |
| `EXTENSION_UNDECLARED` | error | extension | model validation | An attached extension namespace is absent from `extensions_used`. |
| `EXTENSION_UNSUPPORTED` | information | extension | model validation | The implementation preserves but does not interpret a used namespace. |
| `GRID_EXPLICIT_AREA_EXHAUSTED` | error | grid | model validation | Sparse placement cannot fit another child inside the explicit grid. |
| `GRID_PLACEMENT_OUT_OF_BOUNDS` | error | grid | model validation | An explicit grid position or span exceeds the declared tracks. |
| `GRID_PLACEMENT_OVERLAP` | error | grid | model validation | Two children occupy an overlapping explicit grid area. |
| `GRID_PLACEMENT_PARTIAL` | error | grid | model validation | A child declares only one coordinate of a required row or column pair. |
| `GRID_PLACEMENT_WITHOUT_GRID_PARENT` | error | grid | model validation | Grid placement is authored outside a direct grid child. |
| `GRID_SPAN_INVALID` | error | grid | model validation | A grid row or column span is zero. |
| `GRID_STYLE_WITHOUT_GRID_FAMILY` | error | grid | model validation | Grid tracks or flow are authored on a non-grid container. |
| `GRID_TRACKS_REQUIRED` | error | grid | model validation | A grid container lacks an explicit row or column axis. |
| `GRID_TRACK_INVALID` | error | grid | model validation | A fixed track size or fractional weight is not finite and positive. |
| `GRID_TRACK_LIMIT_EXCEEDED` | error | grid | model validation | A grid axis exceeds the profile-zero track limit. |
| `IMAGE_ASSET_INVALID` | error | image | model validation | Image metadata has zero dimensions or an invalid decoder profile. |
| `IMAGE_ASSET_MISSING` | error | image | model validation | Image paint references an absent or non-image asset. |
| `IMAGE_CROP_INVALID` | error | image | model validation | The crop is not a finite positive normalized rectangle inside the source. |
| `IMAGE_PAINT_INVALID` | error | image | model validation | Image transform, opacity or color-conversion identity is invalid. |
| `IMAGE_PAINT_KIND_INVALID` | error | image | model validation | Image paint is authored on an entity outside the image kind. |
| `LAYOUT_CONSTRAINT_FALLBACK` | warning | layout | layout evaluation | Profile zero evaluates a constraint family through its declared freeform fallback. |
| `LAYOUT_FAMILY_PROFILE0_FALLBACK` | warning | layout | layout evaluation | Profile zero evaluates a flex family through its declared stack fallback. |
| `LAYOUT_UNKNOWN_KIND_FALLBACK` | information | layout | layout evaluation | An unknown kind uses its declared container or leaf fallback geometry. |
| `MODEL_ASSET_KEY_MISMATCH` | error | model | model validation | An asset map key differs from the asset's embedded identity. |
| `MODEL_ASSET_VERSION_NOT_OPAQUE` | error | model | model validation | A newer asset schema is represented as a known rather than unknown kind. |
| `MODEL_CHILD_MISSING` | error | model | model validation | A child identity does not exist in the entity map. |
| `MODEL_COMPONENT_MISSING` | error | model | model validation | An instance references an absent or non-component entity. |
| `MODEL_CONTAINMENT_CYCLE` | error | model | model validation | Entity containment contains a cycle. |
| `MODEL_DOCUMENT_VERSION_UNSUPPORTED` | error | model | model validation | The document schema version is newer than the implementation. |
| `MODEL_DUPLICATE_CHILD` | error | model | model validation | One parent lists the same child more than once. |
| `MODEL_DUPLICATE_ROOT` | error | model | model validation | The root list contains the same entity more than once. |
| `MODEL_ENTITY_KEY_MISMATCH` | error | model | model validation | An entity map key differs from the entity's embedded identity. |
| `MODEL_ENTITY_UNREACHABLE` | error | model | model validation | An entity is unreachable from every root. |
| `MODEL_ENTITY_VERSION_NOT_OPAQUE` | error | model | model validation | A newer entity schema is represented as a known rather than unknown kind. |
| `MODEL_IDENTIFIER_INVALID` | error | model | model validation | A semantic identifier violates the lowercase NUIF grammar. |
| `MODEL_MULTIPLE_PARENTS` | error | model | model validation | An entity is listed under more than one parent. |
| `MODEL_NON_FINITE_NUMBER` | error | model | model validation | Authored semantic state contains a non-finite number. |
| `MODEL_RELATION_TARGET_MISSING` | error | model | model validation | A relation endpoint is absent from the entity map. |
| `MODEL_RESOURCE_LIMIT_EXCEEDED` | error | security | model validation | The decoded semantic document exceeds a profile-zero resource limit. |
| `MODEL_RESPONSIVE_RANGE_INVALID` | error | model | model validation | A responsive minimum width exceeds its maximum width. |
| `MODEL_ROOT_HAS_PARENT` | error | model | model validation | A root entity is also listed as a child. |
| `MODEL_ROOT_MISSING` | error | model | model validation | A root identity does not exist in the entity map. |
| `MODEL_TOKEN_KEY_MISMATCH` | error | model | model validation | A token map key differs from the token's embedded identity. |
| `MODEL_TOKEN_MISSING` | error | model | model validation | A property references an absent token. |
| `RESOURCE_DIGEST_INVALID` | error | resource | model validation | An asset resource digest is not canonical SHA-256 identity text. |
| `SNAPSHOT_FAILED` | error | harness | conformance trial | A requested trial snapshot could not be evaluated or rendered. |
| `TEXT_FONT_BINDING_INVALID` | error | text | model validation | Requested, replacement and asset font identities are inconsistent. |
| `TEXT_FONT_HASH_INVALID` | error | text | model validation | A text font hash is not 64 lowercase hexadecimal digits. |
| `TEXT_FONT_NOT_PINNED` | warning | text | layout evaluation | The requested exact font is absent from the evaluation context. |
| `TEXT_FONT_SUBSTITUTED` | warning | text | layout evaluation | Layout uses an explicit replacement font present in the evaluation context. |
| `TEXT_FONT_SUBSTITUTE_NOT_PINNED` | warning | text | layout evaluation | The declared replacement font is absent from the evaluation context. |
| `TEXT_FONT_UNAVAILABLE` | warning | text | layout evaluation | The bound font asset explicitly declares the resource unavailable. |
| `TEXT_METRICS_INVALID` | error | text | model validation | Text size or line height is not finite and positive. |
| `TRIAL_APPLY_FAILED` | error | harness | conformance trial | A generated semantic patch failed atomic application. |
| `TRIAL_CBOR_ENCODE_FAILED` | error | harness | conformance trial | The trial document could not be encoded as deterministic CBOR. |
| `TRIAL_CBOR_FIXPOINT_FAILED` | error | harness | conformance trial | Deterministic CBOR did not reach an exact decode-encode fixpoint. |
| `TRIAL_CHOICE_LIMIT_EXCEEDED` | error | harness | conformance trial | A generated choice stream exceeded its explicit decision budget. |
| `TRIAL_INVERSE_FAILED` | error | harness | conformance trial | The inverse of an applied patch could not be applied. |
| `TRIAL_INVERSE_MISMATCH` | error | harness | conformance trial | Applying a patch and its inverse did not restore the exact base document. |
| `TRIAL_RASTER_NONDETERMINISTIC` | error | harness | conformance trial | Repeated CPU rasterization produced different bytes. |
| `TRIAL_REPLAY_FAILED` | error | harness | conformance trial | Replaying a generated patch from the same base failed. |
| `TRIAL_REPLAY_HASH_MISMATCH` | error | harness | conformance trial | Direct application and replay produced different canonical hashes. |
| `TRIAL_RERENDER_FAILED` | error | harness | conformance trial | Repeated layout or scene construction failed. |
| `TRIAL_SNAPSHOT_FAILED` | error | harness | conformance trial | The trial's failure-reproduction snapshot could not be produced. |
| `TRIAL_TEXT_ENCODE_FAILED` | error | harness | conformance trial | The trial document could not be encoded as canonical text. |
| `TRIAL_TEXT_FIXPOINT_FAILED` | error | harness | conformance trial | Canonical text did not reach an exact decode-encode fixpoint. |
| `UNKNOWN_NAMESPACE_INVALID` | error | extension | model validation | An unknown kind names an invalid extension namespace. |
| `UNKNOWN_NAMESPACE_UNDECLARED` | error | extension | model validation | An unknown kind names a namespace absent from `extensions_used`. |
| `VALIDATION_DIAGNOSTICS_TRUNCATED` | error | security | model validation | The validator reached its retained-diagnostic limit and stopped recording ordinary issues. |
