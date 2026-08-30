---
id: nuif:research:w3c-community-incubation-and-recommendation-process
kind: standard
status: verified
title: W3C Community Group incubation and Recommendation-track requirements
source:
  url: https://www.w3.org/community/
  authors: [World Wide Web Consortium]
  published_at: "W3C Community Group and 2025 Process documentation"
  license: W3C permissive document license where stated
retrieved_at: 2026-08-30
tags: [governance, standards, w3c, community-group, patent, conformance]
confidence: 0.99
claims: []
relations:
  - type: related_to
    target: nuif:research:dtcg
    note: DTCG demonstrates a design-domain Community Group report and patent agreement.
links:
  spec: [spec/00-conformance.md]
  adr: []
  rfc: []
  code: [GOVERNANCE.md, docs/whitepaper/08-governance-and-standardization.md]
  experiments: []
---

# Summary

W3C Community Groups provide a no-fee incubation forum for specifications,
test suites and stakeholder discussion. Anyone with a W3C account can propose a
group; four additional supporters are required before launch. Participants
accept contribution and licensing terms. Community Group reports are not W3C
Standards. Recommendation-track work requires a chartered Working Group,
consensus, wide review and implementation experience.

## Evidence

- Anyone with a W3C account can propose a Community Group and obtain four
  additional supporters. Membership is not required and Community Group
  participation has no fee. Locator: W3C Community Groups, lines 79–94,
  retrieved 2026-08-30.
- Community Group reports carry royalty-free patent commitments and permissive
  copyright terms; a final report can request the stronger Final Specification
  Agreement. Locator: same page, lines 98–105, retrieved 2026-08-30.
- Community and Business Group reports are not W3C Standards. Recommendation
  Track work adds security, privacy, accessibility and internationalization
  review plus broader interoperability work. Locator: same page, lines 108–116,
  retrieved 2026-08-30.
- The W3C Process states that standards quality depends on consensus, public and
  member review, implementation and interoperability experience. Locator: W3C
  Process Document dated 2025-08-18, lines 216–226, retrieved 2026-08-30:
  https://www.w3.org/policies/process/.

## Mechanism

Incubation develops a problem statement, scope, use cases, draft text, tests and
an implementer community. Recommendation-track transition occurs only when W3C
members support a charter and resource the work. Candidate Recommendation and
Recommendation advancement then use formal review and implementation evidence.

## NUIF relevance

**Borrow** Community Group participation for coordination with DTCG and Web
stakeholders after an external implementation exists.

**Reject** creating a NUIF Community Group before the project has external
participants and a bounded implementer draft. A one-project group would add
process without demonstrating stakeholder demand.

## Open questions

- W3C is appropriate only if the interoperable scope is primarily Web-facing
  and browser or design-tool stakeholders commit implementation resources.
- A general cross-platform package format may fit a foundation specification
  process better than the W3C Recommendation Track.
