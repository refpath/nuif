---
id: nuif:research:standards-development-venue-comparison
kind: synthesis
status: verified
title: Standards-development venue requirements for an interface interchange specification
source:
  url: https://www.w3.org/community/
  authors: [World Wide Web Consortium, Khronos Group, Ecma International, OASIS Open, Joint Development Foundation]
  published_at: "Official process documents current on 2026-08-30"
  license: Mixed official documentation terms
retrieved_at: 2026-08-30
tags: [governance, standards-development-organization, w3c, khronos, ecma, oasis, jdf, conformance]
confidence: 0.98
claims: []
relations:
  - type: extends
    target: nuif:research:w3c-community-incubation-and-recommendation-process
    note: The synthesis compares W3C incubation with cross-platform alternatives.
  - type: extends
    target: nuif:research:community-specification-license-and-governance
    note: Repository-based specification licensing remains the pre-organization option.
links:
  spec: [spec/00-conformance.md]
  adr: []
  rfc: []
  code: [GOVERNANCE.md, docs/STANDARDS-ROADMAP.md]
  experiments: []
---

# Summary

W3C, Khronos, Ecma, OASIS and the Joint Development Foundation provide
different entry conditions and intellectual-property boundaries. W3C Community
Groups permit no-fee public incubation but do not produce W3C Standards.
Khronos accepts non-member initiative proposals but reserves detailed Working
Group design for members and couples adoption claims to a conformance test
suite. Ecma Technical Committees require General Assembly formation and member
support. OASIS Open Projects combine public code and specification work with a
sponsor-governed path to an OASIS Standard. The Community Specification process
supplies a repository-based contributor, scope and patent framework without
asserting formal standards-body status.

## Evidence

- A W3C Community Group proposal needs a W3C account and four additional
  supporters. Participation has no fee, and Community Group reports are not W3C
  Standards. Locator: W3C Community Groups, lines 79–116, retrieved
  2026-08-30: https://www.w3.org/community/.
- Khronos permits member and non-member initiative proposals. An Exploratory
  Group develops use cases, requirements and a statement of work without
  detailed design contributions. Detailed Working Group participation requires
  Khronos membership. Locator: Khronos "New Initiative Process Overview",
  sections "New Initiative Process Overview" and "How To Propose", retrieved
  2026-08-30: https://www.khronos.org/exploratory/new-initiative-process/.
- Khronos requires an official Conformance Test Suite pass before a product can
  use a specification's trademarked name or make conformant-product claims.
  Locator: Khronos "About", "Open Standards for 3D, the Metaverse and More",
  retrieved 2026-08-30: https://www.khronos.org/about/.
- Ecma Technical Committees are formed by General Assembly decision. New work
  items require support from at least three Ecma members, of which at most one
  is a not-for-profit member. Royalty-Free Technical Committee operation
  requires General Assembly approval. Locator: Ecma Rules, Article 7.1,
  retrieved 2026-08-30: https://ecma-international.org/policies/rules/.
- OASIS Open Projects support code, APIs, prose specifications and protocols
  under contributor agreements. Their formal path includes public review,
  Statements of Use, project governance approval and an OASIS membership
  ballot. Locator: OASIS Open Projects Handbook, sections 1 and 9; Open Project
  Rules, sections 13–14, retrieved 2026-08-30:
  https://www.oasis-open.org/oasis-open-projects-handbook/ and
  https://www.oasis-open.org/policies-guidelines/open-projects-process/.
- Community Specification 1.0 uses a contributor agreement, bounded scope,
  notices and separate specification and source-code licenses. Locator:
  Community Specification `getting-started.md`, lines 195–245, retrieved
  2026-08-30:
  https://github.com/CommunitySpecification/Community_Specification/blob/main/getting-started.md.

## Mechanism

An incubation venue supplies participation and contribution terms before it
supplies formal publication status. Formal advancement adds a chartered scope,
intellectual-property commitments, public review, consensus and implementation
evidence. Conformance claims require a versioned specification, tests and
independent implementation results that use the same feature profile.

## NUIF relevance

**Borrow** the Community Specification scope and contributor terms when the
first external organization contributes normative text.

**Adapt** W3C Community Group incubation if browser, design-tool and design-token
stakeholders support a Web-facing scope. DTCG provides the closest existing
liaison surface.

**Adapt** Khronos if the center of adoption becomes cross-platform graphics and
content-tool interoperability and multiple member companies will fund a
Working Group and conformance suite.

**Reject** selecting a formal venue before there are independent implementers
and organizational sponsors. Each formal path depends on participation and
intellectual-property commitments that a single repository owner cannot
substitute.

## Open questions

- The primary scope may resolve toward Web authoring, graphics content tools or
  a general document protocol; each direction changes the suitable venue.
- No legal entity currently owns the specification trademark, contributor
  agreement administration or patent-notice process.
- Independent implementation and statement-of-use thresholds have not been
  met.
