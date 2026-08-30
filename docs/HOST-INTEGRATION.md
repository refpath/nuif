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

## Figma path

The first profile is `adapters/figma/PROFILE-DRAFT.md`.

```text
Figma Plugin main thread
  ├─ reads/writes current PageNode and SceneNode objects
  ├─ owns host mutation and undo grouping
  └─ exchanges typed messages with
       Figma UI iframe
         ├─ user selects or downloads .nuif/report files
         └─ parses/serializes the bounded profile
```

The manifest targets only `figma`, requires dynamic page loading and declares
no network domains. The plug-in loads the current page by default; a separate
user choice is required before loading every page. Import mutates nodes only
after showing the report and closes as one undo group. Export does not mutate
the file.

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

Credential-free CI tests mapping functions and snapshots. Live-host CI or
manual certification supplies the final product/version evidence. Passing only
the snapshot tests does not justify a live integration claim.

## Release operation

The native editor release is versioned independently. A future Figma plug-in and
each Adobe `.ccx` use their own semantic versions and changelogs. CI should
build deterministic review bundles, checksums, an SBOM where applicable and
fixture reports. Publication to a vendor marketplace is never inferred from a
Git tag: it requires the vendor account, assigned plug-in ID, disclosure or
review forms, and an explicit authenticated release operation.
