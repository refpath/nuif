---
id: nuif:research:editor-fork-exit-review
kind: synthesis
status: reviewed
title: Editor fork exit conditions and supported accessibility lookup
source:
  url: https://github.com/linebender/xilem
  authors: [Linebender contributors, AccessKit contributors, egui contributors, Iced contributors, Slint contributors]
  published_at: "Upstream Xilem b81d8d7 (2026-08-28); egui 0.36.1; Iced 0.14.0; Slint 1.17.1"
  license: Multiple upstream licences
retrieved_at: 2026-09-06
tags: [dependencies, gui, accessibility, maintenance, testing]
confidence: 0.95
claims: []
relations:
  - type: extends
    target: nuif:research:masonry-editor-stack-decision
    note: Corrects the egui rejection and checks fork divergence against current upstream.
  - type: extends
    target: nuif:research:macos-metal-block-future-incompatibility
    note: Defines the upstream dependency changes required to retire both active forks.
links:
  spec: []
  adr: [adrs/0006-rust-native-editor.md]
  rfc: []
  code: [apps/editor/src/gui/automation.rs, apps/editor/src/gui.rs, apps/editor/Cargo.toml]
  experiments: []
---

# Summary

The upstream Xilem main revision remains
`b81d8d7a631849def6eeab282561439b963862e5` on 2026-09-06. The selected fork is
exactly two commits ahead: the wgpu 29 migration and the replacement of
abandoned font dependencies. No new upstream revision removes the need for
these changes. The smaller ui-events fork changes one dependency declaration
to disable Winit defaults. The active lockfile contains neither `block`,
`metal`, `rustybuzz` nor `ttf-parser`. The separate metal-rs review fork is not
a dependency. Locators: `Cargo.lock`,
[upstream commit](https://github.com/linebender/xilem/commit/b81d8d7a631849def6eeab282561439b963862e5),
and [fork comparison](https://github.com/refpath/xilem/compare/b81d8d7a631849def6eeab282561439b963862e5...1b96eb8db3f88f85db1a3594d80d3480b29392fb),
retrieved 2026-09-06.

## Evidence

### Feature propagation

[ui-events fork commit 8b173c1](https://github.com/refpath/ui-events/commit/8b173c130d7cb9ba6cf91c5ca10e929c17c3b996)
sets `default-features = false` on Winit. Upstream main
`3934f66a2e467072728ca6c2fc8c25ad9578befc` still declares `winit = "0.30.10"`.
Winit 0.30.13's default `wayland-csd-adwaita` feature enables
`sctk-adwaita/ab_glyph`; the fork selects `wayland-csd-adwaita-notitle` instead.
Cargo feature unification does not let a root manifest subtract a feature
enabled by another dependency. Restoring unmodified ui-events therefore
restores the font dependency path on Linux. Locators:
[upstream adapter manifest](https://github.com/endoli/ui-events/blob/3934f66a2e467072728ca6c2fc8c25ad9578befc/ui-events-winit/Cargo.toml),
[Winit features](https://github.com/rust-windowing/winit/blob/v0.30.13/Cargo.toml),
and [Cargo feature unification](https://doc.rust-lang.org/cargo/reference/features.html#feature-unification),
retrieved 2026-09-06.

### Accessibility lookup

The pinned `masonry_testing::TestHarness::access_node` writes a raw widget ID
into an opaque consumer node ID with an unsafe pointer conversion. Its own
comment states that the memory layout is not guaranteed. AccessKit Consumer
0.35 already supplies `TreeState::root`, `Node::children`, `Node::id` and
`TreeState::locate_node`. The last method returns the public local node ID
and tree ID for a valid consumer node. These APIs support traversing and
indexing the existing tree without constructing opaque IDs. Locators:
[pinned harness](https://github.com/refpath/xilem/blob/1b96eb8db3f88f85db1a3594d80d3480b29392fb/masonry_testing/src/harness.rs),
[locate_node](https://docs.rs/accesskit_consumer/0.35.0/accesskit_consumer/struct.TreeState.html#method.locate_node),
retrieved 2026-09-06.

NUIF's `collect_semantics` traverses once and indexes only nodes in
`TreeId::ROOT`, because the editor's widget map belongs to that tree.
Lookup cost is O(N log N + Q log N) for N accessibility nodes and Q queried
widgets, with O(N) additional memory. The semantic records retain the original
widget IDs, authored IDs, roles, labels and values. The editor identity test
also uses public traversal. The dependency still contains the unsafe helper;
NUIF no longer calls it. Locators: `apps/editor/src/gui/automation.rs`,
`collect_semantics`; `apps/editor/src/gui.rs`,
`shell_renders_and_exposes_entity_identity`.

### Toolkit alternatives

| Candidate | Verified capability | Remaining migration requirement |
| --- | --- | --- |
| egui 0.36.1 and egui_kittest | AccessKit queries, actions and a released test harness. Callback preparation accepts command buffers before the paint pass. | The published snapshot renderer uses wgpu. A CPU shell snapshot path with the existing tolerance contract has not been demonstrated. |
| Iced 0.14.0 | Tiny-skia software rendering and test tooling are released features. | AccessKit integration remains an open draft pull request, #3111. It is not a released substitute for the current semantic harness. |
| Floem 0.2.0 | A maintained upstream repository is available. | Accessibility issues #8 and #973 remain open. An equivalent semantic test surface has not been demonstrated. |
| Slint 1.17.1 | Software rendering and system-testing facilities exist. | The framework licence choices differ from this repository's dependency policy, and the DSL-owned element tree requires an editor port. |

Locators, retrieved 2026-09-06:
[egui_kittest manifest](https://github.com/emilk/egui/blob/0.36.1/crates/egui_kittest/Cargo.toml),
[callback contract](https://github.com/emilk/egui/blob/0.36.1/crates/egui-wgpu/src/renderer.rs),
[Iced manifest](https://github.com/iced-rs/iced/blob/0.14.0/Cargo.toml),
[Iced AccessKit pull request](https://github.com/iced-rs/iced/pull/3111),
[Floem issue 8](https://github.com/lapce/floem/issues/8),
[Floem issue 973](https://github.com/lapce/floem/issues/973),
[Slint features](https://github.com/slint-ui/slint/blob/v1.17.1/api/rs/slint/Cargo.toml),
[Slint licences](https://github.com/slint-ui/slint/blob/v1.17.1/LICENSE.md).

### CPU renderer probe

[egui_software_backend 0.0.3](https://docs.rs/egui_software_backend/0.0.3/egui_software_backend/)
supports egui 0.34 and includes an optional egui_kittest integration. Its
published `test_render` feature enables the default image codecs. A resolution
probe on 2026-09-06 selected 166 packages and reported unmaintained `paste`
1.0.15 through the EXR and AVIF codec paths (RUSTSEC-2024-0436). This is an
unmaintained-package warning, not a reported vulnerability. Locators:
[0.0.3 dependencies](https://crates.io/api/v1/crates/egui_software_backend/0.0.3/dependencies),
[upstream manifest at c70e02d](https://github.com/DGriffin91/egui_software_backend/blob/c70e02d3d95c75a82733deb97e516b13a6c86dc6/Cargo.toml),
and [RustSec advisory](https://rustsec.org/advisories/RUSTSEC-2024-0436.html),
retrieved 2026-09-06.

A second local probe used the renderer with default features disabled,
egui and egui_kittest 0.34.3, and an implementation of the public
`TestRenderer` trait with image defaults disabled. It used neither a source
patch nor a forced dependency version outside the declared ranges. The
resolved graph contained 112 packages, no wgpu renderer, no reported
vulnerabilities and no advisory warnings. The 320 × 240 px trial queried and
activated an accessible checkbox, changed a custom-painted canvas, observed
different pixels after the action and identical pixels on repeated renders
before and after it. Locators: `.local-audit/strategy/egui-cpu-core/main.rs`,
its manifest and lockfile, `egui-cpu-core-run.log`,
`egui-cpu-core-audit.json`; local run 2026-09-06. These probe artifacts are
local audit evidence and are not distributed with the repository.

This result demonstrates a supported CPU/semantic integration path on egui
0.34. It does not establish whole-editor parity, performance, native input
behavior, cross-platform reproducibility or compatibility with egui 0.36.
The current upstream manifest still targets 0.34. A migration should evaluate
the existing editor scenario and typography before selecting this stack.

## NUIF relevance

**Adapt** the prior egui comparison. Rendering into an intermediate texture
before the egui paint pass and sampling it during painting is a permitted
integration design. The earlier categorical rejection of egui as a Vello host
was too broad. That design is an inference from the callback contract; a NUIF
prototype and performance measurements have not been completed.

**Borrow** the existing AccessKit public tree APIs to remove NUIF calls to the
unsafe helper. This change neither updates the toolkit API nor expands the
fork. Editor unit tests, lint checks and the semantic/visual editor trial
verify the replacement.

**Reject** a toolkit rewrite solely to remove two dependency-source entries.
The acceptance requirements remain authored identity, semantic actions,
custom canvas rendering, keyboard and text input, deterministic CPU shell
snapshots, and the existing editor trial. A replacement must demonstrate
those requirements before the current shell is removed.

## Fork exit conditions

The preferred exit remains an upstream Masonry revision with a compatible
imaging/wgpu 29 path, maintained SVG/font dependencies, and Winit features
that do not restore the abandoned parser. Both native rendering and the
CPU/semantic harness must pass the existing editor trial. The ui-events
fork can be retired independently when its upstream Winit dependency permits
that feature configuration. A semver override cannot substitute for either
API migration or Cargo's additive feature behavior.

If upstream maintenance does not provide this path, egui is the first
candidate for a bounded editor-shell experiment. The CPU probe establishes
basic rendering and semantic actions on a compatible released version set.
The unresolved items are the full editor snapshot contract, authored identity
mapping, text input and a current-version renderer integration. The probe is
not an editor replacement or evidence of superior performance.
