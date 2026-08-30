---
id: nuif:research:mdbook-static-documentation-pipeline
kind: implementation
status: verified
title: mdBook static documentation and generated navigation
source:
  url: https://rust-lang.github.io/mdBook/
  repository: https://github.com/rust-lang/mdBook
  authors: [Rust project contributors]
  published_at: "mdBook 0.5.4 documentation"
  license: MPL-2.0
retrieved_at: 2026-08-30
tags: [documentation, mdbook, rust, markdown, static-site, search]
confidence: 0.99
claims: [nuif:claim:semantic-automation]
relations:
  - type: depends_on
    target: nuif:research:github-pages-actions-documentation-publishing
    note: mdBook produces the static directory uploaded as the Pages artifact.
links:
  spec: []
  adr: []
  rfc: []
  code: [Cargo.toml, xtask/src/main.rs]
  experiments: []
---

# Summary

mdBook 0.5.4 builds searchable static documentation from Markdown. The
`SUMMARY.md` file determines inclusion, order, hierarchy and source paths.
Preprocessors can transform Markdown before rendering, and alternate backends
can consume the same book representation.

## Evidence

- mdBook describes itself as a command-line Markdown book generator with
  integrated search, syntax highlighting, themes, preprocessors and backends.
  Locator: mdBook 0.5.4 introduction, lines 12–26, retrieved 2026-08-30.
- mdBook requires a strictly formatted `SUMMARY.md`; without that file there is
  no book. Locator: *SUMMARY.md*, lines 13–16, retrieved 2026-08-30:
  https://rust-lang.github.io/mdBook/format/summary.html.
- Preprocessors modify raw Markdown before it reaches the renderer. Locator:
  *Configuring Preprocessors*, lines 13–24, retrieved 2026-08-30:
  https://rust-lang.github.io/mdBook/format/configuration/preprocessors.html.

## Mechanism

NUIF generates a temporary mdBook source directory and `SUMMARY.md` from a
validated documentation catalog. The catalog owns identity, status and
navigation. mdBook owns HTML presentation and client-side search. A renderer
change therefore does not move or duplicate canonical Markdown.

## NUIF relevance

**Borrow** mdBook's Rust-native static renderer and search implementation.

**Adapt** its source-root assumption by staging canonical documents under
`target/docs-src`. The staging transformation removes YAML frontmatter from the
rendered body and rewrites repository-relative links while retaining the source
path in the generated catalog.

**Reject** a committed `SUMMARY.md`. It would duplicate the catalog's inclusion
and ordering state.

## Open questions

- Research faceting and backlink presentation may require generated index pages
  or a later renderer. The catalog boundary permits that change without a
  content migration.
