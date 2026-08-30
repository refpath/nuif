---
id: nuif:research:serde-saphyr-yaml-parser
kind: implementation
status: verified
title: Bounded typed YAML parsing with serde-saphyr
source:
  url: https://github.com/bourumir-wyngs/serde-saphyr
  authors: [serde-saphyr contributors]
  published_at: "serde-saphyr 1.1.0, 2026-08-15"
  license: MIT OR Apache-2.0
retrieved_at: 2026-08-30
tags: [rust, yaml, serde, parser, resource-limits, documentation]
confidence: 0.98
claims: [nuif:claim:bounded-untrusted-input]
relations:
  - type: extends
    target: nuif:research:resource-bounded-serde-and-ciborium
    note: The documentation compiler applies the same bounded-input rule to YAML metadata.
links:
  spec: []
  adr: []
  rfc: []
  code: [xtask/Cargo.toml, xtask/src/main.rs]
  experiments: []
---

# Summary

serde-saphyr 1.1.0 deserializes YAML directly into Serde types. It rejects
duplicate keys by default and exposes configurable resource budgets for input
size, nesting, collections, anchors, aliases and parser lookahead. The crate can
be compiled with only its deserialization feature. These properties fit a
small, read-only metadata boundary better than the deprecated `serde_yaml`
crate or forks that retain an unsafe LibYAML binding.

## Evidence

- The 1.1.0 release added limits for buffered comment events, simple-key
  lookahead and flow nesting. Locator: repository release `1.1.0`, commit
  `ad5c614`, retrieved 2026-08-30.
- The project documents conservative default budgets and incremental
  reader-based parsing for resource-exhaustion control. Locator: repository
  `README.md`, "Pathological inputs & budgets", retrieved 2026-08-30.
- Duplicate keys produce an error by default. First-wins and last-wins policies
  require an explicit option. Locator: repository `README.md`, "Duplicate
  keys", retrieved 2026-08-30.
- `serde_json::Value` is supported for untyped data, while direct typed
  deserialization rejects values that do not match the destination type.
  Locator: docs.rs crate page for 1.1.0, "Overview" and "Notable features",
  retrieved 2026-08-30.
- The package declares Rust edition 2024, dual MIT or Apache-2.0 licensing and
  independent `serialize` and `deserialize` features. Locator: docs.rs
  `Cargo.toml.orig` for 1.1.0, retrieved 2026-08-30.

## Mechanism

The documentation compiler caps each source file before parsing, extracts only
the initial frontmatter block and deserializes that block into a closed metadata
structure. serde-saphyr applies its default syntax budgets and duplicate-key
policy. File inclusion, serialization and untyped tag-driven construction are
not enabled.

## NUIF relevance

**Borrow** typed, budgeted deserialization with default duplicate-key
rejection. Pin 1.1.0 with default features disabled and only `deserialize`
enabled.

**Reject** YAML as a canonical NUIF interchange encoding. It is limited to
repository-authored metadata for the documentation and research toolchain.

**Reject** `serde_yaml`, which is deprecated, and YAML forks backed by an
unmaintained unsafe LibYAML binding. The documentation compiler does not need
their serialization compatibility.

## Open questions

- Parser budget defaults require a regression fixture before metadata is
  accepted from untrusted pull requests at larger scale.
- YAML 1.1 boolean inference requires quoted strings or strict options if a
  future metadata field accepts arbitrary scalar values.
