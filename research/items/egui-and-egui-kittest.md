---
id: nuif:research:egui-and-egui-kittest
kind: repository
status: reviewed
title: egui, eframe, egui-wgpu and the egui_kittest headless harness
source:
  url: https://github.com/emilk/egui
  repository: https://github.com/emilk/egui
  authors: [Emil Ernerfeldt, Lucas Meurer, egui contributors]
  published_at: "0.36.1 (2026-08-07); main commit 5d3e958 (2026-08-27)"
  license: MIT OR Apache-2.0
retrieved_at: 2026-08-29
tags: [gui, immediate-mode, rust, egui, kittest, accesskit, snapshot-test, harness, wgpu, docking]
confidence: 0.9
claims: [nuif:claim:semantic-automation]
relations:
  - type: depends_on
    target: nuif:research:accesskit-semantic-ui-testing
    note: egui_kittest queries and actions are defined over the AccessKit tree that egui emits every frame.
  - type: compares_to
    target: nuif:research:masonry-xilem-and-linebender-test-harness
    note: Immediate-mode harness versus retained widget-tree harness.
  - type: compares_to
    target: nuif:research:iced-slint-gpui-makepad-floem
    note: One row of the toolkit comparison synthesis.
  - type: related_to
    target: nuif:research:renderers
    note: egui-wgpu is a wgpu client; its software-adapter preference informs deterministic snapshot rendering.
  - type: related_to
    target: nuif:research:rust-snapshot-property-fuzz-tooling
    note: The egui_kittest README recommends insta over image snapshots where possible.
links:
  spec: []
  adr: [adrs/0001-rust-reference-core.md, adrs/0003-reference-renderer.md]
  rfc: []
  code: [apps/editor/ARCHITECTURE.md, apps/editor/QA.md, Cargo.toml, .github/workflows/ci.yml, crates/nuif-api]
  experiments: []
---

# Summary

egui is an immediate-mode GUI library for Rust; the application closure re-declares the whole UI every frame and receives a `FullOutput` containing shapes, platform output and an AccessKit `TreeUpdate`. eframe is the official native/web application framework and egui-wgpu is the wgpu render integration. egui_kittest is a headless test harness that runs the egui `Context` without a window, feeds the per-frame AccessKit update into kittest (an accessibility-tree query library built on `accesskit_consumer`), lets tests locate nodes by label, role or value, synthesises pointer/keyboard events or dispatches AccessKit actions, and optionally rasterises the frame with egui-wgpu on a CPU adapter for image snapshot comparison. The harness is used in egui's own CI with git LFS snapshot storage, but the toolkit's minimum supported Rust version (1.95 at tag 0.36.1) exceeds the NUIF pin (1.85.0), and the text stack and docking layers are outside egui core.

NUIF interpretation: egui_kittest is the closest existing implementation of the "AccessKit tree as semantic test surface" pattern. Its `run()` convergence loop, `kittest.toml` thresholds, `UPDATE_SNAPSHOTS` regeneration protocol and CPU-adapter preference are directly reusable design elements regardless of which toolkit NUIF adopts.

## Evidence

- egui 0.36.1 was released 2026-08-07; 0.36.0 on 2026-08-05; 0.35.0 (2026-06-25) introduced the inspection protocol and `egui_mcp`. Locator: `CHANGELOG.md` lines 17, 21, 77-97, main branch, retrieved 2026-08-29.
- The tagged workspace manifest sets `rust-version = "1.95"`, `wgpu = "30.0"`, `kittest = "0.4.0"`, `accesskit = "0.24.1"`, `accesskit_consumer = "0.35.0"` with a comment that kittest 0.4 pins accesskit_consumer 0.35 and blocks upgrades. Locator: `Cargo.toml` lines 27, 73-75, 103, 160 at tag `0.36.1`.
- Crates and versions on crates.io (2026-08-29): egui 0.36.1, egui_kittest 0.36.1, eframe 0.36.1, egui-wgpu 0.36.1, epaint 0.36.1, egui_extras 0.36.1, kittest 0.4.0 (2026-03-24), egui_dock 0.21.1 (2026-08-06), egui_tiles 0.17.1 (2026-08-18). Locator: crates.io API `/api/v1/crates/<name>`.
- egui describes itself as "a simple, fast, and highly portable immediate mode GUI library for Rust" that "runs on the web, natively", with "Accessibility via AccessKit" and epaint as "A simple 2D graphics API for custom painting". Locator: `README.md` lines 22, 98, 130.
- egui_kittest features: `wgpu` (pulls egui-wgpu, pollster, image, wgpu with metal/dx12/vulkan/gles; comment "Enable DX12 because it always comes with a software rasterizer."), `snapshot` (dify, image/png), `eframe`. Locator: `crates/egui_kittest/Cargo.toml` lines 20-44, tag 0.36.1.
- Harness surface: `Harness::new_ui`, `new_ui_state`, `new_eframe`, `builder()`, `step`, `run`, `try_run`, `run_ok`, `run_steps`, `try_run_realtime`, `fit_contents`, `set_size`, `set_pixels_per_point`, `input_mut`, `output`, `kittest_state`, `event`, `key_down/up/press`, `key_combination`, `hover_at`, `drag_at`, `drop_at`, `mask`, `render`, `root`, `spawn_eframe_app`. Locator: `crates/egui_kittest/src/lib.rs` lines 188-905, tag 0.36.1 (identical `pub fn` set on main).
- Per-step data flow: `ctx.run_ui(input, |ui| app.run(...))`, then `self.kittest.update(output.platform_output.accesskit_update.take().expect("AccessKit was disabled"))`. Locator: `src/lib.rs` lines 259-275.
- `run()` loops `step()` until `repaint_delay != Duration::ZERO` (no immediate repaint requested) and returns the step count; exceeding `max_steps` yields `ExceededMaxStepsError` carrying `repaint_causes`. Locator: `src/lib.rs` lines 327-375.
- Node interaction: `click()` synthesises `PointerMoved` and `PointerButton` press/release at the node rect centre; `click_accesskit()` dispatches `accesskit::ActionRequest { action: Action::Click }` and "can also click widgets that are not currently visible"; `focus()` uses `Action::Focus`; `scroll_to_me()` uses `Action::ScrollIntoView`; `type_text`, `value`, `is_focused` exist. Locator: `crates/egui_kittest/src/node.rs` lines 56-200, main.
- kittest queries: `Queryable` trait generates `get_by_label`, `get_by_label_contains`, `get_by_role`, `get_by_role_and_label`, `get_by_value`, `get_by(predicate)` plus `query_*`, `get_all_*`, `query_all_*` variants over a `By` filter; label-by nodes are excluded from label matches. Locator: kittest `src/query.rs` lines 146-236, main commit bd19226 (2026-08-03).
- kittest depends only on `accesskit = "0.24.1"` and `accesskit_consumer = "0.38.0"` (main); its README states it is "inspired by Testing Library" and framework-agnostic. Locator: kittest `Cargo.toml` lines 24-27, `README.md`.
- Snapshot protocol: `Harness::snapshot(name)`, `try_snapshot`, `snapshot_options`, `SnapshotOptions { threshold, max_failed_pixels, output_path }` with per-OS `OsThreshold`, `SnapshotResults` aggregation, `UPDATE_SNAPSHOTS=true` (update failing only) or `force`; `.new.png`, `.diff.png`, `.old.png` side files. Locator: `src/snapshot.rs` lines 13-950; README "Snapshot testing".
- `kittest.toml` defaults: `output_path = "tests/snapshots"`, `threshold = 0.6` (weighted squared YIQ distance per pixel), `max_failed_pixels = 0`, optional `[windows]/[macos]/[linux]` overrides. Locator: README "Configuration".
- wgpu test renderer: `default_wgpu_setup()` calls `WgpuSetupCreateNew::without_display_handle()`, removes `Backends::BROWSER_WEBGPU`, and sorts adapters so `DeviceType::Cpu` ranks first; `WAIT_TIMEOUT` is 10 s and the comment names lavapipe. Locator: `crates/egui_kittest/src/wgpu.rs` lines 10-58.
- README enumerates cross-machine image differences (MSAA sample placement, texture filtering, WGSL floating-point evaluation, derivative variants) and recommends disabling MSAA and avoiding NaN/Inf. Locator: README "What to do when CI / another computer produces a different image?".
- README guidance: "prefer regular Rust tests or `insta` snapshot tests over image comparison tests"; images should be checked in via git LFS at low resolution. Locator: README "Guidelines for writing snapshot tests".
- egui CI checks out with `lfs: true` and uploads `**/tests/snapshots` as artifacts. Locator: `.github/workflows/rust.yml` lines 18, 228, 247, main.
- `Painter` API: `new(ctx, layer_id, clip_rect)`, `with_clip_rect`, `set_opacity`, `add(Shape) -> ShapeIdx`, `set(idx, shape)`, `extend`, `rect_filled`, `rect_stroke`, `line`, `circle`, `image`, `text`, `layout`, `layout_no_wrap`, `round_to_pixel_center`. Locator: `crates/egui/src/painter.rs` lines 47-503, tag 0.36.1.
- egui_dock: tabs, moving tabs between nodes, dragging tabs into new windows, programmatic layout manipulation; badge targets egui 0.36. egui_tiles: horizontal/vertical/grid layouts, tabs, drag-and-drop docking, `unsafe` forbidden. Locator: respective `README.md` files, main.

## Mechanism

Frame loop and tree extraction:

```rust
// egui_kittest/src/lib.rs (0.36.1), simplified
fn step_impl(&mut self, sizing_pass: bool) {
    self.input.predicted_dt = self.step_dt;
    let mut output = self.ctx.run_ui(self.input.take(), |ui| {
        self.response = self.app.run(ui, &mut self.state, sizing_pass);
    });
    self.kittest.update(output.platform_output.accesskit_update.take()
        .expect("AccessKit was disabled"));      // accesskit::TreeUpdate -> accesskit_consumer::Tree
    self.renderer.handle_delta(&mut output.textures_delta);
    self.output = output;
    self.handle_viewport_commands();             // InnerSize, Screenshot
}
```

Invariants: AccessKit must be enabled on the `Context`; every frame produces a complete `TreeUpdate`; a `Node` handle borrowed from the harness is invalidated by the next `step()`, so tests re-query after `run()`. Events queued on nodes are drained one per frame by `step()`.

Typical test:

```rust
let mut harness = Harness::new_ui(|ui| { ui.checkbox(&mut checked, "Check me!"); });
let cb = harness.get_by_label("Check me!");
assert_eq!(cb.accesskit_node().toggled(), Some(Toggled::False));
cb.click();                       // or cb.click_accesskit() for Action::Click
harness.run();                    // converge until no repaint requested
harness.fit_contents();
harness.snapshot("readme_example"); // needs features wgpu + snapshot
```

Rendering path for snapshots: `Harness::render()` clones `FullOutput`, optionally paints a cursor triangle, and calls `WgpuTestRenderer::render(&ctx, &output) -> RgbaImage`; the renderer is created without a display handle on the first CPU adapter found (lavapipe, WARP via DX12, or a GPU fallback). Comparison uses dify with the YIQ per-pixel threshold and an absolute failed-pixel budget.

Custom canvas painting: an editor canvas is an `egui::Painter` obtained from `ui.painter()` or `Painter::new`; shapes are `epaint::Shape` values (paths, rects, meshes, text galleys). Text layout is epaint's own glyph atlas; custom fonts are installed via `Context::set_fonts` (README line 252). Complex script shaping and bidirectional layout are not provided by epaint (NUIF interpretation from the API surface; not verified against an egui issue in this retrieval).

## NUIF relevance

**Borrow**
- The harness contract: run-to-convergence with a bounded step count and reported repaint causes (`ExceededMaxStepsError`), because NUIF QA item 6 requires deterministic snapshots and a bounded loop makes non-convergence a test failure rather than a hang.
- The dual action path (`click()` synthesises pointer input; `click_accesskit()` dispatches `Action::Click` to a possibly invisible node), because QA.md mandates testing "without synthetic mouse input" while GUI wiring tests still need pointer simulation.
- The `kittest.toml` per-OS threshold table, `UPDATE_SNAPSHOTS=true|force` semantics and `.new/.diff/.old` side files, because they encode a regeneration protocol that already survived CI use at egui scale.
- CPU-adapter-first wgpu selection without a display handle, because ADR 0003 retains a CPU/reference path for conformance and this is a concrete implementation of the same policy.

**Adapt**
- Node queries should target NUIF entity identity (`EntityId`, name, role) exposed through AccessKit `author_id`/`html_id` or a NUIF-owned semantic tree, because kittest's label/role queries alone cannot express QA item 2 (identity/type/name/relationship queries).
- Snapshot storage must be moved from image files toward structured `RenderScene` snapshots (insta) with image comparison only for the renderer conformance suite, because the README itself ranks image tests as slow and brittle.
- egui_dock or egui_tiles can host panels, but panel layout must remain ephemeral shell state (ARCHITECTURE.md), so docking state must not be serialised into NUIF documents.

**Reject**
- Adopting egui 0.36 under the current toolchain pin, because `rust-version = "1.95"` contradicts `rust-version = "1.85"` in `Cargo.toml` and `toolchain: 1.85.0` in `.github/workflows/ci.yml`; either the pin moves or the toolkit is excluded.
- Using epaint text as the text oracle for NUIF text diagnostics (QA item 5), because NUIF's text semantics target HarfBuzz-compatible shaping (whitepaper section 06) and epaint does not expose a shaping pipeline.
- Depending on the kittest/accesskit_consumer version lock (egui `Cargo.toml` lines 74-75) in the engine crates, because the engine must stay free of GUI-toolkit version coupling.

## Open questions

- Whether the NUIF project will raise its MSRV to track egui (1.95) and kittest (1.95 on main), or freeze on an older egui release line compatible with 1.85.
- Whether the egui inspection protocol (0.35) and `egui_mcp` can serve as the "local automation endpoint" required by ARCHITECTURE.md, or whether that would make MCP the canonical contract, which ARCHITECTURE.md forbids.
- Whether wgpu CPU adapters (lavapipe, WARP) yield bit-identical output across CI hosts for NUIF's render conformance tolerances, or whether a `vello_cpu`/tiny-skia reference rasteriser is still required.
- Whether epaint's shape model can carry NUIF `RenderScene` commands losslessly (gradients, clipping, images at explicit scale) or whether a Vello scene should be composited into the egui frame as a texture.
