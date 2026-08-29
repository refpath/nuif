---
id: nuif:research:adobe-uxp-host-integration
kind: standard
status: reviewed
title: Adobe UXP host integration, permissions, mutation boundaries, and distribution
source:
  url: https://developer.adobe.com/indesign/uxp/plugins/concepts/manifest/
  authors: [Adobe]
  published_at: "Adobe UXP documentation retrieved 2026-08-30"
  license: proprietary API documentation; facts only recorded here
retrieved_at: 2026-08-30
tags: [adobe, uxp, photoshop, indesign, plugin, adapter, distribution, permissions]
confidence: 0.94
claims: [nuif:claim:semantic-automation, nuif:claim:opaque-preservation]
relations:
  - type: related_to
    target: nuif:research:figma-plugin-and-rest-api-as-automation-surface
    note: Both systems expose user-installed host plugins, but their document APIs and package channels are host-specific.
  - type: supports
    target: nuif:claim:semantic-automation
    note: Photoshop and InDesign expose document object models to UXP scripts and plugins without pointer automation.
  - type: supports
    target: nuif:claim:opaque-preservation
    note: Photoshop's UXP XMP module supplies a namespaced metadata path for adapter identity and provenance.
links:
  spec: [spec/07-extensions-and-dialects.md, spec/12-cli-api-and-automation.md]
  adr: [adrs/0008-vendor-host-adapters.md]
  rfc: []
  code: [crates/nuif-adapter/src/lib.rs, adapters/adobe/PROFILE-DRAFT.md, docs/HOST-INTEGRATION.md]
  experiments: []
---

# Summary

UXP is a JavaScript/HTML plugin runtime whose manifest selects one Adobe host,
entry points and explicit permissions. The current InDesign manifest reference
lists Photoshop (`PS`), InDesign (`ID`) and XD (`XD`) host identifiers. A
plugin can request user-mediated filesystem access without requesting full disk
access, and can omit network permission entirely. InDesign supports UXP scripts
from version 18.0 and packaged plugins from 18.5. A plugin is distributed as a
host-specific `.ccx` package through direct distribution or the Creative Cloud
Marketplace; a Marketplace listing requires a Developer Distribution plugin
identifier and review.

Photoshop exposes a UXP document object model and a lower-level `batchPlay`
action-descriptor API. Adobe recommends the document object model first and
`batchPlay` for gaps. Every state-changing call must run within
`core.executeAsModal`, which provides exclusive mutation scope, cancellation,
progress and history suspension. The UXP XMP module can read and write
namespaced metadata, providing a place for NUIF identity/provenance when the
host document type preserves XMP.

NUIF should therefore ship one adapter profile and package per Adobe host, not
one generic "Adobe" binary. The first public profile should target InDesign
pages and simple page items because its document model is authored layout. A
Photoshop profile must remain narrower and classify responsive layout,
components and interactions as unsupported or preserved metadata. No retrieved
primary source establishes an Illustrator UXP host identifier, so an
Illustrator package must use a separately researched public SDK rather than
assuming the InDesign/Photoshop UXP contract applies.

## Evidence

- A UXP manifest defines one `host`, entry points and permissions. The current
  host union in the InDesign reference is `PS`, `ID` or `XD`; incompatible
  plugins do not install or appear for that host. `localFileSystem` can be
  `plugin`, `request` or `fullAccess`, while network, process launch, webview
  and inter-plugin communication require separate declarations. Locator: Adobe
  InDesign UXP, *Plugin manifest*, `HostDefinition` and
  `PermissionsDefinition`, retrieved 2026-08-30:
  https://developer.adobe.com/indesign/uxp/plugins/concepts/manifest/.
- InDesign supports `.idjs` UXP scripts from 18.0 and plugins from 18.5.
  Scripts have modal UI only; plugins may expose commands or persistent panels.
  Packaged plugins use `.ccx`. Locator: *UXP Scripts and Plugins*, comparison
  table, retrieved 2026-08-30:
  https://developer.adobe.com/indesign/uxp/introduction/next-steps/script-and-plugin/.
- UXP file access is sandboxed by default. InDesign documents
  `localFileSystem: request` as the user-mediated choice and warns against
  asking for `fullAccess` without need. Locator: *File operations*, manifest
  permission and sandbox sections, retrieved 2026-08-30:
  https://developer.adobe.com/indesign/uxp/resources/recipes/file-operation/.
- InDesign packages are created by UXP Developer Tool. A Marketplace package
  needs an identifier from Developer Distribution; packaged output should be
  installed and tested before publication. Locator: *Packaging*, retrieved
  2026-08-30:
  https://developer.adobe.com/indesign/uxp/introduction/next-steps/distribution/packaging/.
- Adobe documents Marketplace and direct `.ccx` distribution as separate
  channels. Direct packages show trust warnings; Marketplace submission uses
  Developer Distribution. Locator: *Distribution Options*, retrieved
  2026-08-30:
  https://developer.adobe.com/indesign/uxp/introduction/next-steps/distribution/distribution-options/.
- Photoshop UXP exposes `require('photoshop').app`, including active/open
  documents. State changes must execute through `core.executeAsModal`.
  Locator: Photoshop UXP, *Photoshop API*, overview and modal example,
  retrieved 2026-08-30:
  https://developer.adobe.com/photoshop/uxp/ps_reference/.
- Adobe describes `batchPlay` as the lower-level action-descriptor API and
  recommends the document object model before using it. Object IDs are
  preferred to indices because indices can change during the session. Locator:
  *BatchPlay Details*, overview and action references, retrieved 2026-08-30:
  https://developer.adobe.com/photoshop/uxp/ps_reference/media/batchplay/.
- `executeAsModal` gives one plugin exclusive mutation access, exposes
  cancellation/progress, and provides the history-state boundary for new code.
  Locator: *Modal Execution in an Async World*, retrieved 2026-08-30:
  https://developer.adobe.com/photoshop/uxp/ps_reference/media/executeasmodal/.
- The UXP XMP module reads, modifies and serializes namespaced metadata and can
  operate on host-provided packets or files. Locator: `require('uxp').xmp`,
  retrieved 2026-08-30:
  https://developer.adobe.com/photoshop/uxp/2022/uxp-api/reference-js/modules/uxp/xmp/getting-started/xmp/.
- Adobe's Illustrator developer landing page describes HTML panels but the
  retrieved UXP host manifest does not list Illustrator. This is recorded as
  an evidence gap, not as proof that Adobe has no private or other Illustrator
  SDK. Locator: Illustrator developer landing page, retrieved 2026-08-30:
  https://developer.adobe.com/illustrator/.

## Mechanism

An InDesign adapter panel requests a NUIF file with
`localFileSystem: request`, parses the canonical document under NUIF resource
limits, maps one declared page/page-item subset through the host DOM, and emits
a `HostAdapterReport`. The report records the InDesign version, UXP API
version, direction, profile, canonical hash, host-object correspondence and
per-property fidelity. Export performs the inverse mapping and writes the NUIF
document plus the same report contract.

A Photoshop adapter uses the DOM for covered document/layer operations and
isolated `batchPlay` descriptors only for covered gaps. All host mutations run
inside one cancellable `executeAsModal` call and one history state. Stable
NUIF identifiers are stored in a NUIF XMP namespace only when the target file
and workflow preserve XMP; otherwise the report marks identity as session-only
and synchronization cannot be claimed.

Pure UXP JavaScript is the default delivery because it is host portable at the
plugin level and avoids native architecture packaging. A hybrid/native plugin
is justified only after profiling proves that canonical parsing or conversion
cannot meet the bounded profile in JavaScript. Each production `.ccx` targets
one host and has a version stream independent from the NUIF editor.

## NUIF relevance

**Borrow** explicit least-privilege permissions, host-version gates, modal
mutation scope, user cancellation and host-specific package compatibility.

**Adapt** Photoshop history suspension and Figma undo grouping to NUIF's
transaction boundary. Every import/export produces a host report rather than
claiming source-byte spans that an API host does not have.

**Reject** one generic Adobe adapter, unconditional `fullAccess`, unrestricted
network access, raw `batchPlay` as the primary model, and an Illustrator UXP
claim without a current primary contract.

## Open questions

- Which InDesign object properties can retain a NUIF entity identifier through
  copy, package, IDML export and reopen?
- Which Photoshop file formats and save paths preserve a custom NUIF XMP
  namespace byte-for-byte?
- Which current public Illustrator SDK is appropriate for a bounded vector
  document profile, and what is its package/update contract?
- Does a pure JavaScript canonical NUIF parser meet the same hostile-input time
  and memory ceilings in each UXP host?
