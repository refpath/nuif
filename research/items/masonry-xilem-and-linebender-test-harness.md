---
id: nuif:research:masonry-xilem-and-linebender-test-harness
kind: repository
status: reviewed
title: Masonry, Xilem and the masonry_testing TestHarness
source:
  url: https://github.com/linebender/xilem
  repository: https://github.com/linebender/xilem
  authors: [Linebender contributors, Xilem Authors, Druid Authors]
  published_at: "v0.4.0 (2025-10-29); main commit b81d8d7 (2026-08-28)"
  license: Apache-2.0
retrieved_at: 2026-08-29
tags: [gui, retained-mode, rust, masonry, xilem, vello, parley, accesskit, snapshot-test, harness]
confidence: 0.9
claims: [nuif:claim:semantic-automation]
relations:
  - type: extends
    target: nuif:research:vello
    note: Masonry renders through Vello (GPU) and, on main, through vello_cpu for headless screenshots.
  - type: depends_on
    target: nuif:research:accesskit-semantic-ui-testing
    note: TestHarness maintains an accesskit_consumer::Tree and accepts ActionRequest events.
  - type: compares_to
    target: nuif:research:egui-and-egui-kittest
    note: Retained widget tree with owned state versus immediate-mode re-declaration.
  - type: compares_to
    target: nuif:research:iced-slint-gpui-makepad-floem
    note: One row of the toolkit comparison synthesis.
  - type: related_to
    target: nuif:research:harfbuzz-unicode
    note: Parley shapes text with HarfRust, a HarfBuzz port.
  - type: related_to
    target: nuif:research:taffy
    note: Masonry implements its own layout passes; Taffy is not used.
links:
  spec: []
  adr: [adrs/0001-rust-reference-core.md, adrs/0003-reference-renderer.md]
  rfc: []
  code: [apps/editor/ARCHITECTURE.md, apps/editor/QA.md, Cargo.toml, crates/nuif-render, crates/nuif-api]
  experiments: []
---

# Summary

Masonry is a retained-mode widget-tree manager: a `RenderRoot` owns the tree, runs rewrite passes (layout, compose, accessibility), and emits a per-redraw `VisualLayerPlan` (Vello scenes) and an AccessKit `TreeUpdate`. Xilem is a reactive view layer that diffs view trees into Masonry widget mutations. The `masonry_testing` crate provides `TestHarness`, a windowless host that injects pointer, text, window and accessibility events, advances a virtual clock, exposes the widget tree and the `accesskit_consumer::Tree`, rasterises frames and compares them against PNG references with `assert_render_snapshot!`. At v0.4.0 the harness rasterises with Vello on wgpu; on main it rasterises with `imaging_vello_cpu` (no GPU), and the workspace has moved to an `imaging` abstraction with Vello, Vello Hybrid, Vello CPU and Skia backends.

NUIF interpretation: Masonry's architecture (owned tree, centralised focus/pointer/accessibility, explicit passes, harness that reads both the scene plan and the accessibility tree) is the closest structural match to the editor described in ARCHITECTURE.md, and its CPU-render snapshot path matches ADR 0003. The costs are API churn (0.4.0 versus main differ in renderer, dependency versions and MSRV), Apache-2.0-only licensing, and an MSRV (1.88 at v0.4.0, 1.96 on main) above the NUIF pin.

## Evidence

- Published versions (crates.io, 2026-08-29): masonry, masonry_core, masonry_testing, masonry_winit, xilem, xilem_core all 0.4.0 (2025-10-29); GitHub release `v0.4.0` 2025-10-29. Locator: crates.io API; `gh api repos/linebender/xilem/releases/latest`.
- v0.4.0 workspace: `edition = "2024"`, `rust-version = "1.88"`, `license = "Apache-2.0"`, `vello = "0.6.0"`, `parley = "0.6.0"` (feature `accesskit`), `winit = "0.30.12"`, `accesskit = "0.21.1"`, `accesskit_winit = "0.29.2"`, `accesskit_consumer = "0.31.0"`. Locator: `Cargo.toml` lines 32-72 at tag v0.4.0.
- main workspace (commit b81d8d7): `rust-version = "1.96"`, new member `masonry_imaging`, `imaging = "0.0.1"` with `imaging_wgpu` (wgpu-28), `imaging_skia`, `imaging_vello`, `imaging_vello_hybrid`, `imaging_vello_cpu`; `vello = "0.8.0"`, `wgpu = "28.0.0"`, `parley = "0.8.0"`, `accesskit = "0.24.0"`, `accesskit_consumer = "0.35.0"`. Locator: `Cargo.toml` lines 9, 40, 48-91, main.
- Masonry README: "Masonry gives you a platform-independent manager, which owns and maintains a widget tree"; built on Imaging (default Vello and wgpu), Parley, AccessKit; "not opinionated about what your user-facing abstraction will be"; backends `masonry_winit` and `masonry_android_view`. Locator: `masonry/README.md`, cargo-rdme section, main.
- masonry_testing crate docs: the harness can "Simulate any external event which Masonry handles", "Control the flow of time", "Take screenshots"; screenshots are compared against `screenshots/<name>.png` adjacent to `Cargo.toml`; `MASONRY_TEST_BLESS=1` updates references; files are losslessly compressed with Oxipng under a size limit; backend features `imaging_vello`, `imaging_vello_hybrid`, `imaging_vello_cpu`, `imaging_skia`. Locator: `masonry_testing/src/lib.rs` lines 11-58, main.
- main `masonry_testing/Cargo.toml` depends on `accesskit_consumer`, `image` (png), `imaging_vello_cpu`, `masonry_core`, `oxipng 9.1.5`; v0.4.0 instead depends on `futures-intrusive`, `pollster` and uses `masonry_core::vello::util::{RenderContext, block_on_wgpu}`. Locator: `masonry_testing/Cargo.toml` lines 21-27 (main) and 21-30 (v0.4.0); `harness.rs` lines 35-36 (v0.4.0).
- `TestHarness` fields include `render_root: RenderRoot`, `access_tree: accesskit_consumer::Tree`, `renderer: Option<VelloCpuRenderer>`, `action_queue`, `clipboard`, `title`. `TestHarnessParams { window_size, background_color, root_padding, scale_factor, panic_on_rewrite_saturation, max_screenshot_size }`, `DEFAULT_SIZE = 400x400`, default max screenshot 8 KiB. Locator: `masonry_testing/src/harness.rs` lines 146-262, main.
- Public harness methods (main): `create`, `create_with_size`, `create_with`, `process_window_event`, `process_pointer_event`, `process_text_event`, `process_access_event(ActionRequest)`, `render() -> RgbaImage`, `redraw() -> (VisualLayerPlan, TreeUpdate)`, `access_tree()`, `access_node(WidgetId)`, `mouse_move`, `mouse_button_press`, `mouse_button_release`, `mouse_wheel`, `mouse_click_on(id, button)`, `mouse_move_to`, `scroll_into_view`, `accessibility_click_on(id)`, `keyboard_type_chars`, `press_tab_key`, `focus_on`, `set_focus_fallback`, `animate_ms`, `set_disabled`, `root_widget`, `get_widget_with_id`, `get_widget`, `take_records_of`, `inspect_widgets`, `edit_root_widget`, `edit_widget`, `pop_action`, `cursor_icon`, `has_ime_session`, `ime_rect`, `clipboard_contents`, `window_size`, `title`, `save_render_snapshot`, `check_render_snapshot`. Locator: `harness.rs` lines 342-1100, main. At v0.4.0 `mouse_click_on(id)` takes no button argument (line 641).
- `assert_render_snapshot!(harness, name)` expands to `check_render_snapshot(env!("CARGO_MANIFEST_DIR"), name, false)`; a missing reference writes `<name>.new.png` and panics; a mismatch writes `<name>.diff.png`; `SKIP_RENDER_TESTS` skips comparison but still runs the paint pass. Locator: `harness.rs` lines 205-247, 1100-1180, main.
- Image comparison is exact: `get_image_diff` returns `None` only when the maximum per-channel distance is 0 and sizes match. Locator: `masonry_testing/src/screenshots.rs` lines 22-36, main.
- `redraw()` calls `render_root.redraw()` and `access_tree.update_and_process_changes(tree_update, &mut NoOpTreeChangeHandler)`. `render()` composes the `VisualLayerPlan` layers into one Vello `Scene` with padding and calls `VelloCpuRenderer::render_source(&mut scene, width, height)`. Locator: `harness.rs` lines 532-584, main.
- `access_node` contains an `unsafe` NodeId conversion citing AccessKit issue #701 ("No public API exists for modifying/creating a accesskit_consumer::NodeId"). Locator: `harness.rs` lines 591-608, main.
- Helper widgets: `Recorder`/`Recording`/`Record` capture widget method calls via `TestWidgetExt::record`; `ModularWidget`, `WrapperWidget`; assertions `assert_any`, `assert_all`, `assert_none`, `assert_debug_panics_inner`. Locator: `masonry_testing/src/lib.rs` lines 60-80.
- Vello 0.10.0 (2026-08-14), vello_cpu 0.2.0 and vello_hybrid 0.2.0 (2026-08-07), parley 0.11.1 (2026-08-16) on crates.io; Vello main README states verification "with Rust 1.88 and later" and that MSRV bumps are not breaking changes. Locator: crates.io API; `vello/README.md` "Minimum supported Rust Version".
- Parley's stack is Fontique (enumeration/fallback), HarfRust (shaping), Skrifa (glyph outlines/metrics), ICU4X. Locator: `parley/README.md` "The Parley text stack".
- Vello CPU "is a 2D graphics rendering engine written in Rust, for devices with no or underpowered GPUs" with `RenderContext`, `Pixmap`, `RenderMode::OptimizeSpeed/OptimizeQuality`. Locator: `sparse_strips/vello_cpu/README.md`, main.

## Mechanism

Ownership and passes: `RenderRoot` owns the widget arena; external events enter through `process_window_event`, `process_pointer_event`, `process_text_event`, `process_access_event`. Each returns `Handled` and schedules rewrite passes; `panic_on_rewrite_saturation` turns pass loops into test failures. `redraw()` runs the remaining passes and returns both outputs of a frame:

```rust
let (visual_layers, tree_update) = harness.redraw();   // VisualLayerPlan, accesskit::TreeUpdate
let tree: &accesskit_consumer::Tree = harness.access_tree();
let node = harness.access_node(widget_id).unwrap();     // accesskit_consumer::Node
assert_eq!(node.role(), accesskit::Role::Button);
harness.accessibility_click_on(widget_id);              // ActionRequest { action: Click, .. }
let (action, source) = harness.pop_action::<ButtonPress>().unwrap();
assert_render_snapshot!(harness, "button_pressed");     // screenshots/button_pressed.png
```

Invariants observable from the source: the accessibility tree is rebuilt from the same frame that produces the scene, so semantic assertions and pixel assertions refer to one state; time is virtual (`animate_ms`), so animations converge deterministically; screenshot references are exact-match PNGs compressed by oxipng preset 5 with an 8 KiB default ceiling that forces small, focused images.

Renderer dependence: at v0.4.0 the harness allocates a wgpu `RenderContext` and blocks on the GPU (`block_on_wgpu`), so screenshot tests require a wgpu adapter in CI; on main the harness allocates `VelloCpuRenderer::new(1, 1)` lazily and needs no GPU. The `imaging` abstraction lets the application choose `imaging_vello`, `imaging_vello_hybrid`, `imaging_vello_cpu` or `imaging_skia` per build.

Xilem layer: views are values; `xilem_core` diffs view trees and applies `WidgetMut` edits to Masonry; the harness tests Masonry widgets directly and Xilem apps through their root widget (the crate docs point to "the tests in Masonry's examples").

## NUIF relevance

**Borrow**
- The two-output frame (`VisualLayerPlan`, `TreeUpdate`) as the harness contract, because NUIF's `Engine::build_render_scene` plus a semantic tree can be exposed the same way and asserted in one test.
- `TestHarnessParams` (fixed window size, scale factor, background, root padding) as explicit evaluation context, because QA item 4 requires layout "at explicit contexts" and conformance/PLAN.md requires fixture ID and evaluation context in every result.
- The bless protocol (`MASONRY_TEST_BLESS=1`, `.new.png`, `.diff.png`, size ceiling, lossless compression), because it constrains repository growth while keeping exact references.
- `vello_cpu` as the deterministic rasteriser for screenshot conformance, because ADR 0003 asks for a CPU/reference backend and `imaging_vello_cpu` demonstrates it inside a harness.

**Adapt**
- `access_node(WidgetId)` and `accessibility_click_on(WidgetId)` address Masonry widget IDs; NUIF needs the same operations keyed by `EntityId`, so a mapping from entity to widget/accessibility node must be maintained by the editor shell.
- The exact-pixel comparator is appropriate for widget chrome; NUIF render conformance declares tolerances (conformance/PLAN.md), so a thresholded comparator must wrap or replace `get_image_diff`.
- The `Recorder` widget pattern can become a NUIF operation recorder that captures protocol `Operation`s issued by the shell, satisfying QA item 3 (inverse/replay logs).

**Reject**
- Treating any Masonry, Vello or Parley type as NUIF state, because ADR 0003 states "No Vello internal data type is normative NUIF state".
- Building against `main` (rust-version 1.96, unpublished `imaging 0.0.1`) for a reference implementation, because the NUIF pin is 1.85.0 and unpublished dependencies break reproducible `--locked` builds.
- Relying on the v0.4.0 GPU screenshot path in CI, because it requires a wgpu adapter and reintroduces the cross-machine nondeterminism egui's README catalogues.

## Open questions

- When the `imaging` abstraction and `masonry_testing` CPU path will ship in a tagged release, and what MSRV that release will carry relative to NUIF's pin.
- Whether AccessKit issue #701 (public `NodeId` construction in `accesskit_consumer`) has been resolved in accesskit_consumer 0.39, which would remove the `unsafe` block in `access_node`.
- Whether Masonry's layout passes can host NUIF's `LayoutSnapshot` (authored to resolved boxes computed by nuif-layout) without a second layout, or whether the canvas must be a single custom widget that paints `RenderScene` directly.
- Whether Apache-2.0-only licensing of the toolkit is acceptable for the reference editor given the workspace's `Apache-2.0 OR MIT` policy.
