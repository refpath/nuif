---
id: nuif:research:github-pages-actions-documentation-publishing
kind: article
status: verified
title: GitHub Pages publication through Actions artifacts
source:
  url: https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site
  authors: [GitHub]
  published_at: "GitHub Pages documentation current on 2026-08-30"
  license: GitHub documentation terms
retrieved_at: 2026-08-30
tags: [documentation, github-pages, github-actions, static-site, publishing]
confidence: 0.99
claims: [nuif:claim:semantic-automation]
relations:
  - type: extends
    target: nuif:research:cargo-workspace-xtask-and-ci-layout
    note: The same xtask boundary can compile documentation before a Pages artifact is uploaded.
links:
  spec: []
  adr: []
  rfc: []
  code: [.github/workflows/ci.yml, xtask/src/main.rs]
  experiments: []
---

# Summary

GitHub Pages supports branch publication and custom GitHub Actions workflows.
The custom workflow path builds static files, uploads one Pages artifact and
deploys that artifact. A pull request can execute the build and omit deployment.
The generated site therefore does not require a committed `gh-pages` branch.

## Evidence

- The publishing-source documentation states that a custom Actions workflow is
  appropriate when the project uses a generator other than Jekyll or does not
  want a branch containing compiled files. Locator: "About publishing
  sources", lines 28–31, retrieved 2026-08-30.
- The documented workflow checks out the repository, builds static files,
  invokes `actions/upload-pages-artifact`, and deploys with
  `actions/deploy-pages` only for the default branch. Locator: "Publishing with
  a custom GitHub Actions workflow", lines 84–90, retrieved 2026-08-30.
- The deployment uses a `github-pages` environment, for which GitHub recommends
  a protection rule that restricts deployment to the default branch. Locator:
  same section, line 90, retrieved 2026-08-30.

## Mechanism

The build job receives read-only repository contents and emits a static site
directory. `actions/upload-pages-artifact` transfers that directory between
jobs. A deployment job with `pages: write` and `id-token: write` publishes the
artifact through the `github-pages` environment. Generated files remain
workflow artifacts rather than source revisions.

## NUIF relevance

**Borrow** the artifact deployment boundary. NUIF can keep Markdown, research
metadata and specification modules in their current paths while an xtask
compiler stages the presentation under `target/`.

**Reject** a generated-output branch. It would create a second review history
without adding a distinct source artifact.

## Open questions

- The custom domain and deployment protection rules remain repository settings.
- Immutable specification snapshots require a separate tagged-source policy;
  the first Pages site publishes the current default-branch view.
