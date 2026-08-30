---
id: nuif:adr:0010
kind: adr
status: accepted
---

# ADR 0010: Bounded retentive Svelte source adapter

Decision delegated to research on 2026-08-30. Evidence:
`nuif:research:svelte-source-adapter-surface`,
`nuif:research:tree-sitter`, and
`nuif:research:dependency-and-subsystem-audit`.

## Context

Svelte source combines declarative markup with JavaScript, TypeScript,
preprocessing, directives, blocks and component-scoped CSS. NUIF needs useful
source interchange without claiming that static parsing reproduces an
application runtime or rewriting unrelated developer source.

## Decision

1. `nuif-svelte` implements only the versioned `nuif-svelte-static-0`
   profile. It maps regular `div` containers and `span` literal text through
   explicit `data-nuif-*` markers and a fixed inline-style vocabulary.
2. `tree-sitter-svelte-next` 0.1.1 supplies the production concrete syntax tree
   and UTF-8 byte ranges. It does not define Svelte semantics.
3. Exact official `svelte` 5.57.0 is the foreign parser/compiler oracle. It is
   test tooling only and never enters a NUIF deliverable.
4. Synchronization edits only recorded scalar spans, rejects stale or structural
   changes atomically, reparses the result, and requires canonical NUIF equality.
5. Scripts, stylesheets, preprocessors, expressions, blocks, directives,
   components, special elements and dynamic attributes are outside profile zero.
   Unmarked source may be retained but has no claimed semantic mapping.
6. Component CSS is deferred. A later profile must define selector ownership,
   cascade, specificity, Svelte scope hashing and source-locality rules before
   it can be accepted.

## Rationale

The official compiler is the only suitable semantic authority, but its printer
may normalize whitespace and quoting. A concrete syntax tree is therefore the
right production mechanism for retained spans, while live official compilation
prevents that community grammar from becoming a self-oracle. The broader
unofficial Rust compiler adds a second semantic implementation and a much
larger dependency graph without improving the deliberately static boundary.

## Consequences

The adapter is useful for generated components and controlled developer tools,
not arbitrary Svelte applications. Its conformance claim includes exact
round-trip and byte-complement preservation plus pinned official parse/compile
acceptance. Runtime rendering equivalence and general CSS equivalence remain
unclaimed.
