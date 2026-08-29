# Test-harness architecture

Status: profile-0 baseline implemented; browser differential generation, fuzz packages, shaped text, perceptual comparison and adapter trials remain planned. This document specifies how round-trip trials run unattended, fail reproducibly, minimize themselves and report in one machine-readable form. Evidence is cited by research record identifier.

## Goals

1. Every conformance suite in `conformance/PLAN.md` runs from `cargo test --workspace` and from `nuif` CLI commands without a display or GPU.
2. A failing implemented trial is reproducible from its seed, iteration, source revision and minimized semantic operation sequence. Hierarchical document reduction and automatic fixture-directory writing remain planned.
3. Oracles are explicit: reference implementation, alternative implementation, self-consistency (metamorphic relations) and declared assertions. No oracle is a human.
4. The editor is a client of the same engine and harness; nothing is testable only through the GUI.

## Workspace layout

```text
Cargo.toml                 workspace; [workspace.lints]; resolver = "3"; Cargo.lock committed
rust-toolchain.toml        pinned toolchain (see ADR 0006 for the MSRV decision)
.cargo/config.toml         [alias] xtask = "run --package xtask --"
crates/
  nuif-core                canonical model
  nuif-protocol            operations, transactions, patches, inverses
  nuif-layout              evaluation context, evaluators (Taffy behind NUIF types)
  nuif-render              render scene, backends (CPU reference; Vello interactive)
  nuif-codec               nuif-text-0, nuif-cbor-0, canonicalizer, migrations
  nuif-query               semantic queries
  nuif-api                 Engine trait, report types, session driver
  nuif-cli                 command surface; JSON output; stable exit codes
  nuif-testing             shared test support: seeded trials, hostile-input measurement, v0 fixture, oracles, reducer, report writer
apps/
  editor                   headless editor driver and binary; Masonry GUI shell remains pending
conformance/
  Cargo.toml               executable profile-0 conformance package
  src/lib.rs               v0 responsive, extension, seeded-trial and parity assertions
  fixtures/<suite>/<id>/   input.nuif, context.toml, expected.*, meta.toml
  fonts/                   pinned fonts referenced by fixtures; no system fonts
  generated/               browser-differential cases written by xtask; never edited by hand
fuzz/                      planned cargo-fuzz package; no targets are implemented yet
xtask/                     implemented research/verify/trial/hostile-input/editor loop; browser gentest and expectation regeneration pending
tools/
  research/                record validator
  git/                     commit lint
```

Rationale: `harness = false` suites enumerate fixture directories at run time and remain compatible with `cargo nextest` (`libtest-mimic-and-data-driven-fixtures`); shared test support lives in a normal crate so the editor, the CLI and the suites use one generator and one reducer; `xtask` replaces shell scripts so generation is reproducible under `--locked` (`cargo-workspace-xtask-and-ci-layout`).

## Fixture format

One directory per case. Files:

| File | Content |
|---|---|
| `input.nuif` | authored document in `nuif-text-0`; `input.cbor` for binary-only cases |
| `context.toml` | evaluation context: viewport, scale factor, locale, writing direction, theme, font set (by hash), capability profile, determinism tier and tolerances |
| `expected.canonical.nuif` | canonical form after decode and re-encode (canonicalization suite) |
| `expected.layout.json` | resolved boxes and diagnostics keyed by entity identifier |
| `expected.scene.json` | render scene serialization (render suite) |
| `expected.png` | reference rasterization from the CPU path (render suite only) |
| `expected.report.json` | fidelity and validation report (adapter, provenance, security suites) |
| `ops.nuif-log` | operation log for operations, merge and replay cases |
| `meta.toml` | unique title, issue reference, tags, `disabled = true|false` with reason, tolerance overrides |

Regeneration uses one variable, `NUIF_UPDATE_EXPECT=1`. A missing expectation fails the case and writes `expected.*.new`. Generated suites are regenerated wholesale by `cargo xtask gentest` and committed separately from hand-written fixtures (`taffy-and-yoga-browser-generated-tests`). Per-asset metadata follows the sample-asset corpus model (`gltf-validator-and-sample-assets`).

## Determinism controls

- Seed: every implemented generator draws from one xorshift PRNG seeded per trial; `nuif trial <seed> <iterations> [snapshot-interval]` records the seed and failing iteration (`deterministic-simulation-testing`).
- Time and randomness: profile 0 has no time-dependent engine semantics; the editor uses monotonically assigned transaction identifiers, and generated values come only from the trial PRNG. Virtual time is required before behavior/animation work begins (`masonry-xilem-and-linebender-test-harness`).
- Floating point: canonical text follows RFC 0005's stated shortest-digit layout and canonical CBOR follows RFCs 0005/0008. Current layout tests assert structural relations and repeatability; foreign-oracle tolerance comparison is not implemented.
- Fonts: the fixture context carries a synthetic font hash, but profile 0 intentionally renders text with a deterministic bitmap proxy. Pinned real fonts, Unicode data, shaping and raster parameters are Gate D work (`text-rendering-reproducibility`).
- Threads: the current suites have no shared mutable global state and pass under libtest's normal parallel execution.
- Environment: CI uses `--locked` and pinned Rust 1.98.0; the separate MSRV job checks 1.96.0.

## Oracles by suite

| Suite | Oracle class | Comparison |
|---|---|---|
| model | assertions | identity uniqueness, containment acyclicity, relation target existence |
| canonicalization | self-consistency | `E(D(E(d))) = E(d)`; hash stability; idempotent canonicalize |
| extensions | self-consistency through an ignorant implementation | byte identity of unknown payloads after decode, edit, encode (`opentimelineio`, `godot-tscn-scene-format`) |
| layout | implemented metamorphic relations; browser/Taffy foreign references planned | responsive direction and relative-position assertions at 360/768/1440 px; no numeric tolerance is normative until the differential corpus is measured (`differential-testing`, `css-flexbox-grid-algorithm-specs`) |
| render | reference rasterization | profile-0 exact for the implemented integer-composited solid-color CPU path; proposed tier 2 bounded and tier 3 perceptual thresholds remain non-normative until the render-tolerance experiment measures NUIF fixtures (`vello-testing-and-cpu-reference`, `flip-perceptual-difference-metric`, `webrender-reftests`) |
| operations | self-consistency and reference model | replay to identical hash; `apply(t⁻¹, apply(t, d)) ≡ d`; commutation of independent operations; undo-copy-redo invariance (`command-pattern-undo-and-event-sourcing`) |
| merge | assertions | three-way merges produce typed conflicts, never arbitrary winners; move and order cases from `crdt-tree-move-operation` |
| provenance | assertions | correspondence records survive representable round trips; minimal-patch locality measured as changed source spans |
| adapter | round trip and fidelity report | `canon(Y(X(d))) = canon(d)` on the representable subset; every deviation explained by a report entry |
| security | measured boundary and one-over cases | readers stop at 16 MiB plus one byte; syntax depth 64 and the RFC 0009 semantic limits are enforced; release cases fail above 2 s, 64 MiB allocated or 16 MiB retained; CPU targets remain capped at 16,777,216 pixels (`resource-bounded-serde-and-ciborium`) |

## Trial loop

The target loop is shared conceptually by CLI, CI and editor automation. Profile 0 currently implements generation, replay, inverse, canonical encodings, responsive layout, CPU rerender and operation ddmin; the context-matrix foreign comparisons, adapters and fixture writer shown below are planned stages.

```text
trial(seed, profile):
  d0   := load(fixture) | generate(seed, profile)          # swarm-selected feature subset
  log  := generate_ops(seed, d0, reference_model)          # preconditions checked against the model
  d1   := apply(log, d0)                                   # engine
  assert replay(log, d0) == d1                             # determinism
  assert apply(inverse(log), d1) ≡ d0                      # inversion
  for ctx in context_matrix: L[ctx] := layout(d1, ctx)     # resolved snapshots
  for R in metamorphic_relations: assert R(d1, L)          # nine relation classes
  for A in adapters: assert roundtrip(A, d1) with report   # fidelity accounting
  bytes := encode(d1); assert canonical(bytes)             # fixpoint
  scene := lower(d1, L[ctx0]); img := raster_cpu(scene)    # tier 1 reference
  compare(img, expected | reftest_pair)                    # oracle by tier
  on failure: reduce(seed, d0, log) -> fixture; write report
```

The implemented reducer runs complement-based ddmin over the semantic operation sequence and re-runs the failing relation after every candidate removal. Hierarchical document reduction, generated-value shrinking and automatic fixture-directory writing remain planned (`delta-debugging-and-test-case-reduction`, `property-based-testing-state-machines`).

## Report schema

One JSON document per run, modelled on the glTF Validator report (`gltf-validator-and-sample-assets`) and required by `apps/editor/QA.md` item 10:

```json
{
  "schema_version": 1,
  "engine": {"version": "0.0.1", "toolchain": "rustc 1.98.0 (…) ", "source_revision": "…", "dirty": false},
  "profile": {"capabilities": ["model", "operations", "layout-profile-0", "render-cpu-profile-0"], "encodings": ["nuif-text-0", "nuif-cbor-0"]},
  "trial": {"seed": 42, "iterations": 10000, "operations_per_iteration": 16, "snapshot_interval": 100},
  "contexts": [{"viewport": [360, 640], "canonical_hash": "nuif-cbor-0:sha256:…", "layout_boxes": 8, "render_commands": 4}],
  "issues": {"errors": 0, "warnings": 0, "information": 4, "hints": 0, "messages": ["…"]},
  "fidelity": [{"context": [360, 640], "entity": "…", "status": {"class": "approximated", "reason": "…"}}],
  "artifacts": [],
  "reproduction": null
}
```

Diagnostic codes are stable strings emitted in machine reports; severities serialize as `error`, `warning`, `information` or `hint`, and command exit status depends only on errors. A complete public code registry is still required before profile publication.

The hostile-input experiment writes a separate `target/hostile-input-report.json` because allocator and elapsed-time measurements are process-level rather than document fidelity entries. It records every input size, expected/observed error class, allocation counters, retained bytes, elapsed microseconds, limits, warmup, allocator method, toolchain and platform. `cargo xtask hostile-inputs` regenerates it and CI uploads it as `hostile-input-report`.

## Editor participation

The editor exposes an in-process session driver (`nuif-api`) that the harness calls without a window: open, apply operation, query accessibility tree, dispatch accessibility action, redraw to a CPU frame, snapshot. The accessibility tree carries entity identifiers (`accesskit-semantic-ui-testing`), so a test asserts "the selected entity is X" by role and identifier rather than by pixel position. GUI screenshot comparison is limited to shell wiring and uses the same tiers as the render suite with per-OS baselines avoided by CPU rasterization. Gesture tests assert the emitted protocol operations, not canvas pixels.

## CI matrix

| Job | Content |
|---|---|
| commit-lint | subject rules |
| research | record validation |
| rust | fmt, check, clippy pedantic, `cargo test --workspace --locked`, 10,000-patch Gate B trial, hostile-input release measurement and report upload (all suites, CPU only) |
| fuzz-smoke (planned) | future fuzz targets with committed seed corpora |
| gentest-check (planned) | future `cargo xtask gentest --check` regenerates browser-differential cases in headless Chrome and fails on diff |
| editor-headless | editor session scripts through `nuif-api`; accessibility-tree assertions; CPU snapshots |
| gpu-optional | interactive backend under tier 3 on a GPU runner; failures are reported, not blocking |

## Non-goals

No GUI pointer automation as an oracle; no per-OS pixel baselines; no hand-edited generated fixtures; no test that depends on network access.
