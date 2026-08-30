---
id: nuif:research:svelte-source-adapter-surface
kind: standard
status: verified
title: Svelte component AST, retentive syntax and compiler-oracle boundary
source:
  url: https://svelte.dev/docs/svelte/svelte-compiler
  authors: [Svelte contributors]
  published_at: null
  license: MIT for Svelte implementation and documentation repository
retrieved_at: 2026-08-30
tags: [adapter, svelte, ast, source-correspondence, css]
confidence: 0.98
claims: [nuif:claim:sync-not-regenerate]
relations:
  - type: supports
    target: nuif:research:tree-sitter
links:
  spec: [spec/09-provenance-and-fidelity.md]
  adr: [adrs/0010-svelte-source-adapter.md]
  rfc: []
  code: [adapters/STATUS.md]
  experiments: []
---
# Summary

The official Svelte compiler is the semantic oracle for `.svelte` syntax. It
exposes component and CSS parsers with source-offset AST nodes and separately
models markup, instance/module scripts and component CSS. A NUIF production
adapter still needs a concrete syntax tree because the official `print` API is
explicitly allowed to change whitespace and quoting. The selected split is the
official compiler for pinned foreign conformance and
`tree-sitter-svelte-next` 0.1.1 for bounded UTF-8 byte spans in Rust.

The first executable profile is deliberately smaller than Svelte's static
surface: regular `div`/`span` elements, double-quoted literal identity and name
attributes, one literal inline-style declaration list and one literal text run.
Scripts, component CSS, expressions and all directives or blocks are rejected
inside the mapped component. Unmarked comments and top-level source may be
retained, but the profile does not infer their runtime relationship to the
marked root.

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
- `tree-sitter-svelte-next` 0.1.1 is a dual MIT/Apache-2.0 grammar compatible
  with Tree-sitter 0.25 and later. Its crate is generated from commit
  `bdea454a8ae7272498b8fe9d4b6b24fbd3dfe7b6`; its Rust API exposes the
  language and node types without a Svelte runtime. The grammar includes
  elements, literal and expression attributes, blocks, scripts and styles, so
  the adapter can reject executable nodes structurally. Locators: crate
  manifest, `src/lib.rs`, `src/node-types.json`, `.cargo_vcs_info.json`,
  retrieved 2026-08-30:
  https://github.com/PRRPCHT/tree-sitter-svelte-next and
  https://docs.rs/tree-sitter-svelte-next/0.1.1/.
- The npm registry reported official `svelte` 5.57.0 as latest on 2026-08-30.
  A foreign fixture must pin that exact package and invoke both
  `parse(source, { modern: true })` and `compile(source, ...)`; floating latest
  is not evidence because the compiler and modern-AST defaults evolve.
- The independent Rust `svelte-compiler` 0.1.4 declares Rust 1.94 and depends
  on a broad compiler stack. Its own `AUDIT.md` reports about 11,300 lines in
  the API module, duplicated modern/legacy paths and manual recovery for
  grammar gaps. This is useful comparative work but is neither the official
  compiler nor a smaller retentive boundary. Locators: crate manifest and
  project audit, retrieved 2026-08-30:
  https://docs.rs/svelte-compiler/0.1.4/svelte_compiler/ and
  https://github.com/themixednuts/svelte/blob/main/AUDIT.md.

## Alternatives and decision

| Candidate | Strength | Blocking mismatch for this profile | Decision |
| --- | --- | --- | --- |
| official `svelte/compiler` | authoritative syntax, diagnostics and generated output | JavaScript runtime boundary; `print` does not retain formatting | pinned foreign oracle |
| `tree-sitter-svelte-next` 0.1.1 | small CST, byte offsets, compatible Tree-sitter line | community grammar, not semantic authority | production span parser, checked against oracle |
| Rust `svelte-compiler` 0.1.4 | typed Rust compiler project with broad Svelte ambition | unofficial, much larger graph and documented recovery debt | reject as production dependency |
| HTML parser alone | mature markup parsing | cannot classify Svelte blocks, directives or embedded regions safely | reject |
| regular expressions | minimal code | cannot prove nesting, quoting or executable-syntax exclusion | reject |

Inline literal `style` is selected over a component `<style>` block for profile
zero. It binds every mapped scalar to the element that owns it, avoids cascade,
specificity and Svelte scope-hash semantics, and provides one exact replaceable
span per property. A later class/CSS profile needs separate selector, cascade,
scope and unused-selector conformance; it is not an implicit expansion of this
profile.

## Mechanism

The Rust parser first enforces encoded-size, syntax-node and mapped-depth
limits. It locates exactly one marked regular-element root, verifies the closed
attribute and inline-style vocabularies, decodes only canonical entity escapes,
and records every mapped UTF-8 span. Export self-imports. Synchronization renders
the before and after profile forms to obtain canonical replacement values,
checks that each retained span is still current, applies replacements from the
end of the source, and self-imports the result. The foreign gate separately
parses and compiles generated and synchronized sources with the exact official
compiler package.

## NUIF relevance

**Borrow** compiler AST offsets and literal-node categories for correspondence.

**Adapt** a static subset of regular elements, literal attributes, literal text
and profile-owned inline CSS declarations. Edits replace original spans rather
than printing the AST. Grammar revision, compiler version and modern-AST mode
are part of provenance.

**Reject** automatic semantic lifting of runes, scripts, expressions, snippets,
blocks, directives, actions, transitions, dynamic components or preprocessors.
They are programs whose behavior depends on inputs and runtime state.

## Falsification and update triggers

The adapter decision fails if the Tree-sitter grammar accepts a generated
fixture that official Svelte rejects, assigns unusable byte spans, or cannot
structurally distinguish one of the excluded executable constructs. The gate
therefore compiles every exported and synchronized fixture with the pinned
official package and maintains negative fixtures for scripts, expressions,
directives, blocks, components and component CSS. A Svelte or grammar update is
one isolated dependency commit that regenerates the foreign lockfile, runs the
complete adapter corpus, and records any AST or diagnostic change before the
pin moves.

## Open questions

- Whether a future component-CSS profile should own one style block per mapped
  root or preserve user CSS through a selector-aware correspondence layer.
- Whether official compiler warnings should become hard failures or a separately
  versioned diagnostic baseline once the profile accepts more than generated
  source.
- Whether the community grammar will publish a stable compatibility and release
  policy; until then, its exact version and source commit remain provenance.
