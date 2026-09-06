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

## Output replacement

Regular-file output is staged in the destination directory, synchronized and
renamed over the destination through `nuif_codec::filesystem::write_atomic`.
A failed staged write preserves the previous file. Existing writable-file
permissions are retained; symlinks, read-only files and non-regular destinations
are rejected. New files receive private permissions. Replacement creates a new
file identity, so other hard links continue to refer to the previous contents.
Extended attributes and ownership are not copied. The containing directory must
permit temporary-file creation and replacement.

This contract covers one file. Document and fidelity-report output are separate
replacements. It does not provide concurrent-writer coordination or directory
entry durability after power loss. The implementation uses the existing
[tempfile 3.27.0 `persist` API](https://docs.rs/tempfile/3.27.0/tempfile/struct.NamedTempFile.html#method.persist)
with an explicit file synchronization before replacement; the optional codec
`filesystem` feature is enabled by the CLI and native editor.
