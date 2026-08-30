# `nuif-svelte-static-0` profile

Status: experimental executable source profile. It is not a Svelte runtime,
browser-layout or arbitrary-component equivalence claim.

## Purpose

This profile maps one statically decidable `.svelte` component to NUIF and back
while retaining comments, whitespace and formatting outside mapped scalar byte
spans. Import never executes JavaScript. The profile is intended for generated
components and controlled developer tooling, not for interpreting an
application.

The component contains exactly one marked top-level regular element. Top-level
whitespace and HTML comments are retained. Any other top-level markup, script,
style or special node is rejected because it can render or execute alongside
the marked root.

## Mapped source

- regular `<div>` elements map to named NUIF containers;
- regular `<span>` elements map to named NUIF literal text;
- every mapped element has literal double-quoted `data-nuif-id`,
  `data-nuif-kind` and `data-nuif-name` attributes;
- the root additionally has exact `data-nuif-profile` and
  `data-nuif-document` literals;
- container `style` is one literal declaration list with pixel `width`,
  `height`, `gap` and per-edge padding, plus fixed `box-sizing: border-box`,
  fixed `display: flex`, literal `flex-direction` and literal `align-items`;
- text `style` has fixed `width: 100%`, the pinned literal `font-family`, and
  pixel `font-size` and `line-height`;
- text carries the exact font digest in `data-nuif-font-sha256` and one literal
  text run using the profile's canonical HTML entity escapes;
- whitespace and comments between mapped container children are formatting,
  not NUIF nodes.

Inline style is intentional. It makes scalar ownership unambiguous and avoids
claiming CSS cascade, selector, specificity or Svelte scope-hash semantics. A
component-CSS profile requires its own version and conformance suite.

## Retentive synchronization law

Import records the exact UTF-8 byte range of every mapped identity, name,
layout scalar, font scalar and text run. Synchronization:

1. proves the retained spans still contain the values imported from the
   original document;
2. rejects entity insertion, deletion or child reordering;
3. renders before and after documents through the same exporter;
4. compares all three correspondence inventories through the shared
   `nuif-adapter` scalar planner;
5. applies only changed mapped spans from the end of the source;
6. reparses the result and requires exact canonical NUIF equality.

All bytes outside the returned edits remain byte-identical. A caller-modified
mapped span fails as stale rather than being overwritten. Every failure is
atomic.

## Limits

| Resource | Limit |
| --- | ---: |
| UTF-8 source | 1 MiB |
| Svelte syntax nodes | 16,384 |
| Mapped element depth | 128 |

The common NUIF model limits apply after import.

## Rejected constructs

The profile rejects components, special elements, self-closing mapped
elements, scripts, module scripts, component CSS, preprocessors, expressions,
blocks, snippets, render/raw tags, spreads, shorthand attributes, directives,
events, bindings, actions, transitions, animations, class/style directives,
extra properties, nested markup in text and noncanonical entity escapes.

## Executable evidence

`cargo xtask gate-svelte` runs unit and release profile tests, 11 mapped scalar
edits, repeated synchronization, exact unchanged-byte-complement comparison,
typed stale/unsupported/structural failures, 13 hostile or excluded-source
trials and a public CLI export/import/sync bridge. It then installs the exact
lockfile with lifecycle scripts disabled and parses and compiles both direct and
CLI synchronized sources through official `svelte/compiler` 5.57.0 in modern
AST mode. Compiler warnings fail the gate.

The gate writes:

- `target/svelte-sync-report.json`;
- `target/svelte-sync-output.svelte`;
- `target/svelte-sync-edited.nuif.json`;
- `target/svelte-sync-cli-report.json`;
- `target/svelte-sync-cli-output.svelte`;
- `target/svelte-compiler-oracle-report.json`.

Tree-sitter Svelte supplies concrete syntax and byte ranges. The official
compiler is the separate semantic oracle. Neither is treated as a Svelte
renderer, and no runtime pixel-equivalence claim is made.
