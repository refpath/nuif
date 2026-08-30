---
id: nuif:research:mcp-stateless-agent-adapter
kind: synthesis
status: verified
title: Stateless MCP agent adapter over the authoritative NUIF core
source:
  url: https://blog.modelcontextprotocol.io/posts/2026-07-28/
  repository: https://github.com/modelcontextprotocol/rust-sdk
  authors: [Model Context Protocol contributors]
  published_at: "MCP specification 2026-07-28 and rmcp 3.1.4, reviewed 2026-08-30"
  license: "Specification documentation terms and Apache-2.0 Rust SDK"
retrieved_at: 2026-08-30
tags: [mcp, agents, json-rpc, stdio, rust, security, stateless, adapters]
confidence: 0.96
claims: [nuif:claim:semantic-automation, nuif:claim:bounded-untrusted-input]
relations:
  - type: extends
    target: nuif:research:wasm-headless-execution
    note: MCP is another thin environment adapter over the same core, but targets external agent processes rather than browser sandboxes.
  - type: related_to
    target: nuif:research:dependency-and-subsystem-audit
    note: The official SDK and Tokio runtime are evaluated at the protocol-shell boundary.
links:
  spec: [spec/11-security.md, spec/12-cli-api-and-automation.md]
  adr: []
  rfc: []
  code: [crates/nuif-mcp, crates/nuif-api, crates/nuif-protocol]
  experiments: [nuif:experiment:mcp-cross-surface]
---

# Summary

MCP 2026-07-28 replaced its protocol handshake and hidden session with a
stateless core. Each request carries protocol and client metadata, optional
`server/discover` reports server support, and application state must be an
explicit value or handle. Tool schemas use JSON Schema 2020-12 and structured
tool results remain machine-readable. Roots, sampling and protocol logging are
deprecated; a stdio server logs only on stderr and reserves stdout for MCP
messages.

That architecture matches NUIF only when MCP remains a process adapter. The
canonical model, validation, hashing and semantic patch behavior stay in the
Rust core. `nuif-mcp-tools-0` therefore exposes four pure tools over inline
canonical text: validate, inspect, canonicalize and apply-patch. Applying a
patch transforms a supplied value and returns a new canonical value; it does
not mutate a host file or keep a hidden document session. The first profile has
no resources, prompts, tasks, sampling, roots, HTTP, OAuth, filesystem,
network, host-document or credential authority.

The implementation uses the official `rmcp` 3.1.4 server and its generated
schemas rather than reproducing a newly changed protocol by hand. It pins the
only supported protocol revision to 2026-07-28 and uses a current-thread Tokio
runtime. A NUIF-owned reader limits a JSON line before the SDK's stdio
transport can accumulate it; document, patch, transaction and operation limits
then apply at progressively more semantic layers.

## Evidence

- The final 2026-07-28 announcement removes `initialize`/`initialized` and
  `Mcp-Session-Id`, requires request metadata for the stateless lifecycle, and
  introduces optional `server/discover`. It recommends explicit application
  handles rather than transport-hidden state. Locator: *The 2026-07-28
  Specification*, “No handshake or sessions”, retrieved 2026-08-30:
  https://blog.modelcontextprotocol.io/posts/2026-07-28/.
- The same announcement marks roots, sampling and logging deprecated. The
  release-candidate detail identifies stderr as the stdio logging replacement
  and records full JSON Schema 2020-12 for tool input and output schemas.
  Locator: “Roots, Sampling, and Logging Are Deprecated” and “Full JSON Schema
  2020-12 for Tools”, retrieved 2026-08-30:
  https://blog.modelcontextprotocol.io/posts/2026-07-28-release-candidate/.
- The transport contract uses newline-delimited UTF-8 JSON-RPC, requires stdout
  to contain only protocol messages, and permits logging on stderr. Locator:
  MCP specification, *Transports*, “stdio”, retrieved 2026-08-30:
  https://modelcontextprotocol.io/specification/2025-06-18/basic/transports.
  The final 2026 revision retained stdio while replacing the HTTP lifecycle.
- The official Rust SDK describes `rmcp` as its protocol crate over Tokio. Its
  current roadmap reports complete dated 2025-11-25 and 2026-07-28 client and
  server conformance suites, while separately tracking features that those
  suites do not exercise. Locator: `ROADMAP.md`, “Conformance” and “Spec
  features without conformance scenarios”, retrieved 2026-08-30:
  https://github.com/modelcontextprotocol/rust-sdk/blob/main/ROADMAP.md.
- `rmcp` 3.1.4 declares Rust 1.88, Apache-2.0, optional protocol features and
  an official `transport-io` server surface. NUIF enables only macros, server
  and stdio; HTTP, client, OAuth, base64, tasks and provider features are
  absent. Locator: crates.io package metadata and the locked manifest,
  retrieved 2026-08-30: https://crates.io/crates/rmcp/3.1.4.
- `rmcp` 3.1.4's `AsyncRwTransport::receive` calls `read_until` into a reusable
  vector without applying the `JsonRpcMessageCodec::new_with_max_length`
  option. The NUIF wrapper therefore must bound the reader itself before
  transport parsing. Locator: `crates/rmcp/src/transport/async_rw.rs` at tag
  `rmcp-v3.1.4`, `receive` and `JsonRpcMessageCodec`, retrieved 2026-08-30:
  https://github.com/modelcontextprotocol/rust-sdk/blob/rmcp-v3.1.4/crates/rmcp/src/transport/async_rw.rs.

## Mechanism

The host launches `nuif-mcp` with piped stdin and stdout. A bounded asynchronous
reader permits at most 4 MiB before a newline and closes the connection on the
first excess byte. The official router verifies the 2026 request envelope,
decodes each tool's generated object schema and dispatches into a stateless
handler. The handler accepts at most 1 MiB each of inline document and patch
text. It decodes documents through `CanonicalText`, measures patch cardinality
through `nuif-protocol`, and delegates mutation to a temporary `nuif-api`
`Session`. Successful values use declared output schemas; execution failures
are tool errors with stable `NUIF_*` prefixes so an agent can correct input.

All tools declare read-only, non-destructive, idempotent and closed-world
annotations because they transform caller-owned values without external side
effects. These are truthful hints, not an authorization mechanism. A host must
still choose whether to launch the process and whether to pass its output into
a privileged product API.

The full NUIF document is deliberately not mirrored into MCP types. Typed MCP
records describe only arguments and bounded results; canonical document and
patch strings cross the boundary. This keeps the protocol shell replaceable
and makes direct Rust, CLI, WASM and MCP results comparable by canonical hash.

## NUIF relevance

**Choose the official Rust SDK** over a handwritten JSON-RPC loop. The 2026
revision changed lifecycle, request metadata, result discrimination, schema
rules and server-to-client interaction together. Reimplementing that surface
would create protocol work unrelated to NUIF semantics.

**Choose in-process Rust** over a TypeScript or Python SDK sidecar. Those SDKs
are valid alternatives for applications already implemented in those
languages, but a NUIF sidecar would need a second private RPC to the Rust core,
duplicate packaging and add another failure boundary. Rust SDK tier status is
therefore a monitored delivery risk, not a reason to duplicate the core.

**Choose stdio first** over Streamable HTTP. Local development clients already
own process launch and authorization, while stdio adds no listener, origin,
TLS, OAuth or multi-tenant policy. A future HTTP profile is a distinct security
product and must define authentication, authorization, request concurrency,
tenant isolation, origin validation, rate limits and deployment observability
before code is enabled.

**Reject file-path tools in profile zero.** They make a model-selected string
an ambient filesystem capability and prevent deterministic cross-surface
tests. Large documents and `.nuif` packages remain available through the CLI,
WASM or direct API where the host explicitly owns bytes and resources.

## Open questions

- Which real MCP hosts correctly consume 2026-07-28 stateless stdio requests,
  output schemas and tool annotations must be proven by a pinned live-client
  matrix; SDK conformance alone is not host-product evidence.
- The 1 MiB inline text limit is conservative and must be retained or changed
  from measured agent workloads, not raised merely to match the 16 MiB codec
  envelope.
- A resource-handle extension may become useful for large documents, but it
  requires an explicit host-owned capability grant and lifecycle rather than a
  server-invented path namespace.
- A remote MCP service remains out of scope until its separate security and
  operations RFC is accepted.
