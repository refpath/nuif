# NUIF — Neutral User Interface Format

**Authored intent, resolved state, stable identity and explicit loss accounting in one portable document model for user interfaces.**

[![CI](https://github.com/refpath/nuif/actions/workflows/ci.yml/badge.svg)](https://github.com/refpath/nuif/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](#license)
[![MSRV](https://img.shields.io/badge/MSRV-1.96-orange.svg)](Cargo.toml)
[![Editor](https://img.shields.io/badge/editor-research%20preview-orange.svg)](apps/editor/README.md)
[![Specification](https://img.shields.io/badge/specification-pre--draft-yellow.svg)](spec/README.md)
[![Research](https://img.shields.io/badge/research-corpus-8a2be2.svg)](research/README.md)

Citation metadata is provided in [`CITATION.cff`](CITATION.cff). The current
software citation identifies the latest published editor prerelease. It does
not cite the draft specification as an accredited standard.

[Problem](#problem) ·
[Scope](#scope) ·
[Architecture](#architecture) ·
[Repository](#repository) ·
[Method](#method) ·
[Status](#status) ·
[Contributing](#contributing) ·
[License](#license)

---

## Problem

Interface designs move between design editors, design systems, source frameworks and automation tools by regeneration. Each export flattens authored constraints, components, token bindings and provenance into pixels, a vendor scene graph or generated code; each import guesses them back. Three consequences follow: round trips lose information, the loss is silent, and entity identity does not survive the trip, so later edits cannot be synchronized as patches.

NUIF treats portability as a synchronization problem. One canonical document keeps the authored intent and the resolved evaluation state side by side under stable identifiers, records every lossy mapping as a typed fidelity class, preserves data it does not understand, and exposes every mutation as a replayable semantic operation. Editors, runtimes and adapters become peers of the document rather than owners of it.

## Scope

| NUIF is | NUIF is not |
|---------|-------------|
| A draft specification for a vendor-neutral authored-interface document model: identity, containment, relationship graphs, layout intent, resolved geometry, paint, text, components, tokens, behavior, provenance, extensions | A Figma clone or a Figma file format; vendor formats are adapters |
| A Rust reference engine (model, protocol, layout, render scene, codecs, query, headless API and CLI) that falsifies the specification | A renderer specification; GPU command streams are implementation detail behind a scene boundary |
| A reference test editor that replicates a conventional design-editor layout and authors NUIF state directly | A product editor; its feature set is fixed to what conformance testing, import and export require |
| An executable conformance kit: fixtures, operation replay, layout matrices, reference rasterization, fidelity diagnostics | A code generator that infers program semantics from pixels |
| A machine-readable research corpus with claims, questions and experiments | A standard; that status requires conformance profiles, neutral governance and independent implementations |

## Architecture

```mermaid
flowchart LR
  subgraph Clients
    CLI[CLI]
    Editor[Reference test editor]
    QA[Automated conformance clients]
  end

  subgraph Engine["Rust reference engine"]
    API[Headless API]
    Proto[Operations, transactions, patches]
    Doc[(Canonical document<br/>containment tree + relationship graphs<br/>authored intent)]
    Eval[Layout evaluation<br/>per evaluation context]
    Res[(Resolved snapshot<br/>boxes, shaped text, diagnostics)]
    Scene[Render scene]
    Codec[Codecs<br/>canonical text, deterministic CBOR]
    Query[Query and diagnostics]
  end

  subgraph Peers["Adapters (peers, not parents)"]
    HTML[HTML/CSS, Svelte, React]
    SVG[SVG]
    Design[Penpot, Figma]
    Native[SwiftUI, Compose, Flutter]
  end

  Ref[Renderer backends<br/>Vello/wgpu, CPU reference]
  Conf[Conformance suites<br/>fixtures, replay, reference images]

  CLI --> API
  Editor --> API
  QA --> API
  API --> Proto --> Doc
  Doc --> Eval --> Res --> Scene --> Ref
  Doc <--> Codec
  Doc --> Query
  Codec <--> Peers
  Peers -- fidelity records + correspondence --> Doc
  Ref --> Conf
  Res --> Conf
  Proto -- replay log --> Conf
```

Principles that the diagram encodes:

- the canonical document is the only owner of authored state; every client, including the editor, mutates it through semantic operations;
- resolved state is derived per evaluation context and never overwrites authored intent;
- identity is semantic and independent of path, order and geometry;
- adapters emit fidelity records (`lossless`, `representable`, `approximated`, `preserved_unrenderable`, `unsupported`) and correspondence records; silent loss is a conformance failure;
- unknown extensions survive transit byte-for-byte;
- collaboration is a profile above canonical documents, not part of them;
- the headless API and CLI are the primary test surface; GUI automation is supplementary.

## Repository

| Path | Content |
|------|---------|
| `docs/whitepaper/` | Research synthesis, architecture thesis, risk register, cross-industry patterns |
| `research/` | Evidence records, claims, open questions, experiment registry, coverage contract, schema |
| `spec/` | Draft normative modules (model, identity, components, layout, paint and text, operations, extensions, serialization, provenance, collaboration, security, automation, semantics) |
| `rfcs/` · `adrs/` | Proposals for the specification · decisions for the reference implementation |
| `crates/` | Rust workspace: model, protocol, layout, pinned text shaping, render, codecs, query, API, CLI and shared seeded testing |
| `apps/editor/` | Reference test editor: architecture, source installation, headless contract, UI specification |
| `conformance/` | Suite plan, test-harness architecture, fixtures including the v0 falsification experiment |
| `adapters/` · `bindings/` · `schemas/` | Adapter contracts, WASM bindings, interchange schemas |
| `tools/` | Research graph ingestion, validators, commit lint |

## Method

Research is structured data. Each record in `research/items/` has a stable identifier, a primary source with locators, a confidence value, typed relations to claims and other records, and links to the specification, decisions, code and experiments it informs. `research/coverage.yaml` maps every architectural front to its evidence and status (`covered`, `experiment-required`, `ongoing`), so gaps are detected structurally. Claims become normative only through the RFC process and an executable conformance fixture.

Testing is designed for automated trial loops: generate or load a document, apply operations, export and import through adapters, compare canonical forms, resolved layouts and reference images, minimize failures, and record a machine-readable report. The harness design is in `conformance/HARNESS.md`.

## Status

| Surface | Maturity | Boundary |
|---------|----------|----------|
| Reference editor | Research preview; current release `0.1.0-alpha.3` | Semantic Versioning applies to the editor application only |
| Draft specification | Pre-draft | No normative conformance profile is published |
| Executable adapter and conformance profiles | Experimental | Results apply only to each declared profile and evaluation matrix |
| Project | Open research project; not a standard | Standards status requires neutral governance and independent implementations |

Gates B through H are complete under the bounded, quantified criteria in `research/AUDIT.md`. The workspace executes structural validation, anchored atomic operations, replay/inversion, canonical text and deterministic CBOR, measured hostile-input limits, responsive profile-0 layout, exact CPU rasterization, pinned NUIF/Taffy/Chrome layout trials, seeded reports and headless and native-shell editor drivers. Gate C explicitly reports the still-missing Grid track/placement schema. Gate D pins shaping, outlines, hard-line layout, encoded-sRGB paint and integer composition; scene and raw-RGBA hashes reproduce on macOS/aarch64, Linux/aarch64 and Linux/x86_64, while PNG encoding is non-normative and paths, images, instances and extension paint remain property-attributed fidelity records. Five retentive adapter profiles are integrated across HTML/CSS, SVG 2, DTCG 2025.10 and Penpot v3 packages, with import, export, synchronization, hostile-input checks and CLI conformance. A machine-audited inventory separates these executable profiles from seven researched or externally bounded targets. Complete fixture authoring, AccessKit-driven deterministic GUI trials, standard-library-only Python v0 reproduction, a metadata-free collaboration register checkpoint, hostile editor interaction trials, a scaling benchmark suite and native host packaging are automated. The native shell exposes the complete model-backed profile-zero editing surface while leaving future-profile sections of the draft UI specification explicit; structural tree collaboration, a foreign collaboration engine, a general-purpose second implementation, signed release distribution and external interoperability review remain incomplete. Specifications are drafts; no conformance profile is published.

Run the automated baseline:

```sh
cargo xtask all # installs/reuses the pinned browser and runs every gate
cargo xtask browser-install # optional browser prefetch
cargo xtask trial 24301 100
cargo xtask gate-b # 10,000 patches; raster sample every 100 patches
cargo xtask hostile-inputs # boundary/one-over time and allocator report
cargo xtask editor-hostile-inputs # semantic, parser and snapshot rejection report
cargo xtask adapter-audit # research/profile/gate coverage for all advertised targets
cargo xtask performance # portable release-mode latency and allocation budgets
cargo xtask gate-c # NUIF/Taffy/pinned-Chrome layout report
cargo xtask gate-d-text # HarfBuzz golden shaping + separate raster report
cargo xtask editor-trial # author the v0 fixture and emit editor evidence
cargo xtask editor-gui-trial # exercise AccessKit and reproduce shell pixels
cargo xtask editor-package # build, smoke-test and archive the host application
cargo xtask editor-launch # package and open the native application
cargo xtask editor-install --user --channel source # persistent local build
cargo xtask editor-doctor --user # verify receipt, binary and integration
cargo xtask editor-update --user --channel alpha --check # resolve only
cargo xtask editor-update --user --channel alpha # verified explicit update
cargo xtask editor-rollback --user # reactivate the retained previous build
cargo xtask editor-uninstall --user # remove only marked managed paths
cargo xtask gate-f # retentive HTML/CSS subset synchronization
cargo xtask gate-f-v0 # full-v0 model sync plus editor/CLI source bridge
cargo xtask gate-g # independent Python v0 parse/write/layout/render
cargo xtask gate-h # exhaustive collaboration register convergence
cargo run --locked -p nuif-cli -- fixture v0-responsive-card /tmp/v0.nuif
cargo run --locked -p nuif-editor -- --headless \
  --script conformance/fixtures/v0-responsive-card/editor-trial.jsonl \
  --document /tmp/v0.nuif --output /tmp/edited.nuif
cargo run --locked -p nuif-editor # launch the native editor
```

`cargo xtask all` bootstraps the pinned Python research-validator environment and Chrome for Testing under ignored `target/`, then runs research validation, Rust verification, the short full-raster trial, the 10,000-patch Gate B trial, the release-mode hostile-input allocation/time trial, the Gate C differential layout trial, both Gate D text/render trials, complete headless editor authoring, the reproducible native-shell trial, bounded and full-v0 retentive HTML/CSS synchronization, the editor/CLI source bridge, the independent Gate G reproduction and exhaustive Gate H collaboration-register convergence. Each measured run leaves a JSON report or snapshot under `target/`; `target/verification-manifest.json` indexes the complete evidence set and records success or the first failed step. CI archives both the individual evidence and this manifest.

## Contributing

Research, specification, conformance, implementation and adapter contributions are accepted; see `CONTRIBUTING.md` for the record schema, the writing register and the single-line commit rule. Governance is described in `GOVERNANCE.md`; security reports in `SECURITY.md`.

## License

Reference code is dual-licensed under Apache-2.0 or MIT (`LICENSE-APACHE`,
`LICENSE-MIT`). The repository has not adopted specification-wide copyright or
patent terms. `docs/whitepaper/08-governance-and-standardization.md` records the
licensing requirements that precede standards-track publication.
