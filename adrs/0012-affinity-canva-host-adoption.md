---
id: nuif:adr:0012
kind: adr
status: accepted
---

# ADR 0012: Prioritize Affinity interchange and Canva host adoption

Decision delegated to research on 2026-08-31. Evidence:
`nuif:research:affinity-interchange-and-adoption`,
`nuif:research:canva-apps-and-connect-adoption`,
`nuif:research:svg` and
`nuif:research:figma-plugin-and-rest-api-as-automation-surface`.

This ADR amends the active vendor priorities in ADR 0008. It does not erase the
historical Adobe UXP research or change ADR 0008's common host-report contract.

## Context

The initial vendor plan selected Figma plus host-specific Adobe UXP packages.
The project needs an accessible desktop product for human interchange trials
and a supported programmable product with a realistic public distribution path.
The all-new Affinity is available at no cost and spans vector, photo and layout
work, but the reviewed official surface does not expose a stable public document
API or native file schema. Canva exposes both an in-editor Apps SDK and
off-platform Connect APIs, with explicit capability and review constraints.

Treating these products as equivalent would either overclaim Affinity
automation or reduce Canva to lossy file conversion. They require different
profiles and evidence.

## Decision

1. Affinity replaces Adobe as the active desktop vendor-adoption priority. Its
   first profile is a user-mediated SVG interchange trial over the already
   executable `nuif-svg-0` subset, not a native plug-in or `.af*` parser.
2. Native Affinity files remain opaque provenance. NUIF will not reverse-
   engineer them or use pointer/keyboard UI automation as conformance evidence.
   A future public scripting/document API requires a new profile.
3. Canva is the primary programmable vendor-adoption path. Its first profile
   uses stable Apps SDK Design Editing APIs on one fixed-dimension current page
   and only the documented supported element subset.
4. A Canva import validates a complete bounded plan and applies it with one
   confirmed `sync`. Locked, unsupported, conflicting or expired sessions fail
   before mutation and every loss is represented in `HostAdapterReport`.
5. `nuif-wasm` may be bundled in the Canva iframe under the documented CSP, but
   only the Apps SDK owns host objects and mutation. Remote executable code,
   workers and preview APIs are absent from the public profile.
6. Canva Connect APIs are a secondary OAuth workflow. Because their current
   format lists omit NUIF, SVG/PDF use is explicitly lossy and no native NUIF
   Connect round trip is claimed.
7. The Affinity kit and Canva app each have independent profile and package
   versions. CI may build evidence and review artifacts. Marketplace submission,
   developer verification, approval and release remain explicit authenticated
   human/organization operations.
8. Adobe UXP research remains available as historical prior art and a possible
   contributor-maintained future adapter, but it is removed from the active
   target inventory and roadmap queue.

## Consequences

- Contributors can run the desktop interchange experiment without purchasing
  a separate design application.
- The project distinguishes evidence from a file-interchange host and an API
  host instead of implying that one adapter architecture fits both.
- Affinity coverage initially inherits the narrow SVG exclusions and cannot
  claim native identity, undo or round-trip fidelity.
- Canva offers a realistic source-bundle and marketplace adoption route, but
  API stability, review, identity disclosure, OAuth and live-host evidence are
  external gates.
- Historical research remains auditable without steering implementation toward
  a vendor path that is no longer prioritized.
