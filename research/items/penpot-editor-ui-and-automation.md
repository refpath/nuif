---
id: nuif:research:penpot-editor-ui-and-automation
kind: repository
status: reviewed
title: Penpot workspace UI, plugin API and RPC automation surface
source:
  url: https://help.penpot.app/user-guide/the-interface/
  repository: https://github.com/penpot/penpot
  authors: [Penpot]
  published_at: "unknown"
  license: MPL-2.0 for the implementation; documentation licence not verified
retrieved_at: 2026-08-29
tags: [penpot, editor-ui, layers-panel, properties-panel, toolbar, plugin-api, rpc, clojurescript, open-source]
confidence: 0.86
claims: [nuif:claim:semantic-automation, nuif:claim:opaque-preservation]
relations:
  - type: extends
    target: nuif:research:penpot
    note: Adds the editor surface and automation APIs to the data-model record.
  - type: compares_to
    target: nuif:research:figma-ui3-editor-layout
    note: Same left/centre/right composition; toolbar at top instead of bottom.
  - type: related_to
    target: nuif:research:figma-plugin-and-rest-api-as-automation-surface
    note: Plugin API mirrors Figma's shape (selection, create*, events, pluginData, export).
  - type: related_to
    target: nuif:research:figma-tools-and-keyboard-shortcuts
    note: Most tool and view bindings coincide.
  - type: related_to
    target: nuif:research:design-editor-ui-conventions-synthesis
    note: One row of the cross-editor comparison.
links:
  spec: [spec/12-cli-api-and-automation.md, spec/07-extensions-and-dialects.md]
  adr: []
  rfc: [rfcs/0004-headless-qa-contract.md]
  code: [apps/editor/ARCHITECTURE.md, adapters/README.md]
  experiments: []
---

# Summary

Penpot's workspace is a single-page ClojureScript/React application over a Clojure backend. The Help Center's interface tour enumerates twenty regions: a horizontal toolbar and main menu at the top, Pages and Layers on the left, an infinite viewport with rulers in the centre, and Design, Prototype and Inspect tabs plus colour and typography palettes, Assets and Design tokens on the right; view mode, history, comments, zoom, presence and file status sit top-right. Plugins run in iframes with a `penpot` global that exposes selection, page and root access, shape constructors, events, per-shape plugin data, export and markup generation. External automation uses the backend RPC (`POST /api/rpc/command/<name>`) with personal access tokens and outbound webhooks. The open frontend source (`frontend/src/app/main/ui/workspace/...`) is a readable reference for panel decomposition.

NUIF interpretation: Penpot confirms the shared left-structure / right-properties / centre-canvas convention and shows that an open editor can expose the same three surfaces NUIF requires (in-editor API, external RPC, event feed). Its file-level RPC is an internal API rather than a normative contract, which is the gap NUIF's CLI/API specification addresses.

## Evidence

Retrieval date for all locators: 2026-08-29.

- Interface tour regions (numbered 1–20): Viewport (1), Toolbar (2, "tools to quickly and easily create different types of layers"), Main menu (3), Pages (4), Layers (5), Rulers (6), Color palette (7), Typography palette (8), Design properties (9), Prototype mode (10), Inspect mode (11), View mode (12), Share/Invite (13), History (14), Comments (15), Zoom (16), Users (17), Assets (18), Design tokens (19), File status (20). Toolbar and main menu are at the top; pages and layers on the left; design/prototype/inspect and palettes on the right. https://help.penpot.app/user-guide/first-steps/the-interface/ — legend.
- The interface guide lists the toolbar tools as board, rectangle, ellipse, text, graphic, path and free drawing and places zoom controls at the top right. https://help.penpot.app/user-guide/the-interface/.
- Design properties "view and edit the attributes of a selected layer"; size and position always present; stroke, shadow, blur optional. Same page.
- Design panel groups per the layers guide: size and position; layout and constraints; opacity and blend; fill, stroke and border radius; shadow (type, position, blur, spread, colour); blur (layer, background); text; export; interactions. Layer types: boards, rectangles/ellipses, text, curves (freehand), paths (bezier), images. https://help.penpot.app/user-guide/designing/layers/.
- Shortcuts: Board B, Rectangle R, Ellipse E, Text T, Image Shift K, Path P, Comments C, Color picker I; zoom Shift 0 (100%), Shift 1 (fit all), Shift 2 (selected); Layers panel Alt L, Assets Alt I, Color palette Alt P, Text palette Alt T, Rulers Ctrl Shift R, Hide UI `\`; Group Ctrl G, Create component Ctrl K, Detach Ctrl Shift K, Duplicate Ctrl D, Flex layout Shift A, Grid layout Ctrl Shift A, Undo Ctrl Z, Redo Ctrl Shift Z; Select all Ctrl A, Select parent Shift Enter. https://help.penpot.app/user-guide/first-steps/shortcuts/. The flexible-layouts guide states Ctrl/Cmd A for flex layout and Ctrl/Cmd Shift A for grid; the discrepancy is recorded as unverified. https://help.penpot.app/user-guide/flexible-layouts/.
- Flex layout properties: direction (row, reverse row, column, reverse column), wrap and alignment, align items, justify content, row/column gap, four-side padding, sizing fix/fit per axis; grid layout adds cell positioning modes. Same flexible-layouts page.
- Rulers measure in pixels; pixel-grid snapping is default and can be disabled; nudge distance set in Preferences. https://help.penpot.app/user-guide/workspace-basics/.
- Plugins: installed through the Plugin manager (Ctrl Alt P / Cmd Alt P) by manifest URL; manifest declares permissions `content:read/write`, `library:read/write`, `user:read`, `comment:read/write`, `allow:downloads`, `allow:localstorage`; "Plugins run separately from the main Penpot app, inside iframes"; only `plugin.js` can use the `penpot` object; types via `@penpot/plugin-types`. https://help.penpot.app/plugins/getting-started/.
- Plugin API reference is hosted at https://doc.plugins.penpot.app/ (link from https://help.penpot.app/plugins/api/).
- Type definitions: `Penpot extends Context` with `selection: Shape[]`, `currentPage: Page | null`, `root: Shape | null`, `currentFile: File | null`, `viewport`, `library`, `theme`, `createRectangle()`, `createBoard()`, `createText(text)`, `createShapeFromSvg(svgString)`, `group(shapes)`, `ungroup(group, ...)`, `on(type, callback, props?)`, `off(listenerId)`, `ui.open/sendMessage/onMessage`, `generateMarkup(shapes, options?)`, `generateStyle(shapes, options?)`; events `pagechange`, `selectionchange`, `themechange`, `shapechange`, `filechange`, `finish`, `contentsave`; per-shape `getPluginData/setPluginData(key, value)` and `getSharedPluginData/setSharedPluginData(namespace, key, value)`; `export(config: Export): Promise<Uint8Array>` with `type`, `scale`, `suffix`, `skipChildren`. https://raw.githubusercontent.com/penpot/penpot-plugins/main/libs/plugin-types/index.d.ts.
- Integration: personal access tokens under "Your account > Access tokens" with expiry options; RPC endpoint `/api/rpc/command/<name>` via POST with JSON or Transit; example `get-profile` with `Authorization: Token …`; team-level outbound webhooks mirror RPC calls labelled WEBHOOK. https://help.penpot.app/technical-guide/integration/.
- Self-hosted instances must enable `enable-access-tokens` in `PENPOT_FLAGS`; `get-file` is a POST to `/api/rpc/command/get-file` with the file id (snippet from Penpot blog and community; not retrieved in full). https://penpot.app/blog/how-to-integrate-penpot-with-your-developer-toolchain-apis-and-webhooks-for-workflow-automation/.
- Architecture: "a typical SPA" with a ClojureScript/React frontend served statically, a Clojure JVM backend persisting to PostgreSQL, a separate exporter, and shared common code. https://help.penpot.app/technical-guide/developer/architecture/.
- Source layout: `frontend/src/app/main/ui/workspace/` contains `sidebar/`, `viewport/`, `shapes/`, `colorpicker/`, `tokens/`, and files including `top_toolbar.cljs`, `left_header.cljs`, `right_header.cljs`, `main_menu.cljs`, `viewport.cljs`, `viewport_wasm.cljs`, `palette.cljs`, `color_palette.cljs`, `text_palette.cljs`, `comments.cljs`, `plugins.cljs`, `presence.cljs`, `nudge.cljs`. https://github.com/penpot/penpot/tree/develop/frontend/src/app/main/ui/workspace.
- `sidebar/` contains `layers.cljs`, `layer_item.cljs`, `layer_name.cljs`, `assets.cljs`, `options.cljs`, `sitemap.cljs`, `history.cljs`, `versions.cljs`, `shortcuts.cljs`, `debug.cljs`, plus `assets/`, `common/`, `options/`. https://github.com/penpot/penpot/tree/develop/frontend/src/app/main/ui/workspace/sidebar.
- `sidebar/options/menus/` holds one module per property group: `align`, `blur`, `bool`, `border_radius`, `color_selection`, `component`, `constraints`, `exports`, `fill`, `frame_grid`, `grid_cell`, `interactions`, `layer`, `layout_container`, `layout_item`, `measures`, `shadow`, `stroke`, `svg_attrs`, `text`, `typography`, `variants_help_modal`, `input_wrapper_tokens`, `token_typography_row`. https://github.com/penpot/penpot/tree/develop/frontend/src/app/main/ui/workspace/sidebar/options/menus.
- Unverified: whether the Penpot RPC API is declared stable or internal; the integration guide gives no stability statement.
- Unverified: documentation licence of help.penpot.app.
- Unverified: default sidebar widths and whether sidebars are resizable (not stated in retrieved pages).
- Unverified: exact on-screen order of Design-tab groups; the layers guide lists groups without stating order authoritatively.

## Mechanism

Workspace composition (from the interface tour):

```text
┌──────────────────────────────────────────────────────────────────────────┐
│ Main menu │ Toolbar: Board Rect Ellipse Text Image Path Curve │ View History│
│ (top-left)│ (top, horizontal)                                  │ Comments Zoom│
├───────────┬──────────────────────────────────────────┬────────────────────┤
│ Pages     │ Viewport (infinite canvas, rulers)       │ Design │ Prototype │
│ Layers    │                                          │ Inspect            │
│ (Alt L)   │                                          │ palettes, Assets   │
│           │                                          │ (Alt I), tokens    │
└───────────┴──────────────────────────────────────────┴────────────────────┘
Hide UI: \    Rulers: Ctrl Shift R
```

Design tab groups (module names in parentheses from `sidebar/options/menus`): measures (size, position, rotation, radius), align, constraints, layout_container / layout_item (flex and grid), layer (opacity, blend, visibility), fill, stroke, border_radius, shadow, blur, text / typography, svg_attrs, exports, interactions, component, color_selection (mixed selections), bool, frame_grid.

Automation surfaces:

1. In-editor: `penpot.selection`, `penpot.currentPage`, `penpot.root`, `create*`, `group/ungroup`, `on('shapechange' | 'selectionchange' | 'filechange' | 'contentsave')`, `shape.setPluginData / setSharedPluginData`, `shape.export()`, `generateMarkup/Style`; sandboxed in an iframe; permissions declared in the manifest.
2. External: `POST /api/rpc/command/<name>` with token auth (JSON or Transit), for example `get-file`; outbound webhooks at team level.
3. Source-level: ClojureScript UI modules per panel, usable as a reference for decomposing a properties panel into per-group components.

## NUIF relevance

**Borrow**
- The per-group module decomposition of the properties panel (`menus/*.cljs`) as a template for the Svelte shell's inspector components in `apps/editor/ARCHITECTURE.md`.
- Manifest-declared permissions (`content:read/write`, `library:read/write`) as a model for scoping automation clients.
- The event set (`shapechange`, `selectionchange`, `filechange`, `contentsave`) as a minimal change feed for replay capture.
- Namespaced shared plugin data per shape as further precedent for opaque extension preservation.

**Adapt**
- Turn the internal RPC command surface into a documented, versioned contract; NUIF's `spec/12-cli-api-and-automation.md` must be normative where Penpot's is implementation-defined.
- Keep the left/centre/right composition but adopt the bottom floating toolbar of UI3 for the test editor, since the toolbar position is the one deliberate divergence between the two references.
- Reuse Penpot's shortcut overlap with Figma (R, E, T, Shift 0/1/2, Ctrl G/K/D) to define a keymap that is familiar in both ecosystems, while resolving Penpot-specific divergences (E for ellipse versus Figma's O; B for board versus F).

**Reject**
- Comments, presence/users indicator, history and versions panels, view mode presentation, share/invite, design-tokens UI beyond what token tests need, Inspect-mode code generation (CSS/HTML/SVG snippets), plugin manager and marketplace, WebGL/WASM viewport variants. Reason: outside the testing/import/export scope or duplicating NUIF's own token and export machinery.

## Open questions

- Does Penpot publish an RPC method catalogue with stability guarantees, or must clients read `backend/src/app/rpc/commands/*`?
- Which of `Shift A` and `Ctrl A` is the current flex-layout binding, and does it vary by platform or version?
- Can `shape.export()` output be made deterministic for snapshot diffing, and does the exporter service affect this?
- Are Penpot sidebars resizable, and what are their default widths?
