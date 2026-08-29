---
id: nuif:research:houdini-pdg-and-hda
kind: implementation
status: reviewed
title: Houdini procedural networks, cooking, dirty propagation and digital assets
source:
  url: https://www.sidefx.com/docs/hdk/_h_d_k__op_basics__overview__cooking.html
  authors: [SideFX]
  published_at: "Houdini 20.x/21 documentation and HDK reference, retrieved 2026-08-29"
  license: Proprietary documentation (SideFX); HDK headers under SideFX licence
retrieved_at: 2026-08-29
tags: [procedural, node-graph, dependency-graph, dirty-propagation, cook, authored-resolved, component, versioning, headless]
confidence: 0.82
claims: [nuif:claim:authored-resolved, nuif:claim:multi-level-ir, nuif:claim:semantic-automation]
relations:
  - type: supports
    target: nuif:research:openusd
    note: Houdini's authored network versus cooked geometry mirrors USD's authored opinions versus composed stage.
  - type: compares_to
    target: nuif:research:mlir
    note: Cooking is on-demand lowering with memoization; MLIR lowering is eager and pass-driven.
  - type: related_to
    target: nuif:research:blender-dna-rna-and-headless
    note: hython and blender --background are equivalent headless automation surfaces over the full data model.
  - type: related_to
    target: nuif:research:godot-tscn-scene-format
    note: HDA definition matching (locked instances) is the same problem as scene instance overrides with a versioned definition.
links:
  spec: [spec/01-model.md, spec/03-components-and-composition.md, spec/04-layout.md, spec/12-cli-api-and-automation.md]
  adr: [adrs/0002-layout-engine.md]
  rfc: [rfcs/0003-authored-resolved-provenance.md, rfcs/0004-headless-qa-contract.md]
  code: [crates/nuif-layout, crates/nuif-api]
  experiments: []
---

# Summary

Houdini stores a scene as networks of operator nodes with parameters and wires; this network is the authored intent and is what a `.hip` file persists. Geometry, images and simulation states are produced by cooking: a pull-based, memoized evaluation in which a node recomputes only when it is asked for its data and is marked out of date. Parameter edits dirty every dependent node transitively; extra (non-wire) dependencies must be declared on every cook because they are cleared during dirty propagation. Digital assets (HDAs) package a subnetwork as a reusable operator type identified by `namespace::name::version`, with instances locked to the definition by default and explicit unlock/save/match operations. `hython` runs the same object model headlessly.

## Evidence

- "In Houdini, cooking refers to evaluating the nodes in the networks to compute the state of the scene in the current frame"; update modes Auto Update, On Mouse Up, Manual with Force Update. SideFX docs, "Cooking" (`basics/cooking.html`), retrieved 2026-08-29.
- "Nodes are never cooked unless they are asked for their data"; a node recooks only when asked and "itself is out of date"; recooking "propagates up the cook chain"; "If Houdini at any point encounters a node that is up to date, then no further cooking will be done"; cooking is framed as functional evaluation without side effects; SOP implementers override `SOP_Node::cookMySop()`. HDK docs, "Cooking" (`_h_d_k__op_basics__overview__cooking.html`), retrieved 2026-08-29.
- "When a parameter changes, everything in the graph that depends on the parameter's data is dirtied accordingly"; parameters are dependencies by default; other-node data must be declared via `OP_Node::addExtraInput()`; extra inputs "are cleared as soon as they are traversed upon the dirty propagation", so the call must happen on every cook; `DOP_Parent::simMicroNode()` tracks simulation dependencies. HDK docs, "Dependencies" (`_h_d_k__op_basics__overview__dependencies.html`), retrieved 2026-08-29.
- `hou.OpNode.cook(force=False, frame_range=())` "Asks or forces the node to re-cook"; `needsToCook(time=hou.time())` "Asks if the node needs to re-cook"; `isTimeDependent(for_last_cook=False)`: a time dependent node "is re-evaluated every time the frame changes"; `cookCount()` counts cooks in the session; `matchesCurrentDefinition()`, `allowEditingOfContents(propagate=False)`, `isLockedHDA()`. SideFX docs, class `hou.OpNode`, retrieved 2026-08-29.
- `hython` is a Python shell that adds `$HHP` to `sys.path`, imports `hou`, loads `.hip` files passed on the command line, accepts `%`-prefixed hscript, and checks out a Houdini Batch licence (falling back to FX). SideFX docs, "Command line scripting" (`hom/commandline.html`), retrieved 2026-08-29.
- HDA internal names are `[namespace::]name[::version]`; a version "can only contain numbers and periods"; without a version Houdini "selects the node with the highest version number"; scoped and un-namespaced definitions take precedence in a documented order; `HOUDINI_OPNAMESPACE_HIERARCHY` overrides. SideFX docs, "Namespaces and versions" (`assets/namespaces.html`), retrieved 2026-08-29.
- A digital asset is created by converting a subnetwork; multiple assets can share a `.hda` library; "you can't change the internal name without recreating the asset"; assets can be saved embedded in the HIP file. SideFX docs, "Create a digital asset" (`assets/create.html`), retrieved 2026-08-29.
- Locked instances "match the current definition"; "Allow editing of contents" unlocks; "Save node type" writes to the library; "Match current definition" relocks and discards; while unlocked, "Other instances of the same asset will get the same changes, but the original definition of the asset still exists on disk". SideFX docs, "Editing digital assets" (`assets/edit.html`), retrieved 2026-08-29.
- Asset Manager: black names use the current definition, yellow means a newer definition exists elsewhere, red means not current; "Use This Definition" pins a definition; priority options for index files, HIP-embedded and latest-date definitions; "Safeguard Operator Definitions" removes unlock menu items. SideFX docs, "Asset Manager window" (`ref/windows/optypemanager.html`), retrieved 2026-08-29.

## Mechanism

Authored state is the operator graph: node types, parameter values (which may be expressions over time and other parameters), wires, and flags. Resolved state is the cook output per node per `OP_Context` (time plus evaluation options), held in per-node caches. Dirtying is push-based: a parameter or input change marks the node and walks outgoing edges, marking dependents; declared extra inputs are consumed during this walk, which is why they are re-declared on every cook. Cooking is pull-based: a consumer (viewport display flag, renderer, script) requests data; if the node is dirty it requests its inputs recursively, recomputes and clears its dirty flag. Time-dependent nodes are dirtied on every frame change. Update modes only change when the UI issues pull requests. The invariant is memoized purity: identical inputs and parameters at a given context yield identical outputs, so caches can be trusted until dirtied.

Digital assets are operator type definitions stored outside the scene. An instance stores its type name and parameter values; its internal subnetwork is not persisted while locked, so the HIP references the definition and resolves contents at load. Unlocking copies the definition's contents into the instance (a local override of the whole subnetwork); saving pushes the copy back as the new definition; matching discards the copy. Definition selection is name-based with version ordering and configurable precedence, so two libraries can supply the same type name and the resolved definition depends on environment and Asset Manager preferences.

## NUIF relevance

**Borrow**
- Separation of persisted authored graph from cached cooked output, with per-context caches invalidated by dependency dirtying; this is the model for NUIF's resolved layer, where layout and text shaping results are caches keyed by evaluation context (spec/04, RFC 0003).
- Explicit dependency declaration for non-structural dependencies (token references, expressions) and the rule that such dependencies are re-established on every evaluation.
- Version in the type name with numeric ordering and highest-wins default for component library resolution.

**Adapt**
- Push-dirty/pull-cook fits NUIF layout evaluation, but NUIF must record which context a resolved value belongs to and never overwrite authored values (spec/02), whereas Houdini caches are anonymous per node.
- HDA lock/unlock is whole-subnetwork override; NUIF instance overrides are per property and per slot, so the analogue is a typed override set with an explicit "detached from definition" state that fidelity accounting can report.

**Reject**
- Definition resolution dependent on environment variables and editor preferences; NUIF component references must resolve to a specific identity and version recorded in the document.
- Loss of the unlocked subnetwork's relation to its definition beyond a type name; NUIF requires provenance records linking overrides to the definition version they were authored against.

## Open questions

- Whether Houdini persists per-node cook caches to disk in any format that could be compared with NUIF resolved caches; the retrieved docs describe in-memory caching only.
- How PDG/TOPs work-item dependencies differ from OP-level dirtying (not retrieved; the `tops/cooking.html` page was located but not fetched).
