# Test-harness architecture

Status: profile-0 baseline, deterministic `nuif-package-0`, narrow cross-decoder `nuif-png-rgba8-0`, Gate C browser/Taffy, Gate D text/render, Gate E complete editor authoring, bounded and full-v0 Gate F HTML/CSS synchronization, SVG/DTCG/Penpot/React/Svelte retentive adapter gates, the pure Figma snapshot mapping gate, `nuif-wasm-api-0`, Gate G independent v0 reproduction, Gate H property-register and existing-tree convergence, and a bounded five-target sanitizer fuzz suite are implemented. Bounded browser/screenshot capture and reconstruction contracts plus the pinned local live-Chromium segment have executable evidence; their portable cross-provider accuracy corpus is not yet a release gate. Perceptual reconstruction comparison, concurrent entity creation and broader foreign-runtime trials remain planned. This document specifies how round-trip trials run unattended, fail reproducibly, minimize themselves and report in machine-readable form. Evidence is cited by research record identifier.

## Goals

1. Every conformance suite in `conformance/PLAN.md` runs from `cargo test --workspace` and from `nuif` CLI commands without a display or GPU.
2. A failing seeded trial is reproducible from its seed, iteration, sampled viewport, snapshot decision, source revision and minimized semantic operation sequence. When a report path is supplied, the CLI also reduces the base document and atomically emits a sibling regression-fixture directory.
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
  nuif-package             deterministic bounded .nuif ZIP package and resource policy
  nuif-media               bounded declared media decoders; narrow PNG RGBA8 profile
  nuif-font                bounded static OpenType inspection and package policy
  nuif-reconstruct         typed observations/proposals and finite correction loop
  nuif-capture             browser-source and strict screenshot capture baselines
  nuif-query               semantic queries
  nuif-api                 Engine trait, report types, session driver
  nuif-wasm                byte-oriented browser/Node binding over nuif-api
  nuif-cli                 command surface; JSON output; stable exit codes
  nuif-testing             seeded trials, hostile-input measurement, v0 fixture, direct Taffy/Chrome oracles, reducer and reports
apps/
  editor                   headless driver plus tested Masonry GUI shell; package-preserving I/O
conformance/
  Cargo.toml               executable profile-0 conformance package
  src/lib.rs               v0 responsive, extension, seeded-trial and parity assertions
  fixtures/<suite>/<id>/   input.nuif, context.toml, expected.*, meta.toml
  fonts/                   pinned fonts referenced by fixtures; no system fonts
  generated/               planned persisted browser cases; runtime Gate C cases are seed-derived
fuzz/                      pinned cargo-fuzz package; five bounded production-core targets
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
| `input.nuif` | deterministic `nuif-package-0` archive; resources remain content-addressed and policy-checked |
| `input.nuif.json` | generated canonical `nuif-text-0` projection for transparent inspection or an independent implementation that intentionally tests only the document profile |
| `input.cbor` | bare `nuif-cbor-0` input for codec-only binary cases; never labelled `.nuif` |
| `context.toml` | evaluation context: viewport, scale factor, locale, writing direction, theme, font set (by hash), capability profile, determinism tier and tolerances |
| `expected.canonical.nuif` | deterministic package form after decode and re-encode (package suite) |
| `expected.canonical.nuif.json` | canonical bare-document form after decode and re-encode (document codec suite) |
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
- Fonts: the fixture uses the content-addressed Ahem 1.50 bytes (`f0a92c…550dc`), HarfRust 0.13.3, Unicode 17.0.0 and Unicode-scalar cluster indices. Eight LTR/RTL goldens were independently captured with HarfBuzz 14.4.0. Five unhinted Skrifa 0.46.2 outlines match normalized `hb-vector` paths in signed 26.6 font units. Pinned Zeno 0.3.3 8-bit grayscale nonzero-fill coverage produces the same three scene and raw-RGBA hashes on macOS/aarch64, Linux/aarch64 and Linux/x86_64. CR/LF/CRLF/NEL/LS/PS hard-line layout is exact; profile 0 does not request automatic soft wrapping (`text-rendering-reproducibility`).
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
| package | independent ZIP writer plus fixpoint and corruption trials | fixed member order/metadata, exact package bytes, manifest/document/resource hashes, no traversal/symlink/encryption/compression/ZIP64 ambiguity, explicit linked-resource resolver |
| image resource | independent decoder plus package/render metamorphic checks | exact RGBA agreement across all PNG row filters, encoded-byte preservation, repeatable fit/crop/sampling/opacity CPU raster, fail-closed metadata and one-over inputs |
| font resource | independent parser plus package/policy metamorphic checks | exact static TrueType metrics/coverage agreement, byte preservation, explicit embedding review and fail-closed sfnt/policy/one-over inputs |
| capture/reconstruction contracts | metamorphic and policy assertions over fixed provider inputs | repeated normalized observations/packages, exact retained source resources, secret-query absence, evidence/omission truthfulness, observation codec fixpoint, typed proposal application, flat-copy rejection and finite-loop stop states; no live-capture or accuracy claim |
| security | measured boundary and one-over cases | bare readers stop at 16 MiB plus one byte; packages stop at 80 MiB with 32 MiB per resource, 64 MiB total embedded resources and 8,192 resources; syntax depth 64 and the RFC 0009 semantic limits are enforced; release bare-codec cases fail above 2 s, 64 MiB allocated or 16 MiB retained; CPU targets remain capped at 16,777,216 pixels (`resource-bounded-serde-and-ciborium`) |

## Trial loop

The target loop is shared conceptually by CLI, CI and editor automation. Profile 0 currently implements generation, replay, inverse, canonical encodings, responsive layout, CPU rerender, operation ddmin, document-aware subtree/scalar reduction, choice-stream shrinking, atomic minimized-fixture writing, adapter-specific round trips, fuzz choice streams and the Gate C foreign layout matrix.

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

The implemented reducer first runs complement-based ddmin over semantic operations. Its document pass removes entity subtrees in progressively finer chunks, prunes containment and relation edges, and relies on full model validation to reject dangling component, token or asset references before the interestingness predicate runs. It then reduces relations, tokens, assets, extension namespaces and known scalar fields; unknown-kind opaque bytes remain fixed. The choice-stream pass deletes contiguous regions, exhaustively lowers bytes and redistributes adjacent numeric choices toward shortlex order. Accepted candidates are content-hash memoized. A fixture writer atomically creates `input.nuif.json`, `operations.json`, `reduction.json` and `fixture.json` and refuses an existing destination. `cargo xtask reduction-profile` exercises all of these paths and archives the report and emitted fixture (`delta-debugging-and-test-case-reduction`, `property-based-testing-state-machines`).

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

The editor hostile-interaction experiment writes `target/editor-hostile-input-report.json`. Its release runner rejects zero, one-over-edge and maximal snapshot requests before raster allocation; accepts the exact one-dimensional edge boundary; rejects non-finite size, position and spacing values plus malformed paint without mutation; and checks missing selection/node errors, atomic multi-operation failure, empty history, redo invalidation and exact patch-log replay. Unit tests additionally exercise bounded script reads, per-line limits, command limits and malformed-line attribution. `cargo xtask editor-hostile-inputs` is blocking and CI archives the report.

The reduction experiment writes `target/reduction-profile-report.json` plus
`target/reduction-profile-fixture/`. Its fixed interestingness predicate reduces
the complete responsive card to the valid three-entity ancestor path, proves
component references cannot dangle, reduces a byte choice stream, records every
accepted transformation and confirms that the atomic writer refuses overwrite.
On a real `nuif trial` failure with a report path, the same machinery emits
`<report-path>.reproduction/` using the recorded failure code, viewport and
snapshot decision.

The standalone `fuzz/` workspace pins nightly 2026-08-28, cargo-fuzz 0.13.2
and libfuzzer-sys 0.4.13 without adding sanitizer dependencies to release
packages. `cargo xtask fuzz-smoke` regenerates target-specific valid seeds from
production fixtures, then runs raw codec, package/archive, resource-decoder,
static-source-adapter and typed-operation targets with explicit input, timeout,
allocation and RSS limits. The operation target maps bytes to valid production
operations rather than maintaining a second document grammar; parser targets
retain malformed bytes. CI runs 512 inputs per target under AddressSanitizer
and archives `target/fuzz-smoke-report.json`. Crash bytes remain local until
reduced and promoted to a named regression fixture.

The 10,000-patch Gate B run writes `target/gate-b-report.json`. `cargo xtask all` also installs or reuses the locked browser oracle and writes `target/verification-manifest.json` on success or at the first failing step. The manifest records revision, dirty state, toolchain, completed steps and the presence of every expected evidence artifact, so CI and autonomous research controllers can make a decision without parsing console output. `cargo xtask manifest` performs the narrower presence audit on an already-generated evidence set; its manifest is labelled `artifact-index`, does not claim that it executed any trial, and fails after writing the index when an artifact is absent.

The adapter inventory audit writes `target/adapter-coverage-report.json` before the executable gates. `cargo xtask adapter-audit` requires all eleven advertised targets to have a reviewed or verified research-record path, next bounded profile and explicit exclusion boundary. Integrated entries must resolve crate, profile and routed gate paths; researched and externally constrained entries must not claim executable directions. This is coverage and claim-boundary evidence, not foreign-runtime conformance.

The layout-differential experiment writes `target/layout-differential-report.json`. It records the source revision, dirty state, toolchain, exact Taffy and browser pins, launch flags, seed, case source, viewport, raw box maps, observed foreign delta, fixture-local assertion value and every typed divergence. Missing browsers, version drift, evaluator defects, Taffy/browser differences beyond the measured bound and unclassified differences fail `cargo xtask gate-c`. Schema-loss records remain available for inputs outside a declared profile, but the bounded explicit-Grid cases permit no schema-loss exemption.

The text-pinning experiment writes `target/text-pinning-report.json`. It records the exact font, shaper, Unicode, outline extractor, rasterizer and independent HarfBuzz oracle pins; expected and observed glyph/outline strings; source/toolchain/platform identity; hard-break/no-soft-wrap semantic trials; repeatability and committed scene/raw-RGBA baselines at three evaluation contexts; PNG artifact hashes; and negative missing/malformed-font cases. `cargo xtask gate-d-text` fails on any pin, golden, semantic, scene/pixel baseline, repeatability or negative-case mismatch. A PNG-reference mismatch is diagnostic because the lossless compressor is outside the pixel boundary. The bounded text profile is lossless and its scene/pixel hashes agree on macOS/aarch64, Linux/aarch64 and Linux/x86_64.

The independent-reproduction experiment writes `target/gate-g-report.json` plus canonical text, layout and PNG artifacts under `target/gate-g-independent`. `cargo xtask gate-g` generates a real package and reference artifacts at three viewports, exports one generated `input.nuif.json` projection, runs the standard-library-only Python document-profile implementation's unit suite, then compares independently computed canonical document bytes, opaque preservation, boxes, decoded RGBA and fidelity. The Python implementation deliberately does not claim package parsing and does not import, link or invoke any Rust workspace package; only the outer differential harness invokes both implementations.

The render-profile experiment writes `target/render-profile-report.json`. It fixes every supported paint input by value, repeats rectangle and ellipse scenes/raw-RGBA rasters/PNG artifacts, rejects out-of-range sRGB channels, and requires entity/property pointers for unsupported path, image and instance kinds plus preserved document/entity extensions. `cargo xtask gate-d-render` fails on any scene/pixel baseline, repeatability, validation or fidelity-attribution mismatch; `cargo xtask gate-d` runs both Gate D reports.

The narrow image-resource experiment writes
`target/image-resources-report.json`. `cargo xtask gate-i-image` compares exact
RGBA output from `png` 0.18.1 and independently implemented `zune-png` 0.5.2
across all row filters with absent/valid `sRGB`, preserves exact encoded bytes
through package fixpoint and an unrelated edit, repeats resource-aware scene
and CPU raster output, and rejects unsupported, corrupt and one-over cases. The
report explicitly excludes broad PNG colour/metadata support, non-identity
transforms, GPU/cross-platform image reproduction and non-PNG formats.

The narrow font-resource experiment writes
`target/font-resources-report.json`. `cargo xtask gate-i-font` compares the
profile's Skrifa interpretation with a committed `hb-info` 14.4.0 capture of
metrics, family, tables and Unicode coverage for the exact pinned Ahem bytes,
while three more static TrueType fixtures exercise acceptance. NUIF-owned sfnt,
checksum and OS/2 validation remains ahead of Skrifa. The gate proves package
byte fixpoint and resource retention, mutates metadata and embedding evidence,
distinguishes six portability outcomes, and rejects synthetic malformed cases
plus real TTC, CFF, variable, COLR, bitmap, CBDT and sbix inputs. The report
explicitly excludes TTC, CFF/CFF2, variable, color, bitmap, SVG and WOFF/WOFF2
fonts; it does not claim shaping/raster equivalence or that technical flags
grant redistribution rights.

The HTML/CSS retentive experiment writes `target/html-sync-report.json` and `target/html-sync-output.html`. It pins Tree-sitter and both grammars, exactly re-imports the declared subset, repeats synchronization, checks the complete unchanged-byte complement of six text/token/padding edits, preserves injected comments/unmapped markup and requires typed stale-span, unsupported-property and one-over-size failures. `cargo xtask gate-f` is blocking; the bounded profile and its non-claims are specified in `adapters/html-css/PROFILE.md`.

The full-v0 follow-on writes `target/html-sync-v0-report.json`, `target/html-sync-v0-output.html`, `target/html-sync-v0-editor-report.json` and `target/html-sync-v0-editor-output.html`. `cargo xtask gate-f-v0` checks 181 source correspondences, the unchanged-byte complement of eight model edits, exact opaque preservation and typed negative cases, then drives a semantic editor name/width edit through CLI synchronization and CLI import to byte-identical canonical NUIF. Target visual limits and arbitrary-CSS non-claims are specified in `adapters/html-css/V0-PROFILE.md`.

The SVG retentive experiment writes `target/svg-sync-report.json`, a direct synchronized SVG and edited canonical document at `target/svg-sync-edited.nuif.json`, plus separate public-CLI synchronization report and SVG. `cargo xtask gate-svg` checks exact import/export, repeatability, the unchanged-byte complement of seven accessibility, paint, geometry and text edits, preserved comments and metadata, and typed unsupported-property, structural, stale-span, derived-geometry, DTD, XML-node and byte-limit cases. The CLI bridge exports a package fixture, synchronizes from the explicit bare-document projection and requires byte-identical canonical document re-import. The mapped SVG 2 subset and arbitrary-SVG non-claims are specified in `adapters/svg/PROFILE.md`.

The DTCG scalar-token experiment writes `target/dtcg-sync-report.json`, a direct synchronized token file and edited canonical document at `target/dtcg-sync-edited.nuif.json`, plus separate public-CLI synchronization report and token file. `cargo xtask gate-dtcg` checks exact import/export, NUIF Integer/Real discrimination inside DTCG `number`, repeatability, the unchanged-byte complement of eight name/type/value/metadata edits, and root/token extension retention. Duplicate members, aliases, undeclared standard members, excessive JSON depth, one-over token count, one-over source bytes, unsupported values, structural changes and stale spans are typed failures. The CLI bridge requires byte-identical canonical document re-import. The mapped DTCG 2025.10 subset and token-model limitations are specified in `adapters/dtcg/PROFILE.md`.

The Penpot v3 package experiment writes `target/penpot-sync-report.json`, a synchronized Penpot package and edited canonical NUIF document at `target/penpot-sync-edited.nuif.json`, plus separate public-CLI synchronization report and Penpot package. `cargo xtask gate-penpot` imports the fixture produced by official `@penpot/library` 1.1.0, checks deterministic export and byte-exact no-op archive retention, applies eight mapped JSON scalar edits, preserves untouched member payloads plus injected opaque binary/JSON data, and requires exact canonical document re-import. Unsafe paths and one-over package/member limits are typed failures. The library importer additionally rejects excessive count/expansion/ratio/depth/value cases, duplicate names, directories, symlinks, encryption and unsupported compression. The mapped package subset and compact/components/libraries/interactions non-claims are specified in `adapters/penpot/PROFILE.md`.

The static React JSX experiment writes `target/react-sync-report.json`, a
synchronized JSX module, an edited canonical document and separate CLI bridge
artifacts. `cargo xtask gate-react` uses Tree-sitter JavaScript byte ranges but
never evaluates JavaScript. It checks 21 correspondences, 11 mapped edits, the
exact unchanged-byte complement, repeated output, typed stale/structural/profile
failures and eleven excluded or hostile sources, including the one-over mapped
JSX depth case. The intrinsic-only mapping and
runtime non-claims are specified in `adapters/react/PROFILE.md`.

The static Svelte experiment writes `target/svelte-sync-report.json`, a
synchronized component, an edited canonical document, separate CLI bridge
artifacts and `target/svelte-compiler-oracle-report.json`. `cargo xtask
gate-svelte` checks 21 correspondences, 11 mapped edits, exact unchanged-byte
complement preservation, repeated output, typed stale/structural/profile
failures and 13 excluded or hostile sources. It then uses the exact npm
lockfile with lifecycle scripts disabled and requires official
`svelte/compiler` 5.57.0 to parse in modern-AST mode and compile both direct and
CLI output without warnings. Tree-sitter owns retained byte ranges; the
official compiler remains a separate semantic oracle. Runtime rendering,
component CSS and executable template semantics are explicit non-claims in
`adapters/svelte/PROFILE.md`.

The Figma pure-mapping experiment writes
`target/figma-snapshot-report.json`. `cargo xtask gate-figma` repeats the
normalized snapshot bytes, maps the exact visible/opaque/fixed-size subset in
both directions through the public CLI, records deterministic repair for
portable identity, reports hidden/transparent/effect/variable properties and
rejects duplicate host IDs plus the byte limit plus one. It does not load a
page, create a node, mutate a host document or test undo inside Figma; those
remain live-host requirements in `adapters/figma/PROFILE-DRAFT.md`.

The WebAssembly cross-surface experiment writes
`target/wasm-conformance-report.json` and generates Node and direct-browser
packages. `cargo xtask gate-wasm` pins wasm-bindgen 0.2.127, initializes the
direct-browser target in pinned Chrome, runs the generated Node ABI through
canonical text/CBOR, validation, atomic patch and history paths, and requires
the output bytes to equal the native CLI after the same patch. It also checks
stale, malformed and one-over-byte failure atomicity and an empty authority
declaration. Browser layout, WASI and vendor plug-in behavior remain separate
trials.

The MCP cross-surface experiment writes
`target/mcp-conformance-report.json`. `cargo xtask gate-mcp` launches the real
stdio binary, opens the 2026-07-28 stateless lifecycle with `server/discover`,
and sends complete metadata on every valid request. An independent Python
driver checks the exact four-tool set, JSON input/output schemas, side-effect
annotations, typed errors, connection survival after a rejected request and a
one-over 4 MiB frame. Canonicalization and atomic patch output must be
byte-identical to the native CLI. Twenty-five repeated validation calls record
wire median, p95 and maximum latency with a catastrophic two-second p95 budget;
this smoke distribution is not a controlled throughput benchmark.

The collaboration register experiment writes `target/collaboration-report.json`. `cargo xtask gate-h` exhausts all 5,040 deliveries through operation-set and replica-log materializers, checks multiple merge orders and duplicate delivery, requires property-attributed multi-value conflicts and inspects canonical text for leaked replica state. Structural operations still fail before register-profile ingestion.

The separate existing-tree experiment writes `target/collaboration-structure-report.json`, exhausts 5,040 deliveries of seven move/delete/cycle/stable-anchor changes through sorted-set and incremental rollback/replay materializers, requires one-parent/acyclic checkpoints plus explicit move, deletion, cycle and anchor conflicts, and runs a 4,096-change release scaling guard. `tools/automerge-oracle` uses pinned `@automerge/automerge` 3.4.1 to merge the seven immutable operation records in different orders and through save/load, writing `target/collaboration-automerge-report.json`. This is foreign transport evidence; Automerge does not provide the NUIF tree materializer. Both executable boundaries are specified in `crates/nuif-collab/README.md` and `spec/10-collaboration-profile.md`.

The capture/reconstruction contract experiment writes
`target/capture-reconstruction-report.json`. `cargo xtask capture-baselines`
uses fixed browser-provider and strict PNG inputs to exercise repeatability,
resource identity, query-secret redaction, evidence classes and omissions,
typed atomic proposal application, flat-copy rejection, observation codec
fixpoints, calibration interpolation/selective review and finite loop stops.
The report carries explicit non-claims for live browser capture, OCR/model
accuracy, a broad or held-out corpus, independent evaluation and training.

The separate live capture experiment writes
`target/live-browser-capture-report.json`. `cargo xtask gate-j-live` installs or
reuses exact Chrome for Testing 152.0.7977.64 and accepts isolated 360, 768,
held-out 900 and repeated 360 px captures through bounded loopback CDP. It
allows at most three recorded fresh-profile attempts per viewport and accepts
only the exact resource/font fixture. It requires loader-specific load plus
image/font readiness, a bounded event-quiet point, stable consecutive
screenshots, structured context, exact
HTML/CSS/PNG/font/probe body set, actual downloaded-font and accessibility
evidence, repeat-identical capture/normalization/screenshot bytes, absence of
five exercised transport/storage secret canaries and lower held-out aggregate
geometry error from two viewports than the one-viewport freeform baseline. The
adapter also enforces aggregate event-byte, command, node, font-use, resource,
decode, write-buffer and connected-capture limits; the gate requires the four
accepted captures and any recorded retries to finish within 120 seconds. The report explicitly excludes
cross-browser/OS, opaque-frame, authenticated-site, canvas/video semantic and
reconstruction-accuracy claims.

## Editor participation

The editor exposes an in-process session driver (`nuif-api`) that the harness calls without a window: create/open, apply operation, query accessibility tree, dispatch accessibility action, redraw to a CPU frame and snapshot. The accessibility tree carries entity identifiers (`accesskit-semantic-ui-testing`), so a test asserts "the selected entity is X" by role and identifier rather than by pixel position. `cargo xtask editor-trial` authors the complete v0 fixture from an empty document, demands byte identity with the direct generator and replay, and emits `target/editor-authoring-report.json` plus canonical document/context/layout/scene/CPU-PNG/fidelity artifacts under `target/editor-authoring-snapshot`. GUI screenshot comparison is limited to shell wiring and uses the same tiers as the render suite with per-OS baselines avoided by CPU rasterization. Gesture tests assert the emitted protocol operations, not canvas pixels.

## CI matrix

| Job | Content |
|---|---|
| commit-lint | subject rules |
| research | record validation |
| rust | fmt, check, clippy pedantic, `cargo test --workspace --locked`, 10,000-patch Gate B trial, hostile-input release measurement, pinned Gate C three-way layout trial, both Gate D text/render trials and all report uploads (all render suites CPU only) |
| reduction-profile | validity-preserving subtree/scalar and choice-stream reduction plus atomic regression-fixture emission (currently part of the `rust` complete gate) |
| wasm | generated Node/direct-browser binding, Node/native byte differential, typed limit failures and downloadable developer artifact (currently part of the `rust` complete gate) |
| fuzz-smoke | five cargo-fuzz/libFuzzer targets, regenerated production seeds, AddressSanitizer, explicit resource limits and an archived campaign report |
| layout-differential | `cargo xtask browser-install` plus `cargo xtask gate-c`; seed-derived cases run in headless Chrome and fail on pin drift or blocking/unclassified divergence |
| editor-headless | editor session scripts through `nuif-api`; accessibility-tree assertions; CPU snapshots |
| gpu-optional | interactive backend under tier 3 on a GPU runner; failures are reported, not blocking |

## Non-goals

No GUI pointer automation as an oracle; no per-OS pixel baselines; no hand-edited generated fixtures; no test that depends on network access.
