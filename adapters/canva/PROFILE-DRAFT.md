# Draft Canva Design Editing profile 0

Status: executable pure normalized mapping and compiled credential-free review
shell; no live-host conformance claim.

Profile identifier: `nuif-canva-design-editing-0`.

Primary evidence: `nuif:research:canva-apps-and-connect-adoption` and ADR 0012.
Executable implementation: `crates/nuif-canva` and `adapters/canva/app`;
conformance gate: `cargo xtask gate-canva`.

## Host and scope

- A public Canva Apps SDK v2 app using generally available `@canva/design`
  APIs and the Design Editor intent.
- One unlocked `current_page` session whose page is `absolute` and has fixed
  dimensions.
- The pure normalized mapper represents groups, rectangles, canonical ellipses,
  rich-text elements, an optional solid page background, ordered layering,
  position, size, rotation, transparency and the explicitly mapped solid-color
  and text properties. Page and element names are nullable because the public
  current-page API does not expose them.
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
page at 16,384 traversed elements, 4,096 rich-text UTF-16 code units per element,
1 MiB of normalized metadata and a 16 MiB NUIF input. Limit-plus-one inputs must
fail before `sync`. These are candidate limits pending live time and allocation
calibration.

The compiled review shell narrows live import further to unnamed opaque
rectangles and canonical ellipses on an empty page of the same dimensions. It
can set an absent or opaque solid page background. Groups, text, alpha and
names remain in the pure schema for fixture and future-host evaluation but are
rejected before live insertion because Apps SDK v2 cannot currently establish
the stable element identity, writable names, font-file identity and exact text
box metrics required by this profile. This narrowing is executable policy, not
a claim that the host lacks those visual features.

## Browser and package boundary

The app iframe may bundle `nuif-wasm-api-0`; Canva's current CSP permits
packaged WebAssembly but forbids third-party scripts, nested frames and workers.
The module parses and validates NUIF locally. Only the Apps SDK reads or mutates
Canva objects. The build declares no remote code, and any optional backend is a
separate authenticated feature with explicit data disclosure.

`adapters/canva/app` pins `@canva/design` 2.12.0. Its published declaration
currently contains one invalid empty statement after `PageRefList`. The build
creates a type-check-only normalized copy, refuses any source shape other than
the one audited defect, and records source and normalized hashes. Runtime code
still bundles the untouched official module; the normalized declaration is not
a replacement SDK or a distributable fork.

## Correspondence and fidelity

Stable Canva page identifiers and session object references are recorded in a
  `HostAdapterReport`. The pure profile uses explicit `nuif-doc:`, `nuif-page:`
  and `nuif:` round-trip markers for generated fixtures; real host IDs are
  deterministically repaired and reported, never silently reused. Persistence
  across duplicate, reorder, close/reopen and copy still requires live trials.

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
CI may publish the license-scoped review evidence artifact but does not submit
or release an app through Canva.

The static gate currently produces `target/canva-app-shell-report.json`, the
Rust plan/TypeScript validation and exact round-trip fixtures, an informational
maximum-profile benchmark and `target/nuif-canva-review-app`. It repeats the
bundle and requires byte/hash-identical reports. All live-host fields remain
`not_run`.
