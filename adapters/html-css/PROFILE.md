# Retentive HTML/CSS profile 0

Status: executable bounded research profile (`nuif-html-css-0`). This profile proves a source-preserving synchronization mechanism; it is not a claim that arbitrary HTML/CSS or the complete NUIF v0 responsive-card fixture is representable.

## Representable NUIF subset

- one document root and no relations, extensions or extension declarations;
- finite real-valued length tokens whose names contain only ASCII letters, digits, `.`, `_` or `-`;
- container and text entities only;
- containers use fixed pixel width/height, stack layout, row/column flow, finite gap and four padding edges, start/center/end/stretch alignment and one `token.spacing` binding;
- text uses fill width, intrinsic height, literal content and pinned font name/hash/size/line-height; all other text-authored state is default;
- containment is represented by DOM nesting and identity by 32-digit `data-nuif-id` values.

Every condition outside this list fails export or synchronization with a `Fidelity::Unsupported` item carrying a document/entity/token target and JSON pointer. The adapter never silently emits a fallback and calls it lossless.

## Source contract

The HTML document declares `data-nuif-profile="nuif-html-css-0"` and `data-nuif-document`. Mapped entity elements declare `data-nuif-id`, `data-nuif-kind` and their profile fields. A single `style[data-nuif-styles]` block contains real CSS declarations for token custom properties, fixed sizing and stack layout.

Tree-sitter 0.26.10 validates the outer HTML with `tree-sitter-html` 0.23.2 and the injected stylesheet with `tree-sitter-css` 0.25.0. Import retains byte ranges for every editable scalar. Input is UTF-8 and capped at 1 MiB before parsing.

Unmarked comments, elements and CSS declarations are outside the semantic projection. Import ignores them and synchronization preserves them byte-for-byte. They are not promoted to NUIF entities and no semantic claim is made about them.

## Laws and edit algorithm

For documents in the declared subset:

1. `import(export(document)).document == document` exactly.
2. An unchanged document produces no source edits.
3. A mapped edit replaces only its recorded byte span. Replacements are applied from the highest byte offset to the lowest, so earlier spans remain valid.
4. Before replacement, each retained span must still equal the profile encoding of the imported value; otherwise synchronization returns `StaleSpan` without partial output.
5. The synchronized source is re-imported and must equal the edited document exactly before it is returned.
6. Structural changes, added/removed mapped properties and semantics outside the subset return `UnmappedChanges` with property-level fidelity.

Gate F changes one token value, four padding edges and escaped text. Its machine report proves exact re-import, repeat-identical edits/output and byte identity of every region outside those six spans while preserving inserted HTML/CSS comments and an unmapped element.

## Automation

- `nuif export <input.nuif> html-css-0 <output.html> [report.json]`
- `nuif import html-css-0 <input.html> <output.nuif> [report.json]`
- `nuif sync html-css-0 <retained.html> <edited.nuif> <output.html> [report.json]`
- `cargo xtask gate-f` writes `target/html-sync-report.json` and `target/html-sync-output.html`.

The full v0 responsive card still requires surface/component/instance/shape/unknown-kind, responsive-rule and opaque-extension mappings. Those remain explicit next-profile work rather than hidden metadata.
