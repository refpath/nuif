# Test-harness architecture

Status: profile-0 baseline, Gate C browser/Taffy, Gate D text/render, Gate E complete editor authoring, Gate F bounded HTML/CSS synchronization and Gate G independent v0 reproduction are implemented; fuzz packages, perceptual comparison and broader adapter trials remain planned. This document specifies how round-trip trials run unattended, fail reproducibly, minimize themselves and report in machine-readable form. Evidence is cited by research record identifier.

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
  nuif-layout              evaluation context and profile-0 reference evaluator
  nuif-render              render scene, backends (CPU reference; Vello interactive)
  nuif-codec               nuif-text-0, nuif-cbor-0, canonicalizer, migrations
  nuif-query               semantic queries
  nuif-api                 Engine trait, report types, session driver
  nuif-cli                 command surface; JSON output; stable exit codes
  nuif-testing             seeded trials, hostile-input measurement, v0 fixture, direct Taffy/Chrome oracles, reducer and reports
apps/
  editor                   headless editor driver and binary; Masonry GUI shell remains pending
conformance/
  Cargo.toml               executable profile-0 conformance package
  src/lib.rs               v0 responsive, extension, seeded-trial and parity assertions
  fixtures/<suite>/<id>/   input.nuif, context.toml, expected.*, meta.toml
  fonts/                   pinned fonts referenced by fixtures; no system fonts
  generated/               planned persisted browser cases; runtime Gate C cases are seed-derived
fuzz/                      planned cargo-fuzz package; no targets are implemented yet
xtask/                     implemented research/verify/Gate B/Gate C/hostile-input/editor loop
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

Persisted expectation regeneration remains a planned extension and will use one variable, `NUIF_UPDATE_EXPECT=1`: a missing expectation fails the case and writes `expected.*.new`, and generated suites are replaced wholesale rather than hand-edited (`taffy-and-yoga-browser-generated-tests`). The implemented Gate C runner instead derives cases from a recorded seed, measures all three engines in one run and stores raw observations in its report; it has no stale golden file to update. Per-asset metadata for future persisted suites follows the sample-asset corpus model (`gltf-validator-and-sample-assets`).

## Determinism controls

- Seed: every implemented generator draws from one xorshift PRNG seeded per trial; `nuif trial <seed> <iterations> [snapshot-interval] [report-path]` records the seed and failing iteration and optionally persists the JSON report (`deterministic-simulation-testing`).
- Time and randomness: profile 0 has no time-dependent engine semantics; the editor uses monotonically assigned transaction identifiers, and generated values come only from the trial PRNG. Virtual time is required before behavior/animation work begins (`masonry-xilem-and-linebender-test-harness`).
- Floating point: canonical text follows RFC 0005's stated shortest-digit layout and canonical CBOR follows RFCs 0005/0008. Gate C records each fixture's observed Taffy/browser maximum and rounds it upward to 0.01 px, capped by the 0.1 px foreign-engine safety bound. Exact agreement retains zero tolerance; no aggregate bound silently replaces the fixture values.
- Fonts: the fixture uses the content-addressed Ahem 1.50 bytes (`f0a92c…550dc`), HarfRust 0.13.3, Unicode 17.0.0 and Unicode-scalar cluster indices. Eight LTR/RTL goldens were independently captured with HarfBuzz 14.4.0. Five unhinted Skrifa 0.46.2 outlines match normalized `hb-vector` paths in signed 26.6 font units. Pinned Zeno 0.3.3 8-bit grayscale nonzero-fill coverage produces the same three scene and PNG hashes on macOS/aarch64, Linux/aarch64 and Linux/x86_64. CR/LF/CRLF/NEL/LS/PS hard-line layout is exact; profile 0 does not request automatic soft wrapping (`text-rendering-reproducibility`).
- Threads: the current suites have no shared mutable global state and pass under libtest's normal parallel execution.
- Environment: CI uses `--locked` and pinned Rust 1.98.0; the separate MSRV job checks 1.96.0.

## Oracles by suite

| Suite | Oracle class | Comparison |
|---|---|---|
| model | assertions | identity uniqueness, containment acyclicity, relation target existence |
| canonicalization | self-consistency | `E(D(E(d))) = E(d)`; hash stability; idempotent canonicalize |
| extensions | self-consistency through an ignorant implementation | byte identity of unknown payloads after decode, edit, encode (`opentimelineio`, `godot-tscn-scene-format`) |
| layout | implemented metamorphic relations plus pinned Taffy 0.14.0 and Chrome for Testing 152.0.7977.64 | responsive v0 at 360/768/1440 px and 12 seeded stack/flex/grid cases; raw three-engine boxes, measured fixture bounds and typed divergences (`differential-testing`, `css-flexbox-grid-algorithm-specs`) |
| text shaping/outlines | pinned HarfRust/Skrifa plus independently captured HarfBuzz 14.4.0 goldens | exact glyph IDs, Unicode-scalar clusters, font-unit advances and direction over eight Ahem cases; five normalized `hb-vector` outline paths; repeated scene runs and typed missing-font failures (`text-rendering-reproducibility`) |
| render | reference rasterization | profile-0 exact for scaled rectangle inclusion, Zeno ellipse coverage, pinned text masks and integer-composited encoded-sRGB solid color; unsupported path/image/instance/extension semantics remain property-attributed; proposed tier 2 bounded and tier 3 perceptual thresholds remain non-normative (`vello-testing-and-cpu-reference`, `flip-perceptual-difference-metric`, `webrender-reftests`) |
| operations | self-consistency and reference model | replay to identical hash; `apply(t⁻¹, apply(t, d)) ≡ d`; commutation of independent operations; undo-copy-redo invariance (`command-pattern-undo-and-event-sourcing`) |
| merge | assertions | three-way merges produce typed conflicts, never arbitrary winners; move and order cases from `crdt-tree-move-operation` |
| provenance | assertions | correspondence records survive representable round trips; minimal-patch locality measured as changed source spans |
| adapter | round trip and fidelity report | `canon(Y(X(d))) = canon(d)` on the representable subset; every deviation explained by a report entry |
| security | measured boundary and one-over cases | readers stop at 16 MiB plus one byte; syntax depth 64 and the RFC 0009 semantic limits are enforced; release cases fail above 2 s, 64 MiB allocated or 16 MiB retained; CPU targets remain capped at 16,777,216 pixels (`resource-bounded-serde-and-ciborium`) |

## Trial loop

The target loop is shared conceptually by CLI, CI and editor automation. Profile 0 currently implements generation, replay, inverse, canonical encodings, responsive layout, CPU rerender, operation ddmin and the Gate C foreign layout matrix. Adapters and automatic minimized-fixture writing shown below remain planned stages.

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

The 10,000-patch Gate B run writes `target/gate-b-report.json`. `cargo xtask all` also installs or reuses the locked browser oracle and writes `target/verification-manifest.json` on success or at the first failing step. The manifest records revision, dirty state, toolchain, completed steps and the presence of every expected evidence artifact, so CI and autonomous research controllers can make a decision without parsing console output. `cargo xtask manifest` performs the narrower presence audit on an already-generated evidence set; its manifest is labelled `artifact-index`, does not claim that it executed any trial, and fails after writing the index when an artifact is absent.

The layout-differential experiment writes `target/layout-differential-report.json`. It records the source revision, dirty state, toolchain, exact Taffy and browser pins, launch flags, seed, case source, viewport, raw box maps, observed foreign delta, fixture-local assertion value and every typed divergence. Missing browsers, version drift, evaluator defects, Taffy/browser differences beyond the measured bound and unclassified differences fail `cargo xtask gate-c`; declared schema-loss records remain visible and non-blocking.

The text-pinning experiment writes `target/text-pinning-report.json`. It records the exact font, shaper, Unicode, outline extractor, rasterizer and independent HarfBuzz oracle pins; expected and observed glyph/outline strings; source/toolchain/platform identity; hard-break/no-soft-wrap semantic trials; repeatability and committed scene/PNG baselines at three evaluation contexts; and negative missing/malformed-font cases. `cargo xtask gate-d-text` fails on any pin, golden, semantic, baseline, repeatability or negative-case mismatch. The bounded text profile is lossless and its hashes agree on macOS/aarch64, Linux/aarch64 and Linux/x86_64.

The independent-reproduction experiment writes `target/gate-g-report.json` plus canonical text, layout and PNG artifacts under `target/gate-g-independent`. `cargo xtask gate-g` generates reference artifacts at three viewports, runs the standard-library-only Python implementation's unit suite, then compares independently computed canonical bytes, opaque preservation, boxes, decoded RGBA and fidelity. The Python implementation does not import, link or invoke any reference package; only the outer differential harness invokes both implementations.

The render-profile experiment writes `target/render-profile-report.json`. It fixes every supported paint input by value, repeats rectangle and ellipse scenes/PNGs, rejects out-of-range sRGB channels, and requires entity/property pointers for unsupported path, image and instance kinds plus preserved document/entity extensions. `cargo xtask gate-d-render` fails on any baseline, repeatability, validation or fidelity-attribution mismatch; `cargo xtask gate-d` runs both Gate D reports.

The HTML/CSS retentive experiment writes `target/html-sync-report.json` and `target/html-sync-output.html`. It pins Tree-sitter and both grammars, exactly re-imports the declared subset, repeats synchronization, checks the complete unchanged-byte complement of six text/token/padding edits, preserves injected comments/unmapped markup and requires typed stale-span, unsupported-property and one-over-size failures. `cargo xtask gate-f` is blocking; the bounded profile and its non-claims are specified in `adapters/html-css/PROFILE.md`.

## Editor participation

The editor exposes an in-process session driver (`nuif-api`) that the harness calls without a window: create/open, apply operation, query accessibility tree, dispatch accessibility action, redraw to a CPU frame and snapshot. The accessibility tree carries entity identifiers (`accesskit-semantic-ui-testing`), so a test asserts "the selected entity is X" by role and identifier rather than by pixel position. `cargo xtask editor-trial` authors the complete v0 fixture from an empty document, demands byte identity with the direct generator and replay, and emits `target/editor-authoring-report.json` plus canonical document/context/layout/scene/CPU-PNG/fidelity artifacts under `target/editor-authoring-snapshot`. GUI screenshot comparison is limited to shell wiring and uses the same tiers as the render suite with per-OS baselines avoided by CPU rasterization. Gesture tests assert the emitted protocol operations, not canvas pixels.

## CI matrix

| Job | Content |
|---|---|
| commit-lint | subject rules |
| research | record validation |
| rust | fmt, check, clippy pedantic, `cargo test --workspace --locked`, 10,000-patch Gate B trial, hostile-input release measurement, pinned Gate C three-way layout trial, both Gate D text/render trials and all report uploads (all render suites CPU only) |
| fuzz-smoke (planned) | future fuzz targets with committed seed corpora |
| layout-differential | `cargo xtask browser-install` plus `cargo xtask gate-c`; seed-derived cases run in headless Chrome and fail on pin drift or blocking/unclassified divergence |
| editor-headless | editor session scripts through `nuif-api`; accessibility-tree assertions; CPU snapshots |
| gpu-optional | interactive backend under tier 3 on a GPU runner; failures are reported, not blocking |

## Non-goals

No GUI pointer automation as an oracle; no per-OS pixel baselines; no hand-edited generated fixtures; no test that depends on network access.
