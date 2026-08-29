---
id: nuif:research:accesskit-semantic-ui-testing
kind: repository
status: reviewed
title: AccessKit as a cross-platform accessibility tree and semantic UI test surface
source:
  url: https://github.com/AccessKit/accesskit
  repository: https://github.com/AccessKit/accesskit
  authors: [Matt Campbell, AccessKit contributors]
  published_at: "accesskit 0.25.0 (2026-08-29); accesskit_consumer 0.38.0 (2026-07-14); accesskit_winit 0.34.0 (2026-08-29)"
  license: Apache-2.0 OR MIT (Chromium-derived portions under a BSD-style licence)
retrieved_at: 2026-08-29
tags: [accessibility, accesskit, semantic-tree, testing, harness, kittest, uia, at-spi, nsaccessibility, aria]
confidence: 0.9
claims: [nuif:claim:semantic-automation]
relations:
  - type: related_to
    target: nuif:research:accessibility-semantics
    note: AccessKit roles and properties are a Chromium-derived superset of WAI-ARIA semantics lowered to platform APIs.
  - type: implements
    target: nuif:research:egui-and-egui-kittest
    note: kittest and egui_kittest query the accesskit_consumer tree and dispatch AccessKit actions.
  - type: implements
    target: nuif:research:masonry-xilem-and-linebender-test-harness
    note: masonry_testing keeps an accesskit_consumer::Tree and accepts ActionRequest.
  - type: related_to
    target: nuif:research:iced-slint-gpui-makepad-floem
    note: Accessibility-tree column of the toolkit comparison.
  - type: related_to
    target: nuif:research:open-ui
    note: Role vocabularies for authored components.
links:
  spec: [spec/13-semantics-accessibility-and-behavior.md]
  adr: [adrs/0001-rust-reference-core.md]
  rfc: []
  code: [apps/editor/QA.md, apps/editor/ARCHITECTURE.md, crates/nuif-core, crates/nuif-query]
  experiments: []
---

# Summary

AccessKit defines a Rust data schema for an accessibility tree (`Node`, `Role`, `Action`, `TreeUpdate`, `ActionRequest`) and platform adapters that expose that tree to Windows UI Automation, macOS NSAccessibility, Unix AT-SPI, Android and iOS. The provider (toolkit) pushes full and incremental `TreeUpdate`s; the adapter retains the tree and pulls nothing, which the project states makes the design suitable for immediate-mode toolkits with stable node IDs. `accesskit_consumer` is the platform-independent tree store used by the adapters; it is also the basis of kittest, egui_kittest and masonry_testing, which turn the same tree into a test oracle: tests query nodes by role, label or value and dispatch `Action`s (Click, Focus, SetValue, ScrollIntoView) without synthesising pointer input.

NUIF interpretation: the AccessKit schema is a concrete, serialisable (serde, JSON Schema via schemars) semantic tree that NUIF can emit from resolved documents; it gives QA item 5 (inspect accessibility semantics) and QA item 3 (execute semantic actions) a shared representation. It is not a normative NUIF model: roles are Chromium-derived and platform-oriented, and the tree carries resolved geometry rather than authored intent.

## Evidence

- Versions: accesskit 0.25.0 published 2026-08-29 (crates.io `max_version`, `updated_at`), changelog entry "0.25.0 (2026-08-29)" with breaking change "Reuse property buffers when cloning nodes" and feature "Add html_id node property (#776)"; previous 0.24.1 (2026-06-12). accesskit_consumer 0.38.0 (2026-07-14; main manifest is 0.39.0 depending on accesskit 0.25.0). accesskit_winit 0.34.0 released 2026-08-29; accesskit_windows 0.34.0, accesskit_macos 0.26.3, accesskit_unix 0.22.1 (2026-07-14). Locator: crates.io API; `accesskit/CHANGELOG.md` lines 1-20; `accesskit/Cargo.toml`, `accesskit_consumer/Cargo.toml`, main commit 42e53b0.
- Schema description: "each node is either a single UI element or an element cluster"; "Each node has an integer ID, a role (e.g. button, label, or text input), and a variety of optional attributes"; "The schema is based largely on Chromium's cross-platform accessibility abstraction"; canonical definition in Rust, other representations generated. Locator: `README.md` "Data schema".
- Push model: the toolkit "initially pushes a complete accessibility tree, then it pushes incremental updates"; "only the platform adapter needs to retain a complete accessibility tree"; "suitable for immediate-mode GUI toolkits, as long as they can provide a stable ID for each UI element". Locator: `README.md` "Platform adapters".
- Adapters listed: Android, iOS (UIAccessibility), macOS (NSAccessibility), Unix (AT-SPI over D-Bus via zbus), Windows (UI Automation); planned: web. Language bindings: C (cbindgen), Python (PyO3). Locator: `README.md` "The following platform adapters are currently available", "Language bindings".
- `Role` enum has 182 variants (lines 62-271); `Action` enum has 22 variants: Click, Focus, Blur, Collapse, Expand, CustomAction, Decrement, Increment, HideTooltip, ShowTooltip, ReplaceSelectedText, ScrollDown, ScrollLeft, ScrollRight, ScrollUp, ScrollIntoView, ScrollToPoint, SetScrollOffset, SetTextSelection, SetSequentialFocusNavigationStartingPoint, SetValue, ShowContextMenu. Locator: `accesskit/src/lib.rs` lines 62-271, 289-320, main.
- `Node` (line 1107) is documented as "A single accessible object. A complete UI is represented as a tree of these."; getters/setters act as properties (`role()`, `set_role`, `add_action`, `supports_action`). Locator: `accesskit/src/lib.rs` lines 1095-1110, 1858-1871.
- `TreeUpdate { nodes: Vec<(NodeId, Node)>, tree: Option<TreeInfo>, tree_id: TreeId, focus: NodeId }`: nodes overwrite by ID; adding a child requires the updated parent; removal is expressed by omitting the child from the parent's `children`; `focus` "must be provided with every tree update". Locator: `accesskit/src/lib.rs` lines 3207-3256.
- `ActionRequest { action, target_tree, target_node, data: Option<ActionData> }`; traits `ActivationHandler`, `ActionHandler`, `DeactivationHandler`. Locator: `accesskit/src/lib.rs` lines 3304-3380.
- Cargo features: `serde`, `schemars` (JSON Schema, with `serde_json`), `pyo3`, `enumn`; the only mandatory dependency is `uuid`. Locator: `accesskit/Cargo.toml` lines 16-30, main.
- Consumer API: `Tree::new(initial_state: TreeUpdate, is_host_focused)` panics unless `TreeUpdate::tree` is `Some` and `tree_id == TreeId::ROOT`; `update_and_process_changes(update, &mut impl ChangeHandler)`; `ChangeHandler { node_added, node_updated, focus_moved, node_removed }`; `TreeState::{root, node_by_id, focus, active_dialog, toolkit_name, subtree_root}`; `NodeRef::{role, label, description, value, parent, children, bounding_box, is_focused, is_hidden, toggled, supports_action, labelled_by, author_id, html_id}`. Locator: `accesskit_consumer/src/tree.rs` lines 67-708; `node.rs` lines 89-985, main.
- winit adapter: `Adapter::{with_event_loop_proxy, with_direct_handlers, with_mixed_handlers, process_event(window, &WindowEvent), update_if_active(|| TreeUpdate)}`; platform adapters expose `update_if_active` so the tree is built only when assistive technology is active. Locator: `adapters/winit/src/lib.rs` lines 127-264; `adapters/{windows,macos,unix}/src/adapter.rs`.
- kittest 0.4.0 depends only on `accesskit` and `accesskit_consumer`; its `By` filter supports label, label-contains, role, value and predicate; `NodeT` wraps `accesskit_consumer::Node`. Locator: kittest `Cargo.toml` lines 24-27; `src/query.rs` lines 146-236; `src/node.rs` lines 9-60.
- egui_kittest `click_accesskit()` sends `Event::AccessKitActionRequest(ActionRequest { action: Action::Click, .. })` and "can also click widgets that are not currently visible"; `focus()` sends `Action::Focus`; `scroll_to_me()` sends `Action::ScrollIntoView`. Locator: egui `crates/egui_kittest/src/node.rs` lines 101-160.
- masonry_testing keeps `access_tree: accesskit_consumer::Tree`, updates it in `redraw()`, and exposes `process_access_event(ActionRequest)` and `accessibility_click_on(WidgetId)`. Locator: xilem `masonry_testing/src/harness.rs` lines 146-160, 466, 576-608, 767.
- egui's workspace notes that kittest 0.4 pins accesskit_consumer 0.35 and blocks accesskit_winit upgrades. Locator: egui `Cargo.toml` lines 73-75 at tag 0.36.1.

## Mechanism

Data model and flow:

```text
toolkit frame ──TreeUpdate{nodes, tree?, tree_id, focus}──▶ accesskit_consumer::Tree (retained)
                                                              │ ChangeHandler: node_added/updated/removed, focus_moved
                                                              ▼
                                            platform adapter (UIA / NSAccessibility / AT-SPI)
                                                              │
assistive technology ──ActionRequest{action, target_tree, target_node, data}──▶ ActionHandler (toolkit)
```

Test-surface instantiation (kittest / masonry_testing): the harness plays the role of the platform adapter. It owns the `accesskit_consumer::Tree`, applies each frame's `TreeUpdate`, evaluates queries over `NodeRef` (role, label, value, `labelled_by`, `bounding_box`, `supports_action`), and injects `ActionRequest`s into the toolkit's event queue. Assertions read the next frame's tree.

Invariants stated by the schema: node IDs are stable across updates; every update carries the current focus; a node's `children` list is the only membership authority; roles are closed enumerations (182 roles, 22 actions in 0.25.0). Property presence is optional, so queries must treat `None` as "not exposed" rather than "false".

Relation to WAI-ARIA (interpretation): AccessKit roles derive from Chromium's `ax::mojom::Role`, which includes every ARIA role plus host-language and platform roles (window, document, list-marker, etc.); the Core Accessibility API Mappings that WAI-ARIA relies on are implemented inside the AccessKit adapters. A NUIF role vocabulary can therefore be lowered to AccessKit `Role` in the editor and to ARIA in a web adapter from one intent-level definition, subject to a mapping table that this record has not verified.

Serialization: with the `serde` feature a `TreeUpdate` is a plain data value; with `schemars` a JSON Schema can be generated, which makes the tree usable as a fixture format.

## NUIF relevance

**Borrow**
- `TreeUpdate` as the wire format for "inspect accessibility semantics" (QA item 5), because it is stable, serialisable, schema-describable and already consumed by two Rust test harnesses.
- `ActionRequest` as the pointer-free action channel for "execute semantic transactions" (QA item 3) in GUI wiring tests, because `Action::Click`, `Focus`, `SetValue`, `SetTextSelection`, `ScrollIntoView` cover editor chrome interactions without coordinates.
- `accesskit_consumer::Tree` plus `ChangeHandler` as the diff oracle for shell state across frames, because `node_added`/`node_updated`/`node_removed` callbacks give a deterministic change log.

**Adapt**
- NUIF entity identity must be carried through the tree; `author_id` (existing) and `html_id` (0.25.0) are the candidate properties for `EntityId` or stable names, so queries by NUIF identity remain possible after lowering.
- NUIF roles should be defined at intent level (spec 13) and lowered to AccessKit `Role` by the editor adapter; the record for nuif:research:accessibility-semantics already requires adapters to report semantic equivalence rather than claiming identical platform trees.
- Version coupling must be isolated in the editor crate; the kittest/accesskit_consumer pin conflict in egui shows that harness, consumer and adapter versions drift independently.

**Reject**
- Making AccessKit `Role` or `Node` normative NUIF state, because the schema is Chromium-derived, platform-oriented and includes resolved bounds and focus rather than authored intent.
- Using platform adapters (UIA, AT-SPI) as the test surface, because they require a window and a live accessibility bus; the consumer tree already provides the same information headlessly.

## Open questions

- Whether AccessKit issue #701 (public construction of `accesskit_consumer::NodeId`) is closed in 0.39, removing the `unsafe` hack in masonry_testing.
- Whether the multi-tree (`TreeId`, subtree) model introduced in recent releases can represent NUIF component instances as grafted subtrees.
- Whether a published role-mapping table from AccessKit `Role` to ARIA roles exists; the mapping is implied by the Chromium lineage but not documented in the repository README.
- Whether the planned web adapter will allow the same `TreeUpdate` to drive DOM ARIA attributes for a browser build of the editor.
