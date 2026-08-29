# Draft Figma Plugin API profile 0

Status: researched mapping specification; no executable Figma plug-in or live
host conformance claim.

Profile identifier: `nuif-figma-plugin-0`.

Primary evidence:
`nuif:research:figma`,
`nuif:research:figma-plugin-and-rest-api-as-automation-surface`, and ADR 0008.

## Host and scope

- Figma Design through Plugin API `1.0.0`; `editorType: ["figma"]`.
- One explicitly loaded `PageNode` per operation.
- Import and export of frames, groups, rectangles, ellipses and literal text.
- Solid sRGB fills; visibility; opacity; ordered containment; freeform
  position; fixed width/height; frame auto-layout row/column, padding, gap and
  primary/counter alignment when exactly representable.
- Component/instance, variable, grid-layout, vector-network, effect,
  interaction and typography fields outside this list receive explicit
  fidelity entries and are not silently flattened.

The `.fig` encoding and undocumented multiplayer protocol are excluded.

## Identity and correspondence

Each host node correspondence records `SceneNode.id` as `host_object_id` and
the affected property name when the entry is property-specific. The plug-in may
store these shared data entries:

| Namespace | Key | Value |
|---|---|---|
| `nuif` | `document_id` | canonical NUIF document identifier |
| `nuif` | `entity_id` | canonical NUIF entity identifier |
| `nuif` | `profile` | `nuif-figma-plugin-0` |

The bridge treats plug-in data as correspondence assistance, not authority. It
scans the loaded scope before mutation. Missing identifiers are assigned;
duplicate identifiers are replaced on every duplicate except the first stable
host-tree occurrence. Every repair appears in the host report. No persistence
claim is made for copy, paste or duplication until a live fixture proves it.

## Import transaction

1. The UI iframe reads a user-selected `.nuif` file under the NUIF encoded and
   semantic limits.
2. Pure mapping builds a host mutation plan and `HostAdapterReport` without
   modifying the file.
3. The UI presents fidelity totals and all unsupported entries.
4. On confirmation, the main thread creates/updates the declared scope. It does
   not call `commitUndo` during the plan, so the host treats the run as one undo
   group when the plug-in closes.
5. Any exception stops the plan, triggers host undo when needed, and returns a
   failed report. Atomicity must be proven in the live-host gate.

## Export transaction

1. Export defaults to the current selection, or the current page when selection
   is empty; document-wide export is a separate user action.
2. The plug-in loads only the required page nodes and records omitted pages.
3. Pure mapping emits canonical NUIF plus a host report.
4. The UI iframe downloads both files. Export does not mutate the host file.

## Resource limits

The profile inherits NUIF profile-zero limits and additionally caps one run at
one loaded page, 16,384 traversed nodes, 4,096 UTF-16 code units per text node,
100 kB per shared-data entry, and 16 MiB combined message payload between main
thread and UI. Limit-plus-one inputs must fail before host mutation. These are
candidate profile limits pending live Figma timing/allocation calibration.

## Required fixtures

- covered one-page import/export and repeated-output determinism;
- reorder without identity drift;
- missing and duplicate shared-data identifiers;
- an invisible instance descendant and the traversal-mode report;
- unloaded-page omission versus explicit document-wide load;
- unsupported component, variable, vector, effect and interaction properties;
- unavailable font and mixed-style text;
- resource-limit and user-cancellation cases;
- undo returns the exact pre-import host tree.

Publication remains blocked until the mapping, checked-in snapshots, compiled
plug-in bundle and live Figma product/version trial all exist.
