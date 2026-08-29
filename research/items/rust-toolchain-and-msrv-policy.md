---
id: nuif:research:rust-toolchain-and-msrv-policy
kind: synthesis
status: reviewed
title: Rust toolchain pin and minimum supported Rust version policy for the NUIF workspace
source:
  url: https://doc.rust-lang.org/cargo/reference/rust-version.html
  authors: [Cargo team, Rust release team, Linebender, gfx-rs, Tokio contributors, Bevy contributors, Zed Industries, Servo, rust-analyzer]
  published_at: "Cargo book (retrieved 2026-08-29); Rust 1.98.0 (2026-08-20); RELEASES.md master (retrieved 2026-08-29)"
  license: "Cargo book and Rust release notes MIT OR Apache-2.0; project READMEs under their repositories' licences"
retrieved_at: 2026-08-29
tags: [rust, toolchain, msrv, rust-version, cargo, resolver, ci, policy, masonry, vello, wgpu]
confidence: 0.9
claims: []
relations:
  - type: extends
    target: nuif:research:cargo-workspace-xtask-and-ci-layout
    note: Resolves the open question on the 1.85.0 pin with release data, dependency MSRVs and comparable project policies.
  - type: depends_on
    target: nuif:research:masonry-editor-stack-decision
    note: The editor stack sets the highest dependency MSRV (1.96 on Masonry main).
  - type: related_to
    target: nuif:research:masonry-xilem-and-linebender-test-harness
    note: Masonry MSRV history per release.
  - type: related_to
    target: nuif:research:vello-testing-and-cpu-reference
    note: Vello's MSRV statement and non-breaking bump policy.
links:
  spec: []
  adr: [adrs/0006-rust-native-editor.md, adrs/0001-rust-reference-core.md]
  rfc: []
  code: [rust-toolchain.toml, Cargo.toml, .github/workflows/ci.yml]
  experiments: []
---

# Summary

Rust stable 1.98.0 was released on 2026-08-20; twelve stable minor releases separate it from the 1.85.0 pinned in the NUIF workspace (2025-02-20), at a fixed interval of 42 days. Cargo's `rust-version` field declares a minimum supported Rust version (MSRV); raising it "is assumed to be a minor incompatibility" and, under resolver version 3 (the edition 2024 default for packages), the resolver prefers dependency versions whose `rust-version` is at or below the declaring package's value. The Cargo book lists N-2, even releases and calendar-year windows as example policies. Comparable projects split into two groups: application-scale projects track the latest stable (rust-analyzer `rust-version = "1.98"`, Zed and Servo pin 1.97.1, Bevy states the MSRV "is generally close to the latest stable release"), while library projects hold an older MSRV and treat bumps as breaking (wgpu 1.87 for the crate, at most stable minus 3 for the repository) or as rolling with a six-month floor (Tokio). Linebender crates state that MSRV increases are not breaking changes. The highest `rust-version` in the planned dependency graph is 1.96 (Masonry main); the engine dependencies require at most 1.88 (Vello, Parley, `icu_properties`), 1.87 (wgpu) or 1.85 (AccessKit, HarfRust, proptest, Skrifa).

NUIF interpretation: the workspace pins `1.98.0` in `rust-toolchain.toml` and declares `rust-version = "1.96"` for every crate, with the policy "toolchain equals latest stable, bumped in a dedicated commit within one release cycle; MSRV equals the toolchain minus two minor versions or the highest dependency MSRV, whichever is greater, re-evaluated at every toolchain bump". The workspace manifest switches to `resolver = "3"` so that MSRV-aware resolution applies, and CI adds an MSRV job at 1.96.0.

## Evidence

Rust releases

- Stable releases from 1.85 (RELEASES.md, rust-lang/rust master): 1.85.0 (2025-02-20), 1.85.1 (2025-03-18), 1.86.0 (2025-04-03), 1.87.0 (2025-05-15), 1.88.0 (2025-06-26), 1.89.0 (2025-08-07), 1.90.0 (2025-09-18), 1.91.0 (2025-10-30), 1.91.1 (2025-11-10), 1.92.0 (2025-12-11), 1.93.0 (2026-01-22), 1.93.1 (2026-02-12), 1.94.0 (2026-03-05), 1.94.1 (2026-03-26), 1.95.0 (2026-04-16), 1.96.0 (2026-05-28), 1.96.1 (2026-06-30), 1.97.0 (2026-07-09), 1.97.1 (2026-07-16), 1.98.0 (2026-08-20). Consecutive `.0` releases are 42 days apart (for example 2026-07-09 to 2026-08-20). Locator: https://raw.githubusercontent.com/rust-lang/rust/master/RELEASES.md, headings "Version 1.85.0" to "Version 1.98.0"; release blog https://blog.rust-lang.org/2026/08/20/Rust-1.98.0/ (HTTP 200) and https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/ (HTTP 200).
- Current stable channel: `channel-rust-stable.toml` has `date = "2026-08-20"` and rustc `version = "1.98.0 (88d9e12ae 2026-08-18)"`. By the 42-day cadence 1.99.0 falls on 2026-10-01 (computed; the release calendar page did not render and was not verified). Locator: https://static.rust-lang.org/dist/channel-rust-stable.toml.
- The machine used for this retrieval has rustup toolchains `stable` (1.97.1 at retrieval), `1.85.0` and `1.95` (`rustup toolchain list`).

Cargo semantics

- `rust-version` "must be a bare version number with at least one component; it cannot include semver operators or pre-release identifiers"; "Changing `rust-version` is assumed to be a minor incompatibility"; the section "Selecting supported Rust versions" lists "N-2: latest version with a 2 release grace window for updating", "Every even release with a 2 release grace window" and "Every version from this calendar year with a one year grace window". Locator: Cargo book "Rust Version" (`reference/rust-version.html`), sections "Setting and Updating Rust Version" and "Selecting supported Rust versions"; semver reference anchor `env-new-rust`.
- Resolver: with `resolver.incompatible-rust-versions = "fallback"` "the resolver will prefer packages with a Rust version that is less than or equal to your own Rust version"; if no compatible version satisfies the requirement "the resolver won't error but will instead pick a version"; with mixed workspace MSRVs the resolver "may pick a lower dependency version than necessary" or "too high of a version". The default is `allow` for resolver versions 1 and 2 and `fallback` for resolver version 3, which edition 2024 packages default to. Locator: Cargo book "Dependency Resolution", section "Rust version"; "Resolver versions".
- Virtual manifests must set `resolver` explicitly (Cargo book "Workspaces", cited in `nuif:research:cargo-workspace-xtask-and-ci-layout`). The NUIF root manifest sets `resolver = "2"`, `edition = "2024"`, `rust-version = "1.85"`; `rust-toolchain.toml` sets `channel = "1.85.0"`, `components = ["clippy", "rustfmt"]`, `profile = "minimal"`; CI pins `dtolnay/rust-toolchain` to 1.85.0. `cargo metadata --locked` reports zero packages outside the workspace. Locator: `Cargo.toml` lines 1-18; `rust-toolchain.toml`; `.github/workflows/ci.yml` lines 44-46; `cargo metadata`, 2026-08-29.

Dependency MSRVs (crates.io `rust_version` of the newest non-yanked version, 2026-08-29)

| Crate | Version (date) | MSRV | Licence |
|---|---|---|---|
| masonry (release) | 0.4.0 (2025-10-29) | 1.88 | Apache-2.0 |
| masonry (main b81d8d7) | unreleased (2026-08-28) | 1.96 | Apache-2.0 |
| imaging, imaging_vello_cpu | 0.0.1 (2026-05-21), 0.0.2 (2026-05-30) | 1.92 | Apache-2.0 OR MIT |
| vello | 0.10.0 (2026-08-14) | 1.88 (0.8.0 was 1.92, 0.9.0 1.88) | Apache-2.0 OR MIT |
| vello_cpu, vello_hybrid, vello_common | 0.2.0 (2026-08-07) | 1.88 (0.0.7 was 1.92) | Apache-2.0 OR MIT |
| parley, fontique | 0.11.1 (2026-08-16) | 1.88 | Apache-2.0 OR MIT |
| skrifa | 0.46.2 (2026-08-21) | 1.85 (0.45.x was 1.89) | MIT OR Apache-2.0 |
| peniko, kurbo | 0.6.1 (2026-05-15), 0.13.1 (2026-05-13) | 1.85 | Apache-2.0 OR MIT |
| wgpu | 30.0.1 (2026-08-22) | 1.87.0 | MIT OR Apache-2.0 |
| accesskit, accesskit_consumer, accesskit_winit | 0.25.0, 0.39.0, 0.34.0 (2026-08-29) | 1.85 | MIT OR Apache-2.0; winit adapter Apache-2.0 |
| taffy | 0.14.0 (2026-08-24) | 1.71 | MIT |
| harfrust | 0.13.3 (2026-08-25) | 1.85 | MIT |
| rustybuzz | 0.20.1 (2024-11-12) | none declared | MIT |
| harfbuzz_rs | 2.0.1 (2021-08-28) | none declared | MIT |
| insta | 1.48.0 (2026-06-11) | 1.66.0 | Apache-2.0 |
| proptest | 1.11.0 (2026-03-24) | 1.85 | MIT OR Apache-2.0 |
| libtest-mimic | 0.8.2 (2026-03-16) | 1.65 | MIT/Apache-2.0 |
| ciborium | 0.2.2 (2024-01-24) | 1.58 | Apache-2.0 |
| minicbor | 2.3.0 (2026-07-23) | none declared | BlueOak-1.0.0 |
| winit | 0.30.13 (2026-03-02) | 1.70.0 | Apache-2.0 |
| egui, egui_kittest | 0.36.1 (2026-08-07) | 1.95 | MIT OR Apache-2.0 |
| kittest | 0.4.0 (2026-03-24) | 1.92 | MIT OR Apache-2.0 |
| blitz-dom | 0.3.0-beta.2 (2026-08-24) | 1.89.0 | MIT OR Apache-2.0 |
| floem (main) | 0.2.0 | 1.91 | MIT |
| cargo-deny | 0.20.2 (2026-07-09) | 1.88.0 | MIT OR Apache-2.0 |
| icu_properties, libloading, glifo, tree_arena | 2.3.0, 0.9.0, 0.3.0, 0.2.0 | 1.88 | Unicode-3.0; ISC; Apache-2.0 OR MIT; Apache-2.0 |
| fearless_simd | 0.7.0 | 1.89 | Apache-2.0 OR MIT |

Locator: crates.io API `/api/v1/crates/<name>` fields `versions[].rust_version`, `versions[].license`, `versions[].created_at`. The resolution probe in `nuif:research:masonry-editor-stack-decision` confirms 1.96 as the highest `rust-version` in the combined graph, followed by 1.92.

Masonry's own MSRV practice (interpretation from the release dates above): 0.3.0 (2025-05-10) required 1.86 while stable was 1.86; 0.4.0 (2025-10-29) required 1.88 while stable was 1.90; main (2026-08-28) requires 1.96 while stable is 1.98. The observed window is therefore between N-0 and N-2 at release time.

Comparable project policies

- rust-analyzer: `Cargo.toml` `rust-version = "1.98"`, `edition = "2024"`; no `rust-toolchain.toml` in the repository (HTTP 404). Locator: rust-lang/rust-analyzer `Cargo.toml` lines 7-8, master.
- Zed: `rust-toolchain.toml` `channel = "1.97.1"`, components rustfmt, clippy, rust-analyzer, rust-src, targets `wasm32-wasip2`, `wasm32-unknown-unknown`, `x86_64-unknown-linux-musl`; GPUI README: "You'll also need to use the latest version of stable Rust". Locator: zed-industries/zed `rust-toolchain.toml`; `crates/gpui/README.md` line 8, main.
- Servo: `rust-toolchain.toml` `channel = "1.97.1"` with a comment listing the other files to update at each bump (`shell.nix`, `support/crown/rust-toolchain.toml`, `.devcontainer/Dockerfile`). Locator: servo/servo `rust-toolchain.toml`, main.
- Bevy: `Cargo.toml` `rust-version = "1.96.0"`; README: "Bevy relies heavily on improvements in the Rust language and compiler. As a result, the Minimum Supported Rust Version (MSRV) is generally close to 'the latest stable release' of Rust." Locator: bevyengine/bevy `Cargo.toml` line 13; `README.md` lines 17-18, main.
- wgpu: "If you're using `wgpu`, our MSRV is 1.87. If you're running our tests or examples, our MSRV is 1.93."; "We will avoid bumping the MSRV of `wgpu` without good reason, and such a change is considered breaking."; "This version can only be upgraded in breaking releases, though we release a breaking version every three months."; "The repository MSRV should never require an MSRV higher than `stable - 3`"; the `wgpu` crate MSRV is bounded by Servo's and `wgpu-core` by Firefox's. Locator: gfx-rs/wgpu `README.md` lines 98-125, trunk.
- Vello: "This version of Vello has been verified to compile with Rust 1.88 and later. Future versions of Vello might increase the Rust version requirement. It will not be treated as a breaking change and as such can even happen with small patch releases." Masonry's README carries the same wording at 1.96. Locator: linebender/vello `README.md` lines 223-228; linebender/xilem `masonry/README.md` lines 209-213.
- Tokio: "Tokio will keep a rolling MSRV (minimum supported rust version) policy of at least 6 months. When increasing the MSRV, the new Rust version must have been released at least six months ago. The current MSRV is 1.71."; "the MSRV is not increased automatically, and only as part of a minor release". Locator: tokio-rs/tokio `README.md` lines 173-190, master.

## Mechanism

Policy expressed as two variables and one invariant:

```text
T  := toolchain pinned in rust-toolchain.toml and CI          # 1.98.0 on 2026-08-29
D  := max(rust-version over the resolved dependency graph)     # 1.96 (Masonry main)
M  := max(T - 2 minor versions, D)                             # 1.96
invariant: D <= M <= T; every crate declares rust-version = M; CI checks M and T
```

Update procedure: a stable release of Rust triggers one commit that raises `T` (toolchain file, CI matrix, report `engine.toolchain` field) and recomputes `M`; a dependency whose MSRV exceeds `M` is held at its previous version until the next `T` bump unless the bump is needed for a defect fix, in which case `M` rises with it. Because Cargo classifies a `rust-version` change as a minor incompatibility, NUIF crates may raise `M` in minor releases and record the change in the changelog, following Tokio's practice of bumping only at a release boundary. `resolver = "3"` makes the resolver prefer versions compatible with `M` when the workspace is resolved under `T`, so `Cargo.lock` does not drift above `M` silently; `cargo hack check --workspace --rust-version` on a 1.96.0 toolchain verifies the invariant.

Why not toolkit-minimum only: `D` follows Masonry's N-0 to N-2 practice, so a policy of `M := D` would be indistinguishable from N-2 in the common case but would leave `M` undefined when the editor crate is absent from a build. Why not latest stable for `M`: independent implementers and distribution packagers build the engine crates, whose own requirement is at most 1.88; a two-cycle window costs nothing measurable in language features for those crates and keeps `cargo install nuif-cli` possible on the previous two stable releases.

## NUIF relevance

**Borrow**
- The Cargo book's N-2 window as the MSRV rule, because it is the first example policy in the reference and matches Masonry's observed practice.
- Zed's and Servo's pattern of one toolchain file with an exact patch version, because reproducibility of snapshots and reports depends on an exact compiler (`conformance/HARNESS.md` records `engine.toolchain`).
- wgpu's split between crate MSRV and repository MSRV, transposed as engine crates versus editor crate: both declare `M`, but only the editor crate depends on packages at `D`.

**Adapt**
- `rust-toolchain.toml` changes to `channel = "1.98.0"`; `Cargo.toml` changes to `rust-version = "1.96"` and `resolver = "3"`; `.github/workflows/ci.yml` changes the pinned toolchain to 1.98.0 and adds an `msrv` job on 1.96.0 running `cargo hack check --workspace --rust-version --locked`.
- ADR 0006 gating decision 1 records the policy in one sentence so that MSRV bumps occur only in toolchain commits.

**Reject**
- Keeping 1.85.0, because every GUI candidate and `imaging` require at least 1.91, and `resolver = "2"` under 1.85 does not perform MSRV-aware resolution.
- Treating MSRV bumps as breaking changes in the wgpu sense, because NUIF is a draft specification with a reference implementation and no downstream crates; Cargo's minor-incompatibility classification is sufficient.
- Tracking Rust nightly or beta for any job, because reproducible snapshots require a fixed stable compiler.

## Open questions

- Whether Masonry's next release raises its MSRV above 1.96 before the editor crate lands; if so `M` follows `D` at that bump.
- Whether `cargo hack --rust-version` on a virtual manifest with `resolver = "3"` reproduces the same lock file as the `T` toolchain, or whether a separate `Cargo.lock` for the MSRV job is needed.
- Whether the Rust release calendar confirms 2026-10-01 for 1.99.0 (not verified).
