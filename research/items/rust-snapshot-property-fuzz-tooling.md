---
id: nuif:research:rust-snapshot-property-fuzz-tooling
kind: synthesis
status: reviewed
title: Rust snapshot, property-based, fuzzing, mutation, runner, benchmark and coverage tooling for round-trip trials
source:
  url: https://github.com/mitsuhiko/insta
  authors: [Armin Ronacher, proptest-rs, Andrew Gallant, rust-fuzz, Cameron Bytheway, model-checking (AWS), tokio-rs, Martin Pool, nextest-rs, criterion-rs, Nikolai Vazquez, Taiki Endo]
  published_at: "insta 1.48.0 (2026-06-11); proptest 1.11.0 and proptest-state-machine 0.8.0 (2026-03-24); quickcheck 1.1.0 (2026-02-10); cargo-fuzz 0.13.2 (2026-06-09); arbitrary 1.4.2 (2025-08-14); bolero 0.13.4 (2025-07-03); kani-verifier 0.67.0 (2026-01-16); loom 0.7.2 (2024-04-23); cargo-mutants 27.1.0 (2026-06-02); cargo-nextest 0.9.143 (2026-08-04); criterion 0.8.2 (2026-02-04); divan 0.1.21 (2025-04-10); cargo-llvm-cov 0.9.0 (2026-08-16)"
  license: mixed (MIT OR Apache-2.0 for most; cargo-mutants MIT; kani Apache-2.0 OR MIT)
retrieved_at: 2026-08-29
tags: [testing, snapshot-test, property-based-test, fuzzing, mutation-testing, test-runner, benchmark, coverage, rust, insta, proptest, nextest]
confidence: 0.88
claims: [nuif:claim:semantic-automation]
relations:
  - type: related_to
    target: nuif:research:property-based-testing-state-machines
    note: proptest-state-machine is the Rust implementation of the model-based pattern.
  - type: related_to
    target: nuif:research:fuzzing-structured-inputs
    note: cargo-fuzz plus arbitrary implement structure-aware fuzzing.
  - type: related_to
    target: nuif:research:golden-master-and-snapshot-testing
    note: insta is the Rust snapshot implementation.
  - type: related_to
    target: nuif:research:delta-debugging-and-test-case-reduction
    note: proptest shrinking and cargo fuzz tmin perform reduction for QA item 9.
  - type: related_to
    target: nuif:research:libtest-mimic-and-data-driven-fixtures
    note: nextest's custom-harness rules apply to fixture crates.
  - type: related_to
    target: nuif:research:egui-and-egui-kittest
    note: egui_kittest recommends insta over image snapshots.
links:
  spec: []
  adr: [adrs/0001-rust-reference-core.md]
  rfc: []
  code: [conformance/PLAN.md, apps/editor/QA.md, Cargo.toml, .github/workflows/ci.yml, crates/nuif-protocol, crates/nuif-codec]
  experiments: []
---

# Summary

The Rust ecosystem provides one maintained tool per testing technique named in conformance/PLAN.md: insta for golden structural snapshots, proptest (with proptest-state-machine) and quickcheck for property-based tests over operation sequences, cargo-fuzz with arbitrary (or bolero as a unified front end) for structure-aware fuzzing of parsers and codecs, kani for bounded model checking of small pure kernels, cargo-mutants for measuring whether tests detect behaviour changes, cargo-nextest as a per-test-process runner with retries and JUnit output, criterion or divan for benchmarks, and cargo-llvm-cov for coverage. loom addresses concurrency interleavings only and is not required by a single-threaded engine. All versions and entry points below were verified against repositories or crates.io on 2026-08-29.

NUIF interpretation: a round-trip trial (encode, decode, canonicalise, apply operations, invert, replay, layout, render, diff) maps onto these tools as generator (proptest/arbitrary), oracle (insta snapshots, reference state machine, canonical-form equality), reducer (proptest shrinking, `cargo fuzz tmin`), runner (nextest with process isolation and JUnit), and adequacy metrics (cargo-mutants, cargo-llvm-cov).

## Evidence

- insta 1.48.0 (2026-06-11): macros `assert_snapshot!`, `assert_debug_snapshot!`, and serde-backed `assert_json_snapshot!`/`yaml`/`ron`/`csv`/`toml` behind features; features `redactions`, `filters`, `glob`; `INSTA_UPDATE` modes `auto` (default, `no` under CI), `new`, `always`, `unseen`, `no`, `force`; `.snap`/`.snap.new` files; inline snapshots via `cargo-insta`; `Settings` for redactions and snapshot paths. Locator: `insta/src/lib.rs` lines 81-190, 206-250; `CHANGELOG.md` "1.48.0"; crates.io.
- insta 1.48.0 added `strip_ansi_escape_codes` and lets explicit `--accept` override `CI=true` check mode; `cargo insta test --profile` forwards to nextest as `--cargo-profile`. Locator: `CHANGELOG.md` lines 6-20.
- proptest 1.11.0 (2026-03-24): "generation and shrinking is defined on a per-value basis instead of per-type"; README states MSRV 1.86 and a policy of at most `<current stable> - 7`; the crate "mainly sees passive maintenance". Locator: `proptest/README.md` "Status of this crate", "MSRV"; `proptest/CHANGELOG.md` "Unreleased".
- proptest-state-machine 0.8.0 (2026-03-24): `ReferenceStateMachine { type State; type Transition; fn init_state() -> BoxedStrategy<State>; fn transitions(&State) -> BoxedStrategy<Transition>; fn apply(State, &Transition) -> State; fn preconditions(&State, &Transition) -> bool }`; `StateMachineTest { type SystemUnderTest; type Reference: ReferenceStateMachine; fn init_test(&RefState) -> SUT; fn apply(SUT, &RefState, Transition) -> SUT; fn check_invariants(&SUT, &RefState); fn teardown(SUT); fn test_sequential(...) }`; `prop_state_machine!` macro; shrinking deletes transitions from the end, then shrinks transitions front to back, then the initial state. 0.8.0 added `Send + Sync` bounds to `strategy::Sequential`. Locator: `proptest-state-machine/src/strategy.rs` lines 45-81; `src/test_runner.rs` lines 19-177; `CHANGELOG.md`; proptest book "State Machine testing".
- quickcheck 1.1.0 (2026-02-10): per-type `Arbitrary`, `quickcheck!` macro and `#[quickcheck]` attribute; `QUICKCHECK_TESTS`, `QUICKCHECK_MAX_TESTS`, `QUICKCHECK_MIN_TESTS_PASSED`; the README states that proptest "improves on the concept of shrinking". Locator: `README.md` lines 27-160, 270-276.
- cargo-fuzz 0.13.2 (2026-06-09): subcommands `init`, `add`, `run`, `fmt`, `tmin`, `cmin`, `coverage`, `list`; requires nightly, libFuzzer, x86-64/AArch64 on Unix; `fuzz` directory must be added to `workspace.members` or created as its own workspace (`--fuzzing-workspace=true`); generated `fuzz/Cargo.toml` contains `[package.metadata] cargo-fuzz = true` and targets at `fuzz_targets/<name>.rs`; crashes are written under `fuzz/artifacts/<target>/crash-<hash>`. Locator: `README.md`; `src/templates.rs` lines 9-49; rust-fuzz book `cargo-fuzz/tutorial.md` lines 21-84.
- arbitrary 1.4.2 (2025-08-14): `#[derive(Arbitrary)]` with feature `derive`; per-field attributes such as `#[arbitrary(default)]`; `fuzz_target!(|input: T| ...)` accepts any `Arbitrary` type; the book shows gating the derive behind an optional `arbitrary` feature in the main crate. Locator: `arbitrary/README.md`; rust-fuzz book `structure-aware-fuzzing.md` lines 256-311.
- bolero 0.13.4 (2025-07-03): `bolero::check!().with_type().cloned().for_each(|v| ...)`, run under `cargo test` or `cargo bolero test <name>` with libFuzzer/AFL/honggfuzz engines; Linux needs `binutils-dev libunwind-dev`. Locator: `README.md`.
- kani-verifier 0.67.0 (2026-01-16): `cargo install --locked kani-verifier && cargo kani setup`; harness `#[kani::proof]` with `kani::any()` and `kani::assume()`; "bit-precise model checker for Rust" checking panics, overflow, UB and assertions; Linux and macOS. Locator: `README.md`; `docs/src/tutorial-first-steps.md` lines 33-174.
- loom 0.7.2 (2024-04-23): permutes concurrent executions under the C11 memory model; enabled via `[target.'cfg(loom)'.dependencies]`. Locator: `README.md`.
- cargo-mutants 27.1.0 (2026-06-02): `cargo mutants`, `-f <file>`; works with `cargo test` or `cargo nextest run` on "non-flaky tests"; CI guidance: PR-diff mode, `--in-place`, GitHub annotations (`--annotations=github`), install via `install-action`. Locator: `README.md`; `book/src/ci.md`.
- cargo-nextest 0.9.143 (2026-08-04): list phase builds with `cargo test --no-run` and lists tests; run phase "executes each individual test in a separate process, in parallel"; `--retries N` marks recovered tests "flaky" (exit code 0 by default; configurable failure since 0.9.131); `-jN`/`--test-threads=N`; `--no-fail-fast`; `--run-ignored=only|all`; JUnit via `[profile.ci.junit] path = "junit.xml"` in `.config/nextest.toml`, written to `target/nextest/ci/junit.xml`; custom harnesses must support `--list --format terse` printing `<name>: test`. Locator: `site/src/docs/design/how-it-works.md`; `features/retries.md` lines 8-41; `machine-readable/junit.md` lines 9-30; `design/custom-test-harnesses.md`; `running.md` lines 86-132.
- criterion 0.8.2 (2026-02-04): bench target with `harness = false`, `criterion_group!`/`criterion_main!`, feature `html_reports`, gnuplot optional. Locator: `README.md` lines 44-76; `CHANGELOG.md`.
- divan 0.1.21 (2025-04-10) requires Rust 1.80.0; `divan::main()` in a `harness = false` bench and `#[divan::bench]` attributes. Locator: `README.md` "Getting Started".
- cargo-llvm-cov 0.9.0 (2026-08-16): `cargo llvm-cov [--lcov|--json|--codecov|--html|--text] [--output-path]`, `cargo llvm-cov nextest`, `cargo llvm-cov report`, `--fail-under-lines <MIN>`, `--branch` and `--doctests` (nightly), `clean --workspace` recommended before mixed runs. Locator: `README.md` lines 12-15, 57-445.
- wasm-bindgen-test coverage requires nightly `-Cinstrument-coverage -Zno-profiler-runtime` and `cfg(wasm_bindgen_unstable_test_coverage)` and can feed `cargo +nightly llvm-cov`. Locator: wasm-bindgen `guide/src/wasm-bindgen-test/coverage.md` lines 3-46.

## Mechanism

Role of each tool in a round-trip trial loop:

| Stage | Tool | Entry point | Output consumed by |
|---|---|---|---|
| generate documents and operation sequences | proptest, arbitrary | `Strategy`, `#[derive(Arbitrary)]` on `Document`/`Operation` | apply/oracle |
| model-based sequence check | proptest-state-machine | `ReferenceStateMachine`, `StateMachineTest`, `prop_state_machine!` | invariants, shrinker |
| structural golden oracle | insta | `assert_snapshot!(canonical_text)`, `assert_json_snapshot!` with redactions for volatile IDs | review via `cargo insta` |
| parser/codec robustness | cargo-fuzz, bolero | `fuzz_target!(|d: &[u8]|)`, `fuzz_target!(|doc: FuzzDocument|)`, `bolero::check!` | artifacts, `cargo fuzz tmin` |
| bounded proof of pure kernels | kani | `#[kani::proof]` over ID allocation, canonical ordering | CI (Linux) |
| adequacy | cargo-mutants, cargo-llvm-cov | `cargo mutants --in-diff`, `cargo llvm-cov nextest --lcov` | thresholds `--fail-under-lines` |
| execution and reporting | cargo-nextest | `cargo nextest run --profile ci --retries 0` | `target/nextest/ci/junit.xml` |
| timing | criterion or divan | `harness = false` bench targets | regression tracking |

Sketch of a proptest-state-machine test over a NUIF tree document (interpretation; types from `crates/nuif-core` and `crates/nuif-protocol`):

```rust
use proptest::prelude::*;
use proptest_state_machine::{ReferenceStateMachine, StateMachineTest, prop_state_machine};

// Reference model: a minimal ordered forest keyed by EntityId.
#[derive(Clone, Debug)]
struct RefTree { parent: BTreeMap<EntityId, Option<EntityId>>, order: BTreeMap<Option<EntityId>, Vec<EntityId>> }

struct RefMachine;
impl ReferenceStateMachine for RefMachine {
    type State = RefTree;
    type Transition = Operation;               // Insert, Remove, Move, Rename, SetExtension

    fn init_state() -> BoxedStrategy<RefTree> { Just(RefTree::single_root()).boxed() }

    fn transitions(s: &RefTree) -> BoxedStrategy<Operation> {
        let ids = s.parent.keys().copied().collect::<Vec<_>>();
        prop_oneof![
            (any::<Entity>(), sample(ids.clone()), 0usize..8).prop_map(|(e, p, i)| Operation::Insert { parent: Some(p), index: i, entity: e }),
            (sample(ids.clone()), sample(ids.clone()), sample(ids.clone())).prop_map(|(e, p, after)| Operation::Move { entity: e, new_parent: Some(p), anchor: Anchor::After(after) }),
            sample(ids.clone()).prop_map(|e| Operation::Remove { entity: e }),
        ].boxed()
    }

    fn preconditions(s: &RefTree, t: &Operation) -> bool {
        match t { Operation::Move { entity, new_parent, .. } => !s.is_ancestor_or_self(*entity, *new_parent), // no cycles
                  Operation::Remove { entity } => !s.is_root(*entity), _ => true }
    }

    fn apply(mut s: RefTree, t: &Operation) -> RefTree { s.apply_reference(t); s }
}

struct EngineTest;
impl StateMachineTest for EngineTest {
    type SystemUnderTest = (PrototypeEngine, Document);
    type Reference = RefMachine;

    fn init_test(_: &RefTree) -> Self::SystemUnderTest { (PrototypeEngine::default(), Document::single_root()) }

    fn apply((mut engine, mut doc): Self::SystemUnderTest, _: &RefTree, op: Operation) -> Self::SystemUnderTest {
        let patch = Patch { base_revision: None, transactions: vec![Transaction { id: 1, operations: vec![op.clone()] }] };
        let before = doc.clone();
        engine.apply(&mut doc, &patch).expect("precondition-satisfying op applies");
        let inverse = engine.invert(&before, &patch);         // hypothetical; QA item 3
        let mut replay = before.clone();
        engine.apply(&mut replay, &patch).unwrap();
        assert_eq!(canonical(&replay), canonical(&doc));      // deterministic replay
        let mut undone = doc.clone();
        engine.apply(&mut undone, &inverse).unwrap();
        assert_eq!(canonical(&undone), canonical(&before));   // inversion
        (engine, doc)
    }

    fn check_invariants((_, doc): &Self::SystemUnderTest, r: &RefTree) {
        assert_eq!(doc.children_order(), r.order);            // model equivalence
        assert!(doc.is_acyclic());
        assert_eq!(decode(&encode(doc)), *doc);                // codec round trip
        assert_eq!(canonical(&decode(&encode(doc))), canonical(doc));
    }
}

prop_state_machine! {
    #![proptest_config(ProptestConfig { cases: 256, .. ProptestConfig::default() })]
    #[test]
    fn engine_matches_reference_tree(sequential 1..40 => EngineTest);
}
```

Failure handling: proptest shrinks by dropping trailing transitions, then shrinking individual transitions, then the initial state; the surviving minimal sequence can be serialised as a conformance fixture (QA item 9). For byte-level codecs, `cargo fuzz tmin` and `cmin` reduce inputs and corpora. nextest executes each case in its own process, so a panic or abort in one fixture cannot poison others, and `--retries 0` in the CI profile makes flakiness a failure rather than a warning.

## NUIF relevance

**Borrow**
- insta with redactions for canonical-text and `RenderScene` snapshots, because the `Document`, `LayoutSnapshot` and `RenderScene` types are plain data and text snapshots are reviewable in pull requests.
- proptest-state-machine as the operation-sequence property engine for `conformance/operations`, because its reference/SUT split matches the "deterministic operation replay" and "inversion" techniques listed in conformance/PLAN.md.
- cargo-fuzz plus `#[derive(Arbitrary)]` behind an optional `arbitrary` feature on `nuif-core`/`nuif-codec`, because the security suite requires fuzzing parsers and path geometry.
- cargo-nextest with a `ci` profile (JUnit path, `--retries 0`, `--no-fail-fast`) as the runner, because QA item 10 requires one machine-readable report per run.

**Adapt**
- Snapshot metadata must include implementation version, capability profile, fixture ID and evaluation context (conformance/PLAN.md), which insta does not model; embed them in the snapshot content or in `Settings::set_info`.
- kani is limited to small bounded harnesses (unwinding); apply it to ID allocation, canonical ordering comparators and cycle checks, not to layout or rendering.
- cargo-mutants runs should be restricted to `--in-diff` on pull requests and full runs on a schedule, because full mutation runs of a layout engine are slow.

**Reject**
- loom as a default dependency, because the engine is single-threaded by design and loom targets memory-model interleavings.
- Nightly-only tools (cargo-fuzz, `--branch` coverage, cargo-udeps) in the required CI matrix, because the toolchain is pinned to stable 1.85.0; run them in an optional nightly job.
- quickcheck for new tests, because its own README defers to proptest for shrinking and the project's per-type `Arbitrary` conflicts with per-value strategies needed for constrained trees.

## Open questions

- Whether proptest's stated MSRV (1.86 on main) is already in effect for 1.11.0, which would exceed the NUIF pin.
- Whether libFuzzer-based fuzzing is acceptable in CI given the nightly requirement, or whether bolero's `cargo test` mode with its built-in generator suffices for the security suite.
- Whether insta binary snapshots are appropriate for small PNG references or whether image references should stay outside insta (as egui and Masonry do).
- Whether a `PrototypeEngine` with `invert` will exist in `nuif-api`; the trait currently exposes `apply`, `layout`, `build_render_scene` only.
