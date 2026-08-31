---
id: nuif:research:canva-apps-and-connect-adoption
kind: standard
status: reviewed
title: Canva Apps SDK and Connect API adoption surface
source:
  url: https://www.canva.dev/docs/apps/design-editing/
  authors: [Canva]
  published_at: "Canva developer documentation retrieved 2026-08-31"
  license: proprietary API documentation; facts only recorded here
retrieved_at: 2026-08-31
tags: [canva, apps-sdk, connect-api, adapter, wasm, oauth, marketplace, distribution]
confidence: 0.96
claims: [nuif:claim:semantic-automation, nuif:claim:opaque-preservation]
relations:
  - type: related_to
    target: nuif:research:affinity-interchange-and-adoption
    note: Canva's Connect API imports Affinity files, while Apps SDK design editing is the stronger programmable NUIF adoption surface.
  - type: related_to
    target: nuif:research:figma-plugin-and-rest-api-as-automation-surface
    note: Both products expose iframe-based user-installed editor apps, but their element, undo, permission and review contracts differ.
  - type: related_to
    target: nuif:research:wasm-headless-execution
    note: Canva's current iframe CSP explicitly admits packaged WebAssembly but forbids third-party scripts and workers.
  - type: supports
    target: nuif:claim:semantic-automation
    note: The Design Editing API exposes typed page snapshots, supported element CRUD and an explicit sync boundary.
links:
  spec: [spec/07-extensions-and-dialects.md, spec/12-cli-api-and-automation.md]
  adr: [adrs/0012-affinity-canva-host-adoption.md]
  rfc: []
  code: [adapters/canva/PROFILE-DRAFT.md, adapters/canva/app/src/host.ts, adapters/canva/app/src/protocol.ts, docs/HOST-INTEGRATION.md, crates/nuif-canva/src/lib.rs, crates/nuif-wasm/src/lib.rs, xtask/src/main.rs]
  experiments: [nuif:experiment:canva-design-editing-adapter]
---

# Summary

Canva exposes two materially different integration surfaces. Apps SDK code runs
inside the editor and the Design Editing API can read and edit supported page
ingredients. Connect APIs are OAuth-protected server APIs for off-platform
workflows such as import, export and return navigation. The Apps SDK is the
primary semantic NUIF adoption path; Connect is a secondary workflow bridge and
cannot currently import or export NUIF natively.

The first profile uses only generally available `current_page` Design Editing
APIs on one fixed-dimension page. The pure mapper normalizes the wider declared
schema; the compiled review shell imports only unnamed opaque rectangles and
canonical ellipses on an empty same-size page, validates the complete plan,
asks for confirmation and calls `sync` once. Canva's documented preview
restriction, product-specific element model, SDK license and app review process
make a broader “Canva adapter” claim unsound. Static CI is now executable; a
named live Canva trial remains unperformed.

## Evidence

- A Canva app is JavaScript embedded in a side-panel iframe. Canva injects API
  packages including `@canva/design`; the app does not gain direct access to an
  undocumented document file. Locator: Apps SDK, *Integrating with Canva*,
  basic app and API package sections, retrieved 2026-08-31:
  https://www.canva.dev/docs/apps/integrating-canva/.
- `openDesign` provides page snapshots, helpers and `sync`. Sessions expire
  after one minute. Supported pages are `absolute`; unsupported pages cannot be
  read or edited, and Canva Docs are explicitly incompatible. Fixed and
  unbounded absolute pages are distinguished and pages carry stable `PageId`
  values. Locator: Apps SDK, *Design Editing API*, core concepts, sessions and
  pages, retrieved 2026-08-31:
  https://www.canva.dev/docs/apps/design-editing/.
- The Design Editing API provides CRUD only for supported elements: embeds,
  groups, rects, shapes and text. Images and videos are represented as rectangle
  fills, text is exposed as rich-text ranges, and tables are unsupported.
  Layering is list order rather than a `z-index`. Locator: same document,
  elements, element types and layering sections.
- Snapshot changes affect the live design only after `sync`; Canva's design
  guidelines recommend applying a logical change as one operation so the user
  can undo it as one action. Apps must present critical unsupported/failure
  states and must not unexpectedly replace or delete the whole design. Locator:
  Apps SDK, *Design Editing API design guidelines*, changing a design and error
  sections, retrieved 2026-08-31:
  https://www.canva.dev/docs/apps/design-guidelines/design-editing-api/.
- The general Design Editing guide still labels `all_pages` as preview while
  the GA package changelog says multi-page editing was promoted to GA. This
  documentation inconsistency is why profile 0 remains on the unambiguous GA
  `current_page` call. Locators: design editing guide above; `@canva/design` GA
  changelog, retrieved 2026-08-31:
  https://www.canva.dev/docs/apps/api/latest/design-changelog/.
- Canva states that preview packages may change without a new version and apps
  using them cannot pass public app review. The same rule applies to preview
  Connect features. Locators: Apps SDK, *Integrating with Canva*, preview APIs;
  Connect API overview, preview APIs, retrieved 2026-08-31:
  https://www.canva.dev/docs/apps/integrating-canva/ and
  https://www.canva.dev/docs/connect/.
- Canva's app iframe CSP allows `wasm-unsafe-eval` but blocks third-party
  JavaScript, frames and web workers. This permits a bundled `nuif-wasm` module
  but not remote executable code or worker-based assumptions. Locator: Apps
  SDK, *Content Security Policy*, allowed features and directives, retrieved
  2026-08-31:
  https://www.canva.dev/docs/apps/content-security-policy/.
- Connect design imports are asynchronous byte uploads requiring OAuth bearer
  authorization and `design:content:write`; the endpoint is rate-limited to 20
  requests per minute per user. The supported list includes native Affinity
  extensions, PDF and other formats, but not NUIF. Locator: Connect APIs,
  *Create design import job* and *Design imports*, retrieved 2026-08-31:
  https://www.canva.dev/docs/connect/api-reference/design-imports/create-design-import-job/
  and https://www.canva.dev/docs/connect/api-reference/design-imports/.
- Connect exports are asynchronous and currently support JPG, PNG, GIF, PPTX,
  MP4, PDF, CSV, HTML bundle and standalone HTML; completed download URLs expire
  after 24 hours. The endpoint requires `design:content:read` and has integration,
  document and user throttles in addition to the per-user request limit.
  Locator: Connect APIs, *Create design export job*, retrieved 2026-08-31:
  https://www.canva.dev/docs/connect/api-reference/exports/create-design-export-job/.
- A public app is submitted as a source bundle through the Developer Portal,
  needs listing/testing material and Canva review, and can be released only
  after approval. Marketplace developers must provide identity and legal-entity
  information; team apps use an Enterprise-team review path. Released or
  rejected apps are changed by creating a new app version. Locators: Apps SDK,
  *Submitting apps*, *App review process*, *Developer verification* and *App
  versioning*, retrieved 2026-08-31:
  https://www.canva.dev/docs/apps/submitting-apps/,
  https://www.canva.dev/docs/apps/app-review-process/,
  https://www.canva.dev/docs/apps/developer-verification/ and
  https://www.canva.dev/docs/apps/versioning-apps/.
- The stable `@canva/design` 2.12.0 declaration exposes a page ID but no element
  ID or writable element/page name. Its create-text options expose width and
  rich-text regions but no explicit text-box height or portable font-file hash.
  These omissions prevent an exact live mapping of NUIF names, persistent
  element identity and pinned text metrics. Locator: the exact registry package
  locked by `adapters/canva/app/package-lock.json`, `index.d.ts`, namespaces
  `DesignEditing.AbsolutePage`, `Element`, `CreateTextElementOpts` and
  `TextRegion`, integrity
  `sha512-oR6b8avm6krC2VO/MPzVNvh7BfUSNhddyDFBhW9U3/kKCxWXV0Cnckd+FuB1JtpXc7F9lsfJQXYog3qapVmYSA==`.
- That same package contains one syntactically invalid empty statement after
  `DesignEditing.PageRefList`, and its license restricts components and
  derivatives to permitted apps on the Canva Platform while requiring the
  license in every copy. The review build therefore verifies and removes only
  that statement in a generated type-check copy, records both hashes, bundles
  the untouched runtime module, includes the license and labels the artifact
  Canva-only. Locator: locked package `index.d.ts`, `LICENSE.md`,
  `adapters/canva/app/scripts/prepare-types.mjs` and generated build report.

## Options considered

### Apps SDK Design Editing API

Selected as the primary path. It exposes typed semantic objects, a transaction-
like sync boundary, user-visible undo and a public distribution channel. The
first profile remains narrow enough to test without inventing Canva semantics.

### Connect API native NUIF workflow

Not currently possible. NUIF is absent from the documented import/export format
lists. Connect can route SVG/PDF with explicit loss or return users to a design,
but cannot prove element-level NUIF round trips. Native NUIF support is an
upstream adoption request, not an adapter feature that this repository can
declare.

### Render or app-element flattening

Rejected. Replacing a page with a screenshot or one opaque app element defeats
editable semantics and conflicts with Canva's design-editing guidance. A render
may be diagnostic evidence only.

### Preview API dependency

Rejected for the public profile. Preview behavior can change without versioning
and blocks public review. Preview experiments may be kept in a separate branch
and evidence class, never in a release bundle.

### Rust-generated plan versus direct WASM document loading

The first review shell accepts a Rust-generated, lossless mutation plan. This
keeps canonical document semantics in `nuif-core`; TypeScript validates only an
untrusted transport and translates the admitted subset into host states. It is
smaller and easier to audit than adding a second document model. Direct `.nuif`
loading through `nuif-wasm-api-0` remains a compatible later entry path, but it
does not remove the host preflight or prove Canva fidelity. It should be added
only when the live transaction trial establishes that direct in-app authoring
is worth the extra package surface.

## Adoption and release path

1. Completed: pure Canva snapshot and mutation-plan types have fixtures,
   bounds, canonical round trips and `HostAdapterReport` output.
2. Completed for static review: the no-network single-file shell compiles the
   stable API surface through an audited declaration normalization, validates
   Rust plans, enforces explicit confirmation and empty-page preflight, tests a
   one-sync mock transaction, records maximum-profile measurements, includes
   the SDK license and packages credential-free CI evidence. It deliberately
   consumes plans rather than embedding WASM in this first boundary.
3. Next: run named live-host trials for read, import, one-sync undo, cancellation,
   locked content, conflicts, session expiry and unsupported ingredients.
4. CI publishes the Canva-only review artifact with digests, exact package
   identities, normalized-declaration evidence and fixture reports. A tagged
   general NUIF release may retain this as verification evidence, but must not
   redistribute it as a general browser SDK. Submission remains a manual
   authenticated action; a Git tag must never publish the app automatically.
5. After human developer verification and Canva approval, create versions in
   the Developer Portal for reviewed updates. Keep the app version independent
   from editor, WASM and format versions.
6. Propose native NUIF MIME import/export to Canva only after the profile has
   public fixtures, independently reviewed fidelity and demonstrated user
   demand. The proposal must specify package safety, unknown-extension retention
   and profile negotiation rather than asking Canva to adopt the entire draft.

## NUIF relevance

**Borrow** Canva's explicit iframe, permission, session, sync, review and
distribution boundaries. They are a useful model for a host adapter that makes
mutation authority and user-visible undo explicit.

**Adapt** `HostAdapterReport`, canonical NUIF operations and a bounded plan
envelope to the Apps SDK edge. WASM is optional for direct in-app `.nuif`
loading, not a prerequisite for a thin host plan consumer. Keep Connect API
OAuth, rate limits, temporary URLs and privacy policy outside the deterministic
core.

**Reject** preview APIs in a public profile, opaque app-element flattening,
remote executable code and any native NUIF interoperability claim until Canva
publishes the corresponding MIME and semantic contract.

## Open questions

- Which element IDs, metadata fields or app-owned data survive duplicate,
  reorder, close/reopen and cross-design copy under generally available APIs?
- Does the review bundle execute its single-sync transaction identically under
  the documented CSP on every supported Canva browser/desktop host?
- Which rich-text, custom-path, image-fill crop and font semantics can be mapped
  exactly without undocumented assumptions?
- When the all-pages documentation and GA package surface agree, what page-
  ordering, session-expiry and atomicity rules are needed for a separate
  multi-page profile?
