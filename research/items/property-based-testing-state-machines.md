---
id: nuif:research:property-based-testing-state-machines
kind: synthesis
status: reviewed
title: Property-based and model-based testing of stateful systems (QuickCheck, eqc_statem, quickcheck-state-machine, proptest-state-machine)
source:
  url: https://doi.org/10.1145/351240.351266
  doi: 10.1145/351240.351266
  repository: https://github.com/proptest-rs/proptest
  authors: [Koen Claessen, John Hughes, Stevan Andjelkovic, Edsko de Vries, proptest contributors]
  published_at: "2000-09-01"
  license: ACM copyrighted paper; Hughes 2016 Springer LNCS; proptest and proptest-state-machine MIT OR Apache-2.0; quickcheck-state-machine BSD-3-Clause
retrieved_at: 2026-08-29
tags: [testing, property-based-testing, model-based-testing, state-machine, shrinking, proptest, quickcheck, reference-model, rust]
confidence: 0.92
claims: [nuif:claim:semantic-automation, nuif:claim:authored-resolved]
relations:
  - type: depends_on
    target: nuif:research:delta-debugging-and-test-case-reduction
    note: Shrinking of transition sequences is delta-style deletion followed by per-argument simplification.
  - type: related_to
    target: nuif:research:deterministic-simulation-testing
    note: Both are seed-driven; state-machine PBT supplies the command generator and reference model for a simulated run.
  - type: related_to
    target: nuif:research:metamorphic-testing-graphics
    note: Metamorphic relations are expressed as invariants checked after each transition.
  - type: related_to
    target: nuif:research:fuzzing-structured-inputs
    note: Coverage-guided fuzzing and PBT share generator design; fuzzing replaces the size-bounded random sampler with a corpus.
  - type: related_to
    target: nuif:research:automerge-yjs
    note: Automerge and yrs use action-sequence generation with a String reference model and seeded convergence scenarios.
  - type: related_to
    target: nuif:research:structured-merge
    note: The reference model pattern gives an oracle for merge and inverse-operation properties.
links:
  spec: [spec/06-operations-and-patches.md, spec/12-cli-api-and-automation.md, spec/02-identity-and-properties.md]
  adr: []
  rfc: [rfcs/0004-headless-qa-contract.md]
  code: [crates/nuif-protocol, crates/nuif-core, crates/nuif-query, crates/nuif-api]
  experiments: [conformance/PLAN.md, conformance/fixtures/v0-responsive-card/README.md]
---

# Summary

QuickCheck (Claessen and Hughes, ICFP 2000) introduced properties as executable universally quantified functions checked on random size-bounded inputs. Stateful extensions (Quviq eqc_statem, Hughes 2016) generate command sequences from an abstract model with preconditions, a state transition function and postconditions, run them against the real system, and shrink failing sequences by deleting commands that do not contribute to the failure. quickcheck-state-machine (Haskell) and proptest-state-machine (Rust) implement the same pattern; the Rust crate exposes `ReferenceStateMachine` and `StateMachineTest` traits and shrinks by removing unseen transitions, deleting transitions while re-checking preconditions, then simplifying individual transitions and the initial state. Rust proptest represents generated values as `ValueTree`s with `simplify`/`complicate` and persists failing seeds in `proptest-regressions` files.

For NUIF the pattern is the operations suite of `conformance/PLAN.md`: generate `nuif_protocol::Operation` sequences from a simplified reference model of the document tree, apply them to the engine through the CLI/API surface, and compare canonical state, resolved boxes and inverse-replay results after each step.

## Evidence

- Properties are Haskell functions such as `prop_RevApp xs ys = reverse (xs++ys) == reverse ys++reverse xs`; `quickCheck` reports "OK: passed 100 tests." Claessen and Hughes, ICFP 2000, DOI 10.1145/351240.351266, §2.1 (PDF https://www.cs.tufts.edu/~nr/cs257/archive/john-hughes/quick.pdf, retrieved 2026-08-29).
- Conditional properties use `==>` and stop after a candidate limit (default 1000) with "Arguments exhausted". Same paper, §2.3.
- `classify` and `collect` print the distribution of generated data so that trivial cases are visible. Same paper, §2.4.
- Generators are `Gen a` with a size parameter (`sized`, `resize`) to bound generated structures; `class Arbitrary a where arbitrary :: Gen a`. Same paper, §3.1–3.2.
- The 2000 paper contains no shrinking and no state-machine framework; the authors observe that errors divide roughly evenly among generators, specification and program. Same paper, §6.6.
- Stateful testing: "We test stateful systems by generating sequences of calls to the API under test", modelling state abstractly with transitions per operation and postconditions relating results to the model; a test passes if all postconditions hold. Hughes, "Experiences with QuickCheck: Testing the Hard Stuff and Staying Sane", LNCS 9600, 2016, DOI 10.1007/978-3-319-30936-1_9, §2 and Fig. 2 (PDF https://publications.lib.chalmers.se/records/fulltext/232550/local_232550.pdf, retrieved 2026-08-29).
- Shrinking searches for the smallest similar failing test, removes unnecessary calls and simplifies arguments; lessons: "Errors are often in the model, rather than the code". Same paper, §2.
- Volvo/AUTOSAR: 20,000 lines of QuickCheck code tested a million lines of C from six suppliers, finding more than 200 problems, over 100 of them ambiguities in the standard. Same paper, §3.
- Parallel testing reuses the sequential model and accepts a run if some interleaving of results matches the model; the dets model is under 100 lines against an implementation of over 6,000 lines. Same paper, §4.
- eqc_statem callbacks: `initial_state()`, `COMMAND_args(S)`, `COMMAND_pre(S)`, `COMMAND_next(S, V, Args)` ("used during both test generation and test execution"), `COMMAND_post(S, Args, R)`; the precondition is "also used when shrinking" so invalid commands do not appear. http://quviq.com/documentation/eqc/eqc_statem.html, version 1.48.3, retrieved 2026-08-29.
- quickcheck-state-machine record: `initModel`, `transition`, `precondition`, `postcondition`, `invariant`, `generator`, `shrinker`, `semantics`, `mock`, `cleanup`; symbolic references stand for values not yet known at generation time. https://github.com/stevana/quickcheck-state-machine, `src/Test/StateMachine/Types.hs` and README, `master`, retrieved 2026-08-29; de Vries, Well-Typed blog 2019-01-23, https://www.well-typed.com/blog/2019/01/qsm-in-depth/.
- proptest-state-machine 0.8.0 (`Cargo.toml` on `main`, depends on proptest 1.11.0). `src/strategy.rs` defines `ReferenceStateMachine` with `type State`, `type Transition`, `fn init_state() -> BoxedStrategy<Self::State>`, `fn transitions(state: &Self::State) -> BoxedStrategy<Self::Transition>`, `fn apply(state: Self::State, transition: &Self::Transition) -> Self::State`, `fn preconditions(state: &Self::State, transition: &Self::Transition) -> bool` (default true), and `fn sequential_strategy(size: impl Into<SizeRange>) -> Sequential<...>`. https://raw.githubusercontent.com/proptest-rs/proptest/main/proptest-state-machine/src/strategy.rs, retrieved 2026-08-29.
- Generation loop: a transition tree is drawn from `transitions(&state)`; if `preconditions` holds it is pushed and the model advanced, otherwise `runner.reject_local("Pre-conditions were not satisfied")`. Same file, `Sequential::new_tree`.
- Shrinking: `enum Shrink { InitialState, DeleteTransition(usize), Transition(usize) }`; `simplify()` first removes transitions never executed before the failure, then deletes transitions from the back re-checking preconditions, then shrinks individual transitions, then the initial state; `complicate()` undoes the last step. Same file; CHANGELOG 0.3.0 "Remove unseen transitions on a first step of shrinking" (#388), 0.3.1 precondition fix (#482).
- `src/test_runner.rs` defines `StateMachineTest` with `type SystemUnderTest`, `type Reference: ReferenceStateMachine`, `fn init_test(ref_state) -> Self::SystemUnderTest`, `fn apply(state, ref_state, transition) -> Self::SystemUnderTest` (ref_state is the state after the transition), `fn check_invariants(state, ref_state)`, `fn teardown(state, ref_state)`, and `fn test_sequential(config, ref_state, transitions, seen_counter)` which checks invariants before the first and after every transition. https://raw.githubusercontent.com/proptest-rs/proptest/main/proptest-state-machine/src/test_runner.rs, retrieved 2026-08-29.
- Macro: `prop_state_machine! { #[test] fn name(sequential 1..20 => MyTest); }` optionally with `#![proptest_config(...)]`; only `sequential` is supported. Same file; book chapter https://proptest-rs.github.io/proptest/proptest/state-machine.html, retrieved 2026-08-29.
- proptest 1.11.0: `Strategy { type Tree; type Value; fn new_tree(&self, runner: &mut TestRunner) -> NewTree<Self> }`; `ValueTree { fn current(&self) -> Self::Value; fn simplify(&mut self) -> bool; fn complicate(&mut self) -> bool }` where simplify moves current to a halfway point between low and high. https://docs.rs/proptest/latest/proptest/strategy/trait.Strategy.html and trait.ValueTree.html, retrieved 2026-08-29.
- `Config`: `cases` 256 (`PROPTEST_CASES`), `max_shrink_iters` u32::MAX (`PROPTEST_MAX_SHRINK_ITERS`), `max_shrink_time` 0, `max_global_rejects` 1024, `max_local_rejects` 65536, `rng_seed`, `failure_persistence` default `FileFailurePersistence::SourceParallel("proptest-regressions")`, which stores seeds, not values. https://docs.rs/proptest/latest/proptest/test_runner/struct.Config.html and https://proptest-rs.github.io/proptest/proptest/failure-persistence.html, retrieved 2026-08-29.
- "Shrinking never shrinks a value to something outside the range the strategy describes." https://proptest-rs.github.io/proptest/proptest/tutorial/shrinking-basics.html, retrieved 2026-08-29.
- Rust quickcheck 1.1.0: `trait Arbitrary: Clone + 'static { fn arbitrary(g: &mut Gen) -> Self; fn shrink(&self) -> Box<dyn Iterator<Item = Self>> }`, default empty iterator. https://docs.rs/quickcheck/latest/quickcheck/trait.Arbitrary.html, retrieved 2026-08-29.
- Linearizability checkers: Knossos checks a history of invoke/complete pairs against a single-threaded model with "linear" and "wgl" algorithms (https://github.com/jepsen-io/knossos); Porcupine implements P-compositionality with a `Model { Init, Step, Equal, Partition }` (https://github.com/anishathalye/porcupine). README level, retrieved 2026-08-29.
- File-synchroniser model: Hughes, Pierce, Arts, Norell, "Mysteries of Dropbox", ICST 2016, DOI 10.1109/ICST.2016.11 uses QuickCheck's state machine library with a trivial model state and inserts conjectured upload/download events to explain observations; found unexpected behaviour in two of three services. PDF https://www.cis.upenn.edu/~bcpierce/papers/mysteriesofdropbox.pdf, Abstract and §III, retrieved 2026-08-29.
- Editor-adjacent examples: ropey `tests/proptest_tests.rs` applies inserts and removes to a `Rope` and a `String` and asserts equality, with 512 cases and a checked-in `proptest-regressions` file (https://raw.githubusercontent.com/cessen/ropey/master/tests/proptest_tests.rs); Automerge `rust/automerge/tests/text.rs` lines 658–712 generates `Action` sequences with `prop_flat_map` and compares `doc.text()` with an expected `String` (https://raw.githubusercontent.com/automerge/automerge/main/rust/automerge/tests/text.rs); yrs `run_scenario(seed, mods, users, iterations)` checks pairwise convergence of block stores (https://raw.githubusercontent.com/y-crdt/y-crdt/main/yrs/src/test_utils.rs). Retrieved 2026-08-29.

## Mechanism

Model-based state-machine property (eqc_statem, quickcheck-state-machine, proptest-state-machine):

```
generate(seed, size):
    m = init_state(seed)                    # reference model
    ops = []
    while len(ops) < size:
        t = transitions(m).sample(seed)     # model-dependent generator
        if preconditions(m, t):
            ops.push(t); m = apply_model(m, t)
        else: reject_local()
    return (m0, ops)

execute(m0, ops):
    sut = init_test(m0); m = m0
    check_invariants(sut, m)
    for t in ops:
        m   = apply_model(m, t)
        sut = apply_sut(sut, m, t)          # postcondition compares sut result with m
        check_invariants(sut, m)
    teardown(sut, m)

shrink(m0, ops, failing):
    ops = drop_unseen(ops)                  # transitions after the failure point
    for i in reversed(range(len(ops))):     # DeleteTransition
        if valid_under_preconditions(m0, ops \ ops[i]) and failing(m0, ops \ ops[i]): ops.remove(i)
    for i in range(len(ops)):               # Transition: ValueTree::simplify on ops[i]
        while ops[i].simplify() and failing(...): pass; ops[i].complicate() as needed
    m0.simplify() while failing(...)        # InitialState
```

Invariants of the method:

- The model is simpler than the system; a model of tens of lines is sufficient for an API of thousands (Hughes 2016 §4).
- Preconditions are enforced during generation and during shrinking, so every shrunk sequence is valid (eqc_statem; proptest-state-machine).
- A failure is persisted as a seed and regenerated, not stored as data (proptest `FileFailurePersistence`).
- Distribution of generated commands is measured (`classify`/`collect`) so that generator bias is visible.

NUIF instantiation (synthesis): `Reference::State` is a tree of `EntityId` with parent, index, name and an `Extensions` map; `Transition` is `nuif_protocol::Operation`; `apply` on the model performs the structural change; preconditions reject moves into descendants, removal of missing entities and duplicate IDs; `check_invariants` compares `nuif_query::roots` and children order with the model, checks that opaque extension bytes on untouched entities are unchanged, and checks that inverse replay of the transaction restores the canonical hash.

## NUIF relevance

**Borrow**

- The `ReferenceStateMachine`/`StateMachineTest` split of proptest-state-machine maps directly onto a reference document model and the `nuif_api::Engine` implementation; the crate is the natural harness for the `operations` suite (proptest-state-machine 0.8.0).
- Precondition-guarded generation and shrinking keep operation sequences valid, which is the property the task requires for minimised failing sequences (eqc_statem; proptest-state-machine `Shrink::DeleteTransition`).
- Seed persistence in `proptest-regressions` files gives replayable failures without storing documents (proptest `Config.failure_persistence`).
- Distribution monitoring (`classify`/`collect` in QuickCheck) should be emitted into the machine-readable report so that swarm or coverage steering can be evaluated.

**Adapt**

- The reference model must also carry a minimal layout semantics for stack containers so that postconditions on resolved boxes (additivity, containment) are checkable without a second layout engine.
- `check_invariants` runs after every transition; for NUIF the expensive checks (export-import round trip, render) should be sampled per seed while cheap structural checks run every step.
- Parallel or linearizability testing (Hughes 2016 §4; Knossos, Porcupine) applies only to the collaboration profile and is out of scope for the single-writer trial-and-error loop.

**Reject**

- Symbolic references in the quickcheck-state-machine style are unnecessary because NUIF entity IDs are chosen by the generator, not returned by the system.
- Rust quickcheck's `shrink` iterator API lacks precondition-aware sequence shrinking and is inferior to proptest-state-machine for this use.

## Open questions

- Should the reference model include component instantiation and override semantics, or should instance-related operations be tested only through metamorphic relations?
- How should `prop_flat_map`-style state-dependent generation be balanced against swarm-style feature omission to avoid generator bias toward shallow trees?
- Can `proptest-state-machine`'s sequential strategy be driven from a corpus (coverage-guided) rather than a fresh seed per case without forking the crate?
