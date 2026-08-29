---
id: nuif:research:deterministic-simulation-testing
kind: synthesis
status: reviewed
title: Deterministic simulation testing (FoundationDB, TigerBeetle VOPR, Antithesis)
source:
  url: https://apple.github.io/foundationdb/testing.html
  repository: https://github.com/apple/foundationdb
  authors: [FoundationDB contributors, Will Wilson, TigerBeetle contributors, Antithesis, Alex Groce, Chaoqiang Zhang, Eric Eide, Yang Chen, John Regehr]
  published_at: "2014-09-01"
  license: FoundationDB Apache-2.0; TigerBeetle Apache-2.0; Antithesis documentation proprietary; Swarm Testing ACM copyrighted paper
retrieved_at: 2026-08-29
tags: [testing, deterministic-simulation, seed, fault-injection, invariants, swarm-testing, replay, scheduler]
confidence: 0.9
claims: [nuif:claim:semantic-automation]
relations:
  - type: related_to
    target: nuif:research:property-based-testing-state-machines
    note: Both drive a system from a seeded generator and check invariants; DST additionally virtualises time, I/O and scheduling.
  - type: related_to
    target: nuif:research:fuzzing-structured-inputs
    note: Swarm testing originated in compiler fuzzing and is used by VOPR to diversify fault distributions.
  - type: depends_on
    target: nuif:research:delta-debugging-and-test-case-reduction
    note: Seed replay yields a reproducible failure; reduction is a separate step over the replayed operation log.
  - type: related_to
    target: nuif:research:golden-master-and-snapshot-testing
    note: Deterministic replay is the precondition for byte-stable snapshots and canonical hashes.
  - type: related_to
    target: nuif:research:harfbuzz-unicode
    note: Text shaping and font loading are nondeterminism sources that must sit behind injectable interfaces.
links:
  spec: [spec/00-conformance.md, spec/06-operations-and-patches.md, spec/08-serialization.md, spec/11-security.md, spec/12-cli-api-and-automation.md]
  adr: []
  rfc: [rfcs/0004-headless-qa-contract.md]
  code: [crates/nuif-protocol, crates/nuif-layout, crates/nuif-render, crates/nuif-cli]
  experiments: [conformance/PLAN.md, conformance/fixtures/v0-responsive-card/README.md]
---

# Summary

Deterministic simulation testing (DST) runs an entire system inside one single-threaded process in which time, scheduling, network, disk and randomness are simulated from one seeded pseudo-random number generator. A failure is reproduced by rerunning the same build with the same seed. FoundationDB introduced the practice with the Flow actor language and the `Sim2` simulator; TigerBeetle's VOPR adds swarm-randomised fault distributions, hash-chained state checkers and a liveness mode; Antithesis moves determinism into a hypervisor so unmodified binaries can be simulated. Swarm testing (Groce et al., ISSTA 2012) supplies the evidence that randomising which features each run enables improves defect discovery.

NUIF is not a distributed system, but its trial-and-error loop has the same nondeterminism sources: operation ordering, floating-point layout, font and image loading, adapter I/O and renderer scheduling. The DST recipe transfers as a design constraint on the headless engine: every source of nondeterminism sits behind an injectable interface and every run is replayable from `(build, seed, fixture)`.

## Evidence

- FoundationDB simulation is "a *deterministic* simulation of an entire FoundationDB cluster within a single-threaded process" and determinism "allows perfect repeatability of a simulated run". https://apple.github.io/foundationdb/testing.html, §Simulation (mirrors `documentation/sphinx/source/testing.rst`), retrieved 2026-08-29.
- Simulated runs have roughly a 10:1 real-to-simulated time ratio and the project runs tens of thousands of simulations nightly. Same page, §Simulation.
- The simulated failure model includes network, machine and datacenter failures, reboots, degraded performance and "swizzle-clogging" (stopping connections in random sequence, then unclogging). Same page, §Simulation.
- Flow is an actor-based extension of C++ whose output feeds "our simulation tool, which conducts deterministic simulations of the entire system". https://apple.github.io/foundationdb/flow.html, retrieved 2026-08-29. On the `main` branch `flow/README.md` now describes cooperative scheduling over standard C++ coroutines.
- The simulator is `class Sim2 final : public ISimulator, public INetworkConnections`; `runLoop()` pops a `TaskQueue<PromiseTask>` and advances virtual time with `deterministicRandom()->random01()`; `delay()` schedules timers on virtual time and buggifies extra delay with probability 0.25. `fdbrpc/sim2.cpp`, `main` branch (lines ~1064–1075 and ~1379), retrieved 2026-08-29; the `release-7.1` file `fdbrpc/sim2.actor.cpp` notes that time is modified only from the main thread.
- Network and disk are simulated by `SimClogging`, `Sim2Conn` and `SimpleFile`, with latency, disconnects and open delays drawn from `deterministicRandom()`. Same file, lines ~277–402 and ~654.
- BUGGIFY sections activate with probability 0.25 and fire with probability 0.25; `buggify()` returns true only if buggify is enabled for the file/line and `deterministicRandom()->random01() < probability`. `flow/include/flow/Buggify.h`, `main` branch, lines 52–53, 92–101, retrieved 2026-08-29.
- `fdbserver` accepts `-r simulation`, `-f TESTFILE`, `-s SEED` ("Random seed."), `-b [on,off]` (buggify, default off), `-fi [on,off]` and `-R/--restarting`. `fdbserver/fdbserver.cpp`, `main` branch, usage text lines ~609–630, retrieved 2026-08-29. The wiki page "How to reproduce a restart test failure" shows `fdbserver -r simulation -f <test> --seed 523887594 --buggify on`.
- The 2014 Strange Loop abstract states that disks, network links and machines are "replaced in testing with software" so that "the exact same series of events can be replayed". https://www.thestrangeloop.com/2014/testing-distributed-systems-w-slash-deterministic-simulation.html, retrieved 2026-08-29. Talk video https://www.youtube.com/watch?v=4fFDFbi3toc (no transcript retrievable); secondary notes at https://alex-ii.github.io/notes/2018/04/29/distributed_systems_with_deterministic_simulation.html record the interface swap `INetwork -> SimNetwork`, `IAsyncFile -> SimFile` and the single-thread requirement.
- Wilson (Antithesis blog, 2024-02-13) states that a "fully-deterministic event-based network simulation" was written before the database, run as a single-threaded process with one RNG and rerun "with the same random seed". https://antithesis.com/blog/is_something_bugging_you/, retrieved 2026-08-29.
- Antithesis distinguishes FoundationDB-style DST, where "all nondeterministic components are pluggable", from running unmodified software inside a deterministic hypervisor; the controlled sources are clocks, thread interleaving and system randomness. https://antithesis.com/docs/resources/deterministic_simulation_testing/, retrieved 2026-08-29.
- The Antithesis hypervisor runs each instance on one physical core, virtualises time and routes I/O through a VMCALL channel; reproducibility enables time-travel debugging. https://antithesis.com/blog/deterministic_hypervisor/ (2024-03-20), retrieved 2026-08-29.
- A "Sometimes" assertion asserts that a state is reached in at least one run; a never-hit sometimes assertion indicates an unreachable state or weak testing. https://antithesis.com/docs/best_practices/sometimes_assertions/, retrieved 2026-08-29.
- TIGER_STYLE requires an average of at least two assertions per function, pair assertions on different code paths, and assertions of both positive and negative space; it states that assertions "downgrade catastrophic correctness bugs into liveness bugs" and are "a force multiplier for discovering bugs by fuzzing". `docs/TIGER_STYLE.md`, `main` branch, §Safety (lines ~105–150), retrieved 2026-08-29. The phrase "assertions as oracles" does not appear in the document.
- VOPR uses a random seed to tune fault-injection parameters; "the seed and Git commit hash can be used to replay back the exact simulation"; storage checkers verify data files byte-for-byte across caught-up replicas. `docs/internals/vopr.md`, `main` branch, lines ~9–39 and §Assertions and Checkers, retrieved 2026-08-29.
- Replay command: `./zig/zig build vopr -- 123` "produces a fully deterministic, reproducible outcome". `docs/internals/HACKING.md`, `main` branch, §Simulation (lines ~48–60), retrieved 2026-08-29.
- `src/vopr.zig` (`main`, 1805 lines): default seed `std.crypto.random.int(u64)`, `var prng = stdx.PRNG.from_seed(seed)`, `options_swarm(&prng)` randomises replica/client counts, packet loss, partition mode, storage fault probabilities and crash probabilities; failure message "you can reproduce this failure with seed={}"; safety mode then `transition_to_liveness_mode(core)` with `fatal(.liveness, "no state convergence: ...")` on timeout. Lines ~83–84, 127–157, 263–265, 349–350, 374ff, 802–808, 888, retrieved 2026-08-29.
- Testing doubles: `src/testing/packet_simulator.zig` (delay, loss, replay, partition modes `none`, `uniform_size`, `uniform_partition`, `isolate_single`, clogging); `src/testing/storage.zig` ("In-memory storage, with simulated faults and latency", read/write fault and misdirect probabilities, `ClusterFaultAtlas` guaranteeing one valid copy); `src/testing/time.zig` (`TimeSim` with tick-based monotonic and drifting realtime clocks); `src/testing/cluster/state_checker.zig` (hash-chain assertions such as `assert(header_b.?.parent == checksum_a)`). `main` branch, retrieved 2026-08-29.
- Liveness mode: pick a core quorum, heal its partitions, freeze non-core faults, require convergence within a timeout. https://tigerbeetle.com/blog/2023-07-06-simulation-testing-for-liveness/, retrieved 2026-08-29. VOPR's default mode swarm-randomises the fault distributions themselves. https://tigerbeetle.com/blog/2025-11-28-tale-of-four-fuzzers/ and https://tigerbeetle.com/blog/2025-04-23-swarm-testing-data-structures/, retrieved 2026-08-29.
- Swarm testing: a "swarm" of random configurations, "each of which omits some features", found 42% more distinct compiler crashes in a week (104 vs 73 for the default Csmith configuration); features can suppress interesting behaviour and compete for space in a test. Groce, Zhang, Eide, Chen, Regehr, ISSTA 2012, DOI 10.1145/2338965.2336763, Abstract and §1 (https://users.cs.utah.edu/~regehr/papers/swarm12.pdf, retrieved 2026-08-29).
- Rust equivalents: turmoil runs multiple hosts "within a single thread" with a seeded RNG and injects latency, drops, partitions and torn writes (https://github.com/tokio-rs/turmoil README, `main`); madsim requires "All I/O-related interfaces must be mocked", provides `Runtime::with_seed`, `MADSIM_TEST_SEED` and `MADSIM_TEST_CHECK_DETERMINISM` (https://github.com/madsim-rs/madsim README, docs.rs 0.2.34). Retrieved 2026-08-29.

## Mechanism

Recipe, with attribution:

1. Single-threaded scheduler over virtual time. All concurrency is cooperative; a task queue ordered by virtual timestamp is drained in one thread (FoundationDB `Sim2::runLoop`; TigerBeetle `TimeSim.tick()`; turmoil).
2. One seeded PRNG. Every random choice, including simulated latency, fault firing and workload generation, is drawn from a generator initialised from the CLI seed (`deterministicRandom()` with `-s SEED`; `stdx.PRNG.from_seed(seed)`).
3. Nondeterminism behind injectable interfaces. Network, disk, clock and randomness are traits with a production and a simulated implementation (`ISimulator`, `INetworkConnections`, `IAsyncFile`; `packet_simulator.zig`, `storage.zig`, `time.zig`; madsim mocks). Antithesis relocates this boundary to the hypervisor.
4. Fault injection at two levels: environment faults (partition, loss, crash, misdirected write, clock drift) and in-code probabilistic hooks (`BUGGIFY`, 0.25 × 0.25).
5. Replay by `(commit, seed)`. The failure report prints the seed; the same binary and seed reproduce the run (`fdbserver -r simulation -s`, `zig build vopr -- <seed>`).
6. Oracles are invariants, not expected outputs: dense assertions (TIGER_STYLE), state checkers with hash chaining, byte-identical storage across replicas, convergence within a liveness timeout, and reachability ("sometimes") assertions.
7. Swarm-randomised configurations: each seed also selects which features and fault classes are enabled and their probabilities (Groce 2012; VOPR `options_swarm`).
8. Volume: many short simulated runs per night, with time compression relative to wall-clock.

```
run(seed, build):
    prng   = Prng::from_seed(seed)
    config = swarm_config(&prng)             # which features/faults are on, and their rates
    env    = SimEnv { clock: VirtualClock, io: SimIo(prng, config), rng: prng }
    sys    = System::new(&env)               # all I/O through env traits
    model  = ReferenceModel::new()
    while env.clock.now() < config.ticks_max:
        env.step()                           # drain one virtual-time task; may fire faults
        if let Some(op) = workload.next(&prng, &model):
            sys.apply(op); model.apply(op)
        check_invariants(&sys, &model)       # assert, never log-and-continue
    assert_convergence(&sys, &model)         # liveness phase
    report { seed, commit, config, coverage, sometimes_hits }
```

Invariants: no wall-clock, thread or OS entropy reaches the system under test; any two runs with equal `(build, seed)` produce identical traces; every failure is emitted with the seed needed to reproduce it.

## NUIF relevance

**Borrow**

- Make `(implementation version, capability profile, fixture, seed)` the replay key of every conformance run, matching the report fields already required in `conformance/PLAN.md` (FoundationDB `-s SEED`; VOPR seed plus commit hash).
- Put every nondeterminism source of the headless engine behind traits with simulated implementations: font and image loading, adapter file I/O, renderer scheduling, timestamps in provenance records (FoundationDB interface swap; madsim mocking rule).
- Adopt assertion density and pair assertions in `nuif-core`, `nuif-protocol` and `nuif-layout` so that invariants (stable IDs, containment, acyclic references, `Extensions` unchanged by unrelated operations) fail inside the loop rather than in later comparison (TIGER_STYLE §Safety).
- Swarm-randomise the operation mix, layout families and adapter set per seed instead of fixing one generator distribution (Groce 2012; VOPR `options_swarm`).
- Add reachability ("sometimes") assertions for rare paths such as move-into-instance, extension preservation through an unaware intermediate, and lossy adapter fallbacks (Antithesis).

**Adapt**

- NUIF has no network or clock to virtualise; the analogue of environment faults is adapter loss (unsupported feature, approximated value), corrupted or truncated inputs, and resource-limit hits from `spec/11-security.md`. Fault injection should target those.
- The liveness phase becomes a convergence phase: after fault injection stops, canonical hashes across the round-trip path must converge, and operation replay from the same base must yield the same hash (`conformance/fixtures/v0-responsive-card/README.md`).
- Time compression is irrelevant; the equivalent budget is operations per second through the CLI/API contract of `spec/12-cli-api-and-automation.md`.

**Reject**

- A deterministic hypervisor is unnecessary: NUIF controls its own process and can achieve determinism at the interface level.
- BUGGIFY-style probabilistic hooks inside production code paths conflict with a library that must be embeddable; fault hooks belong in the simulated trait implementations only.

## Open questions

- Which floating-point paths in layout and rasterisation are deterministic across CPU architectures, and must the seed key include target triple and font rasteriser version?
- Should browser-based differential oracles be excluded from seeded runs, given that a browser cannot be made deterministic from NUIF's side?
- How are seeds and swarm configurations recorded in the report so that a coverage-guided scheduler can prioritise seeds without breaking replayability?
