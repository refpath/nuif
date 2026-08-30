---
id: nuif:research:dependency-and-subsystem-audit
kind: synthesis
status: verified
title: Direct dependency and implementation-subsystem alternatives audit
source:
  url: https://doc.rust-lang.org/cargo/commands/cargo-metadata.html
  repository: https://github.com/refpath/nuif
  authors: [Rust project contributors, Serde contributors, Tree-sitter contributors, Servo contributors, Linebender contributors, Google Fonts contributors, RustCrypto contributors]
  published_at: "dependency releases and upstream manifests current on 2026-08-30"
  license: "mixed permissive upstream licenses; NUIF Apache-2.0 OR MIT"
retrieved_at: 2026-08-30
tags: [rust, dependency, architecture, parser, adapter, layout, rendering, text, editor, benchmark, security]
confidence: 0.93
claims: [nuif:claim:sync-not-regenerate, nuif:claim:semantic-automation, nuif:claim:bounded-untrusted-input]
relations:
  - type: depends_on
    target: nuif:research:cargo-workspace-xtask-and-ci-layout
    note: Cargo metadata is the machine-readable source for the direct dependency inventory.
  - type: extends
    target: nuif:research:masonry-editor-stack-decision
    note: The editor stack comparison is incorporated as one subsystem in the repository-wide audit.
  - type: compares_to
    target: nuif:research:rust-snapshot-property-fuzz-tooling
    note: Benchmark and allocation tools are compared as part of the verification subsystem.
  - type: related_to
    target: nuif:research:macos-metal-block-future-incompatibility
    note: The audit includes the reviewed editor-toolkit fork chain that removed metal-rs, rust-block, rustybuzz and its transitive ttf-parser copy.
links:
  spec: [spec/08-serialization.md, spec/09-provenance-and-fidelity.md, spec/11-security.md]
  adr: [adrs/0002-layout-engine.md, adrs/0003-reference-renderer.md, adrs/0006-rust-native-editor.md]
  rfc: [rfcs/0009-profile-zero-resource-budgets.md]
  code: [Cargo.toml, Cargo.lock, dependencies/index.json, xtask/src/main.rs]
  experiments: [nuif:experiment:profile-zero-performance]
---

# Summary

Cargo metadata reports 29 distinct direct external crates across the workspace.
Each is now registered with a role, a current decision, at least one considered
alternative and repository evidence. The executable audit fails when a direct
crate is added without ownership or when a stale registration remains. Cargo
Deny separately gates advisories, duplicate-version bans, licences and sources;
the two checks answer different questions.

The critical result is not a wholesale dependency replacement. NUIF's unusual
requirements—canonical bytes, exact source correspondence, independent layout
and raster oracles, bounded hostile inputs and a semantically queryable native
editor—make several generally faster or broader libraries worse fits at the
actual boundary. Four version lines warranted immediate compatibility trials:
`json5` 0.4 to 1.3, `sha2` 0.10 to 0.11, `font-test-data` 0.7 to 0.9 and
Tree-sitter 0.26.10 to 0.26.13. The complete tests accepted the JSON5, font-data
and Tree-sitter updates without changing canonical fixtures, hostile-input
classification, pinned font hashes or adapter source spans. SHA-2 0.11 removed
the digest output's hexadecimal formatting implementation; NUIF retains 0.10
because 0.11 provides no required fix or measured benefit that justifies
duplicating a hex adapter across the report-producing crates.

## Evidence

- `cargo metadata --locked --format-version 1` identifies workspace members,
  their direct dependency declarations and the exact resolved graph. The Cargo
  reference defines the JSON output as stable when consumers ignore unknown
  fields. Locator: Cargo `cargo metadata` documentation, retrieved 2026-08-30:
  https://doc.rust-lang.org/cargo/commands/cargo-metadata.html.
- `cargo search` on 2026-08-30 reports current stable lines for the registered
  crates.io dependencies. NUIF is already on the current stable line for Ciborium,
  Criterion, Harfrust, PNG, RFD, roxmltree, serde, serde-bytes, serde-json,
  Skrifa, stats_alloc, Taffy, thiserror, tracing, the HTML and CSS grammars, and
  Zeno. The version-trial candidates are listed in the summary. The three Masonry
  packages are full-SHA Git dependencies and are therefore evaluated as one
  forked toolkit boundary rather than by crates.io maximum version.
- `cargo deny check` passes after the reviewed Xilem and UI Events fork pins.
  `rustybuzz`, metal-rs and rust-block remain absent from the active editor
  graph. `ttf-parser` was later reintroduced only as the bounded `nuif-font`
  package parser, not through the editor toolkit fork. The complete check is a
  CI and release gate rather than a documented exception.
- The version trial ran all workspace unit and documentation tests, the release
  hostile-input allocation profile, text and render goldens, all six executable
  adapter profiles, and workspace Clippy with warnings denied on rustc 1.98.0.
  The three accepted updates passed; SHA-2 0.11 failed at compile time before
  runtime evidence and was reverted (2026-08-30).

## Mechanism

The register maps each direct crate to the subsystem boundary it serves. The
comparison is made at that boundary, so an alternative is accepted only when it
preserves the same observable contract and improves a measured workload.

## Subsystem comparisons

Serialization and hashing

- Serde explicitly separates data structures from format implementations
  through its 29-type data model. This is useful here because the model derives
  typed traversal while NUIF retains control of canonical JSON5 and CBOR output.
  Serde is not a parser and therefore does not weaken the format-specific
  resource checks. Locators: https://serde.rs/data-model.html and
  https://serde.rs/data-format.html, retrieved 2026-08-30.
- Ciborium remains preferable to Minicbor for profile zero because the current
  bounded decoder needs serde's generic logical-value distinctions before its
  own deterministic-order validation. A schema-specific Minicbor profile may be
  worthwhile only after a benchmark demonstrates an end-to-end gain without
  changing unknown-value preservation.
- `json5` 1.0 replaced its Pest grammar with a handwritten parser and 1.1–1.3
  added wide integer and UTF-16 surrogate-pair support. This is a semantic parser
  change, so a major-version update is acceptable only with the complete codec
  and hostile-input suites. Locators: release notes and comparison, retrieved
  2026-08-30: https://github.com/callum-oakley/json5-rs/releases and
  https://github.com/callum-oakley/json5-rs/compare/0.4.1...1.3.1.
- SHA-256 is retained over BLAKE3 because NUIF hashes are exchanged with browser,
  release and independent Python tooling, not used as a high-throughput internal
  hash table. RustCrypto 0.11 changes to Digest 0.11, edition 2024 and newtype
  hash implementations; its MSRV 1.85 is below NUIF's. Locator:
  https://github.com/RustCrypto/hashes/blob/master/sha2/CHANGELOG.md, retrieved
  2026-08-30.

Source adapters

- Tree-sitter supplies concrete nodes, byte offsets, edit descriptions and
  incremental reparsing. `html5ever` is the right browser-grade oracle for full
  WHATWG error correction, but it mutates a caller-supplied tree through
  callbacks and does not provide a concrete source tree. Normalizing and
  reserializing a DOM is incompatible with the byte-complement preservation
  postcondition of the declared adapter. Locators:
  https://tree-sitter.github.io/tree-sitter/using-parsers/ and
  https://github.com/servo/html5ever, retrieved 2026-08-30.
- The official Tree-sitter JavaScript grammar includes JSX in the same concrete
  syntax tree and therefore extends the existing retained-byte contract to the
  static React profile. SWC and Oxc are stronger choices for semantic JavaScript
  transforms, but their normalized AST boundary does not improve a profile that
  explicitly refuses evaluation and patches exact source ranges. Locator:
  https://github.com/tree-sitter/tree-sitter-javascript, retrieved 2026-08-30.
- roxmltree represents XML as a read-only tree and exposes original byte
  positions. Quick XML is an almost-zero-copy pull parser and a better candidate
  for very large streaming documents, while usvg is a better renderer-facing
  normalized SVG model. The current bounded SVG profile needs a small complete
  tree plus exact attribute/text ranges; neither alternative improves that
  contract. Locators: https://docs.rs/roxmltree/0.21.1/roxmltree/ and
  https://github.com/tafia/quick-xml, retrieved 2026-08-30.

Layout, text and rendering

- The NUIF layout kernel remains implementation-owned, with Taffy and browser
  engines as independent differential oracles. Substituting Taffy into the
  reference would erase one of the two implementations being compared; Yoga
  has the same self-oracle problem and adds a foreign-function boundary.
- Harfrust and Skrifa remain the shaping/outline stack. The separate narrow
  package-font profile uses `ttf-parser` behind NUIF-owned sfnt/checksum checks
  and compares its results with Skrifa, avoiding a single-library oracle.
  Fontations describes `read-fonts` as a no-allocation, no-copy parser suitable
  for shaping and subjects the stack to OSS-Fuzz. `ttf-parser` likewise forbids
  unsafe code and allocations in its parser API. Locators:
  https://github.com/googlefonts/fontations and
  https://github.com/harfbuzz/ttf-parser, retrieved 2026-08-30.
- The `png` crate owns the production RGBA8 decoding path; test-only `zune-png`
  supplies independent exact-pixel evidence with unsafe paths disabled and
  integrity checks enabled. Replacing production decoding with the oracle would
  erase that differential boundary.
- Zeno stays the small deterministic glyph-mask rasterizer. Tiny-skia or resvg
  would add broader path and SVG behavior that is outside the current reference
  commands, while Vello remains the interactive renderer and its CPU path is a
  non-normative visual-harness oracle.

Editor and verification

- Masonry remains the only compared editor toolkit that combines a retained
  widget tree, AccessKit semantics and a same-tree CPU visual harness. Its
  alpha churn is contained behind full-SHA `refpath/xilem` and
  `refpath/ui-events` pins. The fork changes dependencies and safe API call sites;
  it contains no new foreign-interface implementation.
- RFD 0.17.2 offers synchronous and asynchronous native dialogs on Windows,
  macOS, Linux/BSD and asynchronous web dialogs. NUIF uses only the synchronous
  desktop path and keeps parsing, fidelity reporting and filesystem writes in
  editor-owned code. Per-platform APIs or Linux `ashpd` would increase platform
  code without changing the user-visible contract. Locator:
  https://docs.rs/rfd/0.17.2/rfd/, retrieved 2026-08-30.
- Criterion is retained for controlled same-machine statistical comparisons;
  the release smoke profile records portable latency and allocation ceilings.
  Divan is a simpler runner and iai-callgrind gives stable Linux instruction
  counts, but neither replaces both current roles.
- thiserror produces standard typed errors, while `anyhow` would erase the error
  categories asserted by conformance tests. Tracing provides structured events
  and spans and is already the toolkit's diagnostics vocabulary. Locators:
  https://docs.rs/thiserror/2/thiserror/ and
  https://docs.rs/tracing/0.1.44/tracing/, retrieved 2026-08-30.

## NUIF relevance

This audit turns dependency choice into checked repository state. It also keeps
implementation libraries separate from the independent engines and formats
used as conformance oracles.

## Decision boundary

**Retain** focused libraries whose data model directly matches a tested NUIF
boundary. Performance claims require a profile workload, not a microbenchmark
from another project.

**Fork** only the native toolkit chain, at full commits, while the reviewed wgpu
and dependency-feature fixes are absent from the selected upstream revision.

**Watch** RFD because native-dialog behavior is platform-owned and must be
covered by release-platform smoke tests. It is not allowed to own document I/O.

**Reject** substituting an oracle into the implementation it verifies, whole
source regeneration in a retentive adapter, or a broad framework solely because
it advertises more format coverage.

## Open questions

- Whether future JSON5 grammar expansion needs an explicit accepted-source
  corpus in addition to the existing canonical, malformed, non-finite, byte and
  depth cases.
- Whether collaboration convergence should gain a portable allocation ceiling;
  adapter import/export/synchronization now runs inside the allocation-aware
  smoke profile after its Criterion fixtures were calibrated.
- Whether a future Masonry release incorporates the refpath wgpu, resvg/usvg and
  UI Events feature corrections, allowing both fork pins to be removed.
