---
id: nuif:research:unity-prefabs-and-yaml-merge
kind: implementation
status: reviewed
title: Unity prefab overrides, YAML identity and UnityYAMLMerge
source:
  url: https://docs.unity3d.com/6000.6/Documentation/Manual/yaml-prefab-serialization.html
  authors: [Unity Technologies]
  published_at: "Unity 6000.6 manual (retrieved 2026-08-29); UnityYAMLMerge manual pages 6000.4 and current"
  license: Proprietary documentation (Unity Terms of Service); engine source not public
retrieved_at: 2026-08-29
tags: [prefab, override, nested-instance, variant, yaml, semantic-merge, identity, scene-graph, version-control]
confidence: 0.86
claims: [nuif:claim:authored-resolved, nuif:claim:sync-not-regenerate, nuif:claim:semantic-automation]
relations:
  - type: compares_to
    target: nuif:research:openusd
    note: Prefab variants and nested prefab overrides are a restricted, single-arc form of non-destructive composition.
  - type: related_to
    target: nuif:research:structured-merge
    note: UnityYAMLMerge is a domain-specific structured three-way merge keyed by object identity rather than line position.
  - type: related_to
    target: nuif:research:godot-tscn-scene-format
    note: Both engines use a per-file local object identity plus a per-asset global identity; Godot separates the two more cleanly.
  - type: related_to
    target: nuif:research:unreal-asset-versioning-and-automation
    note: Unreal keeps binary packages and a class-specific diff tool; Unity chooses text serialization plus a generic structural merge.
  - type: supports
    target: nuif:research:content-addressed-versioning
    note: Unity demonstrates why editable-entity identity and asset identity must be distinct namespaces.
links:
  spec: [spec/02-identity-and-properties.md, spec/03-components-and-composition.md, spec/06-operations-and-patches.md, spec/08-serialization.md]
  adr: [adrs/0004-serialization.md]
  rfc: [rfcs/0001-multi-level-document-model.md]
  code: [crates/nuif-protocol, crates/nuif-codec]
  experiments: []
---

# Summary

Unity serializes scenes and prefabs as a sequence of YAML documents, one per engine object, each addressed by a signed 64-bit local identifier (`fileID`) and a class identifier tag. Cross-file references combine the target file's `.meta` GUID with a `fileID`. Prefab instances are not expanded in the containing file; a `PrefabInstance` document stores a source reference and an override set (`m_Modifications`, added/removed components and GameObjects), and placeholder documents marked `stripped` stand in for referenced nested objects. Prefab variants reuse the same mechanism with a parentless root instance. `UnityYAMLMerge` performs a three-way merge of these files by treating specified arrays as identity-keyed sets, excluding volatile paths, and comparing floats with tolerances; it falls back to a user-specified textual tool for unresolved conflicts. Text serialization must be enabled for any of this to apply. Known failure classes are order instability of override lists, local-only identity that differs between prefab assets, and a YAML dialect that is not meant to be externally produced.

## Evidence

- Unity writes each object of a scene as a separate YAML document introduced by `---`; the tag `!u!<n>` encodes the class ID and `&<n>` the object's file-local ID. Unity Manual, "Format description" (`Manual/FormatDescription.html`), retrieved 2026-08-29.
- Header lines are `%YAML 1.1` and `%TAG !u! tag:unity3d.com,2011:`; references to other objects are `{fileID: n}`; asset references are `{fileID: n, guid: <32 hex>, type: t}`; the scene ends with a `SceneRoots` document listing ordered root transforms. Unity Manual, "YAML scene example" (`Manual/YAMLSceneExample.html`), retrieved 2026-08-29.
- A prefab instance is a document of class ID `1001` and type `PrefabInstance` with `m_SourcePrefab: {fileID: 100100000, guid: ..., type: 3}`; `100100000` is the prefab asset handle created at import. Unity Manual 6000.6, "YAML serialization of prefabs" (`Manual/yaml-prefab-serialization.html`), retrieved 2026-08-29.
- The `m_Modification` block contains `m_TransformParent`, `m_Modifications`, `m_RemovedComponents`, `m_RemovedGameObjects`, `m_AddedGameObjects`, `m_AddedComponents`. Each `m_Modifications` entry has `target`, `propertyPath`, `value`, `objectReference`. Same page.
- Referenced nested objects appear as placeholder documents tagged `stripped`, carrying only `m_CorrespondingSourceObject`, `m_PrefabInstance` and `m_PrefabAsset`. Same page; also Unity blog "Understanding Unity's serialization language, YAML" (N. A. Borromeo), section "Prefab instances, Nested Prefabs, and Variants", retrieved 2026-08-29.
- A variant is identified by `m_Modification.m_TransformParent` equal to `{fileID: 0}` on the root `PrefabInstance`. Same manual page; same blog section.
- `fileID` is local to a file and "can be repeated in different files"; cross-file identity is (GUID from `.meta`, fileID). Unity blog, section on cross-file references, retrieved 2026-08-29.
- Replacing a nested prefab's GUID by hand loses overrides because object `fileID`s in the replacement prefab "will differ" from those referenced by `m_CorrespondingSourceObject`. Unity blog, same section (NUIF reading: identity is asset-scoped, not semantic).
- Local file IDs "are signed 64-bit values and can be negative"; `GlobalObjectId` casts them to `ulong`, so the sign is lost; the docs advise not relying on `targetObjectId` to find an object. Unity Scripting API, `GlobalObjectId`, retrieved 2026-08-29.
- Overrides on a prefab instance are property values, added/removed components, and added/removed child GameObjects; an overridden value "always takes precedence" over the asset value; root instance position and rotation are not explicit overrides. Unity Manual, "Prefab instance overrides" (`Manual/PrefabInstanceOverrides.html`), retrieved 2026-08-29.
- A variant "inherits properties from a base prefab"; overrides take precedence; variants can be based on variants; "Apply all to Prefab Variant parent" pushes overrides one level up. Unity Manual, "Prefab variants" (`Manual/PrefabVariants.html`), retrieved 2026-08-29.
- Nested prefabs "keep their links to their own prefab assets" while forming part of another prefab; adding one from the Hierarchy is itself recorded as an override. Unity Manual, "Nested prefabs" (`Manual/NestedPrefabs.html`), retrieved 2026-08-29.
- Asset Serialization Mode defaults to Force Text; the setting exists "to help with version control merges"; a separate option writes references on one line "which reduces version control noise". Unity Manual, "Editor settings" (`Manual/class-EditorManager.html`), retrieved 2026-08-29.
- `UnityYAMLMerge` is shipped in `Editor/Data/Tools` (Windows) and `Unity.app/Contents/Helpers` (macOS); it can be run from the command line and configured as a merge driver for P4V, Git, Mercurial, SVN, TortoiseGit, UVCS and SourceTree; `mergespecfile.txt` declares fallback tools for unresolved conflicts. Unity Manual, "Smart merge" (`Manual/SmartMerge.html`), retrieved 2026-08-29.
- `mergerules.txt` has four sections. Arrays: `set *.GameObject.m_Component *.fileID`, `set *.Prefab.m_Modification.m_Modifications target.fileID target.guid propertyPath`, `plain *.MeshRenderer.m_Materials`, `plain *.Renderer.m_Materials`; the default for unlisted arrays is a hybrid heuristic match. Exclusions: paths such as `*.SpriteRenderer.m_Color` and `excludeIfContains *.MonoBehaviour.* x y z`; excluded paths modified on both sides become conflicts. Comparisons: relative/absolute epsilons such as `float *.Transform.m_LocalPosition.x 0.0000005` and `float *.Transform.m_LocalRotation.x 0.00005 0.001`. Unity Manual 6000.4, "Smart merge", retrieved 2026-08-29.
- UnityYAML "does not support the full YAML specification"; the manual states that users "cannot externally produce or edit UnityYAML files"; unsupported features include comments, multiple documents in the YAML sense, tags and complex keys. Unity Manual, "UnityYAML" (`Manual/UnityYAML.html`), retrieved 2026-08-29.
- Unity staff (MirceaI) state `m_Modifications` is sorted by `target` then `propertyPath`, but the internal representation of `target` depends on load order, so entries with different GUIDs are "not ... stable in different Editor sessions", producing spurious diffs; reported on 2022.3.20 long-term support (LTS) with references back to 2019. Unity Discussions thread 943063, retrieved 2026-08-29.

## Mechanism

Data model. A file is an ordered list of documents `(classID, fileID, body)`. `fileID` is an `int64` unique within the file. A reference is either intra-file `{fileID}` or inter-file `{fileID, guid, type}` where `guid` is the 128-bit identifier stored in the referenced asset's `.meta` file and `type` distinguishes built-in, importer-generated and native assets. A `GameObject` owns an ordered `m_Component` array of references; a `Transform` owns `m_Father` and an ordered `m_Children` array; containment is therefore encoded twice (parent pointer and child list) and both must agree.

Prefab instances. Instead of copying the source hierarchy, the file stores one `PrefabInstance` document. Its override set is a list of `(target, propertyPath, value, objectReference)` tuples where `target` is an inter-file reference to an object inside the source prefab, and `propertyPath` is a dotted path into that object's serialized property tree (`m_LocalPosition.x`, `m_Name`, array indices). Structural overrides are separate lists: removed components, removed GameObjects, added GameObjects, added components. Any object in the instance that must be referenced from the containing file (as a parent transform or as a reference target) is materialized as a `stripped` placeholder document with its own `fileID`; the placeholder records `m_CorrespondingSourceObject` (identity in the source asset) and `m_PrefabInstance` (the owning instance). Resolution reconstructs the full object graph by instantiating the source prefab, applying `m_Modifications` in order, applying structural add/remove lists and binding placeholders to the instantiated objects. A prefab variant is a prefab file whose root is a `PrefabInstance` with `m_TransformParent = {fileID: 0}`; nesting and variants are therefore the same mechanism composed recursively (variant of variant, instance inside variant).

Authored versus resolved. The saved file contains only authored opinions: the source reference and the sparse override set. The Editor materializes the resolved GameObject graph in memory; `Apply` moves overrides down into the asset and `Revert` deletes them. Because the file omits the resolved graph, any change in the source asset is reflected on load. Root position/rotation are treated as always-instance-local and are not counted as overrides.

Merge. `UnityYAMLMerge` parses base, ours and theirs into document trees keyed by `(classID, fileID)`. Within a document it merges mappings key-wise. Arrays declared `set` are matched by the listed key paths (for `m_Component`, by `fileID`; for `m_Modifications`, by `(target.fileID, target.guid, propertyPath)`), so insertions and removals on both sides merge without positional conflicts. Arrays declared `plain` merge positionally. Unlisted arrays use a heuristic hybrid. Excluded paths are never auto-merged; if both sides changed them the result is a conflict. Float comparison uses per-path epsilons so that re-serialization noise is not reported as change. Unresolved conflicts are delegated according to `mergespecfile.txt`, typically to an interactive textual tool over the partially merged file. The tool is a structural three-way merge over identity-keyed trees rather than a semantic merge: it does not know that `m_Father` and `m_Children` must agree, nor that an override's `target` must exist in the referenced prefab.

Failure classes (source-documented unless marked interpretation).
1. Identity scope: `fileID` is file-local; identity of the "same" object in two prefab assets is unrelated, so replacing a source prefab invalidates `m_CorrespondingSourceObject` targets and overrides are lost (Unity blog).
2. Sign loss: negative `fileID`s are reinterpreted in `GlobalObjectId` (Scripting API).
3. Ordering instability: `m_Modifications` order depends on an internal `target` representation that varies with load order, producing spurious diffs and merge noise (Discussions 943063).
4. Dual encoding of containment: parent pointer and child arrays can be merged independently and disagree (interpretation from the format description; the `set` rule keyed on `fileID` for `m_Component` does not cover `m_Children`).
5. Dialect closure: the YAML subset is declared not externally producible, so third-party tooling has no conformance target (UnityYAML manual page).
6. Prerequisite: none of this works unless Force Text is enabled and the merge driver is installed per version control system (Editor settings; "Smart merge" manual page).

## NUIF relevance

**Borrow**
- Sparse override sets addressed by `(target identity, property path)` with separate structural add/remove lists; this is the minimal information needed to keep an instance non-destructive and matches NUIF's `apply instance override` operation.
- Identity-keyed set merge for child and override lists, with explicit exclusion and tolerance rules declared in a rules file; NUIF's three-way merge can express the same as typed merge policies per relation kind.
- Force-text plus one-line references as a canonicalization concern: NUIF's `nuif-text-0` profile should define a deterministic serialization precisely so that merge tools see only semantic change.

**Adapt**
- Replace file-local `int64` identity with NUIF's stable semantic entity IDs so that the same entity is addressable across component definitions, variants and documents; Unity's failure class 1 disappears when override targets are semantic IDs rather than (asset GUID, local ID) pairs.
- Encode containment once (ordered relation or fractional index) and derive parent pointers, so that merge cannot produce disagreeing parent/child encodings.
- Make override ordering canonical by a total order over `(target id, property key)` defined in the spec, eliminating failure class 3.
- Represent placeholder ("stripped") objects as explicit correspondence records in the provenance layer instead of pseudo-entities in the containment tree.

**Reject**
- A merge tool that is separate from the document model and driven by path-pattern rules; NUIF merge must be defined over typed operations and relations (spec/06) so that structural invariants (acyclic containment, target existence) are checked during merge.
- Treating root transform properties as implicitly instance-local; NUIF should make every instance-level deviation an explicit override with fidelity accounting.
- A closed serialization dialect that third parties may not produce; NUIF serialization profiles are normative and externally implementable.

## Open questions

- How `UnityYAMLMerge` matches objects when the same `fileID` is created independently on both branches (collision on newly added objects); no primary source retrieved describes the generation algorithm for local IDs.
- Whether the hybrid heuristic for unlisted arrays is stable across Unity versions; the manual does not specify it.
- Whether structural overrides (`m_RemovedGameObjects`, introduced later than property overrides) interact correctly with `set` matching in `mergerules.txt`, whose default only lists `m_Modifications`.
