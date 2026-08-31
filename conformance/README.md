# Conformance

The profile-0 baseline is executable through the `nuif-conformance` package and xtask gates. It covers structural validation, canonicalization, parser/serializer round trips, unknown-extension preservation, operations/replay/inversion, responsive stack/flex layout, bounded explicit fixed/`fr` Grid tracks and placement, pinned shaping and hard-line text, deterministic solid rectangle/ellipse and bounded RGBA8 image CPU rendering, measured codec/model/resource limits, pinned browser/Taffy differential layout, editor-driver parity, seven bounded retentive adapter profiles, one pure Figma snapshot/mutation-plan mapping profile and machine reports. Broader CSS Grid, paths/instances, broader image and paint profiles, live vendor adapters and perceptual tiers remain outside the implemented profile.

Install the locked Chrome for Testing build once with `cargo xtask browser-install`, then run `cargo xtask gate-c`. The report at `target/layout-differential-report.json` contains the raw NUIF, Taffy and browser boxes, engine versions, source revision, fixture-local calibration and every classified divergence. A schema-loss classification may describe input outside a declared profile, but cannot excuse a mismatch inside the bounded Grid profile; unclassified or evaluator differences fail the command.

Run `cargo xtask gate-d` for both text and paint. It writes `target/text-pinning-report.json` and `target/render-profile-report.json`; committed hashes, missing font failures, color validation and property-attributed unsupported/preserved fidelity are blocking checks.

Automated and conventional QA run the same headless tests. GUI automation is supplementary; semantic API operations are the primary test interface. `HARNESS.md` specifies the workspace layout, fixture format, determinism controls, oracles, trial loop, reducer and report schema.

Run `cargo xtask gate-h` for exhaustive collaboration register convergence,
existing-tree structural convergence, bounded concurrent creation (including
the separate causal nested-creation profile) and causal-stability compaction.
The creation profile exhausts all 24 delivery
orders for concurrent leaf inserts, reports ID collisions explicitly and checks
canonical metadata absence; nested creation and concurrently created parents
remain rejected. The compaction profile requires an exact locally observed
frontier, compares pre/post checkpoints and receipts across register and
structural materializers, and refuses partial or ahead frontiers.

Run `cargo xtask editor-hostile-inputs` for the release-mode semantic editor
boundary. It rejects excessive snapshot allocations before rendering, checks
finite parsing across every accessibility numeric family, and proves failed
transactions and history boundaries preserve the document and replay log.

Run `cargo xtask performance` for the portable release-mode smoke profile. It measures validation, both codecs, protocol apply, layout, scene lowering, CPU rasterization, end-to-end snapshots, bounded PNG and font inspection, and real embedded-resource package paths; records median/p95 latency and warmed allocation counts; enforces deliberately broad catastrophic-regression budgets; writes `target/performance-profile-report.json`; and compiles the statistical suite. Run `cargo bench -p nuif-conformance --bench profile_zero -- --noplot` and `cargo bench -p nuif-conformance --bench system_surfaces -- --noplot` on controlled hardware for scaling and subsystem comparisons. Shared CI timing is evidence, not a fine-grained cross-machine baseline. See [PERFORMANCE.md](PERFORMANCE.md) for the workload contract, comparison workflow and interpretation rules.

Run `cargo xtask gate-svg` for the bounded `nuif-svg-0` adapter. It checks
exact model round trips, seven byte-local semantic edits, preservation of the
complete unchanged-byte complement, typed hostile cases and a public-CLI
export/sync/import bridge. The machine report and synchronized sources are
written under `target/svg-sync-*`; the declared subset and its exclusions are
specified in `adapters/svg/PROFILE.md`.

Run `cargo xtask gate-dtcg` for the bounded `nuif-dtcg-scalar-0` adapter. It
checks exact scalar-token round trips, integer/real discrimination, eight
byte-local edits, root and token extension preservation, duplicate/depth/count
and source limits, and a public-CLI bridge. Reports and retained sources are
written under `target/dtcg-sync-*`; groups, aliases and composite types remain
outside the declared profile.

Run `cargo xtask gate-penpot` for the bounded `nuif-penpot-v3-0` package
adapter. It imports a fixture from the official JavaScript library, proves
deterministic export and exact no-op archive retention, applies eight mapped
JSON edits while preserving opaque members, exercises hostile ZIP boundaries
and completes an export/sync/import bridge through the public CLI. Reports and
packages are written under `target/penpot-sync-*`; components, libraries,
interactions, media, paths, layout and compact pages remain outside the profile.

Run `cargo xtask gate-react` for the bounded `nuif-react-jsx-0` source profile.
It extracts one directly returned marked intrinsic subtree without executing
JavaScript, applies 11 byte-local scalar edits, preserves unrelated module
source, rejects eleven dynamic/hostile cases and exercises the public CLI bridge.
Reports and synchronized JSX are written under `target/react-sync-*`;
components, hooks, spreads, handlers, runtime expressions, TSX and browser
runtime equivalence remain outside the profile.

Run `cargo xtask gate-svelte` for the bounded `nuif-svelte-static-0` source
profile. It applies 11 byte-local edits, preserves the complete unchanged-byte
complement, rejects 13 executable or hostile inputs, completes the public CLI
bridge, and then parses and compiles both synchronized outputs with exact
official `svelte/compiler` 5.57.0. Reports and components are written under
`target/svelte-*`; scripts, blocks, directives, components, component CSS and
runtime rendering equivalence remain outside the profile.

Run `cargo xtask gate-figma` for the credential-free
`nuif-figma-plugin-snapshot-0` mapping. It repeats normalized snapshot bytes,
round-trips the declared subset through the CLI, repairs portable identity and
exercises unsupported-property and hostile-input paths. It does not run inside
Figma or certify host mutation behavior.

Run `cargo xtask gate-wasm` for `nuif-wasm-api-0`. It compiles the same core
for `wasm32-unknown-unknown`, generates Node and direct-browser JavaScript plus
TypeScript surfaces, initializes the web target in pinned headless Chrome,
exercises load/validate/hash/text/CBOR/patch/undo/redo in Node and requires the
edited canonical bytes to equal the native CLI output.
The report and direct-browser developer package are written to
`target/wasm-conformance-report.json` and `target/nuif-wasm-web`. This gate does
not claim browser-layout, host plug-in or WASI CLI conformance.

Run `cargo xtask adapter-audit` to validate the complete advertised adapter
inventory independently of executable profile tests. It requires research and
explicit boundaries for twelve targets, checks crate/profile/gate references for
the ten integrated profiles and prevents researched or externally blocked
targets from claiming executable directions.

Run `cargo xtask diagnostic-audit` to require every model, layout and trial
diagnostic code to appear exactly once in the public registry with a stable
severity, category, producer and meaning.

Build the external-implementer handoff with `cargo xtask conformance-kit`.
The command verifies the passed in-repository evidence gates, then packages the
specification, schemas, bounded fixtures, adapter profiles and independent
Python reproduction into `target/dist/nuif-conformance-kit-<version>`. The
archive manifest binds every member to the source revision and digest. The kit
is a reproducibility artifact, not an interoperability certification; an
external implementation must publish its own provenance and results.
