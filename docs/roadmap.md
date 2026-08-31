# Research and implementation roadmap

The evidence gates and quantified acceptance criteria are normative for project planning in `research/AUDIT.md`; these phases describe implementation order only.

## Phase 0 — foundation (complete)
Exit: research graph schema, architectural RFCs, compilable core seams, CI and v0 falsification fixture exist.

## Phase 1 — canonical model (complete)
Implement typed properties, relations, components, tokens, extensions, deterministic IDs, validation and operation replay. Exit: structural conformance suite and canonical hash stability.

## Phase 2a — responsive layout falsifier (complete)
Implement freeform + stack/flex subset and a pinned NUIF/Taffy/Chrome context matrix. Exit: responsive-card layout agreement at three viewports, measured per-fixture bounds and classification of every generated divergence.

## Phase 2b — bounded explicit Grid (complete)
Profile 0 now defines positive fixed/`fr` tracks, sparse row/column auto-flow,
zero-based explicit placement, positive spans, no implicit tracks and bounded
resource use. The independent NUIF evaluator implements those rules directly;
Taffy and CSS are lowering targets, not hidden runtime dependencies. Gate C
exercises simple, explicit and spanning Grid cases and passes with no classified,
blocking or unexplained divergence. Intrinsic, percentage, named, repeated,
subgrid, masonry and implicit CSS tracks remain outside this bounded profile.

## Phase 3 — visual/text (complete for profile 0)
Pinned Ahem/HarfRust shaping matches HarfBuzz glyph goldens; unhinted Skrifa 0.46.2 outlines match normalized `hb-vector` goldens; hard-line layout, rectangles, ellipses, encoded-sRGB color and integer composition have normative scene/raw-RGBA baselines across macOS/aarch64, Linux/aarch64 and Linux/x86_64. PNG hashes remain deterministic artifact diagnostics but are not pixel-conformance boundaries. Path, image, instance and extension paint remain explicit unsupported/preserved fidelity rather than hidden fallbacks. Full UAX #14 soft wrapping and expanded vector paints belong to a future profile.

## Phase 4a — bare serialization/protocol (complete for profile 0)
Canonical text + deterministic CBOR plus patch/diff/query CLI. Exit: byte-stable
cycles and measured hostile-input limits. The separate active codec decision
gate records four-scale size, latency, allocation, canonicalization and
decode-then-select evidence. Both implemented codecs pass opaque-data edit
preflight; schema candidates are not timed on partial models. Cap'n Proto is
the next candidate only after a complete mapping. This phase does not include the
portable `.nuif` package, images or general font resources.

## Phase 4b — portable package and resources (active; container segment implemented)
RFC 0010 now has a package layer above `nuif-codec`, stable assets in the core,
explicit verified resource resolution and package-preserving CLI/editor I/O.
The Rust writer, an independent in-repository writer and a standard-library-only
Python archive oracle produce identical bytes;
semantic/resource/package hashes obey distinct fixtures; hostile archives and
package/resource/count one-over cases are blocking through
`cargo xtask gate-i-package`. Existing raw `.nuif` inputs migrate read-only and
new bare forms use `.nuif.json`/`.nuif.cbor`. The executable
`nuif-png-rgba8-0` baseline independently decodes a deliberately narrow PNG
subset. The separately named `nuif-png-basic-rgba8-1` expansion now covers
non-interlaced lossless-to-RGBA8 greyscale, indexed, RGB, greyscale-alpha and
RGBA forms plus valid transparency; both retain encoded bytes and repeat
package-aware CPU image rendering through `cargo xtask gate-i-image`. Gate I
remains open for 16-bit/interlaced/colour-managed PNG and live host/GPU affine
equivalence. Package/session handoff and decoded image surfaces now have
measured sharing, total-byte and allocation ceilings. Static-font inspection
and packaged validation now have warmed allocation ceilings across every
accepted fixture. A Linux/Windows/macOS resource-gate matrix is configured;
successful hosted artifacts are still required before a cross-platform
reproduction claim. The
separate `nuif-opentype-static-single-0` baseline validates one exact static
TrueType face through package encoding/resolution, compares Skrifa results with
a pinned HarfBuzz metadata capture and rejects malformed/policy/one-over cases
through `cargo xtask gate-i-font`. A non-Ahem exact package now also drives
HarfRust shaping with declared global features, font-derived intrinsic metrics,
Skrifa outlines and deterministic CPU pixels through the shared session without
platform-font discovery. TTC, CFF/CFF2, variable/color/bitmap/WOFF2 fonts,
cluster-level fallback, a retained passing hosted cross-platform aggregate and
external implementations remain open. The three-host CI fan-out and fail-closed
twelve-report resource comparator are implemented, so this remaining evidence
no longer depends on manual comparison. Stable text-to-font asset bindings distinguish
requested, replacement and unavailable identities with six blocking fidelity
trials; six more trials cover the exact static runtime path.
Proposed RFC 0013 decomposes the next variable-TrueType profile around one
complete coordinate tuple and shared normalization for shaping, metrics and
outlines. Its first metadata-only gate now bounds `fvar`/`avar` and matches five
coordinate vectors with pinned HarfBuzz 14.4.0 output. A second isolated gate
matches seven HarfBuzz shapes including a FeatureVariations boundary and proves
metric/outline location reuse with matching HarfBuzz advances and paths.
A third isolated gate matches nonzero HVAR advances at four locations and
exercises a valid truncated advance-index map. A fourth matches MVAR x-height,
cap-height and line metrics at eight 13-axis locations in a reproducibly
subsetted OFL fixture. A fifth structurally preflights `gvar`, HVAR, MVAR and
STAT, rejects 38 checksum-repaired graph, profile and packed point/delta
mutations, and enforces warmed parser allocation/time regression ceilings. A
sixth gate proves resource-only package fixpoint, unrelated-edit retention,
explicit linked resolution, asset-policy validation, capability-gated typed
package admission and a byte fixpoint. A seventh gate adds reproducible
OFL-1.1 Noto Sans and Recursive subsets with distinct 2- and 5-axis graphs.
Eight HarfBuzz pipeline oracles agree exactly for metadata, normalization,
shaping, HVAR advances and MVAR metrics; seven outlines are exact and one
five-axis interior control coordinate is within the declared one-unit 26.6
bound. A generated-sfnt gate adds 16 accepted and three rejected packed-`gvar`
boundary cases, including repeated points, shared/private precedence and the
32,767 maximum count. The runtime gate then carries default and interior Noto
Sans coordinates through authorized package loading, shaping, HVAR intrinsic
layout, `gvar` outlines and deterministic CPU pixels while retaining the
normalized record in every resolved run. Direct Rust API delivery is now
executable. A follow-on gate requires the same complete snapshot through the
CLI, generated Node/browser WASM and stdio MCP surfaces. Hosted cross-platform
raster comparison, VVAR and byte-exhaustive malformed enumeration remain
non-claims rather than hidden promotion assumptions.

## Phase 5 — editor (complete for the headless profile-0 instrument)
The entire v0 fixture is authored from an empty document through identity-addressed semantic actions. Direct generation, editor output and operation replay are byte-identical, and the editor writes canonical document, context, layout, scene, CPU raster and fidelity report artifacts. The Rust-native Masonry shell from ADR 0006 and the later Svelte/WASM demonstration are non-normative interface work and cannot redefine this headless result.

## Phase 5b — native editor research preview (complete through alpha.3)
The native shell exposes the semantic driver through identity-backed canvas selection, a file menu with canonical and declared adapter import/export routes, document-aligned background grid and pixel rulers, layer and component browsing, insertion tools, evaluation widths, zoom, inspector transactions, bounded explicit Grid authoring and source-built developer installation. Captured pointer movement for freeform children previews locally and commits one semantic position operation on release; it snaps to whole pixels by default and supports Control-suspended snapping. Stack/Flex drags infer the effective responsive axis from resolved siblings and commit one same-parent `Move`; unchanged order creates no history, while Grid, Constraint, cross-parent and instance-child cases fail closed. Freeform selections expose eight handles; managed-layout children expose the three trailing handles. Resize previews resolved geometry and atomically commits the changed fixed axes plus an anchored freeform position when required; Shift preserves corner aspect ratio and invalid, root or semantically ineffective paths fail closed. Grid track, flow, atomic item position and span edits use the same validated operations as the headless and accessibility surfaces. Open packages pass their digest-verified embedded resources through the same bounded session used by CLI render/snapshot, so the narrow RGBA8 image segment renders without implicit fetching. `cargo xtask editor-gui-trial`, `cargo xtask editor-hostile-inputs` and `cargo xtask editor-install-trial` exercise the semantic, visual, adversarial and lifecycle boundaries. The broader `apps/editor/UI-SPEC.md` remains a draft; multi-selection, persisted aspect-ratio constraints, object smart guides, cross-parent/tree drag, Grid/Constraint reorder and managed leading-edge resize, token authoring and expanded paint are not claimed by this phase.

The editor explicitly supports the tested variable TrueType decoder capability;
its packaged coordinates reach the same resource-aware snapshot and exact-save
paths. Packages declaring any other capability open structurally but read-only.
The driver, accessibility surface and changed-package save boundary return the
exact missing set, while an unmodified copy stays byte-identical. This boundary
is included in the editor hostile-interaction gate.

## Phase 5c — browser and plug-in binding (complete for `nuif-wasm-api-0`)

The byte-oriented WebAssembly module wraps `nuif-api`, canonical text/CBOR,
deterministic packages, explicit package-capability negotiation and semantic
patches without copying the model into JavaScript. A Node/native differential
checks exact edited bare and package bytes, packaged-resource preservation and
typed missing-capability failures. Structural requirement-bearing packages are
read-only until complete-set authorization, including semantic patches and
mode conversion; the direct-browser target initializes its package API in
pinned headless Chrome. Its JavaScript, TypeScript and WASM are
packaged as a CI and tagged-release developer artifact. The module declares no
filesystem, network or host-document authority. The Figma review shell now
compiles against pinned official typings and crosses a mock snapshot into the
Rust core, but its assigned-ID live-host run remains separate. The Canva review
shell now compiles its pinned stable API after one hash-audited declaration
normalization, consumes Rust-generated plans, rejects unsupported live states
before insertion, packages its platform-only SDK license and records mock
single-sync/max-profile evidence. Browser-layout execution, a WASI CLI, npm
publication and live Figma, Affinity and Canva trials remain separate profiles
and version streams.

## Phase 5d — external agent binding (complete for `nuif-mcp-tools-0`)

The MCP process is a stateless stdio adapter over the same API and semantic
patch layer. Its four inline-text tools carry no host authority, support only
the current 2026-07-28 lifecycle, and are differentially checked against the
native CLI through a real child process. Five native release jobs package and
attest the separately versioned binary; source installation remains available
without an application store. Live compatibility with named third-party MCP
hosts, large-document resource handles and any authenticated HTTP service are
separate trials and are not claimed by this phase.

## Phase 5e — direct SDK and foreign binding boundary (direct SDK complete)

`nuif-api::NuifDocument` is the package-aware, byte-oriented façade over the
canonical codecs, verified package/resources and typed session operations.
Text/CBOR load, validation, transaction application, hashes, undo/redo and
bare/package export have one implementation; the WASM binding delegates to it
and the system benchmark suite measures direct text, CBOR and package calls.
The façade separates inert structural package access from session
authorization: requirement-bearing packages reject evaluation, mutation,
history and mode conversion until exact complete-set negotiation succeeds.
The unified performance gate executes every Criterion path once and audits
per-profile adapter direction coverage; controlled benchmarks include package
capability negotiation, variable-font package delivery and all eleven
integrated adapter profiles without treating shared CI timing noise as a
regression threshold.

The experimental `nuif-ffi-0` crate now provides opaque handles, bounded
byte-oriented document/package load and export, capability negotiation,
validation, patch and shared-snapshot calls, allocator-matched buffers, stable
numeric error classes and panic containment. `bindings/nuif_ffi.h` declares
single-thread-at-a-time handle access and is checked by `cargo xtask gate-ffi`
with Rust ABI tests, C11/C++17 consumer compiles, an exact experimental symbol
baseline, a linked POSIX C++ smoke and a linked release-library C
variable-font package/snapshot comparison
under normal, AddressSanitizer and UndefinedBehaviorSanitizer execution on
POSIX. The versioned native archive includes this evidence and hashes every
payload. No stable C ABI is claimed while the semantic API remains `0.0.x`.
Pinned cbindgen 0.29.4 now regenerates the committed header, and the gate rejects
declaration drift under the reviewed experimental compatibility policy. ADR
0011 still requires a separately reviewed `nuif-ffi-1` contract, broader C++
semantics plus pinned UniFFI Swift/Kotlin consumers, full target-matrix
sanitizer evidence and real XCFramework/AAR packages before that surface
becomes stable. The current macOS Swift importer checks the C profile and
allocator boundary but is not a generated native SDK. This is a
promotion gate, not missing logic that should be guessed into the core.

## Phase 5f — standalone developer CLI package (complete locally)

`nuif-cli-tools-0` is packaged for the same Linux x86-64, Linux AArch64,
Windows x86-64, macOS Apple Silicon and macOS Intel release matrix as the native
tools. Each archive contains the binary, licenses, developer instructions and a
smoke report produced by that release binary. The gate requires version and
capability identity, then generates, validates, canonicalizes and inspects a
real profile-0 document before the archive can be indexed. Sibling manifests
bind the source revision, platform, binary/archive digests, command inventory
and explicit filesystem/standard-stream authority; the release index records
the packages under `tools` and includes a separate CycloneDX SBOM. Local
macOS/AArch64 packaging passes. Successful hosted jobs and attestations remain
release-time evidence, not a claim made from workflow configuration.

The CLI declares only the tested variable TrueType decoder capability.
Packages requiring anything else remain available for structural inspection,
bare extraction and exact copying, while evaluation, external-format
conversion, semantic package rewrites and package-mode changes fail with the
exact requirement set. Native package import/export preserves resources and
manifest requirements.

## Phase 6a — first adapters/sync falsifier (complete for bounded HTML/CSS profile 0)
`nuif-html-css-0` maps a declared container/text/finite-token subset through real DOM/CSS syntax with byte-span correspondence. Text, token and four-edge padding edits change only their six spans; comments and unmapped markup survive exactly; unsupported semantics have target/property fidelity. HTML/CSS was intentionally tested before SVG because Gate F and the architecture stop condition concern minimal source patches. This narrow profile remains independently automated even after the full-v0 follow-on; arbitrary HTML/CSS and SVG remain broader adapter work.

## Phase 6b — full-v0 HTML/CSS sync (complete)
`nuif-html-css-v0` carries the complete responsive-card model through 181 retained correspondences. The full trial applies eight local token/padding/text/responsive edits while preserving all other source bytes and opaque payloads; the editor bridge applies name and width edits through semantic actions and the public CLI, then re-imports to byte-identical canonical NUIF. Browser path rendering, instance materialization and unknown visuals remain explicit target limitations.

## Phase 6c — bounded SVG sync (complete)
`nuif-svg-0` maps a fixed surface, freeform groups, rectangles, ellipses and literal pinned-font text to SVG 2 XML. The trial applies seven identity, geometry, paint, text and accessibility edits through 45 retained correspondences, preserves unmarked XML, and rejects scripts, external resources and unsupported SVG geometry before synchronization.

## Phase 6d — bounded DTCG sync (complete)
`nuif-dtcg-scalar-0` maps flat boolean, string and number tokens to the Design Tokens Format Module 2025.10. Namespaced metadata retains NUIF document and token identity and distinguishes integer from real values; the trial applies eight edits through 21 correspondences while preserving unknown extension bytes. Groups, aliases, composite types and token-local extensions require a token-model RFC and a separate profile.

## Phase 6e — adapter inventory (complete for advertised targets)
`adapters/index.json` enumerates twelve advertised targets. The blocking adapter audit requires a primary research record, integration surface, next bounded profile and exclusion boundary for every target; executable entries additionally require a crate, profile document and routed conformance gate. Eleven profiles are integrated: the seven retentive HTML/CSS, SVG, DTCG, Penpot, static React JSX and static Svelte profiles; `nuif-figma-plugin-snapshot-0`; `nuif-canva-design-editing-0`; the one-way `nuif-web-accessibility-0` projection; and the one-way `nuif-web-behavior-0` host lowering. The Figma and Canva profiles prove normalized mapping, CLI parity and deterministic static shells, not plug-in execution in either host. Affinity, SwiftUI, Jetpack Compose and Flutter remain explicitly researched or externally bounded rather than carrying unsupported implementation claims. Affinity is a user-mediated SVG bridge until a public API exists; Canva's compiled current-page plan consumer keeps live mutation, its platform-only SDK license, Connect and marketplace claims separate. Svelte uses Tree-sitter only for retained spans and exact official `svelte/compiler` 5.57.0 as its foreign parse/compile oracle.

## Phase 6f — bounded web accessibility projection (automated)

`nuif-web-accessibility-0` lowers ten roles, role-specific Boolean states and
five stable-identity relationships to inert native HTML/ARIA. It rejects
unsupported roles and state combinations, ambiguous direct/relationship names,
unnamed labels and duplicate relationships before output. The foreign oracle
pins Playwright 1.62.1 and its Chromium, Firefox and WebKit engines, then
compares computed role, accessible name and every admitted Boolean state for
eleven entities. The first macOS/arm64 run has identical full snapshots across
Chromium 151.0.7922.34, Firefox 153.0 and WebKit 26.5. Native platform APIs,
keyboard/focus traces, application behavior and broader semantic value types
remain separate work.

## Phase 6g — bounded behavior state-machine sidecar (automated)

`nuif-behavior-state-machine-0` executes stable-entity `activate` events through
one flat deterministic state machine. Ordered guarded transitions, sequential
Boolean/string actions, visibility/announcement effects, required capability
refusal and explicit optional no-op degradation have complete traces. The Rust
reference and independently written Node interpreter agree for both capability
sets over the five-event fixture. RFC 0012 now carries the same program as one
canonical-CBOR, content-addressed `source` resource under
`nuif-behavior-package-resource-0` without adding it to the semantic
`Document`. The package gate proves document/package hash separation, exact
round trip, hostile refusal and independent Python ZIP inspection; generic
package decode remains inert, the SDK reports exact missing package
requirements before a full-support claim, and runtime effect authorization
stays separate. Timers, internal events, numeric computation,
navigation, animation, networking, scripts, native effects and browser effects
beyond the following projection remain separate profiles and wire-design work.

## Phase 6h — bounded web behavior projection (automated)

`nuif-web-behavior-0` composes the behavior sidecar and accessibility
projection without accepting authored JavaScript. Enabled native button/switch
clicks select the same transitions as the reference runtime; visibility uses
`hidden`, and one advisory announcement per transition uses an unfocused polite
status region. Delimiter-safe program data is interpreted by one finite runtime
authorized by an exact CSP hash. Separate pointer and alternating Enter/Space
keyboard sequences pass all five events' state, transition, retained visibility
and announcement comparisons in Playwright
Chromium 151.0.7922.34, Firefox 153.0 and WebKit 26.5 on macOS/arm64. Checkbox,
radio, disabled-control, focus, navigation, animation, screen-reader speech,
native UI and arbitrary script remain explicit exclusions.

## Phase 7a — collaboration property registers (complete)
`nuif-collab-registers-0` keeps causal metadata outside canonical documents and materializes concurrent register-like semantic operations through operation-set and replica-log algorithms. Every delivery of the three-replica trial converges, and distinct concurrent values remain explicit property conflicts.

## Phase 7b — bounded existing-tree structural collaboration (complete)
`nuif-collab-tree-0` implements move, reorder, trash deletion and later rescue for identities already present in one canonical base. Unique Lamport ordering plus cycle rejection preserves one-parent/acyclic structure; RGA-style stable origins preserve deterministic sibling order without putting clocks, tombstones or position IDs in canonical NUIF. Move/move, delete/move, deleted-parent, delete/descendant-move, cycle and anchor conflicts remain explicit. Two materializers converge over all 5,040 deliveries of a fixture that includes a causal moved-position anchor, plus a 4,096-change scale trial. Pinned Automerge 3.4.1 reproduces the exact immutable operation set under three merge orders, duplicate merge and save/load; it is a foreign transport oracle, not an independent implementation of the tree algorithm. A standard-library-only foreign tree materializer now independently reproduces the bounded parent/order/anchor projection and active positions; canonical CBOR and semantic-conflict attribution remain explicit Rust-owned boundaries. The conservative `nuif-collab-tree-prefix-0` extension now collects a causally closed stable prefix, rebinds active stable anchors to checkpoint base positions and refuses inactive tombstone anchors. Its resumed document, hash and conflict set match complete replay. The separate `nuif-collab-tree-create-0` profile now covers concurrent leaf creation under existing parents, deterministic same-anchor order and explicit ID-collision conflicts over all 24 deliveries of its fixture. The bounded `nuif-collab-tree-create-nested-0` extension now covers a causally selected created parent, `Start` child insertion and base-sibling preservation over all six deliveries of its fixture. The new `nuif-collab-gc-0` profile safely compacts complete histories only when a caller-attested frontier exactly covers local clocks, emits a receipt and preserves the canonical checkpoint. The separately versioned `nuif-collab-tree-create-nested-1` extension accepts causally witnessed `After` anchors for selected created siblings under base or created parents over all 24 deliveries of its fixture. The separately versioned `nuif-collab-mixed-0` profile carries existing-tree structure and property changes in one causal set, resolves structure before properties, and rejects removed property targets over all 24 deliveries of its fixture. The register-only `nuif-collab-gc-prefix-0` extension now collects a causally closed stable prefix, resumes retained dots over a checkpoint-as-causal-base handoff and rejects unsafe concurrent or structural rebasing. Broader structural tombstone/anchor pruning remains future work.

## Phase 8a — mechanical independent reproduction (complete for v0 profile 0)
The standard-library-only Python implementation reads, writes, lays out and rasterizes the v0 profile without importing, invoking or linking the Rust packages. Its differential trial is exact at 360, 768 and 1,440 pixels and stays in the unified CI loop.

## Phase 8b — external reproduction and standards review
The versioned `nuif-conformance-kit-0` developer artifact now packages the
schema, profile specifications, bounded fixtures, adapter profiles, reports and
standard-library-only reproduction from one clean source revision. `cargo
xtask conformance-kit` refuses missing or failed evidence and emits a digest-
bound manifest plus a platform archive. Obtaining reproduction by an
externally authored implementation is still open. External provenance,
interoperability review, neutral governance and a published conformance profile
remain prerequisites for credible standards status; the in-repository
mechanical reproduction, source adapters and kit packaging do not establish
them.

## Phase 9a — canonical research publication (complete)
`cargo xtask docs-check` compiles the repository Markdown into one machine-readable catalog. `cargo xtask docs-build` renders that catalog without a second editable documentation source. `cargo xtask docs-paper` composes the thirteen canonical whitepaper modules into a working technical manuscript and a verified PDF. Pull requests build retained artifacts, while default-branch workflow runs deploy the static site through GitHub Pages. `CITATION.cff` describes the tagged alpha.3 software release; no DOI or peer-review claim is present.

## Phase 9b — implementer draft and incubation (blocked on external evidence)
Meet the implementer-draft gate in `docs/STANDARDS-ROADMAP.md`, including a general-purpose externally maintained implementation, requirement-to-test traceability, legal review of specification and patent terms and organizational supporters. Venue selection follows the resulting scope: W3C for Web and design-tool incubation, Khronos for graphics/content-tool conformance, or OASIS for a governed document protocol. Application alpha versions do not advance this phase.

## Phase 10 — source-backed browser capture (active; local live segment automated)

Create a dedicated browser-capture adapter instead of expanding the retentive
Tree-sitter adapter into a runtime. Pin browser/protocol/OS/context and collect
bounded source, DOM/layout/style, downloaded-resource, font-use, accessibility
and screenshot observations. Exit: repeated normalized observations/resource
hashes reproduce; multi-viewport evidence predicts a held-out context; canvas,
video, cross-origin and local-font gaps remain explicit; secret canaries never
enter exported evidence. `cargo xtask capture-baselines` proves repeatable
normalization, exact resource retention, query-secret redaction, typed proposal
application and cycle rejection from fixed provider input. `cargo xtask
gate-j-live` additionally drives exact Chrome for Testing 152.0.7977.64 with at
most three recorded fresh-profile attempts per viewport. It records a structured runtime context, retains exactly the
five declared response bodies, observes actual downloaded-font and
accessibility results, repeats 360 px bytes exactly, excludes exercised query,
cookie, storage, authorization and custom-header canaries, and beats the 360 px
freeform geometry at held-out 900 px using the 360/768 px observations. The
separate `target/layout-inference-report.json` ranks row stack, column stack,
Grid, linear constraint and freeform alternatives without consulting the
holdout, selects the constraint candidate, and records its 0.0626 normalized
held-out error against 0.2918 for fixed freeform. This one fixture is an
executable falsifier, not an accuracy distribution or proof of authored intent. Cross-OS/browser
reproduction, opaque and cross-origin fixtures, complete matched-style/source
correlation, canvas/video frame handling and real licensed pages remain open,
so the broader phase does not yet exit.

## Phase 11 — screenshot reconstruction baseline (active contract baseline)

Implement the vendor/model-neutral observation and typed-operation boundary from
RFC 0011/specification 14. Compare deterministic OCR/CV, one-shot proposal,
observation-assisted proposal, hierarchical crops, multi-viewport ranking and a
bounded render/difference correction loop. Exit: one harness reports validity,
text, element, tree, geometry, resources, held-out layout, provenance, visual,
confidence, latency and memory/cost; flat screenshot copies fail the editable
profile; an independent evaluator reproduces the main result.

The executable baseline currently proves observation-codec fixpoints, explicit
observed/inferred evidence and omissions, typed atomic proposals, default
flat-copy rejection, deterministic loop termination and training-only ranking
of five bounded layout hypotheses against a live held-out viewport. The
correction loop now has explicit `success`, `no_improvement`, and
`repeated_state` outcomes, with a caller-provided objective threshold so a
perfect score is never assumed by the core. The typed
`nuif-reconstruction-evaluation-0` report now covers every required per-example
family, preserves empty denominators as unscored, rejects screenshot-only
source-resource recall claims and keeps unavailable hardware measurements
explicit. Its deterministic synthetic fixture validates the evaluator
contract, not reconstruction quality. The missing OCR/model baselines, licensed
real held-out corpus, independently reviewed group/near-duplicate assignments,
predeclared FLIP thresholds/viewing
contexts, statistical uncertainty method and independent evaluator keep the
phase open. The pinned test-only LDR-FLIP implementation proves that exact and
perceptual diagnostics can coexist and refuses implicit alpha handling; its one
synthetic local error is not threshold calibration. A deterministic three-example fixture exercises typed
distribution aggregation, including micro/macro separation and explicit
missingness, without presenting it as empirical accuracy evidence.
The typed corpus manifest and audit now pin snapshot/card/evaluator/artifact
digests, separate disclosure and allowed-use policy, and reject exact or
family-level leakage across all four partitions. Its four synthetic policy
records prove the auditor, not the existence, legality, coverage or independence
of a real corpus. The provider-manifest gate now binds every browser,
screenshot, OCR, proposal and correction identity to canonical manifest bytes,
requires that observation bundles carry the complete bounded registry and
rejects dangling proposal identities before mutation. Released/learned
fixtures require external SPDX 3.0.1 or CycloneDX 1.7 inventory identity, and
learned fixtures require a model card. Synthetic digests and source-bundle
development providers prove the contract only; no released model or accuracy
result exists.

## Phase 12 — calibration and conditional adaptation (calibration primitive active; adaptation blocked on Phase 11)

Calibrate decision-level confidence and establish review/abstain risk thresholds
on disjoint data. Only if a stable learnable error distribution remains, create
rights-cleared validated operation traces and compare prompt/tool changes,
retrieval, supervised tuning, LoRA, QLoRA where compatible and sequence-level
distillation. Exit: a candidate beats the untuned closed-loop baseline under the
same frozen holdout/budget without validity, calibration, privacy, licensing or
maintenance regression. Training is skipped if that gate is not met.

The current interpolation/selective-review fixture and
`cargo xtask confidence-calibration` smoke test exercise only typed evaluator
arithmetic, disjoint split enforcement, shifted holdouts and selective-review
policy. They are not evidence of calibrated risk coverage on real data or of a
production threshold.

## Early falsifiers
Stop/rethink if: semantic model requires pervasive vendor-specific exceptions; opaque extensions cannot survive common operations; source synchronization routinely requires whole-file regeneration; independent implementation cannot reproduce normative layout/visual behavior from the spec; deterministic packages do not reproduce across writers; reconstruction optimizes pixels by discarding semantics; or tuning cannot beat the untuned tool-assisted baseline fairly.
