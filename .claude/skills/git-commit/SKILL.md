---
name: git-commit
description: Hard rules for commit messages and git operations in this repository. Single-line conventional subject, no body, no trailers, no co-author lines of any kind. Use before every `git commit`, when reviewing commit history, or when asked about commit conventions.
user-invocable: true
---

# Git commit rules

These rules are enforced by `.githooks/commit-msg` and by the `commit-lint` CI job (`tools/git/commit-lint.sh`). A violating commit is rejected locally and fails CI.

## Message

Exactly one line:

```
<type>: <subject>
```

- `type` is one of: `docs`, `research`, `spec`, `rfc`, `adr`, `feat`, `fix`, `test`, `refactor`, `perf`, `build`, `ci`, `chore`.
- `subject` is imperative mood, lower-case first letter, no trailing period, at most 72 characters for the whole line.
- No scope parentheses, no `!`, no emoji, no ticket numbers in the subject.
- No body. No blank line followed by text. No trailers (`Co-Authored-By`, `Signed-off-by`, `Generated with`, `Claude-Session`, `Reviewed-by`). No URLs.
- Attribution of tooling, models or assistants in commit messages is prohibited. Authorship is the git author only.

Examples from history: `research: close founding coverage gaps`, `fix: clarify query must-use contracts`, `docs: define foundational standard architecture`.

## Operations

- One logical change per commit. Split unrelated edits.
- Commit only when the maintainer asks. Never push unless asked. Never force-push `main`. Never rewrite published history.
- Never amend a commit authored by someone else.
- Do not stage `target/`, `node_modules/`, build output or editor state.
- Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` before a commit that touches Rust.
- Run the research validator (`tools/research/validate.sh`) before a commit that touches `research/`.

## Hook installation

```
git config core.hooksPath .githooks
```
