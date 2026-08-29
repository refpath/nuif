---
id: nuif:research:figma-plugin-and-rest-api-as-automation-surface
kind: standard
status: reviewed
title: Figma Plugin API and REST API as evidence for a programmable, testable editor surface
source:
  url: https://developers.figma.com/docs/plugins/api/figma/
  authors: [Figma]
  published_at: "unknown"
  license: proprietary API documentation (Figma); facts only recorded here
retrieved_at: 2026-08-29
tags: [figma, plugin-api, rest-api, automation, headless, plugin-data, variables, export, mcp]
confidence: 0.9
claims: [nuif:claim:semantic-automation, nuif:claim:opaque-preservation]
relations:
  - type: extends
    target: nuif:research:figma
    note: Adds the call surface and execution model to the node-model record.
  - type: related_to
    target: nuif:research:openpencil
    note: OpenPencil exposes CLI/RPC/MCP headlessly; Figma does not offer headless plugin execution.
  - type: related_to
    target: nuif:research:openfig-fig-kiwi
    note: The public APIs are the stable alternative to the reverse-engineered file format.
  - type: related_to
    target: nuif:research:figma-ui3-editor-layout
    note: Every Design-tab section corresponds to node properties reachable through this API.
  - type: supports
    target: nuif:claim:semantic-automation
    note: Demonstrates a production editor whose document is fully scriptable without pointer input.
  - type: supports
    target: nuif:claim:opaque-preservation
    note: pluginData / sharedPluginData are opaque per-node string stores that survive in the file and the REST export.
links:
  spec: [spec/12-cli-api-and-automation.md, spec/07-extensions-and-dialects.md]
  adr: []
  rfc: [rfcs/0004-headless-qa-contract.md, rfcs/0002-extension-preservation.md]
  code: [apps/editor/ARCHITECTURE.md, apps/editor/QA.md, adapters/README.md]
  experiments: []
---

# Summary

Figma exposes two programmatic surfaces. The Plugin API runs inside an open editor session: a sandboxed JavaScript main thread manipulates the document through a global `figma` object (node creation, selection, properties, events, export, undo grouping, per-node plugin data), while an optional iframe hosts UI and browser APIs. The REST API runs outside the editor without a user present; it is "largely read-only" for design content, returning the node tree as JSON, rendering nodes to images, and, on Enterprise plans, reading and writing variables. Figma's documentation states that plugins cannot run in the background, that a user must initiate them, and that only one plugin runs at a time; no headless plugin execution is offered. A newer MCP server (remote or desktop) allows agents to read design context and create native content, but only through catalogued clients.

NUIF interpretation: Figma proves that a design editor's entire semantic surface can be exercised without pointer input, which is the premise of `apps/editor/QA.md`. Its execution model also shows the gap NUIF must close: the programmable surface is bound to a running GUI session, so headless conformance testing is impossible against Figma itself.

## Evidence

Retrieval date for all locators: 2026-08-29.

- `figma.createFrame(): FrameNode` — "similar to using the F shortcut followed by a click"; the frame defaults to 100×100 with a white background and is parented to `figma.currentPage`. https://developers.figma.com/docs/plugins/api/properties/figma-createframe/.
- The global object exposes `currentPage: PageNode` (settable), `root: DocumentNode`, `editorType` ('figma' | 'figjam' | 'dev' | 'slides' | 'buzz'), `mode` ('default' | 'textreview' | 'inspect' | 'codegen' | 'linkpreview' | 'auth'), `create*` constructors (Frame, Rectangle, Ellipse, Polygon, Star, Text, Component, Page, Section), `getNodeByIdAsync`, `loadAllPagesAsync`, `on/off/once`, `commitUndo`, `triggerUndo`, `notify`, `closePlugin`, `viewport`, `ui`, `clientStorage`, `variables`, `teamLibrary`, `skipInvisibleInstanceChildren`. https://developers.figma.com/docs/plugins/api/figma/.
- `PageNode.selection: ReadonlyArray<SceneNode>`; "Each page stores its own selection separately"; order unspecified; `selectedTextRange`; `loadAsync()` required under dynamic page loading. https://developers.figma.com/docs/plugins/api/PageNode/. Whether assignment to `selection` is permitted was not confirmed in the retrieved text (unverified; the older URL /properties/figma-currentpage/ returns 404).
- `setPluginData(key: string, value: string): void` — entry (pluginId, key, value) limited to 100 kB; private to the plugin ID; privacy is "for stability, not security"; empty string deletes the key. https://developers.figma.com/docs/plugins/api/properties/nodes-setplugindata/.
- `setSharedPluginData(namespace: string, key: string, value: string): void` — readable by all plugins; namespace at least 3 alphanumeric characters; 100 kB limit; `getSharedPluginDataKeys` enumerates a namespace. https://developers.figma.com/docs/plugins/api/properties/nodes-setsharedplugindata/.
- REST `GET /v1/files/:key` query `plugin_data` accepts "Comma separated list of plugin IDs and/or the string shared" and adds `pluginData` and `sharedPluginData` to nodes in the response; other parameters `version`, `ids`, `depth`, `geometry=paths`, `branch_data`; response includes `document`, `components`, `componentSets`, `styles`, `schemaVersion`, `version`. Tier 1, scope `file_content:read`. https://developers.figma.com/docs/rest-api/file-endpoints/.
- REST `GET /v1/files/:key/nodes` (ids, version, depth, geometry, plugin_data); `GET /v1/images/:key` renders nodes with `scale` 0.01–4, `format` jpg/png/svg/pdf, `svg_outline_text`, `svg_include_id`, `svg_include_node_id`, `svg_simplify_stroke`, `contents_only`, `use_absolute_bounds`, `version`; `GET /v1/files/:key/images` returns image-fill URLs expiring within 14 days. Same page.
- The REST API is "Largely read-only" except comments, comment reactions, variables and dev resources; it operates where "a user does not need to be present"; the Plugin API requires that "A user has a particular Figma design or FigJam file open" and can "only read and edit the current file that a user has open". https://developers.figma.com/compare-apis/.
- Plugin execution: main thread runs in a sandbox with ES2020+ but without `fetch`, `XMLHttpRequest`, `setTimeout` or the DOM; UI runs in an iframe created by `figma.showUI()`; the two communicate by message passing; a plugin that never calls `figma.closePlugin()` "runs indefinitely" with a "Running" toast. https://developers.figma.com/docs/plugins/how-plugins-run/.
- "It's not possible to build plugins that run in the background"; users run one plugin at a time; actions are "initiated by the user". https://developers.figma.com/docs/plugins/ — introduction.
- Dev Mode plugins (`editorType: ["dev"]`, capabilities `inspect` / `codegen`) are read-only; "setter methods in the Plugin API do not work in Dev Mode" except metadata such as `pluginData` and `relaunchData`; pages are always dynamically loaded. https://developers.figma.com/docs/plugins/working-in-dev-mode/.
- `figma.on` events: selectionchange, currentpagechange, documentchange, close, run, drop, timer events, stylechange, textreview. `documentchange` requires `"documentAccess": "dynamic-page"` in the manifest and a prior `figma.loadAllPagesAsync()`; Figma "will not call the 'documentchange' callback synchronously and will instead batch the updates". https://developers.figma.com/docs/plugins/api/properties/figma-on/.
- `exportAsync` overloads: `(settings?: ExportSettings): Promise<Uint8Array>` for PNG/JPG/PDF/SVG bytes; `(ExportSettingsSVGString): Promise<string>`; `(ExportSettingsREST): Promise<Object>` returning `JSON_REST_V1`, the REST-compatible node JSON; MP4/GIF/WEBM for animated top-level frames; default PNG at 1x. https://developers.figma.com/docs/plugins/api/properties/nodes-exportasync/.
- `figma.skipInvisibleInstanceChildren: boolean` — default `true` in Dev Mode, `false` in Figma and FigJam; when enabled, `children`, `findAll`, `findOne`, `findAllWithCriteria` skip invisible instance descendants and `getNodeByIdAsync` returns null for them; `findAll`/`findOne` become "up to several times faster" and `findAllWithCriteria` "up to hundreds of times faster in large documents". https://developers.figma.com/docs/plugins/api/properties/figma-skipinvisibleinstancechildren/.
- Variables REST: `GET .../variables/local` and `GET .../variables/published` (scope `file_variables:read`), `POST .../variables` (scope `file_variables:write`, edit permission, 4 MB body, up to 5,000 variables per collection, 40 modes per collection); all require an Enterprise organisation. https://developers.figma.com/docs/rest-api/variables-endpoints/.
- REST authentication is by personal access token or OAuth2; base URL `https://api.figma.com`. https://developers.figma.com/docs/rest-api/.
- MCP server: remote (Figma-hosted) or desktop-app server; agents can read variables, components, layout and design context, generate code, and "create and modify native Figma content directly"; only clients in the Figma MCP Catalog can connect. https://developers.figma.com/docs/figma-mcp-server/.
- Headless execution: no Figma developer page retrieved offers headless or CLI plugin execution. A community feature request confirms plugins cannot auto-run headlessly (secondary source). https://forum.figma.com/suggest-a-feature-11/are-headless-auto-start-figma-plugins-possible-39156.
- Unverified: whether `pluginData` survives copy/paste and duplication of nodes; the setPluginData page does not state it.
- Unverified: whether `figma.currentPage.selection` is assignable (commonly used in plugin code, but not confirmed in the retrieved text).
- Unverified: MCP server plan tiers and write-capability details beyond the quoted summary.

## Mechanism

Call surface relevant to a programmable editor, grouped by the QA capabilities in `apps/editor/QA.md`:

1. Create/open: `figma.root`, `figma.currentPage`, `figma.createFrame()` and siblings, `figma.createPage()`, `loadAllPagesAsync()`; REST `GET /v1/files/:key` for out-of-editor read access.
2. Query: `node.findAll`, `findOne`, `findAllWithCriteria`, `getNodeByIdAsync`, `skipInvisibleInstanceChildren` as a traversal filter; REST `ids`, `depth`, `geometry=paths`.
3. Transact: property setters on nodes, `commitUndo()` to group actions into an undo step, `triggerUndo()`; `figma.on('documentchange')` as a batched change feed.
4. Selection as state: `PageNode.selection`, `selectionchange` event.
5. Opaque data: `setPluginData` (private, keyed by plugin ID) and `setSharedPluginData` (namespaced, public), each ≤100 kB per entry, surfaced by REST via `plugin_data=<id>|shared`.
6. Render: `exportAsync` (PNG, JPG, SVG, PDF, SVG string, JSON_REST_V1, video); REST `GET /v1/images/:key`.
7. Tokens: `figma.variables` in-editor; REST variables endpoints (Enterprise).
8. Execution boundary: plugin main thread inside an open file, initiated by a user; REST outside the editor, read-mostly; MCP server as an agent bridge with catalogued clients.

## NUIF relevance

**Borrow**
- The principle that every inspector control has an API-level property and event, so that tests drive the document through operations rather than pointer input (`nuif:claim:semantic-automation`).
- Batched, asynchronous change notification (`documentchange`) as the model for the editor's event log and replay capture in `rfcs/0004-headless-qa-contract.md`.
- Undo grouping via an explicit commit (`commitUndo`) as the pattern for NUIF transactions with inverse logs.
- A traversal flag that skips invisible instance descendants as a documented performance lever for query evaluation in large documents.
- Opaque, size-bounded, namespaced per-node string stores that survive file save and external export, as precedent for extension preservation (`rfcs/0002-extension-preservation.md`).

**Adapt**
- Replace the plugin-ID-keyed private store with NUIF's dialect/extension namespaces so preserved data is portable rather than tied to a vendor plugin identity (`spec/07-extensions-and-dialects.md`).
- Make the export-to-JSON path (`JSON_REST_V1`) the canonical serialisation rather than a secondary export, because NUIF's canonical document is the neutral format itself.
- Provide the same operation surface in-process and over a local endpoint so the editor is not required to be open (contrast with Figma's editor-bound plugin runtime).

**Reject**
- Editor-bound plugin execution with no headless mode; single-plugin-at-a-time and user-initiated constraints; Enterprise-gated variables API; MCP access restricted to a client catalogue; comments and dev-resources write endpoints; FigJam/Slides/Buzz editor types. Reason: the NUIF test editor must be scriptable headlessly and without plan or client gating.

## Open questions

- Does Figma document plugin-data persistence across copy, paste, duplicate and component instantiation? Needed to compare with NUIF's preservation guarantees.
- Is there an official statement on determinism of `exportAsync` output (identical bytes for identical documents)? Relevant to snapshot comparison.
- Can the MCP server's write path be characterised as a semantic operation API, and does it expose undo grouping or change events?
