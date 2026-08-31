# Draft Canva Design Editing profile 0

Status: researched Apps SDK mapping specification; no reviewed Canva app or
live-host conformance claim.

Profile identifier: `nuif-canva-design-editing-0`.

Primary evidence: `nuif:research:canva-apps-and-connect-adoption` and ADR 0012.

## Host and scope

- A public Canva Apps SDK v2 app using generally available `@canva/design`
  APIs and the Design Editor intent.
- One unlocked `current_page` session whose page is `absolute` and has fixed
  dimensions.
- Groups, rectangles, shapes and rich-text elements with ordered layering,
  position, size, rotation, transparency and the explicitly mapped solid-color
  and text properties.
- Image content only after a separate resource-upload and media-fill profile is
  admitted; profile 0 records existing image/video fills as unsupported.

Canva Docs, whiteboards and other unbounded pages, all-pages editing, tables,
embeds, video, gradients, unavailable fonts, locked content, app elements,
behaviors and preview APIs are excluded. All-pages editing may be reconsidered
under a new profile after the documentation and package used for review agree
on its stable status.

## Transaction boundary

The app opens one session, rejects unsupported or locked input, creates a
complete normalized snapshot or mutation plan, validates every bound, asks for
user confirmation and calls `sync` once. A failed validation, expired session,
conflict or API error leaves the live design unchanged. A successful import is
one Canva undo action. The app does not replace the complete page with an
opaque app element or flattened screenshot.

Sessions are limited to one minute by the host. Profile 0 therefore caps one
page at 16,384 traversed elements, 4,096 rich-text code points per element,
1 MiB of normalized metadata and a 16 MiB NUIF input. Limit-plus-one inputs must
fail before `sync`. These are candidate limits pending live time and allocation
calibration.

## Browser and package boundary

The app iframe may bundle `nuif-wasm-api-0`; Canva's current CSP permits
packaged WebAssembly but forbids third-party scripts, nested frames and workers.
The module parses and validates NUIF locally. Only the Apps SDK reads or mutates
Canva objects. The build declares no remote code, and any optional backend is a
separate authenticated feature with explicit data disclosure.

## Correspondence and fidelity

Stable Canva page identifiers and session object references are recorded in a
`HostAdapterReport`. No persistence of a custom NUIF entity identifier is
claimed until a generally available Canva metadata surface survives duplicate,
reorder, close/reopen and copy trials. Missing or duplicate portable identity
is repaired in the NUIF mapping and reported, never silently reused.

Every supported property has a correspondence entry. Every unsupported Canva
element or property produces item-level fidelity. Images represented as rect
fills, rich-text range conversion and layering by list order are mapped as
Canva semantics rather than forced into a fictitious vendor-neutral object.

## Connect API boundary

Connect API imports and exports are a separate server-side workflow, not this
host profile. They require OAuth scopes and asynchronous jobs. The current
import list includes Affinity files but not NUIF; current exports do not include
NUIF. SVG or PDF bridges must therefore carry an explicit lossy fidelity report.
Native NUIF support requires Canva to admit a NUIF media type and publish the
associated semantic contract.

## Required promotion evidence

- checked-in normalized current-page fixtures and canonical expected NUIF;
- exact repeated pure mapping in both declared directions;
- official SDK typecheck and deterministic single-file review bundle;
- CSP audit proving no remote code, workers or nested frames;
- locked, unsupported, expired-session, conflict and limit-plus-one failures
  before mutation;
- one-sync/one-undo and cancellation trials in a named Canva host/API version;
- public-review checklist, privacy disclosure and developer-verification
  readiness without credentials in the repository.

Marketplace submission and publication remain owner-authenticated operations.
CI may produce the review bundle but does not submit or release it.
