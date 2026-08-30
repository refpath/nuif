---
id: nuif:research:github-wiki-repository-and-indexing
kind: article
status: verified
title: GitHub Wiki repository and indexing boundaries
source:
  url: https://docs.github.com/en/communities/documenting-your-project-with-wikis/about-wikis
  authors: [GitHub]
  published_at: "GitHub Wiki documentation current on 2026-08-30"
  license: GitHub documentation terms
retrieved_at: 2026-08-30
tags: [documentation, github, wiki, search-indexing, publishing]
confidence: 0.99
claims: []
relations:
  - type: compares_to
    target: nuif:research:github-pages-actions-documentation-publishing
    note: Pages can publish compiled canonical sources without a second editable repository.
links:
  spec: []
  adr: []
  rfc: []
  code: [README.md]
  experiments: []
---

# Summary

A GitHub Wiki has an independent Git repository whose live content comes from
its default branch. Public editing can be restricted, but Wiki changes retain a
history separate from the project repository. GitHub also restricts search
engine indexing of most Wikis and directs projects that require indexing to
GitHub Pages.

## Evidence

- Local Wiki editing clones
  `https://github.com/OWNER/REPOSITORY.wiki.git`; only changes pushed to the
  default branch become live. Locator: *Adding or editing wiki pages*, "Adding
  or editing wiki pages locally", lines 58–68, retrieved 2026-08-30:
  https://docs.github.com/en/communities/documenting-your-project-with-wikis/adding-or-editing-wiki-pages.
- Search engines index a public Wiki only when it has at least 500 stars and
  public editing is disabled. GitHub recommends Pages when indexing is needed.
  Locator: *About wikis*, lines 25–28, retrieved 2026-08-30.
- Wikis have a soft limit of 5,000 files, after which pages can become
  inaccessible. Locator: *About wikis*, lines 30–32, retrieved 2026-08-30.

## Mechanism

Wiki page edits create commits in the Wiki repository. A generated mirror would
copy project Markdown into that repository and create a second revision graph.
Direct Wiki edits would create a second authoring surface with no atomic commit
across code, conformance fixtures and their documentation.

## NUIF relevance

**Reject** the Wiki as both canonical documentation and generated mirror. NUIF
requires documentation claims to remain reviewable with the code, research
records and conformance artifacts they describe.

## Open questions

- A one-page Wiki redirect is technically possible but provides no capability
  that the repository homepage and Pages URL do not already provide.
