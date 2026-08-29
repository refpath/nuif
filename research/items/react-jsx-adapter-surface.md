---
id: nuif:research:react-jsx-adapter-surface
kind: standard
status: reviewed
title: React JSX and DOM element source surface
source:
  url: https://react.dev/learn/writing-markup-with-jsx
  authors: [React team]
  published_at: null
  license: CC-BY-4.0 for React documentation
retrieved_at: 2026-08-29
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
  code: [adapters/STATUS.md]
  experiments: []
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
  2026-08-29).

## NUIF relevance

**Borrow** intrinsic DOM element semantics, `key` as a foreign correspondence
hint, literal `data-*` identity and literal style properties.

**Adapt** only a statically analyzable JSX subset: intrinsic elements, literal
attributes, literal text, arrays with fixed order and profile-owned style
objects. Retentive edits use syntax-node byte ranges. Formatting, comments,
imports and unrelated expressions remain unchanged.

**Reject** evaluation of arbitrary JavaScript during import. Components, hooks,
spreads, conditional expressions, loops, context, event handlers and runtime
style values are preserved source or `unsupported` until a separately declared
execution profile supplies inputs and a deterministic runtime.
