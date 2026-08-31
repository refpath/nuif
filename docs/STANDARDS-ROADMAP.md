# Standards-development roadmap

NUIF remains a pre-standard research project with a draft specification and
reference implementation. The editor version `0.1.0-alpha.3` identifies an
application prerelease. It does not establish specification stability,
interoperability, resource-package conformance, reconstruction accuracy or
external review.

## Current publication boundary

The repository can publish source, generated documentation, schemas,
conformance fixtures, implementation reports and a citable technical
manuscript without joining a standards body. This stage uses the project code
licenses and current contribution process. It does not create
specification-wide patent commitments.

The following work is executable without external approval:

- publish the default-branch documentation through GitHub Pages;
- publish tagged source and native editor prereleases through GitHub Releases;
- expose `CITATION.cff` through GitHub's repository citation interface;
- compile a technical manuscript from the canonical whitepaper modules;
- validate schemas, frontmatter, links, adapter coverage and conformance reports
  in GitHub Actions;
- archive a later public release with Zenodo after its GitHub integration is
  enabled by an authorized organization account.

GitHub adds a repository citation control when `CITATION.cff` exists on the
default branch. Zenodo can ingest public GitHub releases after an account owner
connects and enables the repository. Each release receives a DOI. Locators:
GitHub Docs, "About CITATION files" and "Referencing and citing content";
Zenodo, "Enable a repository" and "Archive a release from GitHub", retrieved
2026-08-30:
https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-citation-files,
https://docs.github.com/en/repositories/archiving-a-github-repository/referencing-and-citing-content,
https://help.zenodo.org/docs/github/enable-repository/ and
https://help.zenodo.org/docs/github/archive-software/github-upload/.

## Implementer-draft gate

An implementer draft requires all of the following evidence:

- one bounded model and serialization profile whose normative requirements are
  internally consistent;
- a versioned schema and conformance suite mapped to every normative
  requirement;
- two independently maintained implementations that pass the same profile;
- at least one foreign-system round trip that preserves declared opaque data
  and reports every loss;
- security, privacy, accessibility and internationalization review records;
- a compatibility and deprecation policy for profile revisions;
- an extension namespace and registration process;
- specification copyright, patent, contribution and trademark terms reviewed
  by qualified counsel.

The present repository has an independent Python profile-zero evaluator and
ten executable adapter profiles, including a normalized Figma mapping and
compiled no-network review shell that explicitly exclude live host behavior,
plus bounded three-engine web-accessibility and finite web-behavior
projections.
The behavior program also has one deterministic content-addressed package
transport with an independent ZIP reader; it remains outside the canonical
semantic document and is not a second complete implementation.
These results cover bounded subsets. They do
not constitute two independent implementations of the complete draft.

RFCs 0010, 0011 and 0012 are proposed research inputs. Their package, resource,
behavior-attachment, capture and reconstruction profiles are not prerequisites for a small core
implementer draft unless the selected charter includes them. If included, each
requires its own independent implementation/evaluator report; a model demo or
editor alpha does not satisfy interoperability.

## Incubation gate

Standards-body incubation begins only after the implementer-draft gate and
evidence of external participation. Venue selection follows the demonstrated
adoption surface.

| Venue | Entry condition | Suitable scope | Current disposition |
|---|---|---|---|
| W3C Community Group | proposer plus four supporters; no participation fee | Web authoring, browser semantics, design tokens and Web APIs | Preferred incubation path if Web and design-tool stakeholders participate |
| Khronos New Initiative | Board-reviewed proposal, industry sponsors and later member participation | graphics, 2D/3D content tools, GPU-adjacent interchange and a trademarked conformance program | Alternative if graphics-tool vendors become the primary implementers |
| OASIS Open Project | contributor agreements, project sponsors and governed specification advancement | document packages, APIs and protocols with a path to OASIS and international approval | Alternative if protocol and package governance dominate |
| Ecma Technical Committee | General Assembly formation and at least three supporting members for new work | mature multi-company software-platform specifications | Deferred until member organizations request a committee |
| Community Specification | contributor agreement, scope, notices and specification license | repository-based multi-party specification incubation | Legal framework candidate before or alongside an organization-hosted process |

W3C Community Group reports are not W3C Standards. Khronos detailed Working
Group design requires membership. Ecma and OASIS formal paths require member or
sponsor governance. These conditions are process boundaries rather than
technical quality rankings. Primary locators are recorded in
`research/items/standards-development-venue-comparison.md`.

## Formal-advancement gate

Formal advancement requires a stable chartered scope, recorded consensus,
public review, resolved intellectual-property terms, interoperable
implementations and conformance results. The selected body defines the exact
ballot, review and patent procedures. NUIF cannot claim accreditation before
that process completes.

The corresponding repository evidence includes:

- immutable specification snapshots and versioned conformance profiles;
- implementation reports linked to exact source revisions;
- test-suite coverage for every normative requirement;
- issue dispositions for security, privacy, accessibility and
  internationalization reviews;
- a public list of implementers, exclusions and withdrawn participants;
- release and errata procedures that distinguish compatible corrections from
  new feature levels.

## Vendor adoption

Figma, Affinity, Canva and other hosts can evaluate the draft without replacing
their internal document models. API hosts map a declared NUIF profile through a
supported plug-in or service boundary. Affinity currently uses a separately
declared file-interchange trial because no stable public document API was found.
The headless CLI and WASM binding remain the semantic test surfaces. Vendor-
private state is either represented, preserved as declared opaque data or
reported through structured fidelity diagnostics.

The current Figma mapping and static shell, the Affinity SVG bridge draft and
the Canva Apps SDK draft describe feasible boundaries but do not include their
required live vendor-runtime trials. A vendor adoption claim therefore requires
a signed test fixture, host-version matrix, import/export or transaction report
and maintainer outside the reference-core implementation. Canva marketplace
approval and native NUIF Connect support are separate upstream outcomes. The
integration boundary is specified in `docs/HOST-INTEGRATION.md` and ADRs 0008
and 0012.

## Decision authority

Technical evidence can identify a preferred venue and automate every
repository-side prerequisite. It cannot accept patent obligations, sign a
contributor agreement for another entity, create independent implementations
or record stakeholder consensus. Those acts remain attributable to their human
or organizational participants.
