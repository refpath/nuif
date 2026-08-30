# Retentive Penpot v3 package profile zero

Status: executable integrated profile (`nuif-penpot-v3-0`). The library,
public CLI, blocking `xtask` gate and retained CI artifacts exercise the same
profile boundary.

## Foreign package boundary

The profile reads the current Penpot v3 ZIP-and-JSON representation with
manifest version 1 and file data version 67. It maps exactly one file, one page,
Penpot's root frame, one board and that board's direct rectangle, ellipse and
text children. Manifest library relations must be absent. The legacy
per-shape-member representation is required; the opt-in compact page
representation is not accepted.

Penpot UUIDs map to NUIF document and entity identities. A page UUID is package
structure rather than a NUIF entity. The board maps to a positive fixed-size
surface. Board and leaf geometry maps finite positions and non-negative fixed
dimensions. An absent leaf fill remains absent; a present fill is one opaque
8-bit-exact sRGB colour. Text contains one literal run with positive finite
font size and line height. The pinned Ahem name and SHA-256 are retained in the
shape's `org-nuif` plug-in data because the exported Penpot text object does not
otherwise carry NUIF's content-addressed font identity.

Every mapped entity requires a non-empty name. Tokens, relations, extensions,
portable semantics, responsive values, non-default layout, arbitrary authored
values, nested mapped children, paths, groups, images, boolean/SVG-raw shapes,
strokes, gradients, effects, constraints, grids, components, variants,
libraries, interactions, multiple text runs and media are outside this profile.
Their absence is a profile boundary, not a claim that Penpot lacks them.

## Retentive package laws

For a document and package inside the profile:

1. `import_package(export_document(document).bytes).document == document`.
2. Repeated export produces identical ZIP bytes. Exported member timestamps are
   fixed to the earliest ZIP date and file permissions are fixed.
3. An unchanged synchronization returns the original archive byte-for-byte,
   including central-directory representation.
4. Mapped JSON scalars carry member-qualified UTF-8 byte spans. A change patches
   only those spans, from the highest offset to the lowest within each member.
5. Member payloads without mapped edits remain byte-identical. Unknown members
   are retained and reported as `preserved_unrenderable`; unknown JSON fields
   are reported as unsupported while their bytes remain outside mapped spans.
6. A changed package is rebuilt, re-imported and required to equal the requested
   document exactly before any result is returned.
7. Identity, containment, child order, kind, optional name/fill inventory or
   text metadata-shape changes fail atomically as `UnmappedChanges`.

ZIP container metadata may change when a mapped edit requires rebuilding the
archive. The payload-locality law does not promise central-directory byte
identity after an edit. The unchanged path has the stronger whole-archive law.

## Resource and security contract

The complete package is limited to 16 MiB, 4,096 members, 4 MiB per expanded
member and 32 MiB total expanded data. A member's advertised expanded size may
not exceed 1,000 times its compressed size. JSON is limited to depth 64 and
131,072 values.

Member paths must be ASCII, forward-slash-only and enclosed relative paths.
Duplicate names, directories, symbolic links and encrypted entries are
rejected. Only stored and Deflate compression methods are accepted. The adapter
uses `zip` 8.6.0 with default features disabled and only the Rust-backed Deflate
feature enabled. It reads every member into bounded memory and never extracts
an archive to the filesystem. This design also avoids the filesystem-extraction
surface described by CVE-2025-29787; the selected crate version is newer than
the advisory's 2.3.0 patched boundary.

## Executable evidence

The foreign fixture is produced by the official `@penpot/library` 1.1.0 npm
package and is committed under `conformance/foreign/penpot/`. The crate test
imports that package to the expected canonical model and proves no-op archive
identity. The profile runner additionally checks deterministic native export,
eight scalar edits, exact re-import, untouched member payloads, an opaque binary
member, an unknown JSON field, structural rejection, traversal rejection and
one-over package/member limits.

Run `cargo xtask gate-penpot`. The command also exercises the public CLI:

- `nuif export <input.nuif> penpot-v3-0 <output.penpot> [report.json]`;
- `nuif import penpot-v3-0 <input.penpot> <output.nuif> [report.json]`;
- `nuif sync penpot-v3-0 <retained.penpot> <edited.nuif> <output.penpot> [report.json]`.

`nuif-penpot-v3-0` is accepted as an explicit alias. Evidence is written to
`target/penpot-sync-report.json`, `target/penpot-sync-output.penpot`,
`target/penpot-sync-edited.nuif`, `target/penpot-sync-cli-report.json` and
`target/penpot-sync-cli-output.penpot`.

Primary format sources are Penpot's [technical file-format
reference](https://help.penpot.app/technical-guide/developer/data-model/penpot-file-format/),
the official [`library` source](https://github.com/penpot/penpot/tree/develop/library)
and its [`binfile` v3 implementation](https://github.com/penpot/penpot/blob/develop/backend/src/app/binfile/v3.clj).
