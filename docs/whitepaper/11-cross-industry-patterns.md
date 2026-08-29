---
id: nuif:whitepaper:cross-industry-patterns
kind: whitepaper
status: draft
version: 0.0.1
updated: 2026-08-29
---

# Cross-industry patterns: evidence, adoption and rejection

This document synthesizes 52 research records added on 2026-08-29 from visual-effects interchange, game-engine asset systems, programming-language research on bidirectional transformation and layout verification, distributed-systems testing, and 2D-rendering conformance practice. Each pattern below is classified as borrowed (adopted as is), adapted (adopted with a stated change) or rejected (ruled out with the reason). Record identifiers (`nuif:research:*`) carry the locators; this document does not repeat them.

## Method

Every record was written from primary sources (specifications, source code at a named commit, papers with DOI) and states what was verified and what remains unverified. Claims in this document that rest on a single record inherit that record's confidence. Where two sources conflict, both are recorded and the conflict is listed as an open question rather than resolved by preference.

## Patterns

### Document model and composition

| Pattern | Source | Decision | NUIF artifact |
|---|---|---|---|
| Opinion strength ordering over composition arcs (LIVERPS) | `openusd-composition-and-crate` | Adapt: NUIF needs a total, documented resolution order for library, theme, variant and instance-override opinions; six arc kinds are more than a UI document needs | `spec/03-components-and-composition.md`, question `composition-strength` |
| Flatten as an explicit, named lowering that discards composition | `openusd-composition-and-crate`, `alembic` | Borrow: flattening is a lowering with a fidelity record, never the save format | `spec/00-conformance.md`, `docs/whitepaper/01-architecture.md` |
| Authored network versus cooked output with pull-based, memoized evaluation and push-based dirtying | `houdini-pdg-and-hda`, `hydra-render-delegate` | Borrow for the evaluator: resolved snapshots are pull-evaluated per context and invalidated by hierarchical locator sets rather than global dirty bits | `crates/nuif-layout`, ADR 0002 |
| Prefab override as a sparse modification set against a source definition | `unity-prefabs-and-yaml-merge` | Borrow: instance overrides are sparse property sets keyed by stable identity and property path | `spec/03-components-and-composition.md` |
| Resolved-only interchange (baked samples) | `alembic` | Reject as a canonical form; accept as an explicit cache profile | `spec/08-serialization.md` |

### Identity and ordering

| Pattern | Source | Decision | NUIF artifact |
|---|---|---|---|
| File-local numeric identity with global identity through a second key | `unity-prefabs-and-yaml-merge`, `godot-tscn-scene-format` | Reject: file-local identities orphan cross-file references on replacement; NUIF identities are global from creation | `spec/02-identity-and-properties.md` |
| Path-based addressing in patches | `json-patch-rfc6902-and-merge-patch` | Reject for entities; retain for property paths inside an identity-addressed operation | `spec/06-operations-and-patches.md` |
| Parent link and fractional position as one atomic property | `figma-multiplayer-and-rendering-engineering` | Adapt: parent and order identifier move together; ordering uses a list identifier, not an integer index | `crates/nuif-protocol` (`Move { new_index }` is non-commutative) |
| Tree move with undo/redo of concurrent operations and cycle rejection, mechanized proof | `crdt-tree-move-operation` | Adapt for the collaboration profile; the canonical document keeps a totally ordered log and needs no replica metadata | `spec/10-collaboration-profile.md` |
| Random resource identifiers with path fallback and warnings | `godot-tscn-scene-format` | Borrow the fallback discipline for asset references; a resolved-by-path reference must be diagnosed | `spec/09-provenance-and-fidelity.md` |

### Unknown data preservation and schema evolution

| Pattern | Source | Decision | NUIF artifact |
|---|---|---|---|
| `UnknownSchema` keeps name, version and raw payload and re-emits it verbatim; round trip asserted by a test | `opentimelineio` | Borrow verbatim: this is the executable form of `nuif:claim:opaque-preservation` | `rfcs/0002-extension-preservation.md`, experiment `unknown-extension-roundtrip` |
| Unknown node class preserved as a placeholder that records its original class and properties, written back on save | `godot-tscn-scene-format` | Borrow: entities of unknown kind are preserved, not dropped, and their original kind is restored on export | `spec/07-extensions-and-dialects.md` |
| Unknown data ignored and not re-saved | `blender-dna-rna-and-headless` | Reject: this is the failure mode NUIF exists to prevent | risk register |
| Per-schema version numbers with gap-tolerant upgrade functions and a generated version manifest | `opentimelineio`, `unreal-asset-versioning-and-automation` | Adapt: each core record kind carries a version; migrations are pure functions registered per kind; reading a newer version than known is an error, not silent loss | `spec/08-serialization.md`, `migrate` command |
| Self-describing struct layout embedded in the file | `blender-dna-rna-and-headless` | Reject: NUIF encodings are schema-versioned, not struct-layout-described; the deterministic CBOR profile already carries structure | ADR 0004 |
| Extension prefix registry with a status ladder that requires validator support before release | `gltf-validator-and-sample-assets` | Borrow: EXT namespaces are promoted only with a conformance fixture and validator rule | question `extension-governance` |

### Operations, undo and merge

| Pattern | Source | Decision | NUIF artifact |
|---|---|---|---|
| Every editor action is an operator with typed parameters, invocable from scripts | `blender-dna-rna-and-headless` | Borrow: every editor gesture lowers to a protocol operation that a script can invoke | RFC 0004 |
| Memento-based transaction snapshots | `unreal-asset-versioning-and-automation`, `blender-dna-rna-and-headless` | Reject for the canonical log; inverse operations are recorded instead, because snapshots do not commute and cannot be merged | `spec/06-operations-and-patches.md` |
| Undo restores expected user state under concurrent edits; undo rewrites redo history | `figma-multiplayer-and-rendering-engineering`, `command-pattern-undo-and-event-sourcing` | Adapt: the invariant "undo, copy, redo leaves the document unchanged" becomes a metamorphic relation in the operations suite | `conformance/HARNESS.md` |
| Structural three-way merge keyed by class and identity with declared set-valued fields and float epsilons | `unity-prefabs-and-yaml-merge` | Borrow: a merge-rules declaration per property kind (ordered list, identity set, scalar with tolerance) | `spec/06-operations-and-patches.md` |
| Tree matching becomes the identity map when stable identifiers exist; the residual problem is move and order conflicts | `ast-diff-gumtree-and-structural-merge` | Borrow: no heuristic matching in NUIF-native merges; GumTree-style matching is reserved for adapters without identity | `docs/whitepaper/03-protocol-and-portability.md` |
| Conflicts as first-class states rather than failures | `patch-theory-darcs-pijul` | Borrow: typed conflict objects are document state until resolved | `spec/06-operations-and-patches.md` |
| Operational transformation with server serialization | `operational-transformation-vs-crdt` | Reject as a canonical model; permissible as a collaboration profile | ADR 0005 |

### Canonical encoding

| Pattern | Source | Decision | NUIF artifact |
|---|---|---|---|
| RFC 8949 §4.2 core deterministic encoding rules | `canonicalization-rfc8785-and-cbor-deterministic` | Borrow | `nuif-cbor-0` |
| Float handling: CDE keeps `-0.0` and float `1.0`; dCBOR reduces integral floats to integers and all zeros to `0x00` | same | Decide explicitly: NUIF must pick one; the records show the drafts conflict | question `cbor-float-zero` |
| RFC 8785 number serialization via shortest round-trip and `-0` to `0` | same | Adapt for `nuif-text-0` | `spec/08-serialization.md` |
| Content-addressed deduplication of array values | `openusd-composition-and-crate`, `alembic` | Borrow for the package asset store; reject for editable entities | ADR 0004 |

### Layout

| Pattern | Source | Decision | NUIF artifact |
|---|---|---|---|
| Fixture tests generated from browser layout through WebDriver, compared at `< 0.1` px, with structural (not numeric) handling of known divergences | `taffy-and-yoga-browser-generated-tests`, `differential-testing` | Borrow: the layout differential suite is generated, never edited by hand; divergences are classified per case | experiment `layout-differential` |
| Layout as SMT-solvable constraints with a visual assertion logic | `cassius-web-layout-verification` | Adapt: the assertion vocabulary (no overlap, containment, alignment, text fits) becomes a fixture-level oracle; the SMT encoding itself is out of scope because no formalization covers flex or grid | `conformance/HARNESS.md` |
| Relational constraint synthesis from multi-device examples | `inferui-and-layout-synthesis` | Adapt for import inference only; results are marked inferred with confidence | experiment `layout-inference` |
| Flexbox §9.9.1.2 placeholder and grid intrinsic-sizing divergences | `css-flexbox-grid-algorithm-specs` | Record: NUIF conformance cannot claim exact agreement where the CSS specification is implementation-defined; such cases are tolerance-tiered | `spec/04-layout.md` |

### Rendering determinism

| Pattern | Source | Decision | NUIF artifact |
|---|---|---|---|
| Renderer as a pluggable backend behind a stable scene abstraction | `hydra-render-delegate` | Borrow (already ADR 0003) | `crates/nuif-render` |
| GPU shaders as ground truth compared by a perceptual mean | `vello-testing-and-cpu-reference` | Reject as the conformance oracle; borrow the threshold values for the interactive backend | `conformance/HARNESS.md` |
| CPU `f32` pipeline with tolerance 0 as the reference | `vello-testing-and-cpu-reference`, `resvg-test-suite` | Borrow: the reference rasterization is CPU, pixel-exact, pinned fonts | ADR 0003 |
| Per-test numeric and perceptual thresholds (`idiff`, `oiiotool`, WPT `fuzzy`, WebRender `fuzzy(max,count)`) | `hydra-render-delegate`, `blender-dna-rna-and-headless`, `skia-gold-and-gm-tests`, `webrender-reftests` | Adapt into three declared tiers: exact, bounded per-channel delta with pixel count, perceptual (ꟻLIP mean) | `conformance/HARNESS.md` |
| Reftests (two documents that must render identically) over pixel baselines | `skia-gold-and-gm-tests` | Borrow for equivalence-preserving rewrites | metamorphic relation class 1 |
| Scene capture to a text serialization for deterministic replay | `webrender-reftests` | Borrow: render scenes are serializable fixtures | `crates/nuif-render` |
| WGSL and WebGPU leave rounding, reassociation, sample locations and edge inclusion implementation-defined | `gpu-rendering-nondeterminism` | Record: GPU output is never normative | `spec/05-geometry-paint-text.md` |

### Text

| Pattern | Source | Decision | NUIF artifact |
|---|---|---|---|
| Shaping fixtures as glyph strings with font hash, options and expected `glyph=cluster@dx,dy+adv` output | `text-rendering-reproducibility` | Borrow the format for the text suite | experiment `text-pinning` |
| Hinting off, grayscale coverage, declared subpixel quantum, font SHA-256, Unicode and shaper versions pinned | same | Borrow | `spec/05-geometry-paint-text.md` |

### Testing methodology

| Pattern | Source | Decision | NUIF artifact |
|---|---|---|---|
| Deterministic simulation: single-threaded scheduler, seeded PRNG, all nondeterminism behind injectable interfaces, reproduction by seed | `deterministic-simulation-testing` | Borrow: the trial loop is seed-driven and prints the seed on failure | `conformance/HARNESS.md` |
| Swarm testing (random feature subsets per run) | same | Borrow for operation generators | same |
| Metamorphic relations with tolerant equality; reduction by reversing recorded transformations | `metamorphic-testing-graphics` | Borrow: nine relation classes are defined in the record | same |
| ddmin over operation sequences, hierarchical reduction over the document, choice-sequence shrinking over generated values | `delta-debugging-and-test-case-reduction` | Borrow: three-level reducer | QA contract item 9 |
| Model-based testing with a small reference model and precondition-preserving shrinking | `property-based-testing-state-machines` | Borrow: `proptest-state-machine` over an ordered-forest model | same |
| Structure-aware fuzzing with explicit depth and allocation budgets | `fuzzing-structured-inputs` | Borrow: `arbitrary` does not bound value depth; NUIF bounds depth and node count explicitly | `spec/11-security.md` |
| Snapshot testing with redactions, sorted output and a single update variable | `golden-master-and-snapshot-testing`, `libtest-mimic-and-data-driven-fixtures` | Borrow: `NUIF_UPDATE_EXPECT` is the only regeneration switch | `conformance/HARNESS.md` |
| Machine-readable validation report with severity codes, pointers and per-code policy | `gltf-validator-and-sample-assets` | Borrow as the report schema for `validate`, `import`, `export` | `spec/12-cli-api-and-automation.md` |
| Sample-asset corpus with per-asset metadata, tags and CI validation | same | Borrow for `conformance/fixtures` | `conformance/HARNESS.md` |

### Editor automation

| Pattern | Source | Decision | NUIF artifact |
|---|---|---|---|
| Headless execution with a script (`--background --python`, `hython`, commandlets, `-nullrhi`) | `blender-dna-rna-and-headless`, `houdini-pdg-and-hda`, `unreal-asset-versioning-and-automation` | Borrow: the editor binary runs a session script without a window | `apps/editor/UI-SPEC.md` |
| Plugin API as a programmable surface, but no headless mode and read-mostly REST | `figma-plugin-and-rest-api-as-automation-surface` | Record as the gap NUIF closes; borrow `pluginData`-style opaque per-entity stores as adapter evidence | `adapters/README.md` |
| Accessibility tree as the semantic query and action surface for UI tests | `accesskit-semantic-ui-testing`, `egui-and-egui-kittest`, `masonry-xilem-and-linebender-test-harness` | Borrow: entity identifiers are carried in the accessibility tree; tests query by role and label and dispatch actions without pointer synthesis | ADR 0006 |
| Same-frame scene and accessibility outputs with virtual time and CPU rasterization | `masonry-xilem-and-linebender-test-harness` | Borrow | ADR 0006 |
| Pixel-based UI screenshot tests with per-OS thresholds | `egui-and-egui-kittest` | Reject as the primary editor oracle; permitted only for shell wiring | `apps/editor/QA.md` |

## Ruled out

The following were examined and excluded from the architecture; the reason is recorded so the question is not reopened without new evidence.

- A self-describing binary struct layout (Blender DNA): solves version drift for one implementation but does not preserve data it cannot re-save and does not compose with a schema-versioned interchange model.
- Memento undo as the canonical history: does not commute, cannot be merged, and bloats logs; inverse operations are required by `spec/06`.
- Integer child indices in `Move` and `Insert` as the only order representation: non-commutative under concurrency; a list identifier is required for the collaboration profile and harmless for the canonical form.
- GPU rendering as a normative oracle: implementation-defined by the WebGPU and WGSL specifications.
- Perceptual UI screenshot tests as the primary editor test: platform-dependent text rendering makes them a shell-wiring check only.
- Heuristic tree matching for NUIF-native merges: unnecessary with stable identity and a source of spurious moves.
- Whole-project regeneration as the synchronization model: contradicted by the lens and delta-lens laws that NUIF's patch model must satisfy (`lenses-foster-boomerang`, `bidirectional-evaluation-direct-manipulation`).

## Consequences for the specification

The records imply the following changes. Items 1–3 were decided by follow-up research on 2026-08-29 and are recorded as accepted RFCs; items 4–6 remain proposals.

1. Sibling order is a canonical array without keys; `Insert` and `Move` use anchors (`Start`, `After(id)`); the collaboration profile maps anchors onto a Fugue-family list CRDT (RFC 0006).
2. `nuif-cbor-0` follows draft-ietf-cbor-serialization preferred serialization with narrowing rules stated by value (integral reals and both zeros as integers, finite reals only, strict decoders); `nuif-text-0` hashes through CBOR; string values are verbatim (RFC 0005).
3. Entities of unknown kind load as `Unknown` with typed core fields and an opaque payload; ignorant implementations preserve bytes, knowing ones may re-encode; validation severities follow the glTF pattern (RFC 0007).
4. Every serialized record kind carries a schema version; migrations are registered pure functions; newer-than-known versions load as `Unknown` for entities (RFC 0007) and are diagnosed for other records.
5. Validation, import and export reports follow one schema with stable codes, severities and pointers.
6. Layout conformance declares tolerance tiers per case and classifies every divergence from a browser reference as schema loss, evaluator defect or implementation-defined behavior.

## Open questions raised by this synthesis

Recorded in `research/questions.yaml`: `cbor-float-zero` (decided, RFC 0005), `sibling-order-identifier` (decided, RFC 0006), `unknown-kind-preservation` (decided, RFC 0007), `editor-toolchain-msrv` (ADR 0006), `layout-assertion-vocabulary`, `render-tolerance-tiers`.
