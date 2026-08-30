---
id: nuif:research:macos-metal-block-future-incompatibility
kind: repository
status: verified
title: macOS Metal dependency and Rust uninhabited-static future incompatibility
source:
  url: https://github.com/gfx-rs/wgpu/pull/5641
  repository: https://github.com/gfx-rs/wgpu
  authors: [gfx-rs developers, Linebender developers, refpath maintainers]
  published_at: "wgpu pull request 5641 merged 2026-01-28; refpath/xilem commits eabfe0a and 1b96eb8 created 2026-08-30"
  license: "wgpu MIT OR Apache-2.0; Xilem Apache-2.0"
retrieved_at: 2026-08-30
tags: [rust, macos, metal, wgpu, dependency, future-incompatibility, release-risk]
confidence: 0.95
claims: []
relations:
  - type: depends_on
    target: nuif:research:masonry-xilem-and-linebender-test-harness
    note: The native editor uses the reviewed refpath Xilem fork to reach wgpu 29 and the objc2 Metal bindings.
  - type: related_to
    target: nuif:research:rust-toolchain-and-msrv-policy
    note: Rust 1.98 reported the removed block 0.1.6 dependency as future-incompatible.
links:
  spec: []
  adr: [adrs/0006-rust-native-editor.md]
  rfc: []
  code: [Cargo.lock, apps/editor/Cargo.toml]
  experiments: []
---

# Summary

The previous macOS editor graph resolved `masonry_winit -> imaging_wgpu -> wgpu
28.0.0 -> wgpu-hal 28.0.1 -> metal 0.33.0 -> block 0.1.6`. Rust 1.98 compiled
that graph but reported `block` as future-incompatible. wgpu pull request 5641
replaced metal-rs with `objc2-metal` and `block2`; wgpu 29 contains that
migration. The reviewed `refpath/xilem` commit `eabfe0a` updates the NUIF
Masonry revision to the wgpu 29 API without patching the Objective-C blocks ABI.
NUIF pins its immediate descendant `1b96eb8` by full SHA; that descendant
replaces abandoned font dependencies without changing the graphics migration.

The active editor graph resolves `imaging_wgpu 0.0.2 -> wgpu 29.0.4 ->
objc2-metal 0.3.2 -> block2 0.6.2`.
`block 0.1.6` and metal-rs are absent from the lock file and active dependency
graph. The fork remains a maintenance boundary until the equivalent migration
is available from the selected upstream Xilem revision.

## Evidence

- wgpu pull request 5641 replaced `metal` with `objc2-metal`, `block` with
  `block2`, and `core-graphics-types` with `objc2-core-graphics`. The pull
  request merged on 2026-01-28. Locator: dependency and source-file changes,
  retrieved 2026-08-30: https://github.com/gfx-rs/wgpu/pull/5641.
- `wgpu-hal 29.0.4` declares `block2 0.6.2`, `objc2-metal 0.3.2`, and
  `objc2-quartz-core 0.3.2` for the Metal backend. Locator: the macOS target
  dependencies in `wgpu-hal/Cargo.toml` at tag `v29.0.4`, retrieved 2026-08-30:
  https://github.com/gfx-rs/wgpu/blob/v29.0.4/wgpu-hal/Cargo.toml.
- `refpath/xilem` commit `eabfe0a92ff5ab0e26515383fdeaf288672b3e88`
  updates the imaging dependencies and the three affected wgpu API call sites.
  Locator: commit diff, retrieved 2026-08-30:
  https://github.com/refpath/xilem/commit/eabfe0a92ff5ab0e26515383fdeaf288672b3e88.
- The active pin `1b96eb8db3f88f85db1a3594d80d3480b29392fb` has `eabfe0a`
  as its sole parent and replaces abandoned font dependencies. Locator: commit
  metadata and diff, retrieved 2026-08-30:
  https://github.com/refpath/xilem/commit/1b96eb8db3f88f85db1a3594d80d3480b29392fb.
- `cargo tree -p nuif-editor -i block` reports no matching package after the
  migration. `cargo tree -p nuif-editor` resolves wgpu 29.0.4, objc2-metal
  0.3.2, and block2 0.6.2 from the active lock file (2026-08-30).
- `cargo report future-incompatibilities` reports that no reports are available
  after rebuilding `nuif-editor` against the migrated graph with rustc 1.98.0
  (2026-08-30).
- `cargo test -p nuif-editor --features editor-automation` passes 14 tests
  against the pinned fork commit. A macOS Metal window smoke test reaches the
  event loop and presents without a surface error (2026-08-30).
- Upstream metal-rs commit `9ed9fe9` still declares `block 0.1.6` and its
  README now deprecates the crate in favor of `objc2-metal`. The review-only
  `refpath/metal-rs` branch `move-to-block2`, commit `7e0a178`, replaces the
  production dependency with `block2 0.6.2`, migrates the typed callbacks, and
  replaces the shared-event layout mutation with block2's explicit ABI
  encoding. No pull request was opened and NUIF does not depend on this fork.
  Locator: https://github.com/refpath/metal-rs/tree/move-to-block2, retrieved
  2026-08-30.
- The review fork passes the upstream crate checks and tests on Rust 1.82, its
  declared MSRV. Real macOS probes completed both an `MTLSharedEvent`
  notification and a command-buffer completion handler. Its normal dependency
  graph contains block2 and no rust-block; the all-target development graph
  still reaches rust-block through the deprecated `cocoa 0.26` dev dependency.
  This makes the branch useful for review, but not a complete modernization of
  the deprecated objc stack.

## Mechanism

`masonry_winit` constructs the wgpu instance, acquires each surface texture,
and accesses the Metal presentation layer during macOS live resize. wgpu 29
changes the instance descriptor from a borrowed value to an owned value and
returns `CurrentSurfaceTexture` instead of `Result<SurfaceTexture,
SurfaceError>`. The fork updates those call sites and maps `Timeout` and
`Occluded` to a skipped frame, as specified by the wgpu 29 variant
documentation. The live-resize path calls the objc2 selector spelling exposed
by `objc2-quartz-core`. The dependency update selects wgpu-hal's objc2 Metal
backend, so metal-rs and rust-block no longer participate in compilation or
linking. Locator: `masonry_winit/src/vello_util.rs` and
`masonry_winit/src/event_loop_runner.rs` in `refpath/xilem` commit `eabfe0a`;
`CurrentSurfaceTexture` in wgpu 29.0.4, retrieved 2026-08-30:
https://docs.rs/wgpu/29.0.4/wgpu/enum.CurrentSurfaceTexture.html.

## Decision boundary

**Borrow** wgpu 29's objc2 Metal backend as the maintained replacement for
metal-rs and rust-block.

**Adapt** the selected Xilem revision through a full-SHA fork pin. Each fork
update includes the NUIF editor tests, the reverse dependency trace, and a
macOS Metal window smoke test.

**Contain** the direct metal-rs experiment in the review-only
`refpath/metal-rs` branch. It demonstrates a small block2 migration but changes
callback-facing Rust types and cannot remove rust-block from the legacy Cocoa
development graph. NUIF therefore does not pin or ship the experiment.

## NUIF relevance

**Borrow** Cargo's future-incompatibility report and reverse dependency tree as
the reproducible diagnostics for the pinned toolchain and lock file.

**Adapt** dependency-update review so a macOS editor-stack change verifies the
absence of `block`, the presence of the expected `block2` path, and a package
smoke test in addition to the workspace checks.

**Reject** treating the fork as an indefinite divergence. The pin is removed
when the selected upstream Xilem revision provides an equivalent wgpu version.

## Open questions

- Which upstream Xilem revision first provides a compatible wgpu 29 or later
  dependency graph?
- When does `imaging_skia` publish a release that no longer requires wgpu 28 for
  its optional GPU backend?
