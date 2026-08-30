---
id: nuif:adr:0006
kind: adr
status: accepted
---

# ADR 0006: Rust-native reference editor on Masonry, Vello and AccessKit; toolchain policy

Decision delegated to research on 2026-08-29. Evidence: `nuif:research:masonry-editor-stack-decision`, `nuif:research:rust-toolchain-and-msrv-policy`, `nuif:research:masonry-xilem-and-linebender-test-harness`, `nuif:research:egui-and-egui-kittest`, `nuif:research:iced-slint-gpui-makepad-floem`, `nuif:research:accesskit-semantic-ui-testing`, `nuif:research:vello-testing-and-cpu-reference`, `nuif:research:wasm-headless-execution`.

## Context

`docs/whitepaper/06-language-and-runtime-choice.md` and `apps/editor/ARCHITECTURE.md` proposed a Svelte 5 and TypeScript shell over a Rust/WASM core. The editor's role (`apps/editor/README.md`, RFC 0004) is a test instrument: headless execution, deterministic snapshots through a CPU reference path, an accessibility tree that carries entity identifiers, and the same harness as the CLI.

## Decision

### Stack

1. The reference editor is a Rust binary crate (`apps/editor`) built directly on Masonry (retained widget tree) with Vello rendering and AccessKit. Xilem is not used: its view layer lags the widget set (xilem issue 1710) and entity-to-widget identity must be explicit in the editor.
2. Masonry is pinned by git revision to the main branch (the 0.4.0 release of 2025-10-29 lacks the `Canvas` widget, paints into a Vello 0.6 `Scene`, and screenshots only through wgpu). Main (`b81d8d7`, 2026-08-28) provides `Widget::paint` over an `imaging::Painter`, `Canvas::update_scene` recording an `imaging::record::Scene`, and `masonry_testing` rasterization through `imaging_vello_cpu` without a GPU.
3. The editor canvas is a lowering from NUIF `RenderScene` to `imaging` commands. NUIF's own CPU reference renderer remains the conformance oracle; Masonry's `vello_cpu` path rasterizes shell chrome for headless screenshot tests only.
4. For every type that crosses the widget boundary (Vello, Parley, AccessKit, wgpu), the editor crate follows Masonry's pinned versions. A `cargo metadata` probe of Masonry main with independently chosen Vello 0.10, Parley 0.11 and AccessKit 0.25 produced duplicate Vello (0.8/0.10), wgpu (28/29), Parley (0.8/0.11.1), AccessKit (0.24.1/0.25) and four `accesskit_consumer` versions; the editor build therefore disables NUIF's standalone Vello backend and exchanges pixel buffers with the reference path as bytes.
5. The Svelte shell becomes a later browser demonstration of the WASM bindings. Engine parity in browsers is tested through `wasm-bindgen-test` and Playwright layout oracles; in-browser WebGPU pixel oracles are not used.

### Toolchain and MSRV

6. `rust-toolchain.toml` pins the current stable, 1.98.0 (released 2026-08-20). The pin is raised in one dedicated commit within each release cycle.
7. `rust-version` is 1.96: `max(toolchain − 2, highest dependency MSRV)`, where Masonry main requires 1.96, `imaging` 1.92, Vello and Parley 1.88, wgpu 1.87, AccessKit, HarfRust and proptest 1.85. This coincides with Masonry's own N-0..N-2 practice and with Bevy (1.96), Zed and Servo (1.97.1) and rust-analyzer (1.98).
8. The workspace uses `resolver = "3"` so that dependency resolution is MSRV-aware.
9. CI runs the main job on the pinned toolchain and an `msrv` job that checks the workspace on 1.96.0.

### Licensing

10. Masonry is licensed Apache-2.0 only; Vello, Parley and `imaging` are `Apache-2.0 OR MIT`; AccessKit is `MIT OR Apache-2.0`; `accesskit_winit`, `winit`, `tree_arena`, `insta` and `ciborium` are Apache-2.0 only. Depending on Apache-2.0-only crates does not constrain NUIF's `Apache-2.0 OR MIT` (Apache-2.0 §4 obliges notice retention on redistribution and permits different terms for one's own work); the only downstream effect is on GPLv2-only consumers.
11. `deny.toml` allows exactly: `MIT`, `Apache-2.0`, `Apache-2.0 WITH LLVM-exception`, `BSD-2-Clause`, `BSD-2-Clause-Patent`, `BSD-3-Clause`, `ISC`, `Zlib`, `Unicode-3.0`, `BSL-1.0`, `CC0-1.0`, `0BSD`, `Unlicense`. `BlueOak-1.0.0` is not allowed; `minicbor` is therefore excluded (RFC 0005 selects `dcbor`, BSD-2-Clause-Patent).

## Rationale

- Masonry main is the only candidate whose harness returns the frame's visual layer plan and AccessKit `TreeUpdate` from one `redraw()`, accepts `ActionRequest`s directly, controls time, hosts a custom-painted canvas, and rasterizes without a GPU.
- egui: `egui-wgpu` callbacks paint inside egui's own render pass, while Vello requires compute passes; egui would duplicate the text and vector stack; `egui_kittest`'s query API remains the model for NUIF's harness surface.
- Floem: no AccessKit (issues 8 and 973 open). Blitz: custom paint sources exist but the harness is unpublished. GPUI: builds an AccessKit tree per frame but exposes no query in test contexts and requires latest stable. iced: no accessibility tree. Slint: licence and DSL-owned element tree.

## Widget inventory

Masonry main ships 37 widgets including `Split` (draggable), `CollapsePanel`, `Selector`, `StepInput`, `TextInput`, `VirtualScroll`, `Canvas` and a tooltip layer. Missing for `apps/editor/UI-SPEC.md` and composed in the app crate: tree view (layers panel), drag-and-drop reparenting (emits protocol `Move` from canvas pointer events), menus and tab strip, colour picker, keyboard-shortcut table, multi-line text input (newlines unsupported in `TextInput`), text undo/redo (xilem issue 1417).

## Risks and mitigations

- API churn: paint signature, layout system and renderer changed between 0.4.0 and main without a changelog; 304 days without a release. Mitigation: git pin, single bump commits, harness behind NUIF's session-driver trait, no Masonry types in `crates/`.
- `Split` pointer defect (xilem issue 1581); `unsafe` NodeId workaround in `access_node` (AccessKit issue 701, fixed in `accesskit_consumer` 0.36 while Masonry pins 0.35). Mitigation: isolated in the app crate; `unsafe_code = "forbid"` stays in `crates/`.
- `vello_cpu` regressions affect shell screenshots only (tier 2 tolerance); the render suite uses NUIF's CPU reference.

## Consequences

- `rust-toolchain.toml`, `Cargo.toml` (`rust-version`, `resolver`), `.github/workflows/ci.yml` and `deny.toml` are updated with this ADR.
- `apps/editor/ARCHITECTURE.md`, `apps/editor/UI-SPEC.md` and `docs/whitepaper/06-language-and-runtime-choice.md` reference this ADR.
- The editor exposes entity identifiers through AccessKit `author_id`; the harness queries by role, label and identifier and dispatches actions without pointer synthesis.

## Unverified

Linebender Zulip release announcements were not retrieved; the `anyrender` `CustomPaintSource` trait was not located; the 1.99 release date (2026-10-01) is computed from the cadence, not announced.
