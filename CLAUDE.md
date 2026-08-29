# NUIF maintainer rules

NUIF (Neutral User Interface Format) is a research project and reference implementation for a vendor-neutral authored user-interface document model. Read `README.md`, then `docs/whitepaper/`, `spec/`, `conformance/PLAN.md` and `apps/editor/` before changing anything. `docs/roadmap.md` states the phase gates.

## Hard rules

1. Commit messages: one line, `<type>: <subject>`, imperative, lower-case, no body, no trailers, no co-author or tool attribution of any kind. Skill: `git-commit`. Enforced by `.githooks/commit-msg` and CI.
2. Never commit or push unless asked. Never force-push `main`.
3. Persisted prose follows the research register: established terminology, no marketing language, no invented brand terms, no AI writing patterns, locators for every non-obvious claim. Skill: `research-register`; glossary in `.claude/skills/research-register/references/terminology.md`.
4. Chat replies follow `terse-chat`.
5. Research records in `research/items/` validate against `research/schema/research-item.schema.json` (`kind` and `status` from the enumerations). Run `tools/research/validate.sh` after editing `research/`.
6. Normative text lives only in `spec/`; research and whitepapers motivate, they do not define. Do not call NUIF a standard.
7. Core crates stay independent of the editor and of vendor adapters. Every editor gesture lowers to protocol operations; the CLI/API is the primary test surface, GUI automation is supplementary.
8. Rust: toolchain pinned in `rust-toolchain.toml` (latest stable), `rust-version` = toolchain − 2 or the highest dependency MSRV (ADR 0006), `resolver = "3"`, `unsafe_code = "forbid"` in `crates/`, clippy pedantic clean, `cargo fmt` clean, `deny.toml` licence allow-list.
9. Do not add features, files or dependencies beyond the request. Do not add "nice to have" UI to the test editor; its scope is fixed in `apps/editor/UI-SPEC.md`.
10. Fidelity loss is never silent: import, export, lowering and migration paths emit fidelity records.

## Layout

- `research/` evidence corpus (items, claims, questions, experiments, coverage, index)
- `docs/whitepaper/` synthesis; `docs/roadmap.md` phases
- `spec/` draft normative modules; `rfcs/` proposals; `adrs/` implementation decisions
- `crates/` Rust workspace seams; `apps/editor/` reference test editor; `conformance/` suites and fixtures; `tools/` repository tooling
