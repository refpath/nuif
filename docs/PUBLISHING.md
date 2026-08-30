# Documentation publication

Repository Markdown is the sole editable documentation source. Files remain
beside the implementation, specification module, decision or research record
that they describe. `docs/catalog.json` defines site membership and navigation
without copying document bodies.

## Local commands

The documentation compiler is part of `xtask`:

```sh
cargo xtask docs-check
cargo xtask docs-build
cargo xtask docs-serve
cargo xtask docs-paper
```

`docs-check` validates catalog paths, unique document identifiers, required
frontmatter, file budgets and relative links. It writes the machine-readable
catalog and report under `target/`. `docs-build` stages Markdown with generated
navigation and invokes mdBook 0.5.4. The static site is written to
`target/docs-site`. `docs-serve` rebuilds the staging tree and starts the local
mdBook server. `docs-paper` builds the site and prints the generated technical
manuscript to `target/docs-site/downloads/nuif-research-manuscript.pdf` through
the repository's pinned Chrome for Testing binary.

`docs-paper` keeps Chrome's process sandbox enabled by default. A disposable,
externally isolated CI runner that cannot create Chrome's Linux namespace may
set `NUIF_CHROME_NO_SANDBOX=1`; do not use that override for routine local
rendering.

The pinned renderer can be installed through:

```sh
cargo xtask docs-setup
cargo xtask browser-install
```

Generated navigation, search indexes and HTML are build artifacts. They are not
committed.

The manuscript body is generated from the whitepaper modules listed in
`docs/catalog.json`. The composition metadata does not copy chapter text.
Publication does not imply peer review or specification maturity.

## Metadata boundary

Specification modules, RFCs, ADRs and research records require YAML
frontmatter. `docs/schema/document.schema.json` defines the common identifier,
kind and status fields. Research records retain their additional schema in
`research/schema/research-item.schema.json`.

The compiler accepts a maximum of 2 MiB per Markdown file and 64 KiB per
frontmatter block. It uses typed YAML deserialization with duplicate-key
rejection and parser resource budgets. YAML metadata does not define the NUIF
interchange syntax.

## Hosted publication

The Pages workflow builds the same staged source on pull requests and on the
default branch. Pull requests retain a build artifact and do not deploy. A
default-branch build uploads `target/docs-site` as the GitHub Pages artifact and
deploys through the `github-pages` environment. GitHub documents custom Actions
workflows for generators other than Jekyll and for repositories that do not
want compiled output on a publication branch. Locator: GitHub Docs,
"Configuring a publishing source for your GitHub Pages site", "Publishing with
a custom GitHub Actions workflow", retrieved 2026-08-30:
https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site.

The workflow pins every action to a full commit. It grants read-only repository
contents to the build job. Only the deployment job receives `pages: write` and
`id-token: write`; it does not receive source or release write permission.

GitHub Wiki is not a publication target. GitHub stores each wiki in a separate
Git repository, so enabling direct edits would create a second source history.
GitHub also documents restricted search-engine indexing for wiki content and
recommends GitHub Pages for public documentation. Locator: GitHub Docs,
"Adding or editing wiki pages" and "About wikis", retrieved 2026-08-30:
https://docs.github.com/en/communities/documenting-your-project-with-wikis/adding-or-editing-wiki-pages
and
https://docs.github.com/en/communities/documenting-your-project-with-wikis/about-wikis.

GitBook bidirectional synchronization is not enabled. Its Git Sync can modify
`SUMMARY.md` and synchronize edits from GitBook to the connected branch. That
mode would add another authoring surface. Locator: GitBook documentation,
"Git Sync" and "Configuration", retrieved 2026-08-30:
https://gitbook.com/docs/getting-started/git-sync
and
https://gitbook.com/docs/getting-started/git-sync/content-configuration.

## Immutable records

The Pages site represents the default branch and is not an immutable
specification release. Git tags, release notes and source archives remain the
versioned record. A later citable research release can add `CITATION.cff`, a
Zenodo concept DOI and a version DOI without changing the canonical Markdown
source. Publication prerequisites and venue constraints are recorded in
`research/items/scholarly-publication-and-citation-workflow.md`.
