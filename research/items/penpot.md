---
id: nuif:research:penpot
kind: repository
status: reviewed
title: Penpot open design data model and file format
source:
  url: https://help.penpot.app/technical-guide/developer/data-model/
  authors: [Penpot]
  published_at: null
  license: MPL-2.0 for implementation
retrieved_at: 2026-08-29
tags: [design-editor, svg, shapes, portability]
confidence: 0.98
claims: [nuif:claim:authored-resolved]
relations: []
links:
  spec: [spec/01-model.md, spec/05-geometry-paint-text.md]
  adr: []
  rfc: []
  code: [adapters/README.md, adapters/STATUS.md]
  experiments: []
---
# Summary

Penpot v3 exports a ZIP package containing a manifest, JSON file data and
content-addressed or referenced binary objects. Pages and components contain
shape trees. Library assets include colors, typographies, components and design
tokens.

## Evidence

- The user guide identifies v3 as the current ZIP and JSON format. It lists
  `manifest.json`, per-file metadata, pages, colors, components, typographies,
  tokens, media and an `objects/` directory. The manifest records format
  version, producer version and feature flags.
  https://help.penpot.app/user-guide/export-import/penpot-file-format/
  (retrieved 2026-08-29).
- The technical file-format reference specifies the package paths and fields
  for page shapes, library assets, components, design-token sets/themes and
  storage objects. Data versioning is distinct from package format versioning.
  https://help.penpot.app/technical-guide/developer/data-model/penpot-file-format/
  (retrieved 2026-08-29).
- The conceptual data model defines pages and components as containers of
  hierarchical shape trees. Files can reference shared libraries.
  https://help.penpot.app/technical-guide/developer/data-model/#pages-and-components
  (retrieved 2026-08-29).
- The data guide states that shape fields are commonly optional, absence
  denotes default behavior, and import/export processes remove `null` fields.
  An adapter must therefore compare semantic defaults rather than JSON member
  presence alone. https://help.penpot.app/technical-guide/developer/data-guide/
  (retrieved 2026-08-29).

## NUIF relevance

**Borrow** the inspectable package boundary, explicit feature flags, versioned
data migrations, stable UUIDs and separation of JSON metadata from binary
objects.

**Adapt** one page and a bounded rectangle/ellipse/text/frame subset before
components or libraries. Package import must cap ZIP expansion, member count,
JSON depth and object bytes. Unrecognized members and fields require retentive
preservation.

**Reject** direct mapping of every Penpot shape to a universal NUIF shape.
Penpot constraints, grids, variants, interactions, text runs, paths, effects,
libraries and token themes exceed profile zero and need property-attributed
fidelity records.
