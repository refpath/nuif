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

The reference in-process SDK profile is a byte-oriented façade over the
canonical codecs, package and operation implementation. Bare text/CBOR loading
MUST name its encoding; package loading MUST run the complete package/resource
validation path. A loaded package MUST retain verified embedded resources and
descriptors across neighboring semantic edits. Export to a portable mode MUST
rerun the target mode's resource policy.

Language and process wrappers MAY add transport limits, ownership conversion
and host authorization. They MUST NOT copy the semantic model or independently
implement validation, canonicalization, hashing, package policy or operation
application. Equivalent wrapper calls over the declared common subset MUST
produce the same canonical bytes, hash and diagnostics as the direct SDK.

A package-aware wrapper MUST distinguish structural load from full-support
negotiation. Structural load MAY preserve or inspect unknown required
capabilities, but MUST NOT execute them or claim support. Before evaluation it
MUST compare the manifest requirements with an explicit bounded host set and
fail with the exact unavailable identifiers. A package-preserving wrapper MUST
produce the same deterministic archive bytes as the direct SDK for the same
document, resources, capabilities and target mode.

When any requirement is unavailable, a package-aware SDK or wrapper MUST also
reject semantic mutation, undo/redo, changed package saves and package-mode
conversion atomically. It MAY validate, hash, extract a bare document or copy
the unchanged same-mode package. Successful complete-set negotiation MAY
authorize mutation and evaluation for that loaded session; a failed partial
negotiation MUST NOT do so.

An editor that lacks any required package capability MUST treat the package as
read-only unless a capability-specific authoring profile defines how every
affected resource is updated or explicitly detached. Structural selection,
inspection and exact package copying MAY remain available. A semantic mutation
or changed save MUST fail atomically with the unavailable requirement set;
silently carrying opaque resources onto a new document revision is forbidden.

A future C ABI is a separate versioned profile. It MUST define opaque-handle
lifetime, byte-buffer ownership and release, panic containment, stable error
classes, threading, calling convention and exported-symbol compatibility. C,
Swift or Kotlin bindings are not claimed merely because a shared library or
generated header compiles. Native consumer tests and platform packages are
required before integration status.

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
