# Draft Affinity SVG bridge profile 0

Status: researched user-mediated interchange specification; no native Affinity
parser, plug-in, scripting API or live-host conformance claim.

Profile identifier: `nuif-affinity-svg-bridge-0`.

Primary evidence: `nuif:research:affinity-interchange-and-adoption` and ADR
0012.

## Host and scope

- A named desktop build of the all-new Affinity on a recorded operating system.
- One document and one fixed-size artboard/page per trial.
- The exact shapes, groups, solid encoded-sRGB fills, opacity and pinned literal
  text admitted by `nuif-svg-0`.
- User-mediated SVG import and SVG export through Affinity's documented file
  interface.
- The source NUIF, exported bridge SVG, Affinity-produced SVG, canonical result,
  render artifacts and fidelity report are retained together.

Native `.af`, `.afdesign`, `.afphoto`, `.afpub` and template bytes are opaque
foreign artifacts. Paths, arbitrary transforms, CSS, paint servers, effects,
animation, scripts, external resources, editable Affinity-only objects,
responsive layout and component behavior are excluded.

## Mapping boundary

NUIF-to-Affinity uses the existing `nuif-svg-0` exporter. The SVG and its report
must pass `cargo xtask gate-svg` before a human opens it in Affinity. The user
records the exact Affinity and operating-system versions, imports the SVG and
exports a second SVG without unrelated edits.

Affinity-to-NUIF uses the existing bounded SVG importer. If the produced SVG
contains any construct outside `nuif-svg-0`, the import fails or reports that
construct as unsupported. The trial never rewrites the SVG to force acceptance.
Byte identity is not expected because Affinity may reserialize SVG; canonical
NUIF semantics and separately pinned render evidence are the comparison
surfaces.

## Identity and resources

No stable Affinity object identifier or metadata-preservation contract is
claimed. Correspondence is trial-local and derived from the two SVG mappings.
Embedded or linked fonts and images are outside the first bridge because the
current SVG profile excludes external resources. Native Affinity files may be
retained as opaque provenance but are never decoded by NUIF tooling.

## Required live evidence

- exact pre-Affinity NUIF and SVG bytes plus the exporter report;
- exact Affinity-produced SVG bytes and bounded importer report;
- named Affinity build, operating system, locale and unit settings;
- one unchanged import/export, one reorder, one edit and one unsupported-effect
  trial;
- visual comparison rendered from both sides with text and geometry metrics
  reported separately;
- a second-person review that every loss is represented in the report;
- repeated trials on every operating system advertised by the profile.

The profile remains `external_runtime` until these artifacts are checked in and
the trial runner prevents unsupported SVG from being promoted. A documented
Affinity document API or scripting runtime would require a new profile rather
than silently broadening this bridge.
