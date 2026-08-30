# `nuif-react-jsx-0` profile

Status: experimental executable source profile. It is not a React runtime,
browser-layout or arbitrary-JSX equivalence claim.

## Purpose

This profile maps a small, statically decidable React JSX subtree to NUIF and
back while retaining source outside mapped scalar byte spans. Import never
executes JavaScript. The profile exists for generated components and controlled
developer tooling, not for interpreting an application.

The source must contain exactly one `export default function` with no
parameters and a body containing only a direct `return` of the marked JSX root.
Comments and unrelated module declarations outside that function are retained.

## Mapped source

- lowercase intrinsic `<div>` elements map to named NUIF containers;
- lowercase intrinsic `<span>` elements map to named NUIF literal text;
- every mapped element has literal double-quoted `data-nuif-id`,
  `data-nuif-kind` and `data-nuif-name` attributes;
- the root additionally has exact `data-nuif-profile` and
  `data-nuif-document` literals;
- container `style` is one object literal with numeric `width`, `height`,
  `gap` and per-edge padding, fixed `boxSizing: "border-box"`, fixed
  `display: "flex"`, literal `flexDirection` and literal `alignItems`;
- text `style` has fixed `width: "100%"`, the pinned literal `fontFamily`,
  numeric `fontSize` and a literal pixel `lineHeight`;
- text carries the exact font digest in `data-nuif-font-sha256` and one raw JSX
  text run using the profile's canonical entity escapes;
- whitespace between mapped container children is formatting, not a NUIF node.

React documents numeric style values as receiving property-specific unit
handling, so this profile uses numbers only for properties React interprets as
CSS pixels. It uses a `"<number>px"` string for `lineHeight`, where a number
would instead be unitless. Style keys follow React's camel-cased DOM property
vocabulary.

## Retentive synchronization law

Import records the exact UTF-8 byte range of every mapped identity, name,
layout scalar, font scalar and text run. Synchronization:

1. proves the retained spans still contain the values imported from the
   original document;
2. rejects entity insertion, deletion or child reordering;
3. renders before and after documents through the same exporter;
4. applies only changed mapped spans from the end of the source toward the
   beginning;
5. reparses the result and requires exact canonical NUIF equality.

All bytes outside the returned edits remain byte-identical. A changed comment,
unrelated import/export or other module-level user region is preserved. A
caller-modified mapped span fails as stale rather than being overwritten.

## Limits

| Resource | Limit |
| --- | ---: |
| UTF-8 source | 1 MiB |
| JavaScript/JSX syntax nodes | 16,384 |
| Mapped JSX element depth | 128 |

The common NUIF model limits apply after import. Limits are checked before
constructing the retained model, and every failure is atomic.

## Rejected constructs

The profile rejects component tags, fragments, self-closing mapped elements,
spreads, event handlers, computed or extra style properties, variables,
member access, calls, templates, arrays, conditions, loops, hooks, state,
context, `dangerouslySetInnerHTML`, nested markup in text and any expression
other than the one profile-owned style object containing literal values. It
also rejects TypeScript/TSX syntax; a later TSX profile needs its own grammar,
toolchain and fixtures.

Unmarked JSX elsewhere in a module is outside the extracted document. The
adapter preserves those bytes but makes no statement about their runtime
relationship to the marked component.

## Executable evidence

`cargo xtask gate-react` runs unit and release profile tests, 11 mapped scalar
edits, repeated synchronization, exact unchanged-byte-complement comparison,
typed stale/unsupported/structural failures and eleven hostile or excluded-source
trials. It writes:

- `target/react-sync-report.json`;
- `target/react-sync-output.jsx`;
- `target/react-sync-edited.nuif.json`.

Tree-sitter JavaScript 0.25.0 supplies concrete JSX syntax and byte ranges. It
is a syntax implementation, not a React renderer. The accepted syntax follows
the React documentation for JSX, intrinsic DOM props and literal style
objects; no foreign runtime comparison is claimed by this first profile.
