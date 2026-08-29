---
id: nuif:research:tree-sitter
kind: repository
status: verified
title: Tree-sitter incremental concrete syntax trees
source:
  url: https://tree-sitter.github.io/tree-sitter/
  authors: [Tree-sitter contributors]
  published_at: null
  license: MIT
retrieved_at: 2026-08-29
tags: [source-code, incremental-parsing, syntax-tree, patching]
confidence: 0.99
claims: [nuif:claim:sync-not-regenerate]
relations: []
links:
  spec: [spec/09-provenance-and-fidelity.md, spec/12-cli-api-and-automation.md]
  adr: []
  rfc: [rfcs/0003-authored-resolved-provenance.md]
  code: [adapters/README.md, adapters/html-css/PROFILE.md, crates/nuif-html]
  experiments: [nuif:experiment:html-css-retentive-sync]
---
# Summary
Tree-sitter maintains concrete syntax trees incrementally and can reuse unchanged structure after precisely described text edits. It preserves source ranges and exposes changed ranges between trees.

## Evidence

- The official parser guide defines Tree-sitter as an incremental parser that builds concrete syntax trees and distinguishes named from anonymous nodes. Node ranges use byte offsets as well as row/column points (`https://tree-sitter.github.io/tree-sitter/using-parsers/`, `2-basic-parsing.html`, retrieved 2026-08-29).
- The official editing contract requires an `InputEdit` with start, old-end and new-end byte/point positions before reparsing with the old tree. Previously retained node objects must receive the same edit or be fetched again from the edited tree (`https://tree-sitter.github.io/tree-sitter/using-parsers/3-advanced-parsing.html`, retrieved 2026-08-29).
- Multi-language documents are supported through included ranges, and injection queries identify content that should be parsed with another language. NUIF's bounded adapter instead takes the simpler deterministic route of parsing the HTML tree and then parsing the mapped style element's exact raw-text range as CSS (`3-advanced-parsing.html`; `https://tree-sitter.github.io/tree-sitter/3-syntax-highlighting.html`, retrieved 2026-08-29).
- Tree-sitter grammar design explicitly targets an intuitive concrete tree whose nodes correspond to recognizable source constructs rather than a normalized abstract tree. This is the property a retentive adapter needs to retain attribute, text and declaration byte spans (`https://tree-sitter.github.io/tree-sitter/creating-parsers/3-writing-the-grammar.html`, retrieved 2026-08-29).

## Executable verification

`nuif-html-css-0` pins Tree-sitter 0.26.10, `tree-sitter-html` 0.23.2 and `tree-sitter-css` 0.25.0. Import rejects recovery/error trees, extracts the raw style-element range from the HTML CST, validates that range with the CSS grammar and records the exact scalar byte spans used by correspondence records. Synchronization does not rely on a formatter or AST regeneration: it validates stale spans, applies replacements in descending byte order, reparses both languages and requires exact edited-document equality.

`cargo xtask gate-f` independently checks the complement of the edited ranges: after a token, four padding edges and escaped text change, every byte outside the six recorded spans is identical. HTML/CSS comments and an unmapped element inserted before import survive. The repeated synchronization has the same source and edit list. This verifies the Tree-sitter-based mechanism for the declared profile, not arbitrary source languages or arbitrary HTML.

## Mechanism

The adapter parses HTML into a concrete tree, locates identity-bearing elements and the mapped style raw-text node, then parses that raw range as CSS. Each semantic scalar is paired with an absolute half-open byte span. A source update first regenerates only the profile encodings of the before/after scalar values, checks that the retained span still contains the before encoding, and replaces changed values from the end of the file toward the start. A complete reparse and semantic import is the postcondition; no edited source is returned unless it equals the requested document.

## NUIF relevance
Source adapters need concrete-syntax-aware minimal editing rather than AST regeneration. Tree-sitter is a strong parser substrate, but formatting/comment-preserving patch generation remains adapter-specific and may require language-native tooling for some frameworks.

## Open questions

- Whether broader CSS shorthand, cascade and media-query mappings should use Tree-sitter queries or a CSS semantic parser while retaining Tree-sitter spans.
- How correspondence spans should be rebased after independent source edits that change formatting but leave mapped values equivalent; profile 0 deliberately returns `StaleSpan`.
- Whether a future framework adapter can preserve embedded JavaScript/TypeScript with included ranges alone or needs language-native refactoring APIs.
