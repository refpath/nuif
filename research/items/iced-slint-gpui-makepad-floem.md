---
id: nuif:research:iced-slint-gpui-makepad-floem
kind: synthesis
status: reviewed
title: Rust GUI toolkits compared for a headless-testable native editor (iced, Slint, GPUI, Makepad, Floem, Dioxus Native, egui, Masonry)
source:
  url: https://github.com/iced-rs/iced
  authors: [iced-rs, Slint (SixtyFPS GmbH), Zed Industries, Makepad, Lapce, DioxusLabs, emilk, Linebender]
  published_at: "iced 0.14.0 (2025-12-07); Slint 1.17.1 (2026-07-07); gpui 0.2.2 (2025-10-22); makepad-widgets 1.0.0 (2025-05-13); floem 0.2.0 (2024-11-14); dioxus-native 0.7.10 (2026-07-31); egui 0.36.1 (2026-08-07); masonry 0.4.0 (2025-10-29)"
  license: mixed (see table)
retrieved_at: 2026-08-29
tags: [gui, rust, comparison, iced, slint, gpui, makepad, floem, dioxus, blitz, headless, harness, accessibility, docking, wasm, licence]
confidence: 0.82
claims: [nuif:claim:semantic-automation]
relations:
  - type: compares_to
    target: nuif:research:egui-and-egui-kittest
    note: egui row; full record elsewhere.
  - type: compares_to
    target: nuif:research:masonry-xilem-and-linebender-test-harness
    note: Masonry row; full record elsewhere.
  - type: depends_on
    target: nuif:research:accesskit-semantic-ui-testing
    note: Accessibility-tree column is scored on AccessKit integration.
  - type: extends
    target: nuif:research:vello
    note: Vello-compatibility column.
  - type: related_to
    target: nuif:research:renderers
    note: Rendering backend column.
  - type: related_to
    target: nuif:research:taffy
    note: Floem, GPUI and Blitz use Taffy for layout.
links:
  spec: []
  adr: [adrs/0001-rust-reference-core.md, adrs/0003-reference-renderer.md]
  rfc: []
  code: [apps/editor/ARCHITECTURE.md, apps/editor/QA.md, Cargo.toml, .github/workflows/ci.yml]
  experiments: []
---

# Summary

Eight Rust GUI stacks were examined against the requirements in apps/editor/ARCHITECTURE.md and apps/editor/QA.md: the editor must be driven headlessly by the same engine as the CLI, must produce deterministic snapshots, and must be testable without synthetic pointer input. The comparison criteria are rendering backend, headless harness maturity, accessibility tree, text stack, docking/panels, WASM target, licence and MSRV. Every toolkit except Makepad (crates.io release) now ships a headless harness of some kind. Only egui (egui_kittest), Masonry (masonry_testing), Slint (i-slint-backend-testing) and Blitz (blitz-test-harness) expose a semantic tree that tests can query by role or label; of these, egui and Masonry use AccessKit as the query surface, Slint uses its own element tree with AccessKit only at the platform boundary, and Blitz uses DOM selectors with AccessKit behind a feature. Only Masonry, Floem and Blitz render through Vello. Every toolkit's MSRV (1.88 to 1.96) exceeds the NUIF pin of 1.85.0.

NUIF interpretation: the requirement set favours Masonry (Vello, Parley, AccessKit, CPU screenshot harness, owned tree) for a Rust-native editor, with egui_kittest as the reference for query ergonomics. iced and Slint have credible headless simulators but weaker or non-AccessKit semantic surfaces; GPUI demonstrates that a large editor can be tested headlessly with deterministic executors but ships no accessibility-tree test surface and is tied to Zed's release cadence.

## Evidence

- iced 0.14.0 released 2025-12-07; workspace `rust-version = "1.88"`, `license = "MIT"`, `edition = "2024"`; default features include `wgpu` and `tiny-skia` (software renderer); `cosmic-text = "0.15"`, `wgpu = "27.0"`, `winit = "0.30"`; no `accesskit` dependency appears in the workspace manifest. Locator: `Cargo.toml` lines 25-31, 158-238 at tag 0.14.0; `CHANGELOG.md` line 9.
- iced_test 0.14.0: "A library for testing iced applications in headless mode"; `simulator(view)`, `Simulator::{new, with_settings, with_size, find(selector), point_at, click(selector), tap_key, typewrite, simulate(events), snapshot(theme) -> Snapshot, into_messages}`; `Snapshot::{matches_image(path), matches_hash(path)}`; `snapshot()` renders at `scale_factor = 2.0` through `Renderer::screenshot` and requires `core::renderer::Headless`. Selectors: `&str` (text), `String`, `widget::Id`, `Point`, `selector::id`, `selector::is_focused`. Locator: `test/Cargo.toml`; `test/src/simulator.rs` lines 26-350; `selector/src/lib.rs` lines 14-154, tag 0.14.0.
- iced ships `pane_grid` (`PaneGrid`) in `iced_widget`; README lists "Cross-platform support (Windows, macOS, Linux, and the Web)". Locator: `widget/src/lib.rs` lines 28, 73 at tag 0.14.0; `README.md` line 31.
- Slint 1.17.1 released 2026-07-07 (crates.io); master is 1.18.0 with `rust-version = "1.92"`, `license = "GPL-3.0-only OR LicenseRef-Slint-Royalty-free-2.0 OR LicenseRef-Slint-Software-3.0"`. Locator: `Cargo.toml` lines 78-84, master; `LICENSE.md`.
- Slint renderers: `femtovg` (OpenGL ES 2.0), `skia`, software; "Web using WebAssembly" section in README. The winit backend has an `accessibility` feature pulling `accesskit = "0.24"` and `accesskit_winit = "0.33"`. Core text uses Parley by default (`default = ["std", "unicode", "shared-parley"]`, `accessibility-text` links `parley/accesskit`). Locator: `README.md` lines 55, 136-137; `internal/backends/winit/Cargo.toml` lines 58, 108-109; `internal/core/Cargo.toml` lines 81-92.
- Slint testing backend (i-slint-backend-testing 1.17.1): `init_integration_test_with_mock_time`, `init_integration_test_with_system_time`, `init_no_event_loop`, `mock_elapsed_time`; `ElementQuery::{from_root, match_descendants, match_id, match_type_name, match_inherits, match_accessible_role, match_predicate, find_first, find_all}`; `ElementHandle::{find_by_accessible_label, find_by_element_id, accessible_role, accessible_label, accessible_value, set_accessible_value, invoke_accessible_default_action, invoke_accessible_increment_action, mock_single_click, mock_drag, scroll, size, absolute_position}`; modules `mcp_server` and `systest`. Locator: `internal/backends/testing/lib.rs` lines 13-103; `search_api.rs` lines 227-1091, master; docs.rs i_slint_backend_testing 1.17.1.
- Slint screenshot tests: `tests/screenshots` with `software` and `skia` drivers, test cases generated by `build.rs` from `screenshots/cases`, markers `SLINT_SCALE_FACTOR=`, `BASE_THRESHOLD=`, `ROTATION_THRESHOLD=`; fonts pinned via `SLINT_DEFAULT_FONT`/`SLINT_FONT_PATH`; `SLINT_CREATE_SCREENSHOTS=1` writes references; default colour-difference threshold 0.1. Locator: `tests/screenshots/{main.rs,build.rs,testing.rs}`, master; `docs/testing.md`.
- gpui 0.2.2 (2025-10-22), `license = "Apache-2.0"`, "hybrid immediate and retained mode, GPU accelerated"; README requires "the latest version of stable Rust"; macOS renders with Metal (`objc2-metal`) and text needs `font-kit` (Zed fork); Linux/FreeBSD need `wayland`/`x11`; Windows uses Win32 and DirectWrite; `taffy = "=0.13.0"`; `accesskit.workspace = true` and an `a11y` example exist. Zed repository licence file is GPL (`LICENSE-GPL`). Locator: `crates/gpui/Cargo.toml` lines 3, 9, 50, 96, 120-131, 256; `crates/gpui/README.md`; `LICENSE-GPL`, main commit e3adf43.
- GPUI test infrastructure: `TestAppContext::{build, single, add_window, add_empty_window -> VisualTestContext, simulate_keystrokes, simulate_input, dispatch_action, run_until_parked, simulate_window_resize, simulate_prompt_answer, write_to_clipboard, read_from_clipboard, windows}`; `VisualTestContext::{simulate_mouse_move, simulate_mouse_down, simulate_mouse_up, simulate_click(position, modifiers), simulate_modifiers_change, simulate_resize, window_title}`; `#[gpui::test]` accepts `seed`, `seeds`, `iterations`, `retries`, `on_failure`; `TestPlatform` records prompts and windows. Locator: `crates/gpui/src/app/test_context.rs` lines 21-883; `crates/gpui_macros/src/test.rs` lines 14-88; `crates/gpui/src/platform/test/platform.rs` lines 26-217.
- Zed editor tests: `crates/editor/src/editor_tests.rs` contains 550 `#[gpui::test]` attributes; `EditorTestContext::{set_state(marked_text), set_selections_state, assert_state_with_diff, simulate_keystroke, run_until_parked, buffer_text, display_text, pixel_position}`. Locator: `crates/editor/src/editor_tests.rs` (count via grep); `crates/editor/src/test/editor_test_context.rs` lines 37-439.
- Zed workspace has `crates/workspace/src/dock.rs` and `pane.rs` (docking implemented in the application, not in gpui). Locator: repository tree, main.
- Makepad: crates.io `makepad-widgets` 1.0.0 (2025-05-13); `dev` branch widgets crate is 2.0.0 (`MIT OR Apache-2.0`), repository licence MIT; README: "A cross-platform UI runtime for native and web targets", "Rust-first framework with a scriptable UI DSL"; no `accesskit` path in the tree. Locator: `widgets/Cargo.toml`; `README.md`; tree grep, dev branch commit (pushed 2026-08-29).
- Makepad test crate `libs/makepad_test` 0.1.0 (path dependency, unpublished): `#[makepad_test] fn t(app: TestApp)`, `Selector::id`, `.wait_visible()`, `.click()`, `.fill()`, `.wait_text()`, `.wait_value()`, `app.press_return()`; "drive the app through the existing Studio protocol in headless mode"; documented invocation uses `--test-threads=1`; `MAKEPAD_TEST_VISIBLE=1` targets a running Studio at `127.0.0.1:8001`. Locator: `libs/makepad_test/README.md`, `examples/counter/tests/ui.rs`, dev branch.
- Floem: crates.io 0.2.0 (2024-11-14); main manifest 0.2.0, `rust-version = "1.91"`, `license = "MIT"`; renderers vger, vello, AnyRender Skia (GPU) with tiny-skia CPU fallback; `parley = "0.7.0"`; Taffy layout; `src/headless.rs` exposes `HeadlessHarness::new(view)` and `pointer_down(x, y)`; `src/platform/wasm_stubs.rs` exists; no `accesskit` dependency. Locator: `Cargo.toml` lines 29-97; `README.md` "Features"; `src/headless.rs` lines 1-46; tree grep, main.
- Dioxus Native/Blitz: dioxus-native 0.7.10 (2026-07-31; 0.8.0-alpha.1 pre-release); blitz workspace main is 0.3.0-beta.2 (`blitz-dom` 0.2.4 stable), `license = "MIT OR Apache-2.0"`, `rust-version = "1.91.0"`; dependencies `anyrender_vello 0.14.0`, `anyrender_vello_cpu 0.17.0`, `anyrender_skia 0.11.0`, `parley 0.11.1`, `taffy 0.14.0`, `accesskit 0.24` via `accesskit_xplat`; dioxus-native feature `accessibility` enables AccessKit; README lists "Accessibility using AccessKit" as an intended goal and states beta status. Locator: `Cargo.toml` lines 34-137 (blitz main); `packages/native/{src/lib.rs,Cargo.toml}` (dioxus main); `README.md` "Status", "Goals".
- `blitz-test-harness`: "Headless test harness for Blitz documents"; `Harness::from_html`, `Harness::from_component`, `pump`/`tick` with a controlled animation clock, selectors, layout rects, hit-testing, tree dumps, input synthesis routed "through the real event-dispatch pipeline, without requiring a window"; "No window, GPU, or compositor is required". Locator: `packages/blitz-test-harness/src/lib.rs` lines 1-14, main.
- egui and Masonry rows: see nuif:research:egui-and-egui-kittest and nuif:research:masonry-xilem-and-linebender-test-harness (MSRV 1.95 and 1.88/1.96; AccessKit-native harnesses; Vello only in Masonry).

## Mechanism

Comparison table. "unverified" marks cells not confirmed against a primary source in this retrieval; all other cells cite the Evidence section above.

| Toolkit (version) | Rendering backend | Headless harness | Accessibility tree | Text stack | Docking / panels | WASM target | Licence | MSRV |
|---|---|---|---|---|---|---|---|---|
| egui 0.36.1 | egui-wgpu (wgpu 30), also glow (unverified) | egui_kittest: AccessKit queries, pointer/key synthesis, wgpu CPU-adapter snapshots, `UPDATE_SNAPSHOTS` | AccessKit every frame; kittest queries by role/label/value | epaint glyph atlas, `Context::set_fonts`; no shaping pipeline (unverified) | egui_dock 0.21.1, egui_tiles 0.17.1 (external crates) | yes (eframe web demo, README) | MIT OR Apache-2.0 | 1.95 |
| Masonry/Xilem 0.4.0 (main b81d8d7) | Vello on wgpu; main adds imaging_vello_cpu/hybrid/skia | masonry_testing `TestHarness`: event injection, virtual time, `accesskit_consumer::Tree`, `assert_render_snapshot!`, `MASONRY_TEST_BLESS` | AccessKit `TreeUpdate` per redraw; `access_node(WidgetId)`, `accessibility_click_on` | Parley (Fontique, HarfRust, Skrifa, ICU4X) | none built in (unverified) | xilem_web targets DOM, not Masonry (unverified) | Apache-2.0 | 1.88 (0.4.0), 1.96 (main) |
| iced 0.14.0 | wgpu 27 or tiny-skia software | iced_test `Simulator`: `click(selector)`, `typewrite`, `snapshot(theme)` with `matches_image`/`matches_hash` | none (no accesskit dependency) | cosmic-text 0.15 | `pane_grid` widget | web listed in README | MIT | 1.88 |
| Slint 1.17.1 (master 1.18.0) | femtovg (GLES2), Skia, software renderer | i-slint-backend-testing: `ElementQuery`/`ElementHandle`, mock time, `mock_single_click`, `invoke_accessible_default_action`; CI screenshot drivers with thresholds | own element tree with `accessible_*` properties; AccessKit 0.24 at winit backend (`accessibility` feature) | Parley (`shared-parley` default) | none verified | yes (README "Web using WebAssembly") | GPL-3.0-only OR Slint Royalty-free 2.0 OR Slint commercial | 1.92 |
| GPUI 0.2.2 (Zed main e3adf43) | Metal (macOS); Linux/Windows renderer (unverified: blade) | `TestAppContext`/`VisualTestContext`: deterministic executor, `run_until_parked`, keystroke/mouse simulation, seeded `#[gpui::test]` with iterations/retries; 550 editor tests | accesskit dependency and a11y example; no tree query API in test contexts (unverified beyond grep) | font-kit (Zed fork), CoreText/DirectWrite | Zed `workspace::{dock, pane}` (application level) | no (unverified) | Apache-2.0 (crate); Zed app GPL/AGPL | "latest stable" |
| Makepad widgets 1.0.0 (dev 2.0.0) | own shader-based renderer (README) | makepad_test 0.1.0 (unpublished, dev): `TestApp`, `Selector::id`, `wait_text`, headless via Studio protocol, `--test-threads=1` | none (no accesskit in tree) | own (unverified) | Studio app has panels (unverified) | yes (README "native and web") | MIT (repo); MIT OR Apache-2.0 (crates) | unverified |
| Floem 0.2.0 (main) | vger, Vello, AnyRender Skia; tiny-skia CPU fallback | `floem::headless::HeadlessHarness` (pointer synthesis) | none (no accesskit dependency) | Parley 0.7.0 | none verified | wasm stubs only (unverified) | MIT | 1.91 |
| Dioxus Native 0.7.10 / Blitz 0.3.0-beta.2 | AnyRender: Vello, Vello CPU, Vello Hybrid, Skia | blitz-test-harness: `from_html`/`from_component`, `pump`/`tick`, selectors, layout rects, hit-testing, input synthesis | AccessKit 0.24 via `accesskit_xplat` behind `accessibility` feature | Parley 0.11.1 | none verified | dioxus-web is a separate DOM renderer (unverified) | MIT OR Apache-2.0 | 1.91 |

Harness capability matrix relevant to QA.md items (interpretation): semantic queries without pointer input are available in egui (AccessKit), Masonry (AccessKit), Slint (own tree), Blitz (DOM selectors); iced selects by visible text or widget ID; GPUI dispatches actions and keystrokes but selects by view handles; Makepad selects by element id over a protocol; Floem selects by coordinates.

## NUIF relevance

**Borrow**
- Masonry's combination (owned tree, Vello scene output, AccessKit tree, CPU screenshot harness) as the baseline architecture for a Rust-native editor, because it satisfies headless-testable, Vello-compatible and accessibility-tree-driven simultaneously.
- egui_kittest's query ergonomics (`get_by_role_and_label`, `click_accesskit`) as the API model for a NUIF harness, because they are the most direct expression of QA item 2 and QA item 3 without pointer synthesis.
- GPUI's seeded, iterated, deterministic-executor test attribute as the model for NUIF operation-sequence tests, because it shows that editor-scale interaction tests can be deterministic under a controlled scheduler.

**Adapt**
- Slint's marker-driven screenshot thresholds (`BASE_THRESHOLD=` in the fixture) can be transposed to NUIF render fixtures, because conformance/PLAN.md requires declared tolerances per fixture rather than a global threshold.
- Blitz's `pump`/`tick` clock and iced's `into_messages` drain suggest a NUIF harness API where every interaction yields the list of protocol `Operation`s it produced, because ARCHITECTURE.md requires gestures to become semantic operations before mutation.

**Reject**
- iced as the editor shell, because it has no accessibility tree in 0.14.0 and its selectors are text- or ID-based, which cannot satisfy role/relationship queries.
- Slint for the reference editor, because the GPL/royalty-free/commercial triple licence conflicts with the `Apache-2.0 OR MIT` workspace policy for a vendor-neutral reference implementation, and its DSL owns the widget tree rather than a Rust API.
- Makepad and Floem for a headless-first editor, because neither exposes an accessibility tree and their harnesses are coordinate- or protocol-driven.
- GPUI as a dependency, because it tracks "the latest version of stable Rust", is versioned with Zed's monorepo, and offers no accessibility-tree query API for tests.

## Open questions

- Whether the NUIF MSRV pin (1.85.0) will be raised; every candidate requires 1.88 or newer, so the editor cannot be added to the current workspace without a toolchain decision.
- Whether Blitz (HTML/CSS document model with Vello, Parley, Taffy, AccessKit) is a viable canvas host for NUIF documents, given that NUIF already lowers to CSS-compatible layout through Taffy.
- Whether GPUI's accessibility integration exposes an AccessKit tree to tests in a later release, which would change its column.
- Whether iced will add AccessKit (an open topic in the iced repository; not verified here).
