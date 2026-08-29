# Contributing

NUIF accepts research, specification, conformance, implementation and adapter contributions.

## Research contributions

Add a stable research record under `research/items/` using the repository schema. Prefer primary sources; include retrieval date, source version/commit where available, confidence, claims and explicit graph relationships. Do not silently replace conflicting evidence.

## Specification contributions

Use an RFC for semantic/protocol changes. Every new normative behavior should include or identify a conformance fixture. Vendor-specific behavior belongs in an adapter or extension unless it demonstrates a broadly portable primitive.

## Code contributions

Keep the Rust workspace formatted and warning-free. Core crates must remain independent of editor UI and vendor adapters. New parsing/rendering paths must document untrusted-input/resource-limit considerations.

## Writing register

All persisted prose (documents, specification, research records, comments) uses the technical register defined in `.claude/skills/research-register/SKILL.md`: established terminology from the source field, no marketing language, no invented names for known concepts, a locator for every non-obvious claim. The glossary in `.claude/skills/research-register/references/terminology.md` lists preferred terms.

## Commits

A commit message is exactly one line: `<type>: <subject>` with `type` in `docs`, `research`, `spec`, `rfc`, `adr`, `feat`, `fix`, `test`, `refactor`, `perf`, `build`, `ci`, `chore`; imperative mood; lower-case first letter; no trailing period; at most 72 characters. No body, no trailers and no attribution of tools, models or assistants. Enable the local hook with `git config core.hooksPath .githooks`; CI runs the same check (`tools/git/commit-lint.sh`).

Run `tools/research/validate.sh` after editing `research/`.
