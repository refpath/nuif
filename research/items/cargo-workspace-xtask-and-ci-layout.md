---
id: nuif:research:cargo-workspace-xtask-and-ci-layout
kind: synthesis
status: reviewed
title: Cargo workspace layout, xtask automation, dependency/API linting, reproducible builds and test determinism for engine + apps + tests + fuzz
source:
  url: https://doc.rust-lang.org/cargo/reference/workspaces.html
  authors: [Cargo team, Aleksey Kladov (matklad), Taiki Endo, Embark Studios, Predrag Gruevski, est31, axodotdev, rust-lang]
  published_at: "Cargo book (retrieved 2026-08-29); cargo-hack 0.6.45 (2026-05-30); cargo-deny 0.20.2 (2026-07-09); cargo-semver-checks 0.50.0 (2026-08-01); cargo-udeps 0.1.61 (2026-04-29); cargo-dist 0.32.0 (2026-05-22); cargo-fuzz 0.13.2 (2026-06-09)"
  license: Cargo book MIT OR Apache-2.0; tools MIT OR Apache-2.0 (cargo-hack, cargo-deny, cargo-semver-checks, cargo-udeps, cargo-dist, cargo-xtask spec)
retrieved_at: 2026-08-29
tags: [cargo, workspace, xtask, ci, reproducible-builds, lints, msrv, cargo-hack, cargo-deny, semver, determinism, rust]
confidence: 0.9
claims: []
relations:
  - type: related_to
    target: nuif:research:rust-snapshot-property-fuzz-tooling
    note: Runner, fuzz and coverage tools referenced from the CI matrix.
  - type: related_to
    target: nuif:research:libtest-mimic-and-data-driven-fixtures
    note: Conformance crates are workspace members with harness = false targets.
  - type: related_to
    target: nuif:research:wasm-headless-execution
    note: wasm32 targets appear as CI matrix entries.
  - type: related_to
    target: nuif:research:deterministic-simulation-testing
    note: Test-ordering and process-isolation controls.
links:
  spec: []
  adr: [adrs/0001-rust-reference-core.md]
  rfc: []
  code: [Cargo.toml, .github/workflows/ci.yml, conformance/PLAN.md, crates/nuif-cli, crates/nuif-api]
  experiments: []
---

# Summary

Cargo workspaces share one `Cargo.lock` and one `target` directory across members and support inheritance of package metadata, dependencies and lints from the root manifest (`[workspace.package]`, `[workspace.dependencies]`, `[workspace.lints]`, respected since Rust 1.74). Free-form automation is conventionally implemented as an `xtask` binary crate reachable through a `.cargo/config.toml` alias, which needs no tool beyond cargo. Feature-combination checks (cargo-hack), dependency policy (cargo-deny), public API compatibility (cargo-semver-checks), unused dependencies (cargo-udeps, nightly) and release packaging (cargo-dist) are separate installable subcommands. Reproducibility rests on a committed `Cargo.lock` with `--locked`, a `rust-toolchain.toml` pin, `SOURCE_DATE_EPOCH` for embedded timestamps and `--remap-path-prefix` for embedded paths. libtest runs tests in alphabetical order on a thread pool sized by available parallelism; `--test-threads=1` (or `RUST_TEST_THREADS`) serialises tests that share state, and `--shuffle` remains unstable.

NUIF interpretation: the current repository already uses a virtual manifest with `[workspace.lints]` (`unsafe_code = "forbid"`, clippy `all`/`pedantic` warn) and a pinned toolchain in CI; the missing pieces are a committed `rust-toolchain.toml`, `--locked` in CI, an `xtask` crate for fixture regeneration and differential runs, a `fuzz/` member, `conformance/` test crates, and a CI matrix covering feature powersets, wasm targets and dependency/API policy.

## Evidence

- `[workspace]` keys: `resolver`, `members`, `exclude`, `default-members`, `package`, `dependencies`, `lints`, `metadata`; a manifest with `[workspace]` and no `[package]` "is called a virtual manifest" and must set `resolver` explicitly; "All packages share a common `Cargo.lock` file which resides in the workspace root" and a common output directory. Locator: Cargo book "Workspaces", retrieved 2026-08-29.
- Lint inheritance: `[workspace.lints.rust] unsafe_code = "forbid"` in the root and `[lints] workspace = true` in members; "MSRV: Respected as of 1.74"; lint entries accept `level` and `priority` (lower priority is overridden by higher). Locator: Cargo book "Workspaces" ("The lints table") and "The Manifest Format" ("The [lints] section").
- Package and dependency inheritance: `[workspace.package]` with `version.workspace = true`; `[workspace.dependencies]` with `regex = { workspace = true, features = ["unicode"] }` and `cc.workspace = true` in build/dev dependencies. Locator: Cargo book "Workspaces" ("The package table", "The dependencies table").
- `package.metadata` "is completely ignored by Cargo and will not be warned about", intended for external tools; cargo-fuzz's generated `fuzz/Cargo.toml` uses `[package.metadata] cargo-fuzz = true`. Locator: Cargo book "The Manifest Format"; cargo-fuzz `src/templates.rs` lines 9-10.
- Target conventions: `src/lib.rs`, `src/main.rs`, `src/bin/`, `examples/`, `tests/`, `benches/`; `harness = false` requires a user `main`; `test` and `bench` fields toggle default inclusion; "Each integration test results in a separate executable binary, and `cargo test` will run them serially". Locator: Cargo book "Cargo Targets".
- `cargo test`: arguments after `--` go to the test binary (`cargo test foo -- --test-threads 3`); `--jobs` affects the build only; `--no-fail-fast` runs all executables; `--locked` "Asserts that the exact same dependencies and versions are used as when the existing `Cargo.lock` file was originally generated"; `--frozen` equals `--locked` plus `--offline`; each test's working directory is the package root. Locator: Cargo book "cargo test".
- Cargo FAQ: `Cargo.lock` gives "deterministic builds at different times and on different systems"; it aids `git bisect`, CI stability, MSRV verification and "snapshot testing error messages"; it "does not affect consumers of your package"; `cargo install` ignores it unless `--locked`; `cargo new` tracks it in version control by default. Locator: Cargo book FAQ "Why have Cargo.lock in version control?".
- libtest CLI: `--test-threads` defaults to `available_parallelism`, with `RUST_TEST_THREADS` as a deprecated alternative; default order is alphabetical; `--shuffle` and `--shuffle-seed` are unstable (`-Z unstable-options`, tracking issue #89583); `--list`, `--exact`, `--skip`, `--ignored`, `--include-ignored`, `--format` are stable; `--report-time` and JUnit output are unstable. Locator: rustc book "Tests" (`src/doc/rustc/src/tests/index.md`), lines 77-215.
- rustup: `rust-toolchain.toml` with `[toolchain] channel = "..."`, `components = [...]`, `profile`, optional `targets`; the nearest file up the directory tree applies; `channel` and `path` are mutually exclusive. Locator: rustup book "Overrides" ("The toolchain file").
- xtask: "a polyfill for cargo workflows"; add an `xtask` binary member and `.cargo/config.toml` with `[alias] xtask = "run --package xtask --"`; "It doesn't require any other binaries besides `cargo` and `rustc`"; Cargo itself uses xtasks. Locator: matklad/cargo-xtask `README.md`.
- cargo-hack 0.6.45: `--each-feature`, `--feature-powerset` (with `--depth`, `--exclude-features`/`--skip`, `--group-features`), `--rust-version` (check at the manifest MSRV), `--version-range`, `--workspace`. Locator: `README.md` "Usage".
- cargo-deny 0.20.2: `cargo deny init` and `cargo deny check` covering licenses, bans, advisories and sources; GitHub Action `cargo-deny-action`; README badge states MSRV 1.88.0. Locator: `README.md`.
- cargo-semver-checks 0.50.0: analyses rustdoc JSON; stable toolchains supported, nightly "on a best-effort basis"; `--baseline-version <X.Y.Z>` / `--baseline-rev <REV>`; GitHub Action available. Locator: `README.md` "FAQ", lines 63-121.
- cargo-udeps 0.1.61: "needs Rust nightly to actually run"; install with `--locked`. Locator: `README.md`.
- cargo-dist 0.32.0: builds tarballs and installers and "generates its own CI scripts" (`release.yml` on tag push). Locator: `README.md`.
- `SOURCE_DATE_EPOCH` "is a standardised environment variable that distributions can set centrally" giving seconds since the Unix epoch for the last source modification. Locator: reproducible-builds.org "SOURCE_DATE_EPOCH".
- rustc `--remap-path-prefix` remaps source paths in output (debug info, panics). Locator: rustc book "Command-line Arguments", `--remap-path-prefix`.
- Existing repository state: virtual manifest with `resolver = "2"`, eight members under `crates/`, `[workspace.package]` (`edition = "2024"`, `license = "Apache-2.0 OR MIT"`, `rust-version = "1.85"`), `[workspace.lints.rust] unsafe_code = "forbid"`, `[workspace.lints.clippy] all = "warn"`, `pedantic = "warn"`; CI job pins `dtolnay/rust-toolchain@master` to 1.85.0 with rustfmt and clippy and runs fmt, check, test, clippy `-D warnings` without `--locked`. Locator: `Cargo.toml`; `.github/workflows/ci.yml`.

## Mechanism

Workspace layout (interpretation; every element cites a convention above):

```text
Cargo.toml                     # virtual manifest; resolver = "2"; workspace.package/dependencies/lints
Cargo.lock                     # committed; CI uses --locked
rust-toolchain.toml            # [toolchain] channel = "1.85.0", components = ["rustfmt","clippy"], targets = ["wasm32-unknown-unknown","wasm32-wasip1"]
.cargo/config.toml             # [alias] xtask = "run --package xtask --"
deny.toml                      # cargo-deny policy (licenses allow-list: Apache-2.0, MIT, BSD-3-Clause, ...)
.config/nextest.toml           # [profile.ci] retries = 0, junit.path = "junit.xml"
crates/nuif-*/                 # engine crates (lib targets; [lints] workspace = true)
apps/editor/                   # editor crate(s); depends on crates/* only through nuif-api
conformance/                   # [[test]] harness = false suites; fixtures/ directory tree
fuzz/                          # cargo fuzz init; [package.metadata] cargo-fuzz = true; fuzz_targets/*.rs; member or separate workspace
benches/ (per crate)           # criterion/divan targets with harness = false
xtask/                         # regeneration of generated fixtures, differential runs, report assembly
```

Feature and target policy: engine crates expose an optional `arbitrary` feature (fuzz derives) and `serde` feature; `cargo hack check --workspace --feature-powerset --depth 2 --rust-version` validates combinations at the declared MSRV. Editor crates are excluded from `default-members` so that engine-only commands stay fast.

CI matrix (interpretation): stable pinned toolchain on ubuntu/macos/windows for `cargo fmt --check`, `cargo clippy --all-targets -D warnings`, `cargo nextest run --workspace --locked --profile ci`; `cargo hack` powerset job; `cargo deny check`; `cargo semver-checks --baseline-rev <last tag>` on release branches; `wasm32-unknown-unknown` build plus `wasm-pack test --headless --chrome`; `wasm32-wasip1` build plus wasmtime run; optional nightly job for `cargo fuzz run <target> -- -max_total_time=60`, `cargo llvm-cov --branch`, `cargo udeps`.

Determinism controls:

```text
cargo nextest run --workspace --locked        # one process per test; no shared static state across tests
cargo test -- --test-threads=1                # libtest fallback when tests share process-global state (fonts, env vars)
RUST_TEST_THREADS=1                            # deprecated equivalent
SOURCE_DATE_EPOCH=$(git log -1 --format=%ct)   # embedded build timestamps
RUSTFLAGS="--remap-path-prefix=$PWD=/src"      # embedded paths
```

Invariants: `Cargo.lock` committed and enforced with `--locked`; toolchain pinned identically in `rust-toolchain.toml` and CI; libtest order is alphabetical, so tests must not depend on order; global mutable state (font databases, environment variables) must be process-local or serialised with `--test-threads=1`; nextest process isolation makes per-test global state safe but not shared fixtures on disk.

## NUIF relevance

**Borrow**
- `rust-toolchain.toml` plus `--locked` in every CI command, because the Cargo FAQ ties deterministic builds and snapshot stability to the lock file and the toolchain pin is currently only in `ci.yml`.
- The xtask alias for fixture regeneration and browser-differential runs, because it keeps automation in Rust with no extra binaries, consistent with ADR 0001.
- cargo-deny with an allow-list matching the workspace `Apache-2.0 OR MIT` policy, because the toolkit comparison surfaced Apache-2.0-only (Masonry) and GPL/commercial (Slint) candidates that a policy check would flag.
- cargo-hack `--feature-powerset --rust-version`, because optional `serde`/`arbitrary` features on engine crates must compile in every combination at 1.85.

**Adapt**
- The `fuzz/` crate should be a workspace member only if the pinned stable toolchain can `cargo check` it; otherwise use `--fuzzing-workspace=true` and a nightly job, because cargo-fuzz requires nightly to run.
- cargo-semver-checks applies once `nuif-api` publishes a versioned public API; until then run it on `crates/nuif-api` against the previous tag only.
- nextest's JUnit is the CI-facing report; the NUIF machine-readable report (QA item 10) must be produced by the harness itself and attached as an artifact.

**Reject**
- cargo-dist as a required component now, because the CLI is a prototype (`nuif version` prints 0.0.1) and release packaging is premature.
- cargo-udeps in the required matrix, because it needs nightly.
- Relying on `--shuffle` for order-independence checks, because it is unstable; use nextest process isolation and explicit `--test-threads=1` where state is shared.

## Open questions

- Whether the MSRV pin remains 1.85.0; all candidate GUI toolkits require 1.88 or newer, and proptest's main branch states 1.86.
- Whether editor crates live in the same workspace (shared lock, shared MSRV) or in a nested workspace with its own toolchain file, which the rustup proximity rule supports.
- Whether Windows and macOS runners are required for engine tests, or only for editor and snapshot tests where platform text and GPU differences matter.
