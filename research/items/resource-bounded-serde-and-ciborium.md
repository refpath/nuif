---
id: nuif:research:resource-bounded-serde-and-ciborium
kind: implementation
status: verified
title: "Bounded JSON, CBOR and stream ingestion for untrusted NUIF inputs"
source:
  url: https://docs.rs/serde_json/1.0.151/serde_json/struct.Deserializer.html
  authors: [David Tolnay]
  published_at: "serde_json 1.0.151"
  license: MIT OR Apache-2.0
retrieved_at: 2026-08-29
tags: [security, resource-limits, serde-json, ciborium, allocation-measurement]
confidence: 0.98
claims: [nuif:claim:bounded-untrusted-input]
relations:
  - type: extends
    target: nuif:research:rust-snapshot-property-fuzz-tooling
    note: Adds measured adversarial byte, depth, semantic-cardinality, elapsed-time and allocator bounds to the deterministic trial surface.
  - type: extends
    target: nuif:research:deterministic-simulation-testing
    note: Treats corrupt, over-depth and over-cardinality documents as deterministic fault injections with machine-readable outcomes.
links:
  spec: [spec/11-security.md]
  rfc: [rfcs/0009-profile-zero-resource-budgets.md]
  code: [crates/nuif-codec/src/lib.rs, crates/nuif-core/src/lib.rs, crates/nuif-testing/src/bin/hostile-inputs.rs, apps/editor/src/bin/editor-hostile-inputs.rs, xtask/src/main.rs]
  experiments: [nuif:experiment:hostile-input-budgets, nuif:experiment:editor-hostile-interactions]
---

# Summary

Parser recursion limits do not by themselves bound untrusted document work. A complete boundary must cap bytes before an input is read into memory, cap syntax nesting before or during deserialization, and cap the cardinality and retained data of the decoded semantic model before recursive validation, layout or rendering. Output writers need the same byte cap so an in-memory model cannot expand into an encoding the paired decoder refuses.

The profile-0 reference path applies all three layers. CLI and headless-editor readers stop after the first byte beyond the encoded limit. The text preflight scanner ignores quoted strings and JSON5 comments while counting containers; Ciborium uses its caller-selected recursion limit. An iterative semantic walk measures entity, relation, edge, responsive-rule, property-node, property-depth, containment-depth, string and binary totals. Validation retains at most 1,024 ordinary diagnostics plus one truncation issue. Canonical CBOR map keys are encoded once per multi-entry map before sorting so hostile wide maps cannot force repeated key re-encoding in the comparator.

## Evidence

- `serde_json::Deserializer` 1.0.151 retains a recursion limit by default. Its official `disable_recursion_limit` documentation warns that arbitrarily deep input can overflow the stack and that later recursive operations, including destruction, also require protection. Locator: `Deserializer::disable_recursion_limit`, docs.rs, retrieved 2026-08-29.
- Ciborium 0.2.2 exposes `from_reader_with_recursion_limit`; its source states that inputs beyond the selected bound return `RecursionLimitExceeded` and warns that high limits risk stack exhaustion. The default `from_reader` path uses 256. Locator: `ciborium/src/de/mod.rs`, functions `from_reader_with_buffer` and `from_reader_with_recursion_limit`, main and 0.2.2 source, retrieved 2026-08-29.
- Rust's `Read::take` returns a reader that yields at most the selected number of bytes. NUIF reads `limit + 1`, making the first excess byte distinguishable from an exactly-at-limit EOF without buffering the rest. Locator: `std::io::Read::take`, Rust 1.98 standard-library documentation, retrieved 2026-08-29.
- `stats_alloc` 0.1.10 instruments allocation, deallocation and reallocation requests and provides `Region` snapshots. NUIF measures each case in a single-threaded release binary after a fixed warmup; report metadata records toolchain, OS, architecture, CPU and available parallelism. Locator: `StatsAlloc`, `Stats`, `Region` and `INSTRUMENTED_SYSTEM`, docs.rs source, retrieved 2026-08-29.
- Executable regression: `cargo xtask hostile-inputs`. It writes `target/hostile-input-report.json`, rejects every enumerated one-over byte/depth/cardinality case with the named resource, accepts all semantic boundary classes, and fails when any case exceeds 2 seconds, 64 MiB of allocator traffic or 16 MiB retained at observation time. Core unit tests exercise one-over rejection for every public semantic limit; CI uploads the measured report.

## Mechanism

Ingestion uses a limit-plus-one reader so oversized streams cannot allocate beyond the decision point. Text receives a quote/comment-aware structural preflight; CBOR receives the same depth value through Ciborium's recursion-limit API. Deserialization is followed immediately by iterative semantic accounting. Canonical text and CBOR writers are bounded, and CBOR key encodings are cached only for multi-entry map sorting so both output growth and comparison work remain linear in retained key bytes plus sort comparisons. The isolated release runner creates adversarial inputs before each measured region, warms one fixed fixture, retains the result while sampling allocator counters, and classifies errors without including report construction in the case measurement.

## Measured calibration

The 2026-08-29 Apple Silicon release run used rustc 1.98.0 and covered oversized text, over-depth JSON/JSON5 and CBOR, every semantic cardinality class, single and total strings, total binary payload, containment depth and a 16,384-entry hostile CBOR map. Boundary cases included 8,192 entities and tokens, 4,096 roots, 32,768 relations, 16,384 responsive overrides, 8,191 valid child references, 65,536 property values, 128 containment levels, 8 MiB total strings and 8 MiB CBOR binary. The slowest observed case was below 25 ms, maximum allocator traffic was below 39 MiB, and maximum retained data was below 8.5 MiB. The automated ceilings intentionally retain substantial CI/platform margin and are rerun rather than treated as universal hardware performance claims.

## NUIF relevance

These measurements replace the earlier unsupported one-million-node and depth-1,024 hypotheses. They establish a reproducible profile-0 safety envelope, not a promise that every conforming implementation must share the reference implementation's allocator behavior. A foreign implementation may use tighter operational limits, but it must expose them and must accept the normative boundary fixtures if it claims the profile-0 conformance level.

## Open questions

- Renderer timeout and memory isolation for future image, font, path and GPU resources remains a Gate D concern; those resource classes do not yet exist in executable profile 0.
- Server deployments should add process-level cancellation and tenant quotas around the synchronous deterministic codec budgets.
