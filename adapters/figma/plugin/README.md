# NUIF Figma review shell

This directory contains the compiled, no-network host shell for the bounded
`nuif-figma-plugin-snapshot-0` profile. It is a development and review tool,
not a published Figma Community plug-in and not live-host conformance evidence.

The shell intentionally does not contain a made-up plug-in ID. Figma assigns
that ID when a reviewer creates a development plug-in. The checked-in manifest
is therefore a template and cannot be imported as-is.

## Build and review

Requires the pinned Node range in `package.json`.

```sh
npm ci --ignore-scripts --no-audit --no-fund
npm run check
```

This compiles the main thread against the exact official Figma typings, builds
an inline UI, runs the pure protocol/normalizer tests and emits `dist/` with a
deterministic build report. The manifest declares `allowedDomains: ["none"]`;
the build also rejects remote URLs in generated files.

To create a locally importable manifest, first use Figma's development plug-in
flow to obtain an ID, then run:

```sh
FIGMA_PLUGIN_ID=your-assigned-id npm run package
```

Replace the illustrative value with the exact ID Figma assigned to your
development plug-in, then import `dist/manifest.json`. Do not commit that
generated manifest.

## Export and import flow

Export requires exactly one selected frame and downloads a normalized snapshot:

```sh
nuif import figma-plugin-snapshot-0 selection.figma-snapshot.json selection.nuif.json fidelity.json
```

Import starts from canonical NUIF and creates a mutation plan outside Figma:

```sh
nuif export selection.nuif.json figma-plugin-snapshot-0 plan.json fidelity.json
```

Load `plan.json` in the plug-in UI. It checks the full bounded schema and keeps
Apply disabled until the user explicitly confirms. A successful plan is
committed as one Figma undo step. A failed creation removes every node created
by that attempt before returning the error.

The exact text subset requires Ahem Regular to be installed and its pinned
SHA-256 marker in shared plug-in data. This is a conformance constraint, not a
general font-import solution. General Figma fonts remain outside this profile.

## Evidence boundary

Credential-free CI proves TypeScript checking, deterministic compilation,
no-network packaging, message validation, a mock Plugin API snapshot and its
successful import by the Rust core. Only a reviewer-run Figma trial can prove
actual node creation, font availability, undo behavior, cancellation and
plug-in-data persistence in a named Figma product version.
