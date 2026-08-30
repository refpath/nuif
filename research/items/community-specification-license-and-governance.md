---
id: nuif:research:community-specification-license-and-governance
kind: standard
status: verified
title: Community Specification license and repository governance
source:
  url: https://github.com/CommunitySpecification/Community_Specification/blob/main/getting-started.md
  authors: [Joint Development Foundation, Community Specification contributors]
  published_at: "Community Specification 1.0 repository current on 2026-08-30"
  license: Community Specification License 1.0
retrieved_at: 2026-08-30
tags: [governance, specification-license, patent, contributor-license-agreement, scope]
confidence: 0.99
claims: []
relations:
  - type: related_to
    target: nuif:research:w3c-community-incubation-and-recommendation-process
    note: Both processes distinguish specification governance from source-code licensing.
links:
  spec: []
  adr: []
  rfc: []
  code: [GOVERNANCE.md, CONTRIBUTING.md]
  experiments: []
---

# Summary

Community Specification 1.0 provides repository-based legal and governance
terms for collaborative specification development. Its contributor agreement,
scope, notices and license files define participation, patent coverage and
source-code licensing. The process recommends separate repositories for a
specification and its reference source code where practical.

## Evidence

- The contributor license agreement binds participants to the legal and
  governance terms of the working group. Locator: `getting-started.md`, lines
  199–200, retrieved 2026-08-30.
- `Scope.md` defines the working group's subject matter and bounds the patent
  licensing obligations. Locator: `getting-started.md`, lines 201 and 209,
  retrieved 2026-08-30.
- `Notices.md` records contacts, patent exclusions, implementers and withdrawn
  participants. `License.md` identifies the specification license and the
  separate license for source or sample code. Locator: `getting-started.md`,
  lines 203–213, retrieved 2026-08-30.
- The best-practice section recommends a contributor-agreement check, use of
  the specification license for specifications rather than code, careful scope
  definition and separate specification and code repositories. Locator:
  `getting-started.md`, lines 235–245, retrieved 2026-08-30.

## Mechanism

Each contributor accepts the common agreement before a contribution is merged.
The declared scope bounds patent commitments. Notices record exclusions and
implementer assertions. The specification license grants rights applicable to
independent implementations, while an Open Source Initiative-approved license
continues to govern implementation code.

## NUIF relevance

**Borrow** the explicit scope, notices and contributor-agreement model before a
multi-party specification is published as an implementable draft.

**Adapt** the separate-repository recommendation only when the specification
has independent contributors. The current monorepository keeps experiments,
fixtures and draft modules reviewable at the same revision.

**Reject** applying Community Specification terms retroactively without legal
review and contributor consent. The current code licenses do not establish the
specification-wide patent commitments described by this process.

## Open questions

- The entity that would administer contributor agreements and notices has not
  been selected.
- The patent scope requires legal review after the implementable draft boundary
  is stable.
