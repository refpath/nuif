---
id: nuif:research:godot-tscn-scene-format
kind: repository
status: reviewed
title: Godot text scene format, resource identity and missing-type preservation
source:
  url: https://docs.godotengine.org/en/latest/engine_details/file_formats/tscn.html
  repository: https://github.com/godotengine/godot
  authors: [Godot Engine contributors]
  published_at: "Godot 4.x documentation (latest) and master source, retrieved 2026-08-29"
  license: MIT (engine source); CC BY 3.0 (documentation)
retrieved_at: 2026-08-29
tags: [scene-format, text-serialization, identity, uid, scene-inheritance, override, unknown-type-preservation, diff-stability, scene-graph]
confidence: 0.9
claims: [nuif:claim:opaque-preservation, nuif:claim:authored-resolved, nuif:claim:sync-not-regenerate]
relations:
  - type: supports
    target: nuif:research:unity-prefabs-and-yaml-merge
    note: Godot reaches the same authored-override structure as Unity prefabs with a text format designed for version control from the start.
  - type: compares_to
    target: nuif:research:openusd
    note: Inherited scenes are a single-arc, single-layer analogue of USD sublayer plus override composition.
  - type: implements
    target: nuif:research:gltf
    note: MissingNode/MissingResource are a concrete implementation of preserving unknown typed data, comparable to glTF extension preservation.
  - type: related_to
    target: nuif:research:content-addressed-versioning
    note: Godot's uid is random or path-seeded, never content-derived, and exists to survive file moves.
links:
  spec: [spec/02-identity-and-properties.md, spec/03-components-and-composition.md, spec/07-extensions-and-dialects.md, spec/08-serialization.md]
  adr: [adrs/0004-serialization.md]
  rfc: [rfcs/0002-extension-preservation.md, rfcs/0001-multi-level-document-model.md]
  code: [crates/nuif-codec, crates/nuif-core]
  experiments: []
---

# Summary

Godot's `.tscn`/`.tres` formats are INI-like text serializations of a `PackedScene` (a flattened `SceneState`) or a `Resource`. A file declares a `format` version and a `uid`, lists external resources by `(type, uid, path, id)`, internal sub-resources by `(type, id)`, then nodes by name, type, parent path and property assignments, then signal connections. Node identity in the file is the scene-relative node path; resource identity is a per-file local id plus a project-wide `uid://` that survives renames and moves. Inherited scenes and instanced sub-scenes are stored as sparse overrides against a base `PackedScene`. Since pull request (PR) #60597 (merged 2022-05-05, milestone 4.0) nodes and resources whose class is not registered are loaded into `MissingNode`/`MissingResource` placeholders that record every assigned property and are written back under their original class name on save, so unknown types survive load/save round trips. Local ids for sub-resources are generated randomly and then cached, which gives diff stability only after the first save.

## Evidence

- File descriptor `[gd_scene format=3 uid="uid://..."]`; `format=3` for Godot 4.x, `format=2` for 3.x; `load_steps` is deprecated; five sections (descriptor, external resources, internal resources, nodes, connections) "should appear in order". Godot docs (latest), "TSCN file format", retrieved 2026-08-29.
- `[ext_resource type="Texture2D" uid="uid://ccbm14ebjmpy1" path="res://gradient.tres" id="2_eorut"]`; `[sub_resource type="CapsuleShape3D" id="CapsuleShape3D_fdxgg"]`; node heading `[node name="PlayerCamera" type="Camera" parent="Player/Head" ...]`; valid node keys include `instance`, `instance_placeholder`, `owner`, `index`, `groups`, `node_paths`; the root "must not have a parent= entry", direct children use `parent="."`; comments start with `;` and are discarded on save. Same page.
- `uid` is "a unique string-based identifier representing the scene" enabling tracking when moved. Same page.
- `ResourceUID`: UIDs "allow the engine to keep references between resources intact, even if files are renamed or moved"; `create_id()` is random and unique among loaded UIDs; `create_id_for_path()` is deterministic, "seeded with the provided path and project name"; `id_to_text()` yields `uid://...`; `set_id()` rebinds a UID to a new path. Godot docs (stable), class `ResourceUID`, retrieved 2026-08-29.
- Text loader falls back to path when the UID is unknown: `WARN_PRINT("...invalid UID: " + uidt + " - using text path instead: " + path)`; ext_resource lines are written with `uid=` only when `ResourceSaver::get_resource_id_for_path` returns a valid id. `scene/resources/resource_format_text.cpp`, master, retrieved 2026-08-29.
- Sub-resource ids: if `res->get_scene_unique_id()` is empty, a new id `<class>_<generate_scene_unique_id()>` is generated until unused; duplicates are cleared and regenerated; ext_resource ids are `<counter>_<scene_unique_id>` under `TOOLS_ENABLED`. Same file.
- `FORMAT_VERSION = 4` ("PackedByteArray can be base64 encoded, and PackedVector4Array was added") and `FORMAT_VERSION_COMPAT = 3` ("save as version 3 if not using PackedVector4Array or no big PackedByteArray"); loading refuses `format_version > FORMAT_VERSION` with "Saved with newer format version". `scene/resources/resource_format_text.h` and `.cpp`, master, retrieved 2026-08-29.
- `#define PACKED_SCENE_VERSION 3`; in `SceneState::instantiate`, when `ClassDB::instantiate` fails, `missing_node = memnew(MissingNode); missing_node->set_original_class(snames[n.type]); missing_node->set_recording_properties(true); node = missing_node;`. `scene/resources/packed_scene.cpp`, master, retrieved 2026-08-29.
- On pack (`SceneState::_parse_node`): `MissingNode *missing_node = Object::cast_to<MissingNode>(p_node); if (missing_node != nullptr) { nd.type = _nm_get_string(missing_node->get_original_class(), name_map); }`, so the original class name is written back. Same file.
- `MissingNode::_set` inserts any property while `recording_properties` is true and otherwise only updates existing keys; `_get_property_list` enumerates recorded properties with their runtime Variant type; configuration warnings: "This node was saved as class type '%s', which was no longer available when this scene was loaded." `scene/main/missing_node.cpp`, master, retrieved 2026-08-29.
- `MissingNode` is "An internal editor class intended for keeping the data of unrecognized nodes" with `original_class`, `original_scene`, `recording_properties`, `recording_signals`; `MissingResource` likewise with `original_class` and `recording_properties`; both warn that properties can be freely modified in code regardless of intended type. Godot docs (stable), classes `MissingNode` and `MissingResource`, retrieved 2026-08-29.
- Resource loader path: when `ResourceLoader::is_creating_missing_resources_if_class_unavailable_enabled()`, an unknown class yields `MissingResource` with `set_recording_properties(true)`, later disabled; properties that could not be set are stored under `META_MISSING_RESOURCES`. `resource_format_text.cpp`, master.
- Motivating defect: issue #57427 (2022-01-29, neikeq) shows a node of a missing GDExtension type reverting to `Node`, losing custom properties, the type attribute and signal connections on save; closed by PR #60597. GitHub issue #57427, retrieved 2026-08-29.
- PR #60597 "Implement missing Node & Resource placeholders" (reduz, opened 2022-04-28, merged 2022-05-05, milestone 4.0): on save "both binary and text formats recognize these placeholders and convert them back to their original types"; the aim is that "missing types no longer cause data loss". GitHub PR #60597, retrieved 2026-08-29.
- `recording_signals` was added to `MissingNode` by PR #105449 (merged 2025-10-10); commit `fdecca2f18` on `scene/main/missing_node.cpp`. GitHub API query, retrieved 2026-08-29.
- Counter-evidence on robustness: issue #99863 (2024-11-30, v4.3.stable) reports entire scenes turning into `MissingNode` without a reproduction; closed and archived without a maintainer root cause. GitHub issue #99863, retrieved 2026-08-29.
- Scene-unique names: a node renamed with a leading `%` or marked "Access as Unique Name" is addressable as `%Name` from within the same scene; lookups are cached; access from other scenes goes through an intermediate node (`get_node("%Sword/%Hilt")`). Godot docs (stable), "Scene Unique Nodes", retrieved 2026-08-29.
- `PackedScene.pack()` "Packs the path node, and all owned sub-nodes"; `instantiate()` triggers child scene instantiation; `GEN_EDIT_STATE_MAIN_INHERITED` exists "for the case where the scene is being instantiated to be the base of another one". Godot docs (latest), class `PackedScene`, retrieved 2026-08-29.
- `SceneState` stores a `base_scene_idx` into the variants array for inherited scenes, `NO_PARENT_SAVED`, and flags `FLAG_ID_IS_PATH` and `FLAG_INSTANCE_IS_PLACEHOLDER`. `packed_scene.cpp`, master.

## Mechanism

Serialization model. A `PackedScene` holds a `SceneState`: string tables (names), a variants array (property values, including references to external `PackedScene`s and sub-resources), and a node table where each node record has name, type, parent index, owner, instance reference, an ordered property list of `(name index, variant index)` pairs, groups, and connection records. `pack()` walks the tree from the root, including only nodes owned by the root (nodes created by instanced sub-scenes are represented by their instance root plus overrides, not expanded). The text writer emits the state as `[node ...]` sections in tree order, with parent expressed as a scene-relative path; node order within a parent is implicit in the section order and can be pinned with `index`.

Identity. Three identities coexist. Nodes are identified by path from the scene root (name-based, order-independent, changed by rename or reparent). Sub-resources and external resources get file-local ids of the form `<Class>_<5 random alphanumerics>`; the id is cached on the resource (`scene_unique_id`) so later saves reuse it, but a fresh resource or a duplicate gets a new random id. Files are identified project-wide by a 64-bit `uid` stored in the `.import` or resource file and mirrored in `ext_resource uid=`; the loader prefers the uid and falls back to path with a warning, so moves and renames do not break references as long as the uid cache is current.

Inheritance and overrides. An inherited scene stores a reference to its base `PackedScene` in the variants array (`base_scene_idx`) and then only the nodes and properties that differ: added nodes with their parent path into the base, and property assignments on base nodes. An instanced sub-scene inside a scene is a node with `instance=ExtResource(...)`; child nodes of the instance are addressable for overrides only when the instance is marked editable, and overrides are again stored as sparse property assignments by path. Resolution (`instantiate`) instantiates the base first, then applies the derived state's nodes and properties in order. Authored state is therefore the override set; the resolved tree exists only in memory.

Unknown types. Loading is class-name driven. When `ClassDB` cannot instantiate the recorded class, the loader creates a placeholder whose `_set` records every incoming `(name, value)` pair verbatim while `recording_properties` is on. Because the packer asks the node for `get_property_list()` and its values, the placeholder reports exactly the recorded pairs and the packer substitutes `original_class` for the type name, so the written section is byte-equivalent in content to the original (ordering and formatting are regenerated by the writer). Verified: preservation on save is implemented in `SceneState::_parse_node` (node path) and in the text saver's `MissingResource` handling (resource path), not only at load time. Limits: placeholders are editor-facing (the docs warn users to ignore them), value types are not validated, signals were not recorded until PR #105449 in 2025, and there is at least one unresolved field report (#99863) of scenes collapsing into placeholders.

Diff stability. Text output is regenerated on every save from the in-memory state: comments are dropped, ids are reused when cached, and section order follows tree order. The `format` attribute is written as 3 unless a feature requiring 4 is used, so files do not churn on format version. Random suffixes on ids mean two users adding a sub-resource independently will produce distinct ids and a textual merge will not falsely unify them, at the cost of non-reproducible output for freshly created resources.

## NUIF relevance

**Borrow**
- The two-phase "record everything you cannot interpret, write it back under the original type" placeholder design as a concrete realization of opaque-preservation for whole entities, including the observable rule that the packer treats placeholders identically to real nodes.
- Path-independent project-wide `uid` for files separate from file-local ids for embedded resources, with a documented fallback to path and a warning; NUIF asset references should carry the same (stable id, path hint) pair.
- Writing the compatible format version whenever the newer features are unused, which keeps forward compatibility maximal without a separate export step.

**Adapt**
- Node identity by path is insufficient for NUIF (spec/02 forbids path-dependent identity); the sparse-override structure can be kept but keyed by stable entity IDs so that renames and reparents inside the base do not orphan overrides.
- Random local ids with post-hoc caching should become deterministic ids derived from the entity's stable ID so canonical output is reproducible from the first save.
- Placeholder entities should carry a fidelity status (`preserved_unrenderable`) and a declared origin namespace rather than an editor-only warning.
- Record signals/relations of unknown entities from the start; Godot's three-year gap before `recording_signals` shows why preservation must cover relations as well as properties.

**Reject**
- Type-less recording (the placeholder stores runtime Variant types inferred from values); NUIF preservation must retain the serialized encoding of unknown data rather than reinterpret it through the host type system.
- Implicit ordering by section order with an optional `index` escape hatch; NUIF containment order must be explicit and merge-safe.

## Open questions

- Whether `MissingNode` round trips preserve property order and formatting well enough that a no-op load/save of a scene with unknown types yields an empty textual diff; not verified from retrieved sources.
- The generation algorithm and entropy of `generate_scene_unique_id()` and the collision behaviour on merge when two branches create the same suffix.
- Whether the docs' `format=3` statement will be updated for `FORMAT_VERSION = 4` in master, and how older editors handle a version-4 file (loader refuses with `ERR_FILE_UNRECOGNIZED`).
