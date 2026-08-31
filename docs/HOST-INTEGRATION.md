# Host integration and vendor adoption

Figma or an Adobe application would use NUIF through a small host adapter, not
by embedding the native NUIF editor. The host continues to own its canvas,
document, undo stack and plug-in lifecycle. The adapter translates between the
host object model and a declared NUIF profile, then emits evidence describing
what was exact, approximated, preserved or unsupported.

The decision is ADR 0008. Primary evidence is recorded in
`nuif:research:figma-plugin-and-rest-api-as-automation-surface` and
`nuif:research:adobe-uxp-host-integration`.

## Deliverables for a host

A production host integration consists of five independently reviewable parts:

1. A bounded profile that lists mapped host object kinds and properties,
   resource limits, unsupported semantics and identity rules.
2. Pure import/export mapping functions covered by checked-in host snapshots
   and expected canonical NUIF documents.
3. A thin plug-in shell for file selection, host permission prompts, undo and
   user-visible fidelity confirmation.
4. A `nuif-adapter::HostAdapterReport` for every direction. The report records
   the host/API version and revision, canonical hash, host-object
   correspondences, property fidelity and preservation result.
5. A host-specific package, version stream and release gate. Host packages do
   not inherit the native editor's version.

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
JavaScript model. The generated module has no filesystem, network, Figma or
Adobe authority.

For Figma, the module belongs in the UI iframe, where Figma documents normal
browser APIs including WebAssembly. The plug-in main thread still owns
`SceneNode` access and exchanges bounded messages with that iframe. For Adobe,
the UXP shell owns file tokens and host mutation. In both cases the WASM module
can validate, edit and re-encode complete NUIF packages locally. The shell must
still declare and require the exact package capability set before evaluation;
WASM is not the host adapter and cannot justify a vendor fidelity claim by
itself.

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

Inspection or migration tools may instead call `capability_report` and retain
unknown required resources without claiming full support. Structural decode
never executes a capability. A rendering or behavior host must pass the exact
declared supported set before it presents the package as fully evaluated.

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

## Adobe path

The first profile is `adapters/adobe/PROFILE-DRAFT.md` and targets InDesign.
InDesign UXP is closer to an authored page-layout model than Photoshop. Its
plug-in requests file access with `localFileSystem: request`; network access is
not needed. The adapter reads or creates one document page and the declared
page-item subset, then writes NUIF and host reports through user-selected file
handles.

Photoshop requires a separate profile because layers and pixels do not encode
responsive interface semantics. Covered changes run in one cancellable
`core.executeAsModal` scope and one history state. The document object model is
the first choice; `batchPlay` is isolated to properties absent from that model.
XMP may store NUIF identity only for file types and save paths proven to retain
the namespace.

Each Adobe host is a separate `.ccx` package with its own manifest host and
minimum version. Direct distribution and Creative Cloud Marketplace
publication are separate channels. A Marketplace build needs a Developer
Distribution identifier and review. The repository does not currently claim
an Illustrator package because the retrieved UXP host contract does not list
Illustrator.

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

Credential-free CI tests mapping functions, the compiled shell and snapshots.
Live-host CI or manual certification supplies the final product/version
evidence. Passing static shell tests does not justify a live integration claim.

## Release operation

The native editor, `nuif-wasm` developer binding and Figma review shell are
versioned independently. A promoted Figma integration and each Adobe `.ccx`
use their own semantic versions and changelogs. CI should build review bundles, checksums,
provenance, an SBOM where applicable and fixture reports. Publication to a
vendor marketplace is never inferred from a Git tag: it requires the vendor
account, assigned plug-in ID, disclosure or review forms, and an explicit
authenticated release operation.
