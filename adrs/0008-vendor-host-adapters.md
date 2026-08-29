---
id: nuif:adr:0008
status: accepted
---

# ADR 0008: Vendor products integrate through host adapters

Decision delegated to research on 2026-08-30. Evidence:
`nuif:research:figma-plugin-and-rest-api-as-automation-surface`,
`nuif:research:figma`, and `nuif:research:adobe-uxp-host-integration`.

## Context

The native NUIF editor is a reference implementation and conformance tool. It
is not a plug-in runtime for every design product. Figma and Adobe applications
already own their document stores, undo systems, permissions, extension
runtimes and distribution channels. Embedding the NUIF editor executable would
duplicate the host shell and would not provide access to the host document.

Source adapters use byte spans because they synchronize retained source. Host
APIs expose objects and properties instead of source bytes, so they require a
different correspondence record and evidence report.

## Decision

1. A vendor product integrates NUIF through a host adapter that maps its public
   object API to canonical NUIF documents and operations. It does not embed the
   NUIF editor application.
2. `nuif-adapter::HostAdapterReport` is the common evidence envelope for API
   hosts. It records profile, direction, host/API versions, optional host
   revision, canonical hash, host-object correspondence, fidelity and
   unmapped-data preservation.
3. Every host and product has a separately versioned bounded profile. “Figma”
   and “Adobe” are not fidelity claims by themselves.
4. Figma uses a user-run TypeScript/JavaScript Plugin API bridge with dynamic
   page loading. The default package has no network access. UI-frame file
   transfer and host-object mutation are separate message channels.
5. Adobe UXP delivery is one `.ccx` package per supported host. The initial
   profile targets InDesign. Photoshop is separate and performs all mutations
   inside one `executeAsModal` transaction. Illustrator remains unclaimed until
   its current public SDK and package contract are researched and tested.
6. Host correspondence uses host object identifiers plus namespaced NUIF
   metadata when the host documents such persistence. Duplicate or missing
   persisted identifiers are repaired with new NUIF identifiers and reported;
   silent identity reuse is forbidden.
7. Vendor plug-in versions are independent of the native editor version. A
   plug-in declares the NUIF profile/spec revisions it supports and runs the
   same checked-in fixtures before publication.
8. GitHub Actions may build and retain review artifacts. Publication into
   Figma Community or Adobe Marketplace remains a separately authenticated,
   host-governed release step.

## Consequences

- Vendors can adopt NUIF without adopting the reference editor UI or Rust.
- API and source adapters share fidelity semantics while using correspondence
  appropriate to their medium.
- A credential-free repository can test pure mapping logic and checked-in host
  snapshots, but live-host claims require the named host/version evidence.
- Marketplace identifiers, approvals and publisher accounts remain outside the
  repository and cannot be inferred from a GitHub release.
