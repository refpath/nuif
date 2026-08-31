# Figma plug-in snapshot profile 0

Profile identifier: `nuif-figma-plugin-snapshot-0`.

Status: executable pure mapping plus a compiled no-network review shell. No
live Figma runtime, marketplace package or vendor interoperability claim is
included.

The profile is the deterministic boundary between a thin Figma main-thread
shell and the NUIF engine. The shell normalizes public Plugin API objects into
the JSON snapshot defined by `nuif-figma::PluginSnapshot`; the Rust mapper
converts that snapshot to canonical NUIF. The reverse direction produces a
`PluginMutationPlan` tree for a shell to apply inside one user-initiated run.

## Exact subset

- one selected `FRAME` as the NUIF surface root;
- nested `FRAME` and `GROUP` containers;
- `RECTANGLE`, `ELLIPSE` and `TEXT` leaves;
- ordered containment, finite relative position and finite fixed dimensions of
  at least `0.01`, matching the Figma resize contract;
- one optional solid sRGB fill;
- freeform frames or packed horizontal/vertical auto layout with finite
  non-negative gap/padding and MIN/CENTER/MAX counter-axis alignment;
- visible nodes with node opacity `1`;
- literal text carrying the exact pinned Ahem Regular SHA-256 identity, positive
  font size and positive pixel line height;
- optional portable document/entity identifiers plus deterministic repair.

Figma GRID auto layout, wrapping, SPACE_BETWEEN, fill/hug sizing, constraints,
components and instances, variables, mixed text, strokes, effects, blend modes,
rotation, clipping, masks, interactions, images and vector networks are outside
this subset. The snapshot shell must list every active excluded property in
`unsupported_properties`. Hidden or partially transparent nodes produce an
unsupported appearance entry because the current NUIF model has no first-class
general visibility or node-opacity field. They are never treated as exact.

## Identity

`nuif_document_id` and `nuif_entity_id` accept canonical 128-bit NUIF identity
strings. A valid unique value is retained exactly. A missing, malformed or
duplicate entity value is replaced by SHA-256 domain-separated derivation over
the profile, host document, page and object IDs. Collision retries add a
big-endian nonce. The same snapshot therefore produces the same repaired IDs;
the report classifies repaired identity as `representable`, not `lossless`.

Host object IDs must be non-empty and unique in one snapshot. The report binds
every mapped entity/property to its host object. Root frame page coordinates
are normalized to the surface origin; non-zero root coordinates are
`representable` rather than lossless.

## Limits and failure

- 16 MiB encoded snapshot;
- 16,384 nodes;
- 64 containment levels;
- 4,096 UTF-16 code units per text node;
- 256 KiB combined normalized string payload.

Malformed JSON, duplicate host IDs, non-finite geometry, invalid colours,
dimensions below `0.01`, negative spacing, non-default leaf layout, children on leaves and
limit-plus-one inputs fail before returning a document or plan. Export rejects
NUIF properties outside the subset with a property-attributed
`HostAdapterReport`.

## Gate

`cargo xtask gate-figma` runs the release-mode mapper, CLI bridge, strict
TypeScript check, deterministic shell build and a mock Plugin API fixture
through the Rust importer. It requires repeated snapshot bytes, exact canonical
round trip, explicit loss, duplicate-ID and limit-plus-one rejection, and a
compiled manifest-template bundle with no network domains. The reports are
`target/figma-snapshot-report.json` and
`target/figma-plugin-shell-report.json`.

The shell is `adapters/figma/plugin`. Figma assigns its manifest ID, so CI does
not invent one. Live promotion still requires a Figma product/version record,
host mutation/undo/cancellation trials, page-load and identity persistence
evidence, and a human-confirmed import preview.
