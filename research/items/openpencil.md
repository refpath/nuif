---
id: nuif:research:openpencil
kind: repository
status: reviewed
title: OpenPencil programmable editor, scene graph, Figma codec and DOM/CSS conversion
source:
  url: https://github.com/open-pencil/open-pencil
  authors: [OpenPencil contributors]
  published_at: null
  license: MIT
retrieved_at: 2026-08-29
tags: [editor, cli, mcp, figma, dom-css, automation]
confidence: 0.96
claims: [nuif:claim:semantic-automation]
relations: []
links:
  spec: [spec/12-cli-api-and-automation.md]
  adr: []
  rfc: [rfcs/0004-headless-qa-contract.md]
  code: [apps/editor/README.md, adapters/README.md]
  experiments: []
---
# Summary
OpenPencil exposes its editor engine, scene graph, `.fig`/Kiwi codec, DOM/CSS conversion, CLI, RPC and MCP surfaces. It proves a modern design editor can treat programmatic control as a first-class surface.

## NUIF relevance
Borrow the operational lesson: editor actions, document querying, linting and export must be available headlessly. NUIF differs by making the neutral standard itself canonical instead of centering compatibility with an existing vendor model.
