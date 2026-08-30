---
id: nuif:spec:security
kind: specification
status: draft
---

# 11 — Security and resource limits

Status: draft.

NUIF documents and extensions are untrusted input.

Implementations MUST bound decoded sizes, nesting depth, entity/relation counts, path segment counts, image/font sizes, decompression ratios and renderer resource allocations. Cyclic references MUST be detected where forbidden.

## Executable profile-0 limits

An implementation claiming executable profile-0 conformance MUST accept values at these boundaries and MUST reject the first value above them as a resource-limit error:

| Resource | Limit |
|---|---:|
| encoded document bytes | 16 MiB |
| text or CBOR syntax depth | 64 |
| entities | 8,192 |
| roots | 4,096 |
| tokens | 8,192 |
| relations | 32,768 |
| child references | 8,191 |
| responsive overrides | 16,384 |
| property values | 65,536 |
| property-value depth | 24 |
| containment depth | 128 |
| total retained string bytes | 8 MiB |
| bytes in one string | 1 MiB |
| total retained binary bytes | 8 MiB |
| retained binary bytes in `nuif-text-0` | 512 KiB |

Stream readers MUST stop after reading the first byte beyond the encoded limit; reading an entire larger stream and checking afterward is non-conforming. Syntax depth MUST be checked outside quoted strings and comments. Semantic limits MUST be checked before recursive evaluation. Encoders MUST stop before producing the first byte beyond the encoded limit.

Validators MUST retain no more than 1,024 ordinary diagnostics plus one explicit truncation diagnostic. Implementations MAY use tighter operational limits when they are not making a profile-0 conformance claim, but MUST expose those limits to automation.

The reference conformance run measures each boundary and one-over case in a warmed release process. Its regression ceilings are 2 seconds, 64 MiB of allocator traffic and 16 MiB retained per case. These are reference-implementation CI ceilings rather than portable format semantics; the report MUST identify its allocator method, toolchain, build profile and hardware context.

Fonts, images, SVG/imported data, adapters and plugins require sandbox-aware handling. Script/data-binding extensions are non-core and MUST NOT execute merely by opening a document.

Package readers MUST reject duplicate/traversal/absolute/backslash paths,
symlinks, directory entries, encryption, split archives, unsupported
compression and inconsistent local/central metadata. Implementations MUST
verify declared resource size and digest before image/font/media decoding and
MUST NOT extract untrusted members to a filesystem.

Loading a package MUST NOT initiate network access. Linked resources require an
explicit caller-supplied resolver and exact digest verification. Resource
locators and provenance MUST NOT carry cookies, authorization values or other
credentials.

Observation providers and model output are untrusted inputs. Reconstruction
profiles MUST bound screenshots, observations, candidates, operations,
iterations, model/tool calls, renders, time, memory and GPU use. Generated URLs
and scripts are inert. Text visible in an image is input data and cannot alter
tool authority, security policy or operation grammar.

Screenshot/capture records can contain personal, credential or proprietary
information. Retention, remote inference, telemetry and training are separate
purposes requiring explicit policy. Private/authenticated captures MUST default
to no training.

Headless rendering MUST expose deterministic timeout/memory/resource budgets. GPU failures must not compromise process memory safety.

Image, font, compressed-package, path-segment and GPU budgets are not part of executable profile 0. A later profile MUST calibrate and publish them before adding those resource classes to its conformance claim.
