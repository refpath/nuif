---
id: nuif:research:affinity-interchange-and-adoption
kind: standard
status: reviewed
title: Affinity interchange surface and NUIF adoption path
source:
  url: https://www.canva.com/newsroom/news/all-new-affinity/
  authors: [Canva]
  published_at: 2025-10-29
  license: public product announcement and documentation; facts only recorded here
retrieved_at: 2026-08-31
tags: [affinity, canva, adapter, svg, pdf, interchange, adoption]
confidence: 0.94
claims: [nuif:claim:semantic-automation, nuif:claim:opaque-preservation]
relations:
  - type: related_to
    target: nuif:research:canva-apps-and-connect-adoption
    note: Affinity is part of Canva and Canva's import API accepts native Affinity file extensions, but this does not expose the Affinity file schema.
  - type: related_to
    target: nuif:research:svg
    note: Documented SVG import/export is the narrowest current NUIF bridge that does not depend on an opaque native Affinity format.
  - type: compares_to
    target: nuif:research:adobe-uxp-host-integration
    note: The earlier UXP plan has a public object API; Affinity is more accessible for desktop trials but currently offers only a documented interchange boundary for this project.
  - type: supports
    target: nuif:claim:opaque-preservation
    note: Native Affinity files must remain opaque provenance until a public schema or supported API exists.
links:
  spec: [spec/07-extensions-and-dialects.md, spec/12-cli-api-and-automation.md]
  adr: [adrs/0012-affinity-canva-host-adoption.md]
  rfc: []
  code: [adapters/affinity/PROFILE-DRAFT.md, docs/HOST-INTEGRATION.md]
  experiments: [nuif:experiment:affinity-svg-bridge]
---

# Summary

The all-new Affinity is a no-cost desktop application combining vector, photo
and page-layout tools. That lowers the participation cost for live foreign-host
trials and makes it a useful desktop adoption target. The reachable technical
surface is much narrower than the product surface: official Affinity material
documents SVG/PDF and other file interchange, but the reviewed material does
not publish a stable document-object API, scripting SDK or schema for native
Affinity files.

NUIF should therefore use Affinity first as a user-mediated interchange oracle,
not pretend that it has a native plug-in. The first profile composes the existing
bounded `nuif-svg-0` adapter with a named Affinity import/export trial. Native
`.af`, `.afdesign`, `.afphoto` and `.afpub` bytes are opaque evidence. A public
Affinity API or native NUIF import would justify a new profile later.

## Evidence

- Canva announced the all-new Affinity as one application combining photo
  editing, vector design and page layout and stated that it is free for
  everyone. The download uses an existing or newly created free Canva account.
  Locator: Canva Newsroom, *Introducing the all-new Affinity: Professional
  design, now free for everyone*, announcement and availability sections,
  2025-10-29:
  https://www.canva.com/newsroom/news/all-new-affinity/.
- The official Affinity Designer 2 feature material documents a shared Affinity
  file family, PDF import, SVG import/export and export of slices, layers,
  pages and artboards to SVG/PDF and raster formats. This establishes file
  interchange, not the internals of the all-new native encoding. Locator:
  Affinity Designer 2, *Key features*, interoperability and file control/import/
  export sections, retrieved 2026-08-31:
  https://affinity.help/designer2/en-US.lproj/pages/Introduction/keyFeatures.html.
- Canva's current Connect API design-import overview lists
  `application/affinity` and the `.af`, `.afdesign`, `.afphoto` and `.afpub`
  extensions as accepted input. It also lists PDF, AI and PSD. The endpoint
  accepting bytes does not publish the Affinity object model or return native
  Affinity structure. Locator: Canva Connect APIs, *Design imports*, supported
  file formats, retrieved 2026-08-31:
  https://www.canva.dev/docs/connect/api-reference/design-imports/.
- A search of the official Affinity product/help sitemaps and current developer-
  oriented material on 2026-08-31 located no published scripting or document-
  object reference. A staff response on the official forum to a 2025 request
  for JavaScript API access said there was no additional release information.
  This is an evidence gap, not proof that no private or future API exists.
  Locator: official Affinity forum, *2025 Plugin Development with API Access
  using JavaScript*, staff response, retrieved 2026-08-31:
  https://forum.affinity.serif.com/index.php?/topic/228053-2025-plugin-development-with-api-access-using-javascript/.

## Options considered

### Native Affinity parser

Rejected for the first profile. File extensions and Canva import support do not
constitute a schema. Reverse engineering would create a release-by-release
compatibility burden, uncertain preservation behavior and an unsupported trust
boundary. Native bytes may be retained as content-addressed opaque evidence.

### Desktop UI automation

Rejected as an adapter contract. Pointer/keyboard automation is fragile,
platform-specific and cannot prove document semantics or atomic mutation. It
may assist a recorded manual trial, but cannot establish an executable profile.

### PDF bridge

Useful for render/reference comparison but not the first editable bridge. PDF
can flatten structure, fonts and authoring intent; it cannot satisfy a semantic
round-trip claim without a narrower PDF profile and property-level fidelity.

### SVG bridge

Selected for the first experiment because both sides document the format and
NUIF already has a strict executable SVG subset. The tradeoff is deliberately
narrow coverage: paths, transforms, CSS, effects and external resources remain
excluded until the NUIF SVG profile itself admits them.

## Adoption path

1. Publish a small versioned fixture kit containing canonical NUIF, bridge SVG,
   expected report and render reference for the existing SVG subset.
2. Run user-mediated import/export trials in named Affinity and operating-
   system versions. Retain both SVGs, native file only as opaque provenance,
   renders, environment and property-level fidelity.
3. Require a second reviewer and at least two operating systems before calling
   the bridge interoperable. Do not infer automation, identity persistence or
   undo behavior from file output.
4. Present the fixture kit and capability matrix to the Affinity/Canva team as
   an adoption proposal: native `.nuif` import/export, a documented scripting/
   document API, or a published extension-preservation container would each
   unlock a stronger profile.
5. Version any future API-backed adapter separately. Do not broaden
   `nuif-affinity-svg-bridge-0` in place.

## NUIF relevance

**Borrow** the practical distinction between an authored desktop application
and the file formats it can exchange, plus the value of a no-cost foreign
runtime for repeatable human interoperability trials.

**Adapt** the existing SVG adapter as a checked-in bridge and carry native
Affinity files as digest-pinned opaque provenance with explicit user and
environment evidence.

**Reject** undocumented native-format parsing, pointer automation as a semantic
oracle, and any claim that Canva's ability to ingest an Affinity file exposes
Affinity's internal schema.

## Open questions

- Which SVG constructs does the all-new Affinity preserve structurally across
  import/export on each desktop platform?
- Are IDs or names retained predictably enough for trial-local correspondence,
  and are unknown SVG namespaces preserved or discarded?
- Which exact font, text-range, unit, color-profile and page/artboard settings
  alter output?
- Will Affinity publish a stable scripting/document API or native format
  extension mechanism, and under what compatibility and distribution policy?
