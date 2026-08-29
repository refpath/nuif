---
id: nuif:research:masonry-editor-stack-decision
kind: synthesis
status: reviewed
title: "Editor stack decision: Masonry on imaging with Vello and AccessKit, re-verified against the 0.4.0 release and the main branch"
source:
  url: https://github.com/linebender/xilem
  repository: https://github.com/linebender/xilem
  authors: [Linebender contributors, forest-rs contributors, AccessKit contributors, emilk, DioxusLabs, Lapce, Zed Industries]
  published_at: "masonry 0.4.0 (2025-10-29); xilem main b81d8d7 (2026-08-28); imaging 0.0.1 (2026-05-21); vello 0.10.0 (2026-08-14); accesskit 0.25.0 (2026-08-29); egui 0.36.1 (2026-08-07); blitz 0.3.0-beta.2 (2026-08-24)"
  license: "Apache-2.0 (Masonry, Xilem); Apache-2.0 OR MIT (imaging, Vello, Parley); MIT OR Apache-2.0 (AccessKit)"
retrieved_at: 2026-08-29
tags: [gui, rust, masonry, xilem, imaging, vello, vello-cpu, accesskit, harness, editor, decision, licence, msrv, egui, blitz, floem, gpui]
confidence: 0.85
claims: [nuif:claim:semantic-automation]
relations:
  - type: extends
    target: nuif:research:masonry-xilem-and-linebender-test-harness
    note: Re-verifies the Masonry record against the main branch at b81d8d7 and adds the imaging paint model, the widget inventory and an empirical dependency resolution.
  - type: compares_to
    target: nuif:research:egui-and-egui-kittest
    note: egui-wgpu callback model examined as a host for a Vello scene.
  - type: compares_to
    target: nuif:research:iced-slint-gpui-makepad-floem
    note: Floem, Blitz and GPUI rows re-checked with fresh evidence.
  - type: depends_on
    target: nuif:research:accesskit-semantic-ui-testing
    note: AccessKit version coupling between harness, consumer and toolkit.
  - type: related_to
    target: nuif:research:vello-testing-and-cpu-reference
    note: Vello CPU renderer status and the tolerance model used for the shell screenshot tier.
  - type: related_to
    target: nuif:research:rust-toolchain-and-msrv-policy
    note: Toolchain pin and MSRV policy required by this stack.
links:
  spec: []
  adr: [adrs/0006-rust-native-editor.md, adrs/0003-reference-renderer.md]
  rfc: []
  code: [apps/editor/UI-SPEC.md, conformance/HARNESS.md, Cargo.toml, rust-toolchain.toml, crates/nuif-render]
  experiments: []
---

# Summary

Masonry has one release in the twelve months to 2026-08-29 (0.4.0, 2025-10-29, MSRV 1.88) and a main branch (b81d8d7, 2026-08-28, MSRV 1.96) that differs from that release in the paint model, the renderer abstraction, the dependency set and the widget inventory. At 0.4.0, `Widget::paint` receives a `vello::Scene` (Vello 0.6) and the test harness screenshots through wgpu. On main, `Widget::paint` receives an `imaging::Painter`, a `Canvas` widget records into an `imaging::record::Scene`, and `masonry_testing` rasterizes through `imaging_vello_cpu` without a GPU. The release notes for 0.3.0 and 0.4.0 describe the software as alpha-quality with major breaking changes expected; no changelog file exists on main. Xilem is a view layer over Masonry with incomplete coverage of Masonry's widgets. An empirical `cargo metadata` resolution of Masonry main together with Vello 0.10, Parley 0.11 and AccessKit 0.25 produces two copies of Vello, wgpu, Parley and AccessKit and four of `accesskit_consumer`.

The alternatives re-checked here do not change the ranking from the earlier records: egui-wgpu callbacks draw into egui's own render pass; Floem still has no accessibility tree; Blitz exposes `<canvas>` custom paint sources but its harness crate is unpublished and its document model is HTML/CSS; GPUI now builds an AccessKit tree per frame but its test contexts expose no tree query and the crate requires the latest stable toolchain.

NUIF interpretation: the proposed stack (Masonry, Vello, AccessKit) is confirmed with three corrections. The editor targets Masonry main pinned by git revision, not 0.4.0, because only main has the CPU harness and the `Canvas` widget. The canvas integration is a lowering from the NUIF render scene to `imaging` commands, not the injection of a `vello::Scene`, because main has no entry point for a Vello scene. Xilem is deferred; the editor uses Masonry's widget tree directly. The editor is confined to `apps/editor`, follows Masonry's dependency versions for every type that crosses the widget boundary, and exposes the harness through NUIF's own session-driver trait.

## Evidence

Masonry and Xilem releases, MSRV and stability

- crates.io versions of `masonry`: 0.1.0 (2022-11-30, MIT), 0.1.1 and 0.1.2 (2023-02-05, Apache-2.0, MSRV 1.65), 0.2.0 (2024-05-07), 0.3.0 (2025-05-10, MSRV 1.86), 0.4.0 (2025-10-29, MSRV 1.88). `masonry_core`, `masonry_testing`, `masonry_winit`, `xilem`, `xilem_core` are all at 0.4.0. GitHub releases: v0.1.0 (2024-05-07), v0.3.0 (2025-05-10), v0.4.0 (2025-10-29). Locator: crates.io API `/api/v1/crates/masonry`; `gh api repos/linebender/xilem/releases`, retrieved 2026-08-29.
- Workspace manifests: tag v0.3.0 `rust-version = "1.86"`, `vello = "0.5.0"`, `wgpu = "24.0.3"`, `parley = "0.4.0"`, `accesskit = "0.19.0"`, `accesskit_winit = "0.27.0"`, `winit = "0.30.10"` (lines 33-59). Tag v0.4.0 `rust-version = "1.88"`, `vello = "0.6.0"`, `parley = "0.6.0"`, `accesskit = "0.21.1"`, `accesskit_winit = "0.29.2"`, `accesskit_consumer = "0.31.0"`, `winit = "0.30.12"` (lines 36-72); release notes state wgpu 26. Main b81d8d7 `rust-version = "1.96"`, `license = "Apache-2.0"`, `imaging = "0.0.1"`, `imaging_wgpu` with feature `wgpu-28`, `imaging_vello`, `imaging_vello_hybrid`, `imaging_vello_cpu`, `vello = "0.8.0"`, `wgpu = "28.0.0"`, `kurbo = "0.13.1"`, `parley = "0.8.0"` (feature `accesskit`), `peniko = "0.6.1"`, `winit = "0.30.13"`, `accesskit = "0.24.0"`, `accesskit_winit = "0.32.2"`, `accesskit_consumer = "0.35.0"` (lines 38-91). Locator: `Cargo.toml` at each ref.
- Release notes v0.3.0: "This is alpha-quality software. There are plenty of missing features and other issues." Release notes v0.4.0: "This release has an MSRV of 1.88"; the software is described as alpha-quality, "We expect to continue active development, including making major breaking changes", and "we plan to start keeping a changelog after this release". Locator: GitHub release bodies v0.3.0, v0.4.0.
- No `CHANGELOG.md` exists at the repository root or under `masonry/` at b81d8d7 (root listing: `.clippy.toml`, `ARCHITECTURE.md`, `AUTHORS`, `Cargo.lock`, `Cargo.toml`, `LICENSE`, `README.md`, `docs`, crate directories). Milestones: `0.3.0` closed, `0.4.0` open with one issue; no later milestone. Locator: `gh api repos/linebender/xilem/contents`; `gh api repos/linebender/xilem/milestones?state=all`.
- README main: "An experimental Rust architecture for reactive UI"; "Xilem is a UI framework, whereas Masonry is a toolkit for building UI frameworks"; "This version of Masonry has been verified to compile with Rust 1.96 and later". Locator: `README.md` lines 5, 34, 140; `masonry/README.md` line 211.
- Linebender blog, "Linebender in 2026 Q1" (2026-04-19): "Masonry has moved to imaging as an abstraction over the 2D rendering engine"; new widgets "Svg, Divider, CollapsePanel, StepInput, RadioButtons, Switch, Clip, Split"; "Masonry now has a new layout system"; "Masonry is using ui-events for more of the integration with system capabilities, including IME"; "Because imaging supports a wide variety of back-ends, Masonry can now operate in a wider variety of environments, including Vello CPU for rendering not requiring a GPU". No later status post exists; the 2026 posts are dated 2026-04-19, 2026-07-11, 2026-08-08 and 2026-08-12 and the last three concern fearless_simd and hyperbezier curves. Locator: https://linebender.org/blog/tmil-25/; https://linebender.org/blog/ index. Zulip announcements were not retrieved (unverified).

Paint model and canvas embedding

- v0.4.0: `fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene)` with `use vello::Scene`. Locator: `masonry_core/src/core/widget.rs` lines 13, 271 at v0.4.0.
- Main: `fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, painter: &mut Painter<'_>)` and `post_paint` with the same signature; `use crate::imaging::Painter`; `masonry_core/src` contains no `vello::` path (grep, 0 matches); `masonry_core` re-exports `imaging` (`pub use imaging;`). Locator: `masonry_core/src/core/widget.rs` lines 20, 393-408; `masonry_core/src/lib.rs` line 82, b81d8d7.
- `Canvas` widget (main only; absent from the v0.4.0 widget listing): "A widget allowing custom drawing. A canvas takes a painter callback; every time the canvas is repainted, that callback is run with an `imaging` record::Scene"; `Canvas::update_scene(this: &mut WidgetMut<Self>, f: impl FnOnce(&mut MutateCtx, &mut Scene, Size))` clears the scene, runs the callback and requests a render; `with_alt_text`; action `CanvasSizeChanged { size }`. Locator: `masonry/src/widgets/canvas.rs` lines 1-80, b81d8d7.
- `imaging` 0.0.1 (2026-05-21, MSRV 1.92, Apache-2.0 OR MIT, "This is the initial release"): "backend-agnostic 2D imaging recording + streaming API" with `Painter` streaming into any `PaintSink` and `record::Scene` retaining an owned command stream. `Painter` methods include `replay(&record::Scene)`, `fill`, `fill_rect`, `stroke`, `glyphs`, `blurred_rounded_rect`, `draw_image(ImageBrushRef, Affine)`, `push_clip`, `push_group`, `record_mask`, `with_masked_group`; `record::Scene::append_transformed`. Locator: forest-rs/imaging `imaging/README.md`, `imaging/CHANGELOG.md`, `imaging/src/painter.rs` lines 112-500, `imaging/src/record.rs` line 766, commit 89b364b.
- `imaging_vello` lowers `record::Scene` into a `vello::Scene` (`VelloSceneSink::new(&mut vello::Scene, surface_clip)`) and renders native Vello scenes; the crate documentation states that "Semantic `imaging::record::Scene` values can be lowered to native Vello scenes". No path from an existing `vello::Scene` into an `imaging` sink was found. `imaging_vello` 0.0.2 depends on `vello ^0.7.0` or `^0.8.0`; `imaging_wgpu` 0.0.1 offers `wgpu ^27.0.1` or `^28.0.0`; `imaging_vello_cpu` 0.0.2 depends on `vello_cpu ^0.0.9`. Locator: `imaging_vello/src/lib.rs` lines 6-63, `imaging_vello/src/scene_sink.rs` lines 15-65; crates.io dependency endpoints.
- `masonry_imaging` (main, unpublished): "owns the bridge between Masonry paint output and concrete imaging backends", exposes `vello`, `vello_hybrid`, `vello_cpu` (headless only) and `skia` modules and "host-neutral texture rendering helpers for writing into caller-provided WGPU targets". Locator: `masonry_imaging/src/lib.rs` lines 7-26, 50-59.

Harness on main

- `TestHarness` holds `renderer: Option<VelloCpuRenderer>` from `imaging_vello_cpu`, created lazily as `VelloCpuRenderer::new(1, 1)`; `render() -> RgbaImage`; `redraw() -> (VisualLayerPlan, TreeUpdate)`. Locator: `masonry_testing/src/harness.rs` lines 30, 150, 532, 563, 576, b81d8d7.
- `access_node` writes a raw `u64` into an `accesskit_consumer::NodeId` under `#[expect(unsafe_code)]`, citing AccessKit issue 701. AccessKit issue 701 ("Consumer `NodeId` public API") was closed 2026-04-13; `accesskit_consumer` 0.36.0 (2026-05-11) added "Allow looking up nodes by LocalNodeId and TreeId (#707)". Masonry main pins `accesskit_consumer = "0.35.0"`, so the `unsafe` block persists there. Locator: `harness.rs` lines 592-604; `gh api repos/AccessKit/accesskit/issues/701`; `accesskit_consumer/CHANGELOG.md` section 0.36.0.
- Tests at b81d8d7: 252 `#[test]` attributes across `masonry/src`, `masonry_core/src` and `masonry/tests`; 207 reference PNG files under `masonry/screenshots`. Locator: shallow clone, `grep -rh "#\[test\]" | wc -l`, `ls masonry/screenshots | wc -l`.

Widget inventory

- v0.4.0 `masonry/src/widgets/`: align, button, checkbox, flex, grid, image, indexed_stack, label, portal, progress_bar, prose, scroll_bar, sized_box, slider, spinner, split, text_area, text_input, variable_label, virtual_scroll, zstack (21 modules). Main adds badge, badged, canvas, collapse_panel, disclosure_button, divider, pagination, passthrough, radio_button, radio_group, resize_observer, selector, selector_item, step_input, svg, switch (37 modules). Locator: `gh api repos/linebender/xilem/contents/masonry/src/widgets` at v0.4.0 and main.
- Tracking issue #1710 (2026-03-31) lists every widget above as available in Masonry; Xilem views are missing for Align, Pagination, Selector and StepInput and marked uncertain for DisclosureButton, Passthrough, ScrollBar, SelectorItem and TextArea. Locator: issue body table.
- `Split` builder: `split_axis`, `split_fraction`, `split_point(SplitPoint::{Fraction, FromStart, FromEnd})`, `min_lengths`, `bar_thickness`, `min_bar_area`, `draggable`, `solid_bar`; a drag test `drag_moves_split_point` exists. Locator: `masonry/src/widgets/split.rs` lines 21-175, 852.
- Layers: `LayerStack` is "the top-level stack of visible layers owned by RenderRoot"; "Other layers can represent tooltips, menus, dialogs, etc."; a tooltip layer exists in `masonry/src/layers/tooltip.rs`. No menu, tab strip, tree view, colour picker or drag-and-drop facility was found by name (grep for `DragAndDrop`, `ContextMenu`, `Popup`, `Tooltip` over `masonry/src/widgets` and `masonry_core/src/core` returns only `Split::draggable`, slider drag and window drag helpers). Locator: `masonry_core/src/app/layer_stack.rs` lines 17-23; `masonry/src/layers/`.
- Text: `TextInput` "does not support newlines entered by the user, although pre-existing newlines are handled correctly" and wraps a `TextArea`; IME events are modelled as `Ime::{Enabled, Disabled, Preedit(text, span), Commit}` on the core event type. Locator: `masonry/src/widgets/text_input.rs` lines 22-33; `masonry_core/src/core/events.rs` lines 189-222.

Open issues relevant to a canvas-heavy editor (linebender/xilem, open on 2026-08-29; 105 open issues in total)

- #388 (2024-06-12) first-class text editing widget; #266 (2024-05-05) caret movement and editing actions; #1417 (2025-10-14) undo and redo in text inputs; #1341 (2025-08-15) most text widgets lack tests; #1562 (2026-01-08) high CPU when focusing a text box on Linux; #1581 (2026-01-16) `Split` bar area does not receive exclusive pointer events; #918 (2025-04-04) memory usage; #685 (2024-10-17) safety rails for widgets with many children; #1264 (2025-08-03) scale factor tracking; #1451 (2025-11-05) wasm `Atomics.wait`. Locator: `gh api search/issues` queries `ime`, `text input`, `split`, `performance large`, `panic`.

Dependency resolution probe

- A scratch manifest (edition 2024, cargo 1.97.1) depending on `masonry`, `masonry_testing`, `masonry_winit` at git rev b81d8d7 plus `vello 0.10`, `vello_cpu 0.2`, `parley 0.11`, `taffy 0.14`, `harfrust 0.13`, `accesskit 0.25`, `accesskit_consumer 0.39`, `proptest 1.11`, `libtest-mimic 0.8`, `ciborium 0.2`, `insta 1.48` resolves to 549 packages. Duplicates: `vello` {0.8.0, 0.10.0}; `vello_cpu` and `vello_common` {0.0.7, 0.2.0}; `wgpu` {28.0.0, 29.0.4}; `wgpu-core` {28.0.1, 29.0.4}; `accesskit` {0.24.1, 0.25.0}; `accesskit_consumer` {0.35.0, 0.36.0, 0.38.0, 0.39.0}; `parley` and `fontique` {0.8.0, 0.11.1}; `skrifa` {0.40.0, 0.44.0}; `harfrust` {0.5.2, 0.12.0, 0.13.3}. Single versions: `peniko` 0.6.1, `kurbo` 0.13.1, `winit` 0.30.13, `taffy` 0.14.0, `imaging` 0.0.1, `ui-events` 0.3.0. Highest `rust-version` in the graph: 1.96 (Masonry crates, `tree_arena`, `linebender_include_doc_path`), then 1.92 (`imaging` crates). Locator: scratch `cargo metadata --format-version 1`, 2026-08-29. `vello` 0.10.0 depends on `wgpu ^29.0.3` (crates.io dependency endpoint).

Alternatives re-checked

- egui 0.36.1 (2026-08-07, MSRV 1.95, MIT OR Apache-2.0). `egui_wgpu::CallbackTrait::paint(&self, info: PaintCallbackInfo, render_pass: &mut RenderPass<'static>, callback_resources: &CallbackResources)` issues "draw commands into the same wgpu::RenderPass that is used for all other egui elements"; `prepare(&self, device, queue, screen_descriptor, egui_encoder, callback_resources) -> Vec<CommandBuffer>` runs before that pass and `finish_prepare` after all `prepare` calls. Locator: docs.rs egui-wgpu `CallbackTrait`. A Vello scene therefore cannot be drawn in `paint` (Vello requires compute passes); it would be rendered to a texture in `prepare` and sampled in `paint` (interpretation; no retrieved example demonstrates it). egui repository issue #8411 (closed 2026-08-11) mentions updating `vello_cpu`; the use site was not examined (unverified).
- Floem: crates.io 0.2.0 (2024-11-14, MSRV 1.80); main manifest 0.2.0 with `rust-version = "1.91"`, `license = "MIT"`, `parley 0.7.0`, `taffy 0.9.2`, optional `vello` feature through `floem_vello_renderer`; accessibility issues #8 (2023-04-14) and #973 (2025-11-11) remain open. Locator: floem `Cargo.toml` lines 29-150; `gh api search/issues q="repo:lapce/floem accesskit"`.
- Blitz main 0.3.0-beta.2 (`rust-version = "1.91.0"`, `MIT OR Apache-2.0`, `anyrender 0.13.0`, `anyrender_vello 0.14.0`, `anyrender_vello_cpu 0.17.0`, `parley 0.11.1`, `taffy 0.14.0`, `accesskit 0.24`): a `<canvas src="<u64>">` element is queued as `SpecialOp::LoadCustomPaintSource` and stored as `SpecialElementData::Canvas(CanvasData { custom_paint_source_id })`. `blitz-test-harness` is a path crate not published on crates.io. The `CustomPaintSource` trait definition in the anyrender repository was not located (unverified). Locator: blitz `Cargo.toml` lines 34-137; `packages/blitz-dom/src/mutator.rs` lines 39, 947-949, 1219-1230; crates.io lookup `blitz-test-harness` (not found).
- GPUI: crates.io 0.2.2 (2025-10-22, Apache-2.0, no `rust_version`); README: "pre-1.0. There will often be breaking changes between versions. You'll also need to use the latest version of stable Rust"; Zed `rust-toolchain.toml` pins 1.97.1. Main has `crates/gpui/src/window/a11y.rs`: "Every frame, we build a TreeUpdate and send it to the platform-specific adapter" with node IDs derived from `GlobalElementId`; `crates/gpui/src/app/test_context.rs` contains no `accesskit` or `a11y` symbol (grep, 0 matches). Locator: `crates/gpui/README.md` line 8; `a11y.rs` lines 1-60; `test_context.rs`.

Licence facts

- Masonry `Cargo.toml` inherits `license.workspace = true` with workspace `license = "Apache-2.0"`; `masonry/LICENSE` is the Apache License Version 2.0 text; the repository has `LICENSE` and `AUTHORS` and no `NOTICE` file. `tree_arena` 0.2.0, `accesskit_winit` 0.34.0 and `winit` 0.30.13 are Apache-2.0 only; `accesskit` and `accesskit_consumer` are MIT OR Apache-2.0; `vello`, `vello_cpu`, `parley`, `fontique`, `peniko`, `kurbo`, `imaging`, `ui-events`, `anymore`, `understory_virtual_list`, `resvg` are Apache-2.0 OR MIT. Locator: `masonry/Cargo.toml` lines 1-10; repository root listing; crates.io `license` fields.

## Mechanism

Integration paths available on Masonry main (interpretation, each element cited above):

```text
interactive path
  nuif-render RenderScene --lowering--> imaging::Painter / PaintSink   (nuif "imaging" render delegate)
                                          |  Canvas::update_scene(record::Scene)
                                          v
  Masonry RenderRoot --redraw()--> (VisualLayerPlan, accesskit::TreeUpdate)
                                          |  imaging_vello (vello 0.8, wgpu 28) on screen
                                          |  imaging_vello_cpu (vello_cpu 0.0.7) in masonry_testing
headless and snapshot path
  nuif-render CPU reference --> pixmap bytes --> Painter::draw_image     (no shared crate version)
```

Type-crossing rule derived from the probe: a type that crosses the widget boundary must be a single version. `accesskit::TreeUpdate` and `ActionRequest` cross between Masonry and the NUIF harness, so the editor harness uses Masonry's AccessKit line (0.24 at b81d8d7), not 0.25. `imaging` types cross through `Canvas`, so the NUIF imaging delegate uses Masonry's `imaging` line (0.0.1). `vello::Scene` never crosses because main offers no entry point; NUIF's own Vello backend (0.10, wgpu 29) is therefore a feature that the editor build disables, and the duplicate copies of Vello, wgpu and Parley in the probe disappear from the editor binary. Pixel buffers cross as bytes, so the CPU reference path keeps its own `vello_cpu` version.

Xilem's position: `xilem` diffs a view tree into Masonry widget mutations; its view coverage lags the widget set (issue #1710) and the editor needs explicit `WidgetId` to `EntityId` maps for the accessibility surface. Building on Masonry directly removes one layer whose identity allocation is internal to the framework.

Churn model: between v0.4.0 and b81d8d7 the `Widget` trait's paint signature, the renderer abstraction, the layout system and the dependency set changed, with no changelog; the release interval was 172 days (0.3.0 to 0.4.0) and 304 days from 0.4.0 to the retrieval date without a release. A git-revision pin with a single "toolkit bump" commit per update is the only reproducible way to consume main under `--locked`.

## NUIF relevance

**Borrow**
- Masonry main at a pinned git revision as the editor shell, because it is the only candidate that produces a Vello-compatible scene plan and an AccessKit `TreeUpdate` from one `redraw()` and rasterizes headlessly through `imaging_vello_cpu`.
- `Canvas::update_scene` with an `imaging::record::Scene` as the canvas contract, because it is the sanctioned custom-drawing widget and its recorded scene can be validated and replayed by the harness.
- `Split` (draggable, `min_lengths`), `CollapsePanel`, `Portal` with `ScrollBar`, `VirtualScroll`, `Selector`, `StepInput`, `TextInput`, `Switch`, `RadioGroup`, `Flex`, `Grid`, `ZStack`, `Align` as the widget basis for UI-SPEC regions A to E and the Design sections, because each exists on main and is listed as available in issue #1710.
- The `LayerStack` and tooltip layer for the floating toolbar, command palette and dialogs, because the layer model is documented for "tooltips, menus, dialogs".

**Adapt**
- ADR 0006's "Vello (interactive rendering)" becomes "imaging with the Vello backend": `nuif-render` gains an `imaging` render delegate that lowers `RenderScene` into `Painter` calls; the standalone Vello backend remains for the CLI and for tier 3 experiments and is disabled in the editor build.
- ADR 0006 gating decision 4 (versions pinned together) is extended: the editor crate declares `accesskit`, `accesskit_consumer`, `imaging`, `parley` and `kurbo` at Masonry's versions and no other NUIF crate depends on them; the harness comparator wraps `masonry_testing::TestHarness` behind the NUIF session-driver trait.
- Widgets absent on main are composed in the editor crate: layers tree from `VirtualScroll`, `DisclosureButton` and `Flex` with reparenting implemented as protocol `Move` operations triggered by pointer events on the canvas widget rather than a toolkit drag-and-drop protocol; tool group menus and the command palette from layers plus `TextInput` and `VirtualScroll`; colour and token controls from `StepInput`, `Slider`, `Selector` and `Canvas`; keyboard bindings from a shortcut table in the application, since Masonry has none.
- Shell screenshot tests use tier 2 tolerances (per-channel delta of at most 1) rather than Masonry's exact comparator, because the harness rasterizes with `vello_cpu` 0.0.7 through `imaging_vello_cpu` while the NUIF reference path is separate.

**Reject**
- Masonry 0.4.0 for the editor, because it lacks `Canvas`, paints into a `vello::Scene` of Vello 0.6, screenshots only through wgpu and pins AccessKit 0.21; none of the harness requirements in `conformance/HARNESS.md` is met by that release.
- Xilem for the reference editor at this stage, because its view coverage lags Masonry and the editor's state is NUIF state addressed by entity identity; the option stays open for demonstrations.
- egui as a Vello host, because `CallbackTrait::paint` is confined to egui's render pass and the text and vector stack would be duplicated, as the earlier record concluded.
- Blitz as the editor shell, because the harness crate is unpublished, the custom paint trait could not be verified, and the UI-SPEC panels would be authored in HTML and CSS against a beta document engine.
- GPUI, because tree queries are absent from its test contexts and its README requires the latest stable toolchain.

## Open questions

- Whether a Masonry 0.5.0 release with the `imaging` paint model, and an MSRV at or below the NUIF policy value, will be tagged before the editor crate is added; no milestone or announcement exists as of 2026-08-29.
- Whether `imaging` will offer an ingestion path for an existing `vello::Scene`, which would allow the standalone NUIF Vello backend to feed the canvas without a second lowering.
- Whether the `Split` pointer exclusivity defect (#1581) affects the resizable panels B and D in practice; a fixture in the editor harness is required.
- Whether Masonry will move to `accesskit_consumer` 0.36 or later, removing the `unsafe` node lookup in `masonry_testing`.
- Whether Xilem's `Canvas` view and its state diffing can be adopted later without changing the harness contract.
