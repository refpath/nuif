# Retentive HTML/CSS v0 model profile

Status: executable research profile (`nuif-html-css-v0`). It carries the complete NUIF v0 responsive-card model through HTML/CSS and applies scalar semantic changes as byte-local source edits. It is not an arbitrary-HTML importer or a claim that browsers render every preserved NUIF kind.

## Model projection

- document identity, relations, extension declarations and document extensions are quoted JSON attributes on the marked `html` element;
- token identity is part of a CSS custom-property name, while safe token names and finite real or single-atom string values have retained source spans;
- every entity is an identity-bearing `div`; DOM nesting is the canonical roots/children order;
- kind, optional name, authored property values, semantics and opaque extensions are quoted JSON attributes;
- width/height intent, position, layout family/direction/gap/padding/alignment and sRGB fill are real CSS declarations;
- text content is escaped element text and its pinned font metadata is a quoted JSON attribute;
- width-conditioned direction/gap overrides have both a mapped JSON rule and a marked `@media` block. Import requires the rendered block to equal the rule, preventing derived CSS from drifting silently.

The responsive mapping accepts `min_width` and/or `max_width`, direction and gap. Theme predicates and responsive width/height overrides are outside this profile. Token names use ASCII letters, digits, `.`, `_` and `-`; string token values must be safe single CSS atoms. Input is UTF-8 and capped at 1 MiB.

All model fields used by `v0-responsive-card` round-trip exactly. Shape kind, path identity, component identity/reference, unknown-kind payloads and extension bytes survive, but target fidelity remains explicit:

- paths have no authored geometry in v0 and are not rendered by this adapter;
- instances retain their component reference but are not materialized into browser DOM;
- unknown kinds and opaque extensions are `preserved_unrenderable`.

These target limitations coexist with lossless source correspondences. A field may be stored exactly while still lacking browser behavior.

## Retentive laws

For documents inside the profile:

1. `import_v0_source(export_v0_document(document).source).document == document`.
2. Every editable scalar has a unique half-open UTF-8 byte span. Before synchronization, its retained bytes must equal the imported value's profile encoding.
3. Changed spans are replaced from the end of the source toward the beginning. No formatter or whole-file generator touches the retained source.
4. The result is reparsed as HTML and CSS and must import to the requested edited document exactly.
5. A repeated synchronization from the same retained source and edited document produces the same source and edit list.
6. Root/entity insertion, removal or reorder; mapped-property set changes; stale values; unsupported profile values; inconsistent derived CSS; malformed syntax; and oversized input return typed errors without partial output.

Unmarked comments, elements, rules and declarations are retained byte-for-byte and remain outside the NUIF semantic projection. Such CSS can still affect a browser through the cascade; therefore exact NUIF round-trip does not imply browser-render equivalence after arbitrary unmapped CSS is inserted.

## Automated evidence

`cargo xtask gate-f-v0` runs two paths:

- the model trial exports the eight-entity responsive card, injects unmapped CSS/HTML, changes a token, four padding edges, escaped text and one responsive rule, then requires eight exact span edits, exact re-import, repeat determinism, byte identity everywhere else and exact opaque payload survival;
- the editor bridge generates the fixture through the CLI, edits name and width through the headless semantic editor, synchronizes through the CLI, imports through the CLI and requires canonical NUIF byte identity with the editor output.

Negative trials cover unsupported token values, structural edits, ordinary stale spans, derived-responsive CSS drift and the one-over source limit. The report also requires property-attributed fidelity for path, instance, unknown-kind and extension target limitations.

Artifacts:

- `target/html-sync-v0-report.json` and `target/html-sync-v0-output.html`;
- `target/html-sync-v0-editor-report.json` and `target/html-sync-v0-editor-output.html`.

CLI:

- `nuif export <input.nuif> html-css-v0 <output.html> [report.json]`
- `nuif import html-css-v0 <input.html> <output.nuif> [report.json]`
- `nuif sync html-css-v0 <retained.html> <edited.nuif> <output.html> [report.json]`

The smaller [`nuif-html-css-0`](PROFILE.md) profile remains as an independently exercised mechanism proof with a deliberately narrow rejection boundary.
