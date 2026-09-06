---
id: nuif:research:rustcrypto-hash-migration
kind: synthesis
status: reviewed
title: Coordinated SHA-2 and WebSocket dependency migration
source:
  url: https://github.com/RustCrypto/hashes/blob/sha2-v0.11.0/sha2/CHANGELOG.md
  authors: [RustCrypto Developers, Snapview contributors]
  published_at: "SHA-2 0.11.0 (2026-03-25); base16ct 1.0.0 (2026-01-03)"
  license: Apache-2.0 OR MIT
retrieved_at: 2026-09-06
tags: [dependencies, hashing, encoding, websocket]
confidence: 0.98
claims: []
relations:
  - type: extends
    target: nuif:research:content-addressed-versioning
    note: Separates the SHA-256 identity contract from library output formatting.
  - type: extends
    target: nuif:research:live-chromium-cdp-capture
    note: Replaces the transport-only upgrade hold with a coordinated Digest migration.
links:
  spec: [spec/08-serialization.md]
  adr: []
  rfc: []
  code: [Cargo.toml, dependencies/index.json, crates/nuif-codec, crates/nuif-capture]
  experiments: []
---

# Summary

SHA-2 0.11 moves to Digest 0.11, Rust edition 2024 and Rust 1.85. Its
changelog documents new hash types, backend configuration changes and the
removal of the `std` feature. The returned byte-array type does not implement
`LowerHex`. This is an encoding API change; the SHA-256 algorithm and digest
length remain unchanged. Locator: RustCrypto `sha2-v0.11.0`, `sha2/CHANGELOG.md`,
`sha2/src/lib.rs` and its documented digest examples, retrieved 2026-09-06.

## Evidence

- [Tungstenite 0.30 changelog](https://docs.rs/crate/tungstenite/0.30.0/source/CHANGELOG.md)
  records server-side rejection of malformed client keys, Rand/SHA updates and
  Rust 1.85. Its SHA-1 dependency uses Digest 0.11. The NUIF debugger connection
  uses SHA-1 only through the RFC 6455 handshake; artifact identities use
  SHA-256. Locators: the packaged 0.30.0 manifest and `src/handshake/mod.rs`,
  retrieved 2026-09-06.
- [base16ct 1.0](https://docs.rs/base16ct/1.0.0/base16ct/) provides lowercase
  encoding and a `HexDisplay` formatter. The crate has no dependencies and supports Rust 1.85. Its safe
  string API uses unchecked UTF-8 conversion internally after hex encoding. Its optional `alloc` feature supplies
  `lower::encode_string`; `HexDisplay` formats borrowed bytes. Locators:
  [manifest](https://github.com/RustCrypto/formats/blob/base16ct/v1.0.0/base16ct/Cargo.toml),
  `base16ct/src/lib.rs`, `base16ct/src/lower.rs` and `base16ct/src/display.rs`,
  retrieved 2026-09-06.
- [const-hex 1.19](https://docs.rs/const-hex/1.19.0/const_hex/) supplies runtime
  and constant-evaluation encoding, formatting and architecture-specific
  acceleration. These additional facilities are not required for the fixed
  32-byte digest encoding in this migration. No throughput comparison with
  const-hex was performed. Locator: crate documentation and manifest,
  retrieved 2026-09-06.

## NUIF relevance

**Adapt** SHA-2 and Tungstenite together. The resolved graph retains one
version each of Digest, crypto-common and block-buffer. A transport-only
upgrade would retain Digest 0.10 for SHA-2; the coordinated update removes
that constraint. SHA-2 default features are disabled because the workspace
uses fixed-output hashing without object identifiers or allocating hash
interfaces. The workspace Rust 1.96 minimum exceeds these dependencies'
Rust 1.85 minimum.

**Borrow** base16ct's supported lowercase encoder for digest strings and its
formatter when a prefix is written in the same formatting operation. The
library owns encoding details; no compatibility fork, trait shim or repeated
byte-formatting implementation is required. The core document model remains
independent of hash implementations.

**Reject** changing the content-addressing algorithm to BLAKE3 solely to
update the implementation library. Such a change would alter resource
identities and require a versioned interoperability decision under
`spec/08-serialization.md`.

## Validation contract

Acceptance requires the workspace tests and lint checks, the independent
canonical-CBOR and font digest fixtures, browser/WASM parity, and the live
Chromium capture gate. Expected hashes and pixel references are not regenerated
for the migration. Dependency advisories, licences and the minimum Rust
version are checked on the resolved graph. Performance claims require separate
measurements; the version update alone establishes none.
