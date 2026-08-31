# NUIF conformance kit

This archive is the external-implementer handoff for the declared NUIF profile
0. It contains the normative specification, machine-readable schemas, bounded
fixtures, expected reports, and the standard-library-only Python reproduction.
It is a reproducibility kit, not a certification or a claim that an external
implementation has passed.

## Reproduce the baseline

1. Read `spec/00-conformance.md` and the profile-specific specification files.
2. Use the fixture and report files under `conformance/` as the normative test
   inputs and expected observations.
3. Run the independent reference helper in `implementations/python/` without
   importing the Rust workspace. Its output must be compared against the
   supplied fixture reports, not treated as a second standard.
4. Record implementation name, revision, toolchain, profile identifier,
   fixture IDs, context, raw outputs, and every classified divergence.

An implementation must not claim conformance from parsing the archive alone.
Publish a separate implementation report with the exact source and test
provenance, then request review through the repository's contribution process.
The kit intentionally excludes credentials, private host documents, live
vendor SDKs, and unsigned distribution binaries.

The package manifest binds every member to the source revision and SHA-256
digest. The archive is a developer artifact; release attestations and checksums
authenticate provenance but do not replace independent interoperability review.
