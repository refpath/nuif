# NUIF WebAssembly binding

`nuif-wasm-api-0` is a byte-oriented browser and JavaScript binding over the
same `nuif-api`, codec and semantic-operation crates used by the CLI and native
editor. It is an experimental developer package, not a stable npm release.

The generated package has no filesystem, network, host-document or rendering
authority. A Figma, Adobe or browser integration owns those capabilities and
passes only selected NUIF or patch bytes into this module.

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
headless Chrome, runs the generated Node binding, requires byte-identical
output from the native CLI, and leaves the browser package under
`target/nuif-wasm-web/`.
