---
id: nuif:research:accessibility-semantics
kind: synthesis
status: verified
title: Portable accessibility semantics, HTML lowering and foreign browser oracles
source:
  url: https://www.w3.org/TR/html-aria/
  authors: [W3C Web Applications Working Group, W3C Accessible Rich Internet Applications Working Group, W3C Browser Testing and Tools Working Group, Microsoft Playwright contributors]
  published_at: 2026-08-11
  license: W3C document license and Apache-2.0 implementation documentation
retrieved_at: 2026-08-31
tags: [accessibility, semantics, aria, html, accessible-name, webdriver, playwright, cross-browser]
confidence: 0.99
claims: []
relations:
  - type: compares_to
    target: nuif:research:open-ui
    note: Open UI studies control anatomy while this record tests the smaller cross-engine accessibility projection boundary.
  - type: related_to
    target: nuif:research:accesskit-semantic-ui-testing
    note: Both use role/name semantics as an observable test surface, but one targets web engines and the other the native editor shell.
links:
  spec: [spec/13-semantics-accessibility-and-behavior.md]
  adr: []
  rfc: []
  code: [crates/nuif-html/src/accessibility.rs, crates/nuif-testing/src/bin/accessibility-mapping.rs, tools/accessibility-oracle/check.mjs, xtask/src/main.rs]
  experiments: [nuif:experiment:accessibility-mapping]
---

# Summary

Portable accessibility needs two distinct contracts: authored semantic intent
in NUIF and target exposure through a host accessibility model. WAI-ARIA
defines roles, states and properties; ARIA in HTML constrains how they may be
used with native elements; Accessible Name and Description Computation defines
the user-agent result; and Core-AAM/HTML-AAM map that result toward platform
APIs. Copying arbitrary ARIA attributes into a document model would therefore
be weaker than a bounded semantic profile with role-specific validity,
deterministic lowering and observed browser results.

The best current test architecture is a small pure Rust projection plus a
foreign three-engine oracle. The projection prefers native HTML semantics,
uses explicit ARIA only where the profile has no exact element, retains stable
entity IDs for attribution and rejects semantics it cannot represent. The
oracle compares computed role, accessible name and supported state rather than
checking emitted attributes alone. Full tree differences are kept separately
from required-subset loss.

## Evidence

- ARIA in HTML is a W3C Recommendation updated 11 August 2026. Its element
  table defines implicit semantics and permitted role/state/property use and
  explicitly discourages redundant explicit roles. This supports native
  `button`, checkbox/radio `input`, `main`, `nav`, named `section` and `p`
  lowering before explicit ARIA. Locator:
  https://www.w3.org/TR/2026/REC-html-aria-20260811/, retrieved 2026-08-31.
- Accessible Name and Description Computation 1.2 defines flat computed names,
  author/content/prohibited name sources and `aria-labelledby` precedence. A
  direct name and label relationship can otherwise disagree, so the bounded
  profile rejects that ambiguity and requires named roles to have a resolvable
  name. Locator: https://www.w3.org/TR/accname-1.2/, retrieved 2026-08-31.
- Core Accessibility API Mappings 1.2 explains that user agents expose web
  roles, values, Boolean states and relationships through differing platform
  APIs. It is a Candidate Recommendation Draft, not evidence that platform
  trees are byte-identical. NUIF must consequently separate semantic loss from
  host-tree difference. Locator: https://www.w3.org/TR/core-aam-1.2/,
  retrieved 2026-08-31.
- WebDriver defines Get Computed Role and Get Computed Label endpoints. Those
  operations confirm that a conformance test must ask the browser for computed
  semantics rather than infer success from source attributes. The current
  endpoints do not expose a complete portable accessibility property bag.
  Locator: https://w3c.github.io/webdriver/#get-computed-role and
  https://w3c.github.io/webdriver/#get-computed-label, retrieved 2026-08-31.
- Playwright 1.62.1 supplies a version-coupled Chromium, Firefox and WebKit set,
  role locators and ARIA snapshots. Its documentation says each Playwright
  release requires particular browser binaries and that its Firefox/WebKit
  builds are patched test engines rather than branded Firefox/Safari. This is
  appropriate as a repeatable foreign oracle, but the report must retain those
  non-claims. Locators: https://playwright.dev/docs/browsers and
  https://playwright.dev/docs/aria-snapshots, retrieved 2026-08-31.

## Mechanism

`nuif-web-accessibility-0` accepts at most 4,096 valid entities and 8,192
relationships. It admits ten roles with explicit name and Boolean-state rules,
maps five relationship kinds through stable entity IDREFs, retains target order
and rejects duplicate targets, unnamed labels, invalid containment, unknown
roles/states/relationships and competing direct/relationship names. Names are
whitespace-normalized before they become oracle expectations. The owned tree
must be acyclic and every owned target has at most one ARIA owner. Output is
inert HTML without scripts, external URLs or invented behavior.

`cargo xtask gate-accessibility` first generates the fixture, expected mapping
and static negative-test report through Rust. It then installs exact
Playwright 1.62.1 browser revisions and asks each engine to locate every NUIF
entity by its computed role, accessible name and supported state. Full body and
per-node ARIA snapshots, versions and mismatch categories are written to
`target/accessibility-mapping-report.json`. The gate is intentionally separate
from the main Rust loop because the three downloaded engines form a larger
foreign-runtime matrix, like sanitizer fuzzing and hosted platform jobs.

The first macOS/arm64 run passed all eleven nodes under Chromium 151.0.7922.34,
Firefox 153.0 and WebKit 26.5. All three produced the same bounded ARIA snapshot
covering all ten admitted roles. This verifies the named projection and oracle
path on one host; CI is configured to produce independent Linux evidence.

## NUIF relevance

NUIF should carry semantic intent independently of visual kind and lower it
through profiles, not vendor-specific accessibility objects in the core. One
authoritative role/name/state/relationship representation can feed HTML,
AccessKit, SwiftUI, Android and other adapters, but each target must report its
own unsupported or approximated surface. Browser agreement is valuable
interoperability evidence, not a substitute for native assistive-technology or
interaction testing.

The current wire model has only one direct accessible name and Boolean states.
It cannot honestly claim direct descriptions, numeric values and levels,
live-region details, table/grid metadata, composite focus management or
behavior. Those additions need schema and operation design before widening the
profile; adapters must not smuggle them through arbitrary strings and call the
result portable.

## Open questions

- Which direct description, numeric value/range, level, orientation and
  live-region fields belong in the portable baseline rather than extensions?
- Beyond profile 0's acyclic single-owner rule, which portable relation model
  can reconcile semantic ownership with DOM containment without relying on
  target-specific tree repair?
- Which keyboard and focus traces are the minimum foreign behavior oracle for
  button, checkbox, radio and switch semantics?
- Which native API harnesses can compare macOS AX, Windows UIA, Linux AT-SPI,
  Android Accessibility and iOS UIAccessibility without reducing them to web
  role strings?
