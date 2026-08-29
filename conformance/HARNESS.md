# Test-harness architecture

Status: design; no harness code exists yet. This document specifies how the Rust workspace and the reference editor are laid out so that round-trip trials run unattended, fail reproducibly, minimize themselves and report in one machine-readable form. Evidence is cited by research record identifier.

## Goals

1. Every conformance suite in `conformance/PLAN.md` runs from `cargo test --workspace` and from `nuif` CLI commands without a display or GPU.
2. A failing trial is reproducible from a seed, a fixture identifier and an engine version, and is reduced to a minimal fixture automatically.
3. Oracles are explicit: reference implementation, alternative implementation, self-consistency (metamorphic relations) and declared assertions. No oracle is a human.
4. The editor is a client of the same engine and harness; nothing is testable only through the GUI.

## Workspace layout

```text
Cargo.toml                 workspace; [workspace.lints]; resolver = "2"; Cargo.lock committed
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
  nuif-testing             shared test support: generators, reference model, oracles, reducers, report writer
apps/
  editor                   reference test editor (binary crate; UI-SPEC.md)
conformance/
  Cargo.toml               one package, [[test]] harness = false per suite
  tests/                   model.rs canonicalization.rs extensions.rs layout.rs render.rs
                           operations.rs merge.rs provenance.rs adapter.rs security.rs
  fixtures/<suite>/<id>/   input.nuif, context.toml, expected.*, meta.toml
  fonts/                   pinned fonts referenced by fixtures; no system fonts
  generated/               browser-differential cases written by xtask; never edited by hand
fuzz/                      cargo-fuzz package; targets: decode_text, decode_cbor, apply_patch, layout, render_scene
xtask/                     gentest (browser differential), regen (expectations), report (aggregate), corpus checks
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
| `expected.scene.ron` | render scene serialization (render suite; format after `webrender-reftests` capture) |
| `expected.png` | reference rasterization from the CPU path (render suite only) |
| `expected.report.json` | fidelity and validation report (adapter, provenance, security suites) |
| `ops.nuif-log` | operation log for operations, merge and replay cases |
| `meta.toml` | unique title, issue reference, tags, `disabled = true|false` with reason, tolerance overrides |

Regeneration uses one variable, `NUIF_UPDATE_EXPECT=1`. A missing expectation fails the case and writes `expected.*.new`. Generated suites are regenerated wholesale by `cargo xtask gentest` and committed separately from hand-written fixtures (`taffy-and-yoga-browser-generated-tests`). Per-asset metadata follows the sample-asset corpus model (`gltf-validator-and-sample-assets`).

## Determinism controls

- Seed: every generator draws from one PRNG seeded per trial; the seed is printed on failure and accepted by `nuif replay --seed` (`deterministic-simulation-testing`).
- Time and randomness in the engine and editor are injected interfaces; tests use virtual time (`masonry-xilem-and-linebender-test-harness`).
- Floating point: canonical encoders apply RFC 8785 number serialization in text and the chosen RFC 8949 §4.2 policy in binary (`canonicalization-rfc8785-and-cbor-deterministic`); layout results are compared with declared tolerance, never by string equality of floats.
- Fonts: fixtures reference fonts by SHA-256; Unicode data version, shaper version, hinting mode (off), anti-aliasing mode (grayscale coverage) and subpixel quantum are part of the context (`text-rendering-reproducibility`).
- Threads: suites with shared state run under `--test-threads=1`; nextest gives per-test process isolation where available.
- Environment: CI runs with `--locked`, `SOURCE_DATE_EPOCH` set, and the pinned toolchain.

## Oracles by suite

| Suite | Oracle class | Comparison |
|---|---|---|
| model | assertions | identity uniqueness, containment acyclicity, relation target existence |
| canonicalization | self-consistency | `E(D(E(d))) = E(d)`; hash stability; idempotent canonicalize |
| extensions | self-consistency through an ignorant implementation | byte identity of unknown payloads after decode, edit, encode (`opentimelineio`, `godot-tscn-scene-format`) |
| layout | reference implementation (browser via WebDriver for the CSS-compatible subset) and metamorphic relations | boxes within `0.1` px for generated cases; declared tolerance for hand-written cases; divergence classified as schema loss, evaluator defect or implementation-defined (`differential-testing`, `css-flexbox-grid-algorithm-specs`) |
| render | reference rasterization | tier 1 exact (CPU `f32`, tolerance 0); tier 2 bounded (per-channel delta `<= 1`, differing pixels `<= n`); tier 3 perceptual (ꟻLIP mean `< 0.01` smoke, `< 0.001` strict at 67 PPD) for the interactive backend only (`vello-testing-and-cpu-reference`, `flip-perceptual-difference-metric`, `webrender-reftests`) |
| operations | self-consistency and reference model | replay to identical hash; `apply(t⁻¹, apply(t, d)) ≡ d`; commutation of independent operations; undo-copy-redo invariance (`command-pattern-undo-and-event-sourcing`) |
| merge | assertions | three-way merges produce typed conflicts, never arbitrary winners; move and order cases from `crdt-tree-move-operation` |
| provenance | assertions | correspondence records survive representable round trips; minimal-patch locality measured as changed source spans |
| adapter | round trip and fidelity report | `canon(Y(X(d))) = canon(d)` on the representable subset; every deviation explained by a report entry |
| security | fuzz targets with budgets | no panic, no allocation above budget, depth `<= 1024`, node count `<= 10^6` for parsers (`fuzzing-structured-inputs`) |

## Trial loop

The loop is the same for CLI, CI and the editor harness.

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

Reduction runs at three levels in order: ddmin over the operation sequence with preconditions re-checked, hierarchical reduction over the document tree keeping validity, then choice-sequence shrinking of generated values (`delta-debugging-and-test-case-reduction`, `property-based-testing-state-machines`). The output is a fixture directory that reproduces the failure without the generator.

## Report schema

One JSON document per run, modelled on the glTF Validator report (`gltf-validator-and-sample-assets`) and required by `apps/editor/QA.md` item 10:

```json
{
  "engine": {"version": "0.0.1", "commit": "…", "toolchain": "1.85.0"},
  "profile": {"capabilities": ["inspect", "layout", "render"], "encodings": ["nuif-text-0"]},
  "trial": {"seed": 42, "fixture": "layout/stack-wrap-001", "tier": "exact"},
  "context": {"viewport": [360, 640], "scale": 1.0, "locale": "en", "fonts": ["sha256:…"]},
  "issues": {"numErrors": 0, "numWarnings": 1, "numInfos": 0, "messages": [
    {"code": "LAYOUT_TOLERANCE_EXCEEDED", "severity": 1, "pointer": "/entities/…", "detail": {"expected": 12.0, "actual": 12.3}}
  ]},
  "fidelity": [{"entity": "…", "class": "approximated", "reason": "…"}],
  "artifacts": ["diffs/layout/stack-wrap-001.json"]
}
```

Codes are stable strings listed in `spec/12-cli-api-and-automation.md`; severity is `0` error, `1` warning, `2` information, `3` hint; the exit status depends only on errors.

## Editor participation

The editor exposes an in-process session driver (`nuif-api`) that the harness calls without a window: open, apply operation, query accessibility tree, dispatch accessibility action, redraw to a CPU frame, snapshot. The accessibility tree carries entity identifiers (`accesskit-semantic-ui-testing`), so a test asserts "the selected entity is X" by role and identifier rather than by pixel position. GUI screenshot comparison is limited to shell wiring and uses the same tiers as the render suite with per-OS baselines avoided by CPU rasterization. Gesture tests assert the emitted protocol operations, not canvas pixels.

## CI matrix

| Job | Content |
|---|---|
| commit-lint | subject rules |
| research | record validation |
| rust | fmt, check, clippy pedantic, `cargo test --workspace --locked` (all suites, CPU only) |
| fuzz-smoke | each fuzz target for 60 s with the committed corpus |
| gentest-check | `cargo xtask gentest --check` regenerates browser-differential cases in headless Chrome and fails on diff |
| editor-headless | editor session scripts through `nuif-api`; accessibility-tree assertions; CPU snapshots |
| gpu-optional | interactive backend under tier 3 on a GPU runner; failures are reported, not blocking |

## Non-goals

No GUI pointer automation as an oracle; no per-OS pixel baselines; no hand-edited generated fixtures; no test that depends on network access.
