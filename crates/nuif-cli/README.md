# NUIF command-line tool

`nuif` is the explicit-filesystem developer interface to the reference engine.
It validates, inspects, canonicalizes, patches, lays out, renders, packages and
converts declared adapter profiles without requiring the native editor or an
MCP host.

```sh
nuif capabilities
nuif validate document.nuif
nuif inspect document.nuif
nuif snapshot document.nuif snapshot 1440 900
nuif export document.nuif svg-0 output.svg fidelity.json
```

Run `nuif --help` for the exact command forms in this package. Machine-facing
commands emit JSON. Inputs are bounded by the profile limits reported by
`nuif capabilities`; package and adapter operations fail closed outside their
declared subsets.

The CLI declares exactly one optional package capability:
`nuif-opentype-variable-truetype-single-0`. Its layout, render, snapshot and
rewrite paths resolve those verified font bytes through the shared runtime.
Package behavior and all other extension capabilities remain structurally
inspectable, hashable, extractable and byte-preservingly copyable, but commands
that evaluate or rewrite them fail atomically with
`PACKAGE_CAPABILITIES_REQUIRED`. Native `.nuif` import/export retains verified
resources and requirements instead of silently rebuilding a document-only
archive. `nuif snapshot` writes the common full report to
`expected.report.json` and prints only a compact artifact summary.

The archive is an unsigned research-preview developer package. Verify its
SHA-256 entry and GitHub artifact attestation before use. The binary has no
background service or implicit network authority. It reads or writes only the
paths/stdin/stdout selected by the caller.

Build the same tool from a reviewed checkout with:

```sh
cargo install --path crates/nuif-cli --locked
```
