---
id: nuif:research:gitbook-bidirectional-git-sync
kind: article
status: verified
title: GitBook bidirectional Git synchronization and content structure
source:
  url: https://gitbook.com/docs/integrations/git-sync
  authors: [GitBook]
  published_at: "GitBook documentation current on 2026-08-30"
  license: GitBook documentation terms
retrieved_at: 2026-08-30
tags: [documentation, gitbook, git-sync, publishing, navigation]
confidence: 0.98
claims: []
relations:
  - type: compares_to
    target: nuif:research:github-pages-actions-documentation-publishing
    note: GitBook synchronizes editor state while Pages deploys a build artifact.
  - type: related_to
    target: nuif:research:github-wiki-repository-and-indexing
    note: Both introduce an editing or revision surface outside the canonical project commit.
links:
  spec: []
  adr: []
  rfc: []
  code: [README.md]
  experiments: []
---

# Summary

GitBook Git Sync is bidirectional. Repository commits update a GitBook space,
and GitBook editor changes update the configured repository branch. GitBook
uses `.gitbook.yaml` to select one root and uses `SUMMARY.md` as the table of
contents. When no summary exists, GitBook can infer and later create or update
one from editor state.

## Evidence

- GitBook states that Git Sync automatically synchronizes changes from its
  editor and commits from GitHub or GitLab. Locator: *GitHub & GitLab Sync*,
  "Overview", retrieved 2026-08-30.
- `.gitbook.yaml` selects a root directory; other paths are relative to that
  root. Locator: *Content configuration*, "Root", retrieved 2026-08-30:
  https://gitbook.com/docs/getting-started/git-sync/content-configuration.
- `SUMMARY.md` mirrors the GitBook table of contents and GitBook creates or
  updates it when content is edited in GitBook. Locator: same document,
  "Summary", retrieved 2026-08-30.
- GitBook warns that creating README files through its editor can produce
  duplicates, rendering conflicts and unpredictable precedence. Locator:
  *Troubleshooting*, "Be sure to only create readme files in your repo",
  retrieved 2026-08-30:
  https://gitbook.com/docs/getting-started/git-sync/troubleshooting.

## Mechanism

The GitBook space stores presentation and editing state. Synchronization maps
that state to Markdown, README and summary files on a selected branch. This is
useful when GitBook is an accepted authoring system, but it does not implement
a read-only compilation boundary from scattered canonical repository sources.

## NUIF relevance

**Reject** GitBook Git Sync for the canonical NUIF documentation. The project
requires source changes to pass research, link, metadata and conformance checks
in one repository transaction. A bidirectional service would add a second
writer and service-owned publication state.

## Open questions

- GitBook could consume a dedicated exported branch, but that branch would
  duplicate generated documentation and provide no required capability over
  Pages artifacts.
