---
id: nuif:rfc:0009
kind: rfc
status: accepted
---

# RFC 0009 — Bound profile-0 ingestion and semantic resources

Status: accepted (primary evidence and calibration in `nuif:research:resource-bounded-serde-and-ciborium`).

## Motivation

The draft security chapter required bounds but carried unmeasured depth-1,024 and one-million-node examples from unrelated parsers. A byte limit alone does not prevent recursive stack exhaustion, large decoded collections, diagnostic amplification or repeated canonical-key work. Applying a codec limit only after `read_to_end` also permits the input allocation the limit is intended to prevent.

## Decision

Profile-0 decoders and encoders enforce these limits:

| Resource | Limit |
|---|---:|
| encoded document | 16 MiB |
| text/CBOR syntax depth | 64 |
| entities / tokens | 8,192 each |
| roots | 4,096 |
| relations | 32,768 |
| responsive overrides | 16,384 |
| child references | 8,191 |
| property values | 65,536 |
| property-value depth | 24 |
| containment depth | 128 |
| total retained strings | 8 MiB |
| one retained string | 1 MiB |
| total retained binary data | 8 MiB |
| binary data in `nuif-text-0` | 512 KiB |

Readers consume at most 16 MiB plus the first excess byte and reject that byte without reading the remaining stream. Text nesting is counted outside strings and JSON5 comments before deserialization. CBOR uses an explicit recursion limit. The decoded semantic walk is iterative for property values and runs before recursive validation or evaluation. A canonical writer stops before appending the first byte beyond its profile limit.

Validation retains at most 1,024 ordinary diagnostics and one `VALIDATION_DIAGNOSTICS_TRUNCATED` issue. It may continue bounded structural work after the retention cap, but it cannot amplify malformed input into an unbounded report.

The reference hostile-input trial is a release build after one fixed warmup. Each enumerated case must produce its expected acceptance/error class within 2 seconds, no more than 64 MiB of allocator traffic and no more than 16 MiB retained at observation time. Those three values are reference-CI regression ceilings rather than cross-implementation format semantics; foreign implementations report their own allocation method while accepting the normative boundary fixtures.

## Compatibility

No published profile-0 documents exist. The limits become part of the first executable profile. `nuif-text-0` has a lower binary ceiling because its canonical JSON byte arrays expand substantially; the same semantic document remains representable in `nuif-cbor-0` up to the 8 MiB model ceiling.

## Security

Bounded bytes, syntax and semantic cardinality convert parsing and validation work into finite functions of declared profile limits. They do not replace process-level cancellation, tenant quotas or sandboxing in servers. Future images, fonts, compressed packages, paths and GPU resources require separately measured limits before their profiles can be accepted.

## Conformance tests

- exactly-at-limit entity, property, containment, string and CBOR-binary cases are accepted;
- the first byte, syntax level or semantic item over each tested limit returns `ResourceLimit` naming the exceeded resource;
- strings, escapes and JSON5 comments do not affect the text depth count;
- a wide canonical CBOR map completes within the allocator/time ceiling and malformed document shape remains classified separately;
- CLI and editor readers retain at most the selected limit plus one byte;
- validation output is capped and ends with the truncation diagnostic;
- `cargo xtask hostile-inputs` writes a machine report with the cases, limits, measurements, toolchain, warmup, allocator method and platform.

## Implementation

`nuif-core::resource_usage` owns semantic accounting. `nuif-codec` owns encoded, syntax and encoding-specific bounds. The CLI and headless editor bound reads before passing bytes to the codecs. `nuif-testing` uses an instrumented system allocator in an isolated single-threaded binary, and CI uploads its JSON report.

## Rejected alternatives

- Keep depth 1,024 and one million nodes: unsupported by measurements and unsafe for recursive downstream layout.
- Enforce only encoded bytes: shallow compact inputs can still create excessive semantic nodes or diagnostic output.
- Apply the limit after `fs::read`/`read_to_end`: too late to bound ingestion allocation.
- Use wall-clock timeout as the sole guard: nondeterministic across machines and incapable of preventing memory amplification.
- Share an 8 MiB binary limit between text and CBOR: canonical text expansion can exceed the encoded limit and allocate a very large generic JSON value tree.
