---
id: nuif:research:scholarly-publication-and-citation-workflow
kind: synthesis
status: verified
title: Scholarly citation, archival DOI, preprint and software-paper workflow
source:
  url: https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-citation-files
  authors: [GitHub, Zenodo, Quarto, arXiv, Journal of Open Source Software]
  published_at: "publisher documentation current on 2026-08-30"
  license: Documentation terms vary by publisher
retrieved_at: 2026-08-30
tags: [research, citation, doi, zenodo, quarto, arxiv, joss, publication]
confidence: 0.98
claims: [nuif:claim:semantic-automation]
relations:
  - type: related_to
    target: nuif:research:github-release-delivery-and-provenance
    note: Zenodo can archive tagged software releases separately from editor package provenance.
links:
  spec: []
  adr: []
  rfc: []
  code: [research/README.md, docs/VERSIONING.md]
  experiments: []
---

# Summary

`CITATION.cff` provides machine-readable software and preferred-publication
metadata on GitHub. Zenodo can archive public GitHub releases and assign a DOI
to each release. Quarto renders one manuscript source to citeable HTML and PDF.
arXiv accepts topical, refereeable scientific contributions. The Journal of
Open Source Software (JOSS) reviews research software only after demonstrated
research use, sustained public development and feature completeness.

## Evidence

- GitHub parses `CITATION.cff` from the repository root and exposes APA and
  BibTeX citations. A `preferred-citation` can identify a paper or technical
  report instead of the software. Locator: *About CITATION files*, lines 24–70,
  retrieved 2026-08-30.
- Zenodo archives a public repository and issues a DOI for each GitHub release.
  Locator: GitHub, *Referencing and citing content*, lines 21–35, retrieved
  2026-08-30:
  https://docs.github.com/en/repositories/archiving-a-github-repository/referencing-and-citing-content.
- Quarto can generate citation metadata and citeable HTML from article
  frontmatter. Locator: Quarto, *Creating Citeable Articles*, retrieved
  2026-08-30:
  https://quarto.org/docs/authoring/create-citeable-articles.html.
- arXiv submissions must be topical and refereeable; new authors or categories
  may require endorsement. Locator: arXiv, *Submission Guidelines*, lines
  177–186, retrieved 2026-08-30:
  https://info.arxiv.org/help/submit/index.html.
- JOSS requires more than six months of active public history, demonstrated
  research impact, open-source practice, iterative development and
  feature-complete software. It also requires disclosure of generative-model
  assistance. Locator: JOSS, *Submitting a paper*, "Scope and significance" and
  "Pre-review screening criteria", retrieved 2026-08-30:
  https://joss.readthedocs.io/en/latest/submitting.html.

## Mechanism

The repository retains one Quarto manuscript and one bibliography. GitHub
Actions renders HTML and PDF without committing either output. `CITATION.cff`
identifies the software until a reviewed paper supplies the preferred citation.
Zenodo stores immutable release snapshots. arXiv and peer-review venues receive
the manuscript only after their respective evidence gates are met.

## NUIF relevance

**Borrow** CFF and Zenodo for immediate citation and archival identity.

**Adapt** the paper claim to the bounded profile-zero experiment. The first
manuscript reports the model, executable evidence and limitations; it does not
claim universal format coverage or standards status.

**Reject** treating a Pages site, DOI, preprint or software-paper acceptance as
equivalent to standards publication.

## Open questions

- Authors, order, affiliations and ORCID identifiers require human confirmation
  before archival deposit or manuscript submission.
- The peer-review venue depends on whether the eventual contribution is an HCI
  study, a software artifact or an interoperability specification.
