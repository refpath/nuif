# Terminology

Read the section relevant to the changed prose. The linked specification,
profile or implementation owns the definition and its current scope. Foreign
terminology is appropriate when describing that system; identify the mapping
rather than treating its vocabulary as NUIF semantics.

## Document model and resources

Definitions: [model](../../../../spec/01-model.md),
[identity](../../../../spec/02-identity-and-properties.md),
[assets and resources](../../../../spec/05-geometry-paint-text.md) and
[core types](../../../../crates/nuif-core/src/lib.rs).

| Term | Use |
|------|-----|
| containment tree | Ordered ownership and lifetime relationships; distinguish them from typed non-ownership relations. |
| entity, stable identity, `EntityId` | Durable authored objects whose identities are independent of names, positions and vendor identifiers. |
| authored value, authored intent | Editable input to evaluation. |
| resolved value, resolved snapshot | Derived results for a named evaluation context. |
| asset, `AssetId` | A semantic asset with stable identity. |
| resource, `ResourceDigest` | Immutable bytes identified by a digest; paths and locators are resolution hints. |
| extension, namespace, opaque preservation | Unknown data retained without a claim that its meaning is interpreted or rendered. |
| design token | An authored token; use a vendor's term such as variable when describing its own data model. |

## Layout, rendering and text

Definitions: [layout](../../../../spec/04-layout.md),
[geometry, paint and text](../../../../spec/05-geometry-paint-text.md),
[layout implementation](../../../../crates/nuif-layout/src/lib.rs) and
[render profile](../../../../conformance/render/README.md).

| Term | Use |
|------|-----|
| layout family | `freeform`, `stack`, `flex`, `grid` or `constraint` in the core; specification proposals and foreign systems may have other families. |
| intrinsic size, min-content, max-content, fit-content | Name the actual sizing behavior or authored intent. |
| evaluation context | Explicit viewport, scale, font and other inputs used by the named evaluator; check its supported fields. |
| resolved box, transform | Evaluated geometry rather than authored layout intent. |
| render scene, `RenderScene`, renderer backend | The lowered rendering representation and its consumer. |
| CPU reference renderer | The NUIF conformance path; distinguish it from Masonry's CPU rasterization of editor chrome. |
| shaping, glyph run, cluster, advance | Text-shaping results; distinguish shaping from layout and rasterization. |
| font substitution, font fallback | Identify the selected replacement and its fidelity rather than implying original-font equivalence. |

## Operations, correspondence and fidelity

Definitions: [operations](../../../../spec/06-operations-and-patches.md),
[fidelity](../../../../spec/09-provenance-and-fidelity.md),
[protocol implementation](../../../../crates/nuif-protocol/src/lib.rs) and
[adapter contracts](../../../../crates/nuif-adapter/src/lib.rs).

| Term | Use |
|------|-----|
| semantic operation, transaction, patch | Document mutations and their atomic or ordered grouping. A command can be an input to the editor or CLI. |
| inverse operation, replay | Undo semantics and application of recorded mutations. |
| canonical form, canonical hash | A profile-defined representation or digest; normalization can describe a specific preprocessing step. |
| three-way merge, structural merge, conflict | Name the implemented algorithm and the conflict it surfaces. |
| correspondence record | A mapping between document identity/property and a foreign construct. |
| lowering, lifting, reconciliation | Direction and mechanism of a representation change; use flattening when structure is discarded. |
| fidelity class | `lossless`, `representable`, `approximated`, `preserved_unrenderable` or `unsupported`, for a stated mapping and profile. |
| evidence class, confidence | Origin of available evidence and predicted correctness; neither establishes fidelity by itself. |

## Testing and editor

Definitions: [conformance plan](../../../../conformance/PLAN.md),
[test harness](../../../../conformance/HARNESS.md),
[editor architecture](../../../../apps/editor/ARCHITECTURE.md),
[editor QA](../../../../apps/editor/QA.md) and
[editor UI scope](../../../../apps/editor/UI-SPEC.md).

| Term | Use |
|------|-----|
| fixture, expected output, reference image | State which observable behavior is compared and where its expectation comes from. |
| test oracle, differential testing | The deciding reference and comparisons between named implementations. |
| metamorphic relation, property-based testing, shrinking | Relations across executions, generated input strategies and failure reduction. |
| coverage-guided fuzzing, delta debugging | Guided input exploration and reduction of a failing case; distinguish them from ordinary generated testing. |
| deterministic replay, round trip, idempotence | Name the tested relation; these are different assertions. |
| conformance profile, capability profile | Bounded requirements and declared support; a declaration alone is not passing evidence. |
| headless API, in-process session driver | Semantic automation surfaces shared with the editor; check the implemented command and action types. |
| reference editor, test editor | The research instrument governed by the editor UI scope. |
| canvas, viewport, layers panel, properties panel, accessibility tree | Editor regions and semantic controls; keep shell state distinct from document state. |
