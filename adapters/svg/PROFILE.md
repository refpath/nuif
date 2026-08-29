# Retentive SVG basic-shape profile zero

Status: executable crate profile (`nuif-svg-0`). CLI, `xtask` and CI integration
remain required before the repository status matrix lists the profile as an
integrated adapter.

## Model projection

- one positive, fixed-size surface maps to the root SVG element and its
  `viewBox`;
- freeform containers map to `g` elements;
- fixed-size, positioned rectangles map to `rect` geometry;
- fixed-size, positioned ellipses retain NUIF bounding-box attributes and map
  derived centre/radius geometry to `ellipse` attributes;
- fixed-size, positioned text maps to one literal SVG text node with the pinned
  font name, content hash, font size and line height;
- absent fill maps to `none`; present fill is opaque sRGB with 8-bit-exact
  channels and maps to lowercase six-digit hexadecimal notation;
- entity name, role and accessible name map to `data-nuif-name`, `role` and
  `aria-label`;
- `data-nuif-document`, `data-nuif-id`, `data-nuif-kind` and
  `data-nuif-profile` retain canonical identities and the profile marker.

The profile excludes tokens, relations, extensions, responsive rules, layout
families other than default freeform, property values, semantic states, paths,
images, components, instances and unknown kinds. Paths, transforms, CSS,
strokes, gradients, patterns, clipping, masks, filters, animation, scripts,
external resources, text spans and per-character positioning require separate
profiles.

## Retentive laws

For a document inside the profile:

1. `import_source(export_document(document).source).document == document`.
2. Each mapped scalar has a UTF-8 byte span. Derived ellipse geometry uses
   multiple ordered correspondence records against the authored geometry.
3. Synchronization compares every retained scalar with a canonical export of
   the imported document before applying any edit.
4. Changed spans are replaced from the highest byte offset to the lowest.
5. The synchronized source is re-imported and must equal the requested edited
   document exactly.
6. Repeated synchronization from the same retained source and edited document
   produces identical source, edit records and reports.
7. Containment, order, kind, optional mapped-property inventory and unsupported
   semantic changes return typed errors without partial output.

Unmarked elements, attributes, comments and processing instructions are outside
the semantic projection and remain byte-identical during synchronization. They
can affect SVG rendering. Exact NUIF round trips therefore do not imply visual
equivalence after arbitrary unmarked SVG is inserted.

## Resource and parser contract

Input is UTF-8 and limited to 1 MiB. `roxmltree` 0.21.1 parses at most 16,384
nodes with DTD parsing disabled and no external entity resolver. Mapped elements
must use the SVG namespace and be direct children of mapped surface/container
elements. Duplicate identities, inconsistent kind/tag pairs, non-canonical
numbers, non-canonical colors and inconsistent derived ellipse geometry fail
import.

The crate tests cover exact export/import, deterministic multi-property
synchronization, escaped text, unmarked-source locality, stale spans,
structural rejection, path fidelity, DTD rejection and the source byte limit.
