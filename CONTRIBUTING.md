# Contributing

NUIF accepts research, specification, conformance, implementation and adapter contributions.

## Research contributions

Add a stable research record under `research/items/` using the repository schema. Prefer primary sources; include retrieval date, source version/commit where available, confidence, claims and explicit graph relationships. Do not silently replace conflicting evidence.

## Specification contributions

Use an RFC for semantic/protocol changes. Every new normative behavior should include or identify a conformance fixture. Vendor-specific behavior belongs in an adapter or extension unless it demonstrates a broadly portable primitive.

## Code contributions

Keep the Rust workspace formatted and warning-free. Core crates must remain independent of editor UI and vendor adapters. New parsing/rendering paths must document untrusted-input/resource-limit considerations.

## Commits

Use focused conventional-style commit messages such as `docs:`, `research:`, `spec:`, `feat:`, `fix:` and `test:`.
