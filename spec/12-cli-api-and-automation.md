---
id: nuif:spec:cli-api-and-automation
kind: specification
status: draft
---

# 12 — CLI, API and automation surface

Status: draft.

A conforming reference implementation MUST expose semantic operations without GUI automation.

Required command classes: `inspect`, `query`, `validate`, `canonicalize`, `diff`, `patch`, `render`, `layout`, `snapshot`, `migrate`, `capabilities`, `replay`, `import` and `export`.

Every command MUST support machine-readable output and stable diagnostic codes. Headless commands SHOULD accept stdin/stdout for pipeline use.

The editor MUST route mutations through the same operation layer available to CLI/API clients. AI/MCP adapters are optional clients of this interface and are never the canonical protocol.

The experimental `nuif-mcp-tools-0` profile is a stateless, stdio-only process
adapter for MCP `2026-07-28`. It exposes `validate`, `inspect`, `canonicalize`
and atomic `apply_patch` as pure inline-text transforms over the authoritative
core. It MUST NOT infer filesystem paths, retain a hidden document session, or
gain network, credential, package-resource or host-product authority. Every
request is independently bounded and carries current protocol metadata; a
client does not perform the retired initialization handshake. MCP tool
annotations describe side effects but do not grant authority.

Capture and reconstruction systems are also optional clients. They MUST submit
bounded typed transactions through the ordinary operation API, and MUST receive
the same validation, stale-revision, atomicity and diagnostic behavior as the
CLI/editor. A model/provider cannot gain direct mutation access to internal
document structs.

An automation surface supporting reconstruction SHOULD expose distinct commands
or calls for `observe`, `propose`, `evaluate` and `correct`. Every call records
input/output hashes, budgets, provider artifact identity and machine-readable
diagnostics. Model weights, low-rank adapters, processors and training data are
operational artifacts outside the NUIF document and core conformance profile.
