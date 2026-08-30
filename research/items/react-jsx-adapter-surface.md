---
id: nuif:research:react-jsx-adapter-surface
kind: standard
status: verified
title: React JSX and DOM element source surface
source:
  url: https://react.dev/learn/writing-markup-with-jsx
  authors: [React team]
  published_at: null
  license: CC-BY-4.0 for React documentation
retrieved_at: 2026-08-30
tags: [adapter, react, jsx, source-correspondence, dom]
confidence: 0.98
claims: [nuif:claim:sync-not-regenerate]
relations:
  - type: supports
    target: nuif:research:tree-sitter
links:
  spec: [spec/09-provenance-and-fidelity.md]
  adr: []
  rfc: []
  code: [adapters/STATUS.md, adapters/react/PROFILE.md, crates/nuif-react/src/lib.rs]
  experiments: [nuif:experiment:react-jsx-retentive-sync]
---
# Summary

JSX is a JavaScript syntax extension whose elements lower to immutable React
element objects. DOM components use React's DOM property vocabulary. JSX
expressions, component calls and control flow make arbitrary source a program
rather than a declarative document.

## Evidence

- The JSX guide requires closed tags, one enclosing returned root and
  camel-cased property names for many DOM properties. JavaScript expressions
  are embedded with braces.
  https://react.dev/learn/writing-markup-with-jsx (retrieved 2026-08-29).
- `createElement(type, props, ...children)` accepts intrinsic tag strings,
  component types and heterogeneous React children. `key` and `ref` are special
  fields, and returned elements and props are immutable.
  https://react.dev/reference/react/createElement (retrieved 2026-08-29).
- Common DOM components accept `aria-*` and `data-*` attributes, event handlers
  and a `style` object. The `style` keys use camel-cased CSS property names and
  numeric values receive property-dependent unit handling.
  https://react.dev/reference/react-dom/components/common (retrieved
  2026-08-30).
- React describes inline `style` as a JavaScript object inside the JSX
  expression, recommends classes for static styles, and shows that quoted JSX
  attributes are literal strings while braces admit arbitrary expressions.
  https://react.dev/learn/javascript-in-jsx-with-curly-braces (retrieved
  2026-08-30).
- Tree-sitter JavaScript 0.25.0 includes JavaScript and JSX in one grammar. Its
  Rust `LANGUAGE` constant is compatible with the workspace Tree-sitter API;
  syntax nodes expose byte offsets through Tree-sitter's standard interface.
  https://github.com/tree-sitter/tree-sitter-javascript and
  https://docs.rs/tree-sitter-javascript/0.25.0/tree_sitter_javascript/
  (retrieved 2026-08-30).

## NUIF relevance

**Borrow** intrinsic DOM element semantics, `key` as a foreign correspondence
hint, literal `data-*` identity and literal style properties.

**Adapt** only a statically analyzable JSX subset: intrinsic elements, literal
attributes, literal text and profile-owned style objects. The executable first
profile intentionally excludes arrays. Retentive edits use syntax-node byte
ranges. Formatting, comments, imports and unrelated module source remain
unchanged.

**Reject** evaluation of arbitrary JavaScript during import. Components, hooks,
spreads, conditional expressions, loops, context, event handlers and runtime
style values are preserved source or `unsupported` until a separately declared
execution profile supplies inputs and a deterministic runtime.

## Mechanism

The executable profile parses the entire module with pinned Tree-sitter
JavaScript, locates exactly one literal profile marker and verifies that its
paired intrinsic JSX root is the direct return value of a synchronous,
zero-argument, default-exported function. It converts only exact literal
attributes, a fixed style-object vocabulary and raw escaped text. Every mapped
scalar keeps its original UTF-8 byte span; synchronization proves those spans
are fresh, replaces only changed spans and reimports the result for canonical
document equality. Source, syntax-node and mapped-depth limits are checked on
the import boundary.

## Open questions

- A TSX profile needs a separately pinned grammar and a decision on whether
  type-only source is merely retained or participates in correspondence.
- Runtime-backed components need an explicit input/state matrix, a sandboxed
  React renderer and evidence distinct from this non-executing source profile.
- Class names and imported stylesheets need a CSS provenance and cascade model;
  silently resolving them during JSX import would make results environment
  dependent.
- The first profile has syntax and round-trip evidence but no browser/runtime
  equivalence claim. Such a claim requires a separately versioned renderer
  oracle and layout comparison corpus.

## Executable boundary

`nuif-react-jsx-0` requires one directly returned marked intrinsic subtree in a
zero-argument default-exported function. It maps fixed flex containers and
literal pinned-font text through 21 scalar correspondences and never invokes a
React, Node or browser runtime. This is deliberately stricter than valid JSX:
the distinction prevents syntax acceptance from being mistaken for program
evaluation or runtime equivalence.
