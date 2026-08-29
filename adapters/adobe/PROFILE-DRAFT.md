# Draft Adobe InDesign UXP profile 0

Status: researched mapping specification; no executable `.ccx` package or live
host conformance claim.

Profile identifier: `nuif-indesign-uxp-0`.

Primary evidence: `nuif:research:adobe-uxp-host-integration` and ADR 0008.

## Host and scope

- InDesign 18.5 or newer through a manifest-v5 UXP plug-in targeting host
  `ID`.
- One document and one page per operation.
- Rectangles, ellipses, groups and text frames with ordered containment,
  page-relative bounds, visibility, opacity, solid sRGB fill and literal text.
- Fixed pixel sizing only. The adapter converts host measurement values at an
  explicitly recorded scale and classifies non-pixel or ambiguous conversions.
- Master pages, spreads beyond the selected page, linked assets, tables,
  threaded text, paragraph/character styles, effects, interactive media,
  scripting and arbitrary XMP are outside the first profile.

Photoshop and Illustrator are separate profiles. This document does not claim
their object models or packaging.

## Permissions and packaging

The pure UXP plug-in requests `localFileSystem: request`, declares no network
permission and exposes one panel. It is packaged as a host-specific `.ccx`
with a Developer Distribution identifier only when publication is requested.
No `fullAccess`, process-launch, webview, IPC or generated-code permission is
part of profile 0.

## Identity and correspondence

Every mapped page item receives a host correspondence entry. A namespaced NUIF
label or XMP property may store the NUIF entity identifier only after live
fixtures prove retention through save, close/reopen, copy and package. Until
then, identifiers are session correspondences and synchronization is not
claimed. Existing XMP outside the NUIF namespace is never rewritten.

## Import and export

Import parses and maps the complete candidate before asking for confirmation.
The active document is mutated only after confirmation, through one host undo
scope. Cancellation, API failure or an unsupported required property leaves the
document unchanged.

Export reads the selected page, emits canonical NUIF and a
`HostAdapterReport`, then writes both through user-selected UXP file handles.
The report records InDesign version, UXP API version, profile, direction,
canonical hash, unit conversion, host-object identity and every fidelity
classification.

## Resource limits

The profile inherits NUIF profile-zero limits and additionally caps one run at
one page, 16,384 traversed page items, 4,096 UTF-16 code units per text frame,
1 MiB retained adapter metadata and a 16 MiB NUIF file. Limit-plus-one inputs
must fail before host mutation. These are candidate limits pending live
InDesign time/allocation calibration.

## Required fixtures

- covered page-item import/export and deterministic repetition;
- z-order and group containment;
- explicit unit conversion at two document ruler settings;
- missing, duplicate and reopen identity cases;
- overset/threaded text and unavailable font fidelity;
- linked images and unsupported effects;
- resource-limit and user-cancellation cases;
- undo restores the exact pre-import host document state;
- packaged `.ccx` trial on each supported operating system and named InDesign
  version.

Publication remains blocked until the pure mapping, host snapshots, package and
live-host trial exist.
