# Contributing

NUIF accepts research, specification, conformance, implementation and adapter contributions.

## Research contributions

Add a stable research record under `research/items/` using the repository schema. Prefer primary sources; include retrieval date, source version/commit where available, confidence, claims and explicit graph relationships. Do not silently replace conflicting evidence.

## Specification contributions

Use an RFC for semantic/protocol changes. Every new normative behavior should include or identify a conformance fixture. Vendor-specific behavior belongs in an adapter or extension unless it demonstrates a broadly portable primitive.

## Code contributions

Keep the Rust workspace formatted and warning-free. Core crates must remain independent of editor UI and vendor adapters. New parsing/rendering paths must document untrusted-input/resource-limit considerations.

## Writing register

Persisted prose uses established technical terminology, complete grammar and
direct statements. Use third-person descriptions for research and specification
claims; imperative steps are appropriate for operational instructions. Remove
marketing language, metaphorical names for established concepts, filler and
repeated claims. Preserve qualifiers, identifiers, versions, numbers and units.
Expand unfamiliar acronyms at first use. Use parallel lists for enumerations
and tables for comparable facts. Follow BCP 14 semantics for requirement words
in normative specification text.

Every non-obvious factual claim needs a locator: a source URL with a section,
page or version, or a repository path, revision or executable fixture. Research
records also retain retrieval dates. Separate source statements from repository
interpretation and proposed behavior from observed results. State uncertainty
explicitly; comparisons require a metric and source. Claims of completeness or
readiness require evidence within the declared profile. Specification status
follows `GOVERNANCE.md`.

The [terminology reference](.agents/skills/research-register/references/terminology.md)
lists terms for the model and implementation. The
[research-register skill](.agents/skills/research-register/SKILL.md) provides the
editing and review workflow. Agents select it automatically when relevant.

## Commits

A commit message is exactly one line: `<type>: <subject>` with `type` in `docs`,
`research`, `spec`, `rfc`, `adr`, `feat`, `fix`, `test`, `refactor`, `perf`, `build`,
`ci`, `chore`; imperative mood; lower-case first letter; no trailing period; at
most 72 characters. No scope parentheses, breaking-change marker, emoji, ticket
numbers, URLs, body, trailers or attribution of tools, models or assistants.
The optional local hook (`.githooks/commit-msg`) is enabled with
`git config core.hooksPath .githooks`; CI runs `tools/git/commit-lint.sh`.

Run `tools/research/validate.sh` after editing `research/`.
