---
id: nuif:research:wasm-headless-execution
kind: synthesis
status: reviewed
title: Headless execution of the Rust engine in browsers, Node and WASI runtimes for differential tests
source:
  url: https://wasm-bindgen.github.io/wasm-bindgen/wasm-bindgen-test/index.html
  authors: [wasm-bindgen contributors, wasm-pack contributors, Bytecode Alliance, rust-lang, Microsoft (Playwright)]
  published_at: "wasm-bindgen 0.2.127 and wasm-bindgen-test 0.3.77 (2026-08-08); wasm-pack 0.15.0 (2026-05-15); wasmtime 48.0.1 (2026-08-24); Playwright v1.62.1 (2026-07-30)"
  license: wasm-bindgen, wasm-pack MIT OR Apache-2.0; wasmtime Apache-2.0 WITH LLVM-exception; Playwright Apache-2.0
retrieved_at: 2026-08-29
tags: [wasm, headless, browser, node, wasi, wasmtime, playwright, differential-testing, wasm-bindgen, rust]
confidence: 0.85
claims: [nuif:claim:semantic-automation]
relations:
  - type: extends
    target: nuif:research:wasm-component-model
    note: wasm32-wasip2 outputs components; wasmtime executes them.
  - type: related_to
    target: nuif:research:differential-testing
    note: Browser layout as the oracle for differential layout tests.
  - type: related_to
    target: nuif:research:taffy-and-yoga-browser-generated-tests
    note: Taffy drives Chrome through WebDriver to generate fixtures.
  - type: related_to
    target: nuif:research:cargo-workspace-xtask-and-ci-layout
    note: wasm targets in the CI matrix.
  - type: related_to
    target: nuif:research:webgpu-security
    note: Browser WebGPU availability constrains in-browser rendering tests.
links:
  spec: [spec/12-cli-api-and-automation.md]
  adr: [adrs/0001-rust-reference-core.md]
  rfc: []
  code: [apps/editor/ARCHITECTURE.md, conformance/PLAN.md, .github/workflows/ci.yml, crates/nuif-layout, crates/nuif-api, crates/nuif-wasm, tools/wasm/smoke.cjs, xtask/src/main.rs]
  experiments: [nuif:experiment:wasm-cross-surface]
---

# Summary

Three headless execution paths exist for a Rust engine compiled to WebAssembly. `wasm-bindgen-test` compiles `#[wasm_bindgen_test]` functions for `wasm32-unknown-unknown` and runs them under Node by default or, with `wasm_bindgen_test_configure!(run_in_browser)` or `WASM_BINDGEN_USE_BROWSER=1`, in headless Chrome, Firefox or Safari through WebDriver; `wasm-pack test --headless --chrome --firefox` wraps the runner and driver management. `wasmtime` executes `wasm32-wasip1` modules and `wasm32-wasip2` components (both Tier 2 targets shipped by rustup) with capability-scoped filesystem access (`--dir`) and direct function invocation (`--invoke`), which suits CLI-equivalent conformance runs without a browser. Playwright launches Chromium, Firefox and WebKit headlessly and evaluates JavaScript in the page (`page.evaluate`), which is the mechanism for browser differential layout tests: a page renders the CSS-equivalent of a NUIF fixture, the test reads `getBoundingClientRect` results, and compares them against the WASM engine's `LayoutSnapshot` loaded in the same page or in Node.

NUIF interpretation: the engine's differential layout suite should run in three layers: wasmtime for fast, GPU-free CLI parity; `wasm-bindgen-test` under Node for binding-level round trips; Playwright for browser-oracle comparisons where the oracle is the browser's own layout, not WebGPU rendering.

## Evidence

- wasm-bindgen-test: "an experimental test harness for Rust programs compiled to Wasm using `wasm-bindgen` and the `wasm32-unknown-unknown` target"; tests are written with `#[wasm_bindgen_test]` and run with `cargo test --target wasm32-unknown-unknown`; `#[wasm_bindgen_test(unsupported = test)]` falls back to `#[test]` on native targets; tests "must be in the root of the crate, or within a `pub mod`". Locator: wasm-bindgen `guide/src/wasm-bindgen-test/{index.md,usage.md}`, main (2026-08).
- Versions: wasm-bindgen 0.2.127 and wasm-bindgen-test 0.3.77 published 2026-08-08; wasm-pack 0.15.0 published 2026-05-15. Locator: crates.io API; wasm-bindgen `CHANGELOG.md` "[0.2.127]".
- Browser configuration: default is Node; `WASM_BINDGEN_USE_BROWSER=1`, `WASM_BINDGEN_USE_DEDICATED_WORKER`, `..._SHARED_WORKER`, `..._SERVICE_WORKER`, `WASM_BINDGEN_USE_DENO`, `WASM_BINDGEN_USE_NODE_EXPERIMENTAL`; forced per crate via `wasm_bindgen_test_configure!(run_in_browser | run_in_dedicated_worker | run_in_shared_worker | run_in_service_worker | run_in_node_experimental)`. Locator: `guide/src/wasm-bindgen-test/browsers.md` lines 1-35.
- Headless drivers: `wasm-pack test --headless --chrome --firefox --safari`; without wasm-pack set `CHROMEDRIVER`, `GECKODRIVER` or `SAFARIDRIVER` (or `CHROMEDRIVER_REMOTE`) and run `cargo test --target wasm32-unknown-unknown`; `webdriver.json` or `WASM_BINDGEN_TEST_WEBDRIVER_JSON` supplies capabilities; `NO_HEADLESS=1` serves the tests for a visible browser. Locator: `browsers.md` lines 58-165.
- CI examples run `cargo test` natively, then `wasm-pack test --headless --chrome` and `--firefox` (GitHub Actions). Locator: `guide/src/wasm-bindgen-test/continuous-integration.md`.
- wasm-pack test: wraps `wasm-bindgen-test-runner`; accepts a crate path, `--release`, environment flags `--node --firefox --chrome --safari --headless`, `--panic-unwind` (nightly, `-Z build-std`), and passes extra arguments to `cargo test`. Locator: wasm-pack `docs/src/commands/test.md`.
- Coverage on wasm requires nightly `-Cinstrument-coverage -Zno-profiler-runtime` and `--cfg=wasm_bindgen_unstable_test_coverage`, then `cargo +nightly llvm-cov test --target wasm32-unknown-unknown`. Locator: `guide/src/wasm-bindgen-test/coverage.md` lines 12-46.
- wasmtime 48.0.1 (2026-08-24): README shows `rustup target add wasm32-wasip2`, `rustc hello.rs --target wasm32-wasip2` and running the resulting component; built on Cranelift; supports WASI. Locator: wasmtime `README.md` lines 63-131.
- wasmtime CLI: `wasmtime --dir=. --dir=/tmp demo.wasm args...` grants directory capabilities (`--dir=host::guest` mapping); `wasmtime run --invoke 'add(1, 2)' add.wasm` calls exported functions of modules or components and skips `wasi:cli/run`; `-W`/`--wasm` configures proposals. Locator: `docs/WASI-tutorial.md` lines 157-243; `docs/cli-options.md` lines 22-142, 336.
- rustc targets: `wasm32-wasip1` is Tier 2, cross-compiled, ships `std` with a self-contained sysroot, installed by `rustup target add wasm32-wasip1`, and "will generate core WebAssembly modules"; `wasm32-wasip2` is Tier 2 and "outputs a component" built on the component model. Locator: rustc book `platform-support/wasm32-wasip1.md`, `wasm32-wasip2.md`.
- Playwright: latest release v1.62.1 (2026-07-30); `npx playwright install chromium|firefox|webkit`, `install --with-deps`; projects run the same test under `chromium`, `firefox`, `webkit`; `page.evaluate(() => document.location.href)` returns serialisable values from page scripts, including `async` functions. Locator: `docs/src/browsers.md`; `docs/src/evaluating.md` lines 15-44; `gh api repos/microsoft/playwright/releases/latest`.
- Taffy generates layout fixtures by driving Chrome for Testing through ChromeDriver with `fantoccini`, downloading a matching Chrome/driver pair once per version. Locator: Taffy `scripts/gentest/src/main.rs` lines 11-67; `CONTRIBUTING.md` lines 26-35.
- egui_kittest removes `Backends::BROWSER_WEBGPU` from its test setup because it relies on blocking screenshots. Locator: egui `crates/egui_kittest/src/wgpu.rs` lines 20-27.
- The NUIF whitepaper and ADR 0001 assign the WASM boundary to Rust and the editor shell to a web stack; ARCHITECTURE.md draws the Rust core behind a WASM boundary. Locator: `docs/whitepaper/06-language-and-runtime-choice.md`; `adrs/0001-rust-reference-core.md`; `apps/editor/ARCHITECTURE.md`.
- Figma documents that a plug-in UI iframe can use browser APIs including
  WebAssembly, while host-document access remains in the plug-in API. This
  supports a capability-free WASM core in the iframe and a thin, separately
  tested host adapter. Locator: https://developers.figma.com/docs/plugins/,
  retrieved 2026-08-30.

## Mechanism

Three execution layers and their contracts:

```text
1. wasm32-wasip1 / wasip2  ──▶ wasmtime run --dir=fixtures::/fixtures nuif-cli.wasm layout /fixtures/x.nuif --context ...
   oracle: identical stdout/JSON to the native CLI (parity test); no browser, no GPU; capability-scoped FS.

2. wasm32-unknown-unknown ──▶ cargo test --target wasm32-unknown-unknown  (Node by default)
   #[wasm_bindgen_test] fn layout_roundtrip() { let snap = engine.layout(doc, ctx); assert_eq!(canonical(snap), expected) }
   oracle: expectations embedded or fetched; exercises wasm-bindgen glue and JS-facing API.

3. Playwright ──▶ chromium/firefox/webkit headless
   page.setContent(html_from_fixture); const boxes = await page.evaluate(() => [...document.querySelectorAll('[data-nuif-id]')]
       .map(e => { const r = e.getBoundingClientRect(); return [e.dataset.nuifId, r.x, r.y, r.width, r.height]; }));
   const ours = await page.evaluate(() => nuif.layout(doc, ctx));   // WASM engine loaded in the same page, or run in Node
   compare(boxes, ours, tolerance_from_context);
   oracle: browser layout; only for semantics where NUIF declares CSS equivalence (conformance/PLAN.md).
```

Invariants and constraints from the sources: wasm-bindgen tests must be in the crate root or a `pub mod`; browser runs need a driver binary and a browser on the host (wasm-pack manages drivers); Safari runs only on macOS; wasmtime exposes files only under `--dir` mappings; `wasm32-wasip2` produces components, so a CLI targeting wasip2 must use `wasi:cli/run` or `--invoke`; blocking GPU readback is unavailable in browsers, so in-browser tests should assert on scene data or layout boxes, not on rendered pixels.

Reproducibility: pin browser versions (Playwright pins browser builds per release; Taffy pins Chrome for Testing), pin wasmtime (`cargo install --locked wasmtime-cli` at a fixed version), and record the browser name and version in the differential report as part of the evaluation context.

The implemented first layer is `nuif-wasm-api-0`: a
`wasm32-unknown-unknown` module generated with wasm-bindgen 0.2.127. It accepts
only explicit canonical-text/CBOR and patch byte arrays, exposes validation,
hashing, encoding, bounded atomic application and exact undo/redo, and declares
no host authority. `cargo xtask gate-wasm` generates Node and direct-browser
packages, initializes the web target in pinned headless Chrome, drives the Node
package, and requires the edited canonical bytes to equal the native CLI
result. This closes binding and browser-package initialization only; it does
not close browser-layout, WASI CLI or host-adapter behavior described above.

## NUIF relevance

**Borrow**
- wasmtime with `--dir` as the sandboxed executor for a `wasm32-wasip1` build of `nuif-cli`, because it yields CLI parity tests without a browser and demonstrates the capability boundary that the security suite needs.
- The wasm-bindgen-test/wasm-pack headless flow for binding-level tests of the engine's JS API, because ARCHITECTURE.md requires the editor's in-process API to mirror CLI semantics and this tests the boundary itself.
- Taffy's pinned Chrome for Testing approach for browser oracles, because differential layout results are only reproducible with a pinned browser build.

**Adapt**
- Playwright rather than raw WebDriver for the browser differential suite, because one runner covers Chromium, Firefox and WebKit and `page.evaluate` returns structured results; the runner should live under `xtask` or a `tests/browser` package and emit the NUIF report format.
- Browser-differential fixtures should be generated from NUIF fixtures into HTML with `data-nuif-id` attributes so that boxes are keyed by `EntityId`, mirroring Taffy's generated fixtures.

**Reject**
- In-browser WebGPU rendering as a conformance oracle, because blocking readback is unavailable (egui_kittest removes `BROWSER_WEBGPU`) and GPU results vary by implementation; render conformance stays on the CPU reference path.
- Node-only testing as the sole WASM layer, because browser layout is the oracle for CSS-equivalent semantics and only a browser provides it.

## Open questions

- Whether `wasm32-wasip2` component output should be the CLI's WASM form now, or whether `wasip1` modules are preferable until the component-model boundary (nuif:research:wasm-component-model) is adopted.
- Whether wasm-bindgen's browser runner or Playwright should own the browser differential tests; running both duplicates driver management.
- Whether font availability in headless browsers can be pinned tightly enough for text-dependent layout fixtures, or whether differential tests must exclude text metrics.
- Whether `wasm32-unknown-unknown` builds of `nuif-render` with `wgpu` are needed at all for tests, given that in-browser pixel assertions are rejected above.
- Whether to acquire wasm-bindgen's immutable prebuilt CLI archives with
  per-platform checksums instead of compiling its full optional test-runner
  tool graph. The latter currently warns about future-incompatible HTTP-server
  dependencies, but those dependencies are absent from the NUIF module and
  workspace runtime graph.
