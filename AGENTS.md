# NUIF repository contract

This file is the canonical repository contract for agents. Contribution rules,
including the writing register and commit format, belong in
[CONTRIBUTING.md](CONTRIBUTING.md). Specification status and decision procedures
belong in [GOVERNANCE.md](GOVERNANCE.md).

## Context

Read [README.md](README.md) and [docs/roadmap.md](docs/roadmap.md), then the code
and guidance that own the affected mechanism. Use `docs/whitepaper/` for
architectural motivation, `spec/` for draft interchange semantics, `rfcs/` for
proposals, `adrs/` for implementation decisions, and `conformance/PLAN.md` for
test profiles. Research evidence lives in `research/`; Rust implementations in
`crates/`; the reference editor in `apps/editor/`; repository tooling in
`tools/` and `xtask/`.

## Implementation boundaries

- Core crates remain independent of the editor and vendor adapters. Every
  editor gesture lowers to semantic protocol operations before mutation.
- Normative interchange text belongs in `spec/`. Research and whitepapers
  motivate proposals; they do not define semantics. Use the maturity boundaries
  in `GOVERNANCE.md` rather than describing NUIF as a published standard.
- The CLI and in-process API are the primary test surfaces. GUI automation
  verifies shell wiring, focus and pointer/keyboard behavior. For editor work,
  read [UI-SPEC.md](apps/editor/UI-SPEC.md),
  [ARCHITECTURE.md](apps/editor/ARCHITECTURE.md) and [QA.md](apps/editor/QA.md).
- Keep changes within the request and the editor's declared scope. Additions
  to that scope follow the existing RFC process.
- Import, export and lowering preserve unknown data where supported and emit
  explicit fidelity records for loss. Profile targets and declared capabilities
  do not establish implemented behavior; inspect the code and fixtures.
- Use the Rust toolchain pinned in `rust-toolchain.toml`. Follow
  [ADR 0006](adrs/0006-rust-native-editor.md) for the toolchain and MSRV policy.
  Preserve workspace resolver 3, the Clippy pedantic checks and `deny.toml`
  license policy. Core and SDK crates forbid unsafe code; the explicit C ABI
  exception belongs in `nuif-ffi`, as defined in
  [the binding boundary](docs/SDK-AND-BINDINGS.md#c-c-swift-and-kotlin-decision).

## Work and delivery

- Inspect Git state before editing. Preserve unrelated changes and local
  client settings; stage only the intended files.
- Commit or push only when requested. An existing request is sufficient
  authorization. Never force-push `main`, rewrite published history or amend
  another author's commit. Use the commit format in `CONTRIBUTING.md` and one
  logical change per commit.
- For Rust changes, run `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets --locked -- -D warnings` and
  `cargo test --workspace --locked`. For research changes, use the validator
  specified in `CONTRIBUTING.md`. For documentation changes, use
  [the documentation checks](docs/PUBLISHING.md#local-commands). Select additional
  conformance checks for the affected mechanism rather than unrelated suites.
- Report checks actually run, observed results and material limitations.
  Distinguish source declarations, planned work, local test evidence and
  independent review or reproduction. Text searches locate candidates for
  inspection; they do not prove semantic correctness.
- Keep chat concise, grammatical and precise. Preserve qualifiers, identifiers,
  units, versions, commands and error strings. State uncertainty and next actions
  directly; omit filler and repeated summaries.

## Skills and discovery

Shared skills live in tracked `.agents/skills/<name>/SKILL.md`. The AI selects
and applies them automatically from their descriptions when the task, changed
mechanism or delivery stage requires them. Keep implicit invocation enabled;
never ask the user to invoke a skill or choose a slash command. Load supporting
references only when relevant.

Codex discovers `.agents/skills` natively. Claude Code discovers project skills
under `.claude/skills`; expose each shared skill there with an ignored individual
directory symlink to `../../.agents/skills/<name>`. Keep `.claude/skills` as a real
directory, preserve local entries and repair only repository-owned dangling
links. The entire `.claude/` tree is ignored local client state.

Clients without native instruction discovery must read this file themselves.
Clients without native skill discovery must inspect the descriptions in
`.agents/skills/*/SKILL.md`, select matching skills and read their instructions
before the relevant work. Reassess skill selection when the task or delivery
stage changes.
