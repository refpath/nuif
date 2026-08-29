---
id: nuif:research:figma
kind: standard
status: reviewed
title: Figma public plugin/document node model
source:
  url: https://developers.figma.com/docs/plugins/api/nodes/
  authors: [Figma]
  published_at: null
  license: proprietary API documentation
retrieved_at: 2026-08-29
tags: [adapter, scene-graph, components, variables, layout]
confidence: 0.97
claims: []
relations: []
links:
  spec: []
  adr: []
  rfc: []
  code: [adapters/README.md, adapters/STATUS.md]
  experiments: []
---
# Summary

Figma exposes document content through a versioned REST response and an
in-editor plugin API. The public model includes document/canvas containment,
frames, components, instances, vectors, text, auto layout, grid layout, paints,
styles and variables. The public contract is an API model, not the `.fig` file
encoding.

## Evidence

- `GET /v1/files/:key` returns a document node, component/style maps, schema
  version and file version. `ids`, `depth`, `geometry=paths` and `plugin_data`
  control projection. The endpoint requires `file_content:read` and is subject
  to plan-dependent rate limits.
  https://developers.figma.com/docs/rest-api/file-endpoints/#get-file and
  https://developers.figma.com/docs/rest-api/rate-limits/ (retrieved
  2026-08-29).
- The REST node catalogue distinguishes `DOCUMENT`, `CANVAS`, `FRAME`,
  `COMPONENT`, `INSTANCE`, basic shapes, vectors and text. Properties are
  conditional on node type.
  https://developers.figma.com/docs/rest-api/file-node-types/ (retrieved
  2026-08-29).
- The official plugin typings define stable node IDs, ordered children,
  geometry, paints, auto-layout sizing/alignment/padding/gap, grid tracks and
  variable bindings. The plugin API is the writable document surface.
  https://github.com/figma/plugin-typings/blob/master/plugin-api-standalone.d.ts
  (`BaseNodeMixin`, `ChildrenMixin`, `LayoutMixin`, `AutoLayoutMixin`,
  `GridLayoutMixin`, `GeometryMixin`; retrieved 2026-08-29).
- `setPluginData` stores plugin-private string data with a 100 kB limit per
  entry. REST retrieval includes only requested plugin IDs or shared data.
  https://developers.figma.com/docs/plugins/api/properties/nodes-setplugindata/
  (retrieved 2026-08-29).
- The Variables API represents boolean, float, string and color values by mode;
  local variable read/write scopes can be plan-restricted.
  https://developers.figma.com/docs/rest-api/variables-endpoints/ (retrieved
  2026-08-29).

## NUIF relevance

**Borrow** stable foreign node IDs, explicit node kinds, ordered containment,
component references, auto-layout fields, variable bindings and plugin data for
correspondence metadata.

**Adapt** REST JSON as a bounded read fixture and use a plugin companion for
writes. Every report must record file version, schema version, requested depth,
geometry mode and omitted plugin-data namespaces. A first profile can map one
page with frames, rectangles, ellipses and pinned text.

**Reject** undocumented `.fig` or multiplayer protocols as normative
dependencies. REST access is authenticated, rate-limited and primarily a read
surface; a credential-free bidirectional conformance gate requires checked-in
API fixtures and a separately tested plugin bridge.
