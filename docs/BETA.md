# Developer beta scope and acceptance

Status: proposed acceptance contract, 2026-09-06. The editor remains
`0.1.0-alpha.3`. Passing the automated checks below does not change that version
or establish external usability.

## Workflow and compatibility boundary

The candidate developer beta supports a local authored-document workflow:
import a declared HTML/CSS profile, inspect its fidelity report, edit supported
properties through the reference editor, synchronize those properties into the
retained source, and re-import the result. The initial profiles are
[`nuif-html-css-0`](../adapters/html-css/PROFILE.md) and
[`nuif-html-css-v0`](../adapters/html-css/V0-PROFILE.md).

Profile identifiers and their fixtures define compatibility. The v0 profile
carries model fields that the browser cannot render; those fields remain
explicitly classified. Structural source edits, arbitrary React execution,
live Figma/Canva interoperability and screenshot reconstruction are outside
this beta acceptance contract. Their research continues under separate
profiles. No later dependency update may silently broaden or reinterpret an
existing profile's accepted semantics.

Canonical text and CBOR are the review and persistence boundaries for this
workflow. Resource packages retain their separate experimental capability
requirements. The reference editor's application version does not version the
specification, and a beta label would not declare an externally standardized
format.

## Automated acceptance evidence

Every release candidate requires a clean revision, passing checks and archived
reports from that revision. A missing, stale or failed report is not a passing
result.

| Criterion | Reproduction command or test | Acceptance |
|---|---|---|
| Canonical document and operation behavior | `cargo xtask gate-b` | Exact fixture, replay and preservation assertions pass |
| Bounded source synchronization | `cargo xtask gate-f` and `cargo xtask gate-f-v0` | Exact edited-document re-import, deterministic edits, byte locality and declared refusals |
| Generated workflow variation | `cargo xtask source-workflow` | Every generated case and CLI subprocess test passes |
| Native editing and operation replay | `cargo xtask editor-trial` and `cargo xtask editor-gui-trial` | Semantic actions, replay and repeated CPU-render assertions pass |
| File-save failure handling | `cargo test -p nuif-codec --features filesystem` | Staged failures preserve destinations; permissions and unsupported destinations follow the documented contract |
| Hostile inputs | `cargo xtask hostile-inputs`, `cargo xtask editor-hostile-inputs`, `cargo xtask fuzz-smoke` | Resource ceilings and refusal checks pass; fuzz execution has no detected crash |
| Dependency and language compatibility | `cargo xtask dependency-audit`, `cargo deny check`, `cargo audit`, Rust 1.96 workspace check | Reviewed graph, accepted licence policy, no forbidden dependency and no compiler failure |
| Reproducible source-built installation | `cargo xtask editor-install-trial` | Install, smoke test, update and rollback checks pass on the recorded host |

`cargo xtask all` includes the generated source workflow and produces the
verification manifest. Coverage-guided fuzzing and the dependency advisory
check remain separately invoked checks. The manifest identifies the exact
artifacts; it is not an independent evaluation of the project.

## Generated corpus method

`crates/nuif-testing/src/bin/source-workflow.rs` evaluates 48 cases: two HTML
profiles, sibling and grouped containment, 1/8/64/256 text entities, and three
source variants. The variants are canonical source, foreign HTML/CSS regions
with LF endings, and those regions with CRLF endings. Text includes empty
strings, escaped delimiters, combining characters, multiple scripts and emoji.
Groups contain at most eight text children.

Each case checks initial import, no-op identity, scalar edits, repeated edit
planning, a post-import fixpoint, a second edit, byte locality, structural-edit
refusal and stale-span refusal. The report records case dimensions, document
hashes, source size, checks, revision and environment. Separate CLI subprocess
tests check in-place synchronization, unsupported-edit preservation and
report-path alias rejection. Headless editor subprocess tests reject report,
script and expected-document collisions before reading or writing documents. Results are written to
`target/source-workflow-report.json` and archived by CI.

This is a generated regression corpus. Its dimensions are selected by the
implementation authors, not sampled from production documents. Export/import
agreement is vulnerable to shared implementation mistakes. It does not measure
browser visual equivalence, real-document acceptance rate or editing success
for unfamiliar users. Independent layout/text oracles in other gates provide
additional evidence only for their own fixture matrices.

## Reproduction workflow

After the [development prerequisites](../CONTRIBUTING.md) are installed, the
following sequence reproduces the editor-to-source path using the committed
responsive-card script:

```sh
cargo build --locked -p nuif-cli -p nuif-editor
mkdir -p target/beta-example
target/debug/nuif fixture v0-responsive-card target/beta-example/input.nuif.json
target/debug/nuif export target/beta-example/input.nuif.json html-css-v0 target/beta-example/source.html target/beta-example/export-report.json
target/debug/nuif-editor --headless --script conformance/fixtures/v0-responsive-card/editor-trial.jsonl --document target/beta-example/input.nuif.json --output target/beta-example/edited.nuif.json
target/debug/nuif sync html-css-v0 target/beta-example/source.html target/beta-example/edited.nuif.json target/beta-example/synchronized.html target/beta-example/sync-report.json
target/debug/nuif import html-css-v0 target/beta-example/synchronized.html target/beta-example/reimported.nuif.json target/beta-example/import-report.json
cmp target/beta-example/edited.nuif.json target/beta-example/reimported.nuif.json
```

A zero exit status from `cmp` establishes canonical byte equality for this
example. The fidelity reports remain necessary: exact model retention includes
opaque fields whose browser rendering is unsupported. The
[file-output contract](../crates/nuif-cli/README.md#output-replacement) describes
single-file replacement and its durability and concurrency limits.

## External acceptance requirements

Beta promotion additionally requires evidence outside the local generated
suite: an independently authored, permission-cleared document corpus; recorded
source import/edit/synchronization outcomes; installation and workflow trials
on each advertised operating system; and external users completing the scoped
workflow without undocumented intervention. Corpus selection, task definitions,
failures and exclusions must be reported with the outcomes. No task-success
rate or representative sample claim exists yet.

The proposed beta scope can be narrowed to the hosts and profiles with such
evidence. General interchange, neutral-standard governance and trained
reconstruction are separate decisions governed by the
[standards roadmap](STANDARDS-ROADMAP.md) and
[research experiment registry](../research/experiments/index.yaml).
