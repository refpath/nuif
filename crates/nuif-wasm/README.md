# NUIF WebAssembly binding

`nuif-wasm-api-0` is a byte-oriented browser and JavaScript binding over the
same `nuif-api`, codec and semantic-operation crates used by the CLI and native
editor. It is an experimental developer package, not a stable npm release.
Bare and package loading, validation, capability negotiation, hashing,
canonical export and history delegate to `nuif-api::NuifDocument`; this crate
owns only the JavaScript byte boundary and its transport limits.

The generated package has no filesystem, network, host-document or rendering
authority. A Figma, Canva or browser integration owns those capabilities and
passes only selected NUIF or patch bytes into this module. Affinity currently
uses the SVG interchange path outside the host because no stable public
document API is claimed.

```js
import init, { NuifDocument, capabilities } from "./nuif.js";

await init();
const contract = JSON.parse(new TextDecoder().decode(capabilities()));
const document = new NuifDocument(bytes, "nuif-text-0");
const validation = JSON.parse(
  new TextDecoder().decode(document.validationReport()),
);
const revision = document.canonicalHash();
const nextRevision = document.applyPatch(patchJsonBytes);
const output = document.exportBytes("nuif-cbor-0");
document.free();
```

Portable `.nuif` packages retain digest-verified embedded images, fonts and
other inert resources across authorized edits and deterministic export.
Structural load is suitable for inspection, bare extraction and exact
same-mode copying. If the manifest has requirements, `applyPatch`, undo/redo
and mode-changing package export fail with
`NUIF_PACKAGE_CAPABILITIES_REQUIRED` until the complete set is authorized. A
plug-in that evaluates or changes such a package must declare its supported
capability identifiers as a bounded JSON string array first:

```js
const text = new TextEncoder();
const structural = NuifDocument.fromPackage(packageBytes);
const report = JSON.parse(
  new TextDecoder().decode(
    structural.packageCapabilityReport(text.encode(JSON.stringify(hostCapabilities))),
  ),
);
structural.requirePackageCapabilities(
  text.encode(JSON.stringify(hostCapabilities)),
);
const authorizedOutput = structural.exportPackage("portable");
structural.free();

const document = NuifDocument.fromPackageWithCapabilities(
  packageBytes,
  text.encode(JSON.stringify(hostCapabilities)),
);
const output = document.exportPackage("portable");
document.free();
```

The capability transport is limited to 64 KiB, 256 unique identifiers and 128
bytes per identifier. `fromPackage` does not execute a capability, fetch a
linked resource or imply that the host supports the manifest. Package inputs
are limited to the `nuif-package-0` 80 MiB archive budget.

Inputs and outputs remain canonical byte records instead of a JavaScript copy
of the NUIF data model. `applyPatch` is atomic and enforces 4 MiB, 1,024
transaction and 16,384 operation limits in addition to the core document
limits. Errors begin with a stable code such as
`NUIF_PATCH_LIMIT_EXCEEDED` followed by a human-readable message.

Build and test both the Node conformance package and direct-browser package:

```sh
cargo xtask gate-wasm
```

The command pins `wasm-bindgen` 0.2.127, initializes the web target in pinned
headless Chrome, and runs the generated Node binding. It requires byte-identical
bare and package output from the native CLI, exact preservation of a packaged
behavior resource, read-only structural mutation rejection and typed
missing-capability negotiation failure. The browser package is left under
`target/nuif-wasm-web/`.
