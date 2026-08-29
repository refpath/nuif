---
id: nuif:research:svelte-source-adapter-surface
kind: standard
status: reviewed
title: Svelte component AST and scoped-style source surface
source:
  url: https://svelte.dev/docs/svelte/svelte-compiler
  authors: [Svelte contributors]
  published_at: null
  license: MIT for Svelte implementation and documentation repository
retrieved_at: 2026-08-29
tags: [adapter, svelte, ast, source-correspondence, css]
confidence: 0.98
claims: [nuif:claim:sync-not-regenerate]
relations:
  - type: supports
    target: nuif:research:tree-sitter
links:
  spec: [spec/09-provenance-and-fidelity.md]
  adr: []
  rfc: []
  code: [adapters/STATUS.md]
  experiments: []
---
# Summary

The Svelte compiler exposes component and CSS parsers with source-offset AST
nodes. A component can contain markup, instance/module scripts and CSS. Template
expressions and blocks introduce runtime semantics. Component CSS is scoped by
compiler-generated selector classes.

## Evidence

- `svelte/compiler` exposes `compile`, `parse`, `parseCss`, `preprocess` and
  `print`. Modern AST nodes contain `start` and `end` offsets. The root separates
  markup, CSS, instance script and module script.
  https://svelte.dev/docs/svelte/svelte-compiler#parse (retrieved 2026-08-29).
- `print` emits valid Svelte plus a source map but may change whitespace and
  quoting. It is therefore not an edit-locality mechanism for retained source.
  https://svelte.dev/docs/svelte/svelte-compiler#print (retrieved 2026-08-29).
- The AST distinguishes static text, expression tags, HTML tags, regular
  elements, components, blocks and directives. This supplies a structural
  boundary between literals and executable expressions.
  https://svelte.dev/docs/svelte/svelte-compiler#AST (retrieved 2026-08-29).
- Component CSS is scoped by default through a hash-derived class added to
  affected elements and selectors. Scoped keyframe names are also rewritten.
  https://svelte.dev/docs/svelte/scoped-styles (retrieved 2026-08-29).

## NUIF relevance

**Borrow** compiler AST offsets and literal-node categories for correspondence.

**Adapt** a static subset of regular elements, literal attributes, literal text
and profile-owned CSS declarations. Edits replace original spans rather than
printing the AST. Compiler version and modern-AST mode are part of provenance.

**Reject** automatic semantic lifting of runes, scripts, expressions, snippets,
blocks, directives, actions, transitions, dynamic components or preprocessors.
They are programs whose behavior depends on inputs and runtime state.
