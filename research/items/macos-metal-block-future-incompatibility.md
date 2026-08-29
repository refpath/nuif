---
id: nuif:research:macos-metal-block-future-incompatibility
kind: repository
status: reviewed
title: macOS Metal dependency and Rust uninhabited-static future incompatibility
source:
  url: https://github.com/gfx-rs/metal-rs/blob/master/Cargo.toml
  repository: https://github.com/gfx-rs/metal-rs
  authors: [gfx-rs developers, rust-block contributors, Rust compiler team]
  published_at: "metal-rs master and rust-block master (retrieved 2026-08-30); rust-lang/rust issue 74840 (2020-07-27)"
  license: "metal-rs and rust-block MIT OR Apache-2.0; Rust repository MIT OR Apache-2.0"
retrieved_at: 2026-08-30
tags: [rust, macos, metal, wgpu, dependency, future-incompatibility, release-risk]
confidence: 0.95
claims: []
relations:
  - type: depends_on
    target: nuif:research:masonry-xilem-and-linebender-test-harness
    note: The native editor reaches metal-rs through the pinned Masonry imaging and wgpu stack.
  - type: related_to
    target: nuif:research:rust-toolchain-and-msrv-policy
    note: Rust 1.98 reports the dependency as future-incompatible while still compiling it.
links:
  spec: []
  adr: [adrs/0006-rust-native-editor.md]
  rfc: []
  code: [Cargo.lock, apps/editor/Cargo.toml]
  experiments: []
---

# Summary

The macOS editor resolves `masonry_winit -> imaging_wgpu -> wgpu 28.0.0 ->
wgpu-hal 28.0.1 -> metal 0.33.0 -> block 0.1.6`. Rust 1.98 compiles this graph
but reports `block` as future-incompatible because it declares an Objective-C
runtime symbol as a static of an uninhabited Rust type. The current metal-rs
master still declares `block = "0.1.6"`; upgrading from the pinned metal-rs
release to its current branch therefore does not remove the warning.

NUIF interpretation: retain the audited upstream dependency until metal-rs or
its wgpu consumer changes the Objective-C block binding. Do not introduce a
workspace-local fork of foreign unsafe ABI code solely to suppress a warning.
Re-evaluate the graph at every Rust or editor-stack update, and make a future
compiler hard error a release blocker for the macOS package.

## Evidence

- `cargo tree -i block` at revision `efbc5b8` resolves `block 0.1.6` through
  `metal 0.33.0`, `wgpu-hal 28.0.1`, `wgpu 28.0.0`, the Xilem imaging crates and
  `masonry_winit` into `nuif-editor` (2026-08-30).
- `cargo report future-incompatibilities --id 1` under rustc 1.98.0 identifies
  `block/src/lib.rs:64` and the `static of uninhabited type` lint. The command
  references rust-lang/rust issue 74840 (2026-08-30).
- metal-rs master declares version 0.33.0, Rust 1.82 and
  `block = "0.1.6"`. Locator: `Cargo.toml`, `[package]` and `[dependencies]`,
  lines 1-36, retrieved 2026-08-30:
  https://github.com/gfx-rs/metal-rs/blob/master/Cargo.toml.
- rust-block master declares `_NSConcreteStackBlock` with the `Class` type that
  produces the warning in version 0.1.6. Locator: `src/lib.rs`, Objective-C
  runtime declarations, retrieved 2026-08-30:
  https://github.com/SSheldon/rust-block/blob/master/src/lib.rs.
- Rust issue 74840 records the compiler problem caused by raw references to an
  extern never value and links the implementing fix. Locator: issue title,
  description and relationship to pull request 78324:
  https://github.com/rust-lang/rust/issues/74840.

## Decision boundary

**Retain** the current dependency while it compiles on both the declared MSRV
and pinned toolchain and while `cargo deny` reports no licence, ban or source
failure.

**Re-evaluate** after each Masonry, wgpu, metal-rs or Rust update by running
`cargo report future-incompatibilities` and tracing the reverse dependency
tree.

**Reject** a local source patch without an upstream review reference. This code
implements the Objective-C blocks ABI; a warning-only fork would move unsafe
foreign-interface ownership into NUIF without increasing adapter or editor
capability.

## NUIF relevance

**Borrow** Cargo's future-incompatibility report as the reproducible diagnostic
for the pinned toolchain and lock file.

**Adapt** dependency-update review so a macOS editor-stack change includes the
reverse `block` dependency trace and package smoke test in addition to the
workspace checks.

**Reject** treating a successful current build as evidence that the dependency
will compile indefinitely; the compiler explicitly classifies the declaration
as code that can become an error in a future release.

## Open questions

- Which metal-rs or wgpu release first replaces or repairs the rust-block
  binding?
- In which Rust release does the lint become a hard error, if scheduled?
