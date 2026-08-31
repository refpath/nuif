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

The CLI declares no support for package behavior or other extension
capabilities. It can structurally validate, inspect, hash, extract and
byte-preservingly copy a package that declares them. Commands that evaluate or
rewrite that package—layout, render, snapshot, external adapter export, a
changed `.nuif` save or package-mode conversion—fail atomically with
`PACKAGE_CAPABILITIES_REQUIRED`. Native `.nuif` import/export retains verified
resources and requirements instead of silently rebuilding a document-only
archive.

The archive is an unsigned research-preview developer package. Verify its
SHA-256 entry and GitHub artifact attestation before use. The binary has no
background service or implicit network authority. It reads or writes only the
paths/stdin/stdout selected by the caller.

Build the same tool from a reviewed checkout with:

```sh
cargo install --path crates/nuif-cli --locked
```
