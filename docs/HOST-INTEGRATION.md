# Host integration and vendor adoption

Figma and Canva use NUIF through small host adapters, not by embedding the
native NUIF editor. Affinity is currently a file-interchange and foreign-runtime
target because no stable public Affinity document-object or scripting API was
located in the reviewed official material. In every case the product continues
to own its canvas, document and undo lifecycle. NUIF maps only a declared
profile and emits evidence describing what was exact, approximated, preserved
or unsupported.

The architectural decision is ADR 0008; ADR 0012 sets the current vendor
priority and evidence boundaries. Primary evidence is recorded in
`nuif:research:figma-plugin-and-rest-api-as-automation-surface` and
`nuif:research:affinity-interchange-and-adoption` and
`nuif:research:canva-apps-and-connect-adoption`.

## Deliverables for a host

A production host integration consists of five independently reviewable parts:

1. A bounded profile that lists mapped host object kinds and properties,
   resource limits, unsupported semantics and identity rules.
2. Pure import/export mapping functions covered by checked-in host snapshots
   and expected canonical NUIF documents.
3. For API hosts, a thin plug-in shell for file selection, host permission
   prompts, undo and user-visible fidelity confirmation. For interchange-only
   hosts, a documented user-mediated trial and exact input/output artifact set.
4. A `nuif-adapter::HostAdapterReport` for every direction. The report records
   the host/API version and revision, canonical hash, host-object
   correspondences, property fidelity and preservation result.
5. A host-specific package or interchange kit, version stream and release
   gate. These artifacts do not inherit the native editor's version.

The portable contract is canonical NUIF plus the report. Rust is optional for a
vendor implementation. A TypeScript or JavaScript plug-in may implement the
same mapping directly from the specification and conformance fixtures.

Rust hosts use the package-aware `nuif-api::NuifDocument` façade documented in
`docs/SDK-AND-BINDINGS.md`. Browser plug-ins use the thin WASM wrapper over that
façade. A stable C/Swift/Kotlin ABI is deliberately not claimed during the
`0.0.x` semantic-API phase; ADR 0011 defines the native-binding promotion gate.

## Browser binding boundary

`nuif-wasm-api-0` packages parsing, validation, canonical text/CBOR,
deterministic `.nuif` load/export, explicit manifest-capability negotiation and
bounded semantic patch/history operations for browser and JavaScript consumers.
It retains verified embedded resources without exposing a second mutable
JavaScript model. The generated module has no filesystem, network, Figma,
Canva or Affinity authority.

When a Figma or Canva integration accepts complete `.nuif` bytes directly, the
module belongs in the host's UI iframe. Figma documents normal browser APIs;
Canva's current content-security policy admits packaged WebAssembly while
blocking third-party scripts, nested frames and workers. The host API still
owns every document mutation. The smaller first review shells instead consume
precomputed, lossless plans and keep canonical processing outside the iframe;
this avoids shipping an unused document runtime while preserving the same host
preflight. Affinity has no equivalent documented embedding boundary, so its
first profile invokes the existing SVG adapter outside the product and uses
user-mediated import/export. WASM can validate, edit and re-encode complete
NUIF packages locally, but it is optional for a plan consumer, is not a host
adapter and cannot justify a vendor fidelity claim by itself.

## Local package evaluation

A Rust host that opens a package passes only its verified embedded resources to
the in-process session:

```rust
let package = NuifPackage::decode(bytes)?;
package.require_capabilities(&host_capabilities)?;
let session = Session::with_resources(
    package.document.clone(),
    package.embedded_resources(),
)?;
let snapshot = session.snapshot(&context)?;
```

Inspection or extraction tools may instead call `capability_report` and retain
unknown required resources without claiming full support. Structural decode
never executes a capability and does not authorize a semantic rewrite: a tool
must negotiate the complete set or explicitly detach the package before
migration. A rendering or behavior host must pass the exact declared supported
set before it presents the package as fully evaluated.

`Session::with_resources` rechecks every SHA-256 binding and enforces count,
single-resource and total-byte limits before it can render. It grants no linked
resource or network authority. Package/session handoff shares immutable byte
buffers, so cloning a package or opening a render session does not duplicate the
complete embedded-resource payload. The CLI and reference editor use this path,
so opening a package containing a `nuif-png-rgba8-0` or
`nuif-png-basic-rgba8-1` image resolves it locally;
a bare document or unresolved link continues to emit item-level fidelity.
Hosts that need authenticated or remote resources keep that policy outside the
session, resolve explicitly, verify against the descriptor, and then create a
new bounded session.

The release allocation trial hands an 8 MiB embedded buffer from package to
session with the same allocation pointer; map/session construction allocates
under 1 MiB and retains under 1 MiB. Scene lowering separately stores one
decoded surface for repeated image uses and enforces a 64 MiB unique decoded
surface total before inflation. These are regression ceilings measured by
`cargo xtask gate-i-package` and `cargo xtask gate-i-image`, not promises about
an arbitrary host allocator.

## Figma path

The executable pure-mapping profile is
`adapters/figma/SNAPSHOT-PROFILE.md`; the live-host promotion contract remains
`adapters/figma/PROFILE-DRAFT.md`.

```text
Figma Plugin main thread
  ├─ reads/writes current PageNode and SceneNode objects
  ├─ owns host mutation and undo grouping
  └─ exchanges typed messages with
       Figma UI iframe
         ├─ user selects or downloads .nuif/report files
         └─ parses/serializes the bounded profile
```

`nuif-figma` implements the normalized JSON boundary between those two
threads. It maps one visible, opaque, fixed-size frame subtree to canonical
NUIF and produces a deterministic mutation-plan tree in the other direction.
`adapters/figma/plugin` compiles the thin main-thread and iframe shell against
the pinned official typings. The release-mode gate covers exact round trips,
identity repair, unsupported-property evidence, hostile bounds, static message
validation, a no-network bundle and a TypeScript fixture imported by Rust. It
does not execute `figma.create*`, font loading, page loading, undo or plug-in
messaging in Figma; those remain live-host promotion evidence.

The manifest template targets only `figma`, requires dynamic page loading and
declares no network domains. Figma assigns the required plug-in ID; a reviewer
passes that assigned value at packaging time, and CI never substitutes a fake
ID. The shell only reads the selected subtree on the current page. Import
validates a precomputed plan, stays disabled until confirmation, removes nodes
created by a failed attempt and commits success as one undo step. Export does
not mutate the file.

Host node IDs are recorded in correspondence evidence. A shared `nuif`
plug-in-data namespace may carry document/entity identifiers for
synchronization, subject to the 100 kB entry limit. Because Figma does not
document every copy/duplicate persistence case, the bridge must scan imported
identifiers, replace duplicates and report the repair. A changed plug-in ID
must not be used as the only identity store because private plug-in data becomes
inaccessible under another ID.

The REST API is suitable for authenticated read snapshots and server-side
validation, not the primary write path. The writable path is the user-run
Plugin API. Dev Mode plug-ins are read-only and are not the import target.

## Affinity path

The first Affinity contract is `adapters/affinity/PROFILE-DRAFT.md`. The
all-new Affinity combines vector, photo and page-layout tools in one no-cost
desktop product, which makes live interchange trials accessible to contributors.
That product position is useful evidence for adoption priority; it does not
create a public plug-in API or disclose the native `.af`, `.afdesign`,
`.afphoto` or `.afpub` encodings.

Profile 0 is consequently a user-mediated SVG bridge:

```text
canonical NUIF
  -> nuif-svg-0 export + fidelity report
  -> user imports SVG into a named Affinity version
  -> user exports SVG
  -> nuif-svg-0 import + host trial report
```

The bridge accepts only the existing `nuif-svg-0` basic-shape and pinned-text
subset. An Affinity-exported SVG containing paths, transforms, effects, CSS,
external resources or other excluded SVG features is rejected or reported as
unsupported; it is never silently simplified. Native Affinity files remain
opaque evidence artifacts. Canva's Connect API accepting Affinity file
extensions proves a supported Canva ingestion route, not that their schema is
public or suitable for a NUIF parser.

No headless automation, stable identity persistence, undo integration or exact
native round trip is claimed. Promotion requires named desktop versions on
each supported operating system, retained input/output files, screenshots or
renders, a complete fidelity report and a second-person review. A future public
Affinity scripting or document API would justify a separately versioned host
profile; undocumented UI automation and native-format reverse engineering do
not.

## Canva path

The first programmable profile is `adapters/canva/PROFILE-DRAFT.md` and uses
the stable Apps SDK Design Editing API. The pure normalized mapping covers one
unlocked, fixed-dimension `current_page`, optional opaque background, groups,
rectangles, canonical ellipses and pinned literal text. Images are media fills
on rectangles in the Canva model. Canva Docs, unbounded pages, tables, embeds,
video, unsupported elements, unavailable fonts and preview-only APIs are
outside profile 0.

`adapters/canva/app` is now a deterministic no-network review shell pinned to
`@canva/design` 2.12.0. It downloads a bounded current-page snapshot and accepts
a Rust-generated lossless mutation plan. The transport validator admits the
full pure schema, but host preflight permits only a nullable page name, an
optional opaque solid background and unnamed opaque rectangles/canonical
ellipses on an empty page of exactly the same size. The public API does not
expose the stable element IDs, writable names, portable font-file identity or
exact text-box height needed to call groups/text exact, so those plans fail
before insertion.

After explicit confirmation, the shell builds all supported states and calls
`sync` once. Static tests prove one sync in a mock session and prove named,
text, group, alpha, nonempty, locked and unbounded cases insert nothing; only a
live trial can establish the resulting Canva undo action, expiry and conflict
behavior. The app never replaces the complete design with an opaque app element
or raster screenshot.

The current stable package contains one invalid empty statement in its
generated declaration. The build recognizes exactly that fragment, writes a
type-check-only normalized copy and records original/result hashes; runtime
bundling still resolves the untouched official module. Canva's package license
limits derivatives to permitted apps on the Canva Platform and requires the
license in every copy, so the review artifact includes it and is not a general
browser distribution.

Canva Connect APIs are a secondary off-platform workflow. They support OAuth
imports of listed foreign formats and asynchronous exports to formats such as
PDF, PNG, JPG, PPTX, GIF, MP4, CSV and HTML. The current import list includes
Affinity files but not NUIF. Therefore SVG/PDF may be used as explicitly lossy
bridges, while exact NUIF import/export requires either the Apps SDK mapping or
future native `application/nuif+zip` support from Canva. Connect download URLs
are temporary and API scopes, rate limits, user authorization and server-side
privacy obligations remain outside the core.

The review shell uses only generally available APIs. Canva documents that
preview APIs may change without versioning and prevent public review. CI can
build and test a source bundle, but a public release still requires developer
verification, source upload, listing and testing material, Canva review, and an
explicit owner-triggered release. A team app is not the default open-source
distribution path because Canva limits team apps to Enterprise teams.

## Conformance gate

A host profile becomes `integrated` in `adapters/index.json` only when all of
these pass:

- checked-in host input and expected canonical NUIF output;
- exact repeated import/export results for the declared subset;
- a valid host report with non-empty host/API/profile identity;
- one fidelity entry for every mapped or excluded authored property;
- stable correspondence after reorder and reopen where the host supports it;
- duplicate/missing identity repair cases;
- maximum and limit-plus-one document, node, text and metadata inputs;
- cancellation and atomic-failure tests that leave the active host document
  unchanged;
- one native-host trial recording product version and package version.

Credential-free CI tests mapping functions, the compiled shell, Rust-to-
TypeScript plan validation, exact canonical round trips, deterministic rebuilds,
the SDK license and snapshots. It also records informational parse, preflight,
mock-apply and hostile-rejection scaling through the 16,384-element maximum.
Live-host CI or manual certification supplies the final product/version
evidence. Passing static shell tests does not justify a live integration claim.

## Release operation

The native editor, `nuif-wasm` developer binding, Figma review shell, Affinity
interchange kit and Canva app are versioned independently. CI builds review
bundles, checksums and fixture reports; provenance and an SBOM are added where
the distribution form supports them. The Canva artifact is review evidence
under its platform-only SDK license, not a general release SDK. Publication to
a vendor marketplace is never inferred from a Git tag:
it requires the vendor account, assigned app identifier, identity and legal
disclosures, review forms, and an explicit authenticated release operation.
