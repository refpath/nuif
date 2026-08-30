# NUIF fuzz harness

This standalone cargo-fuzz package keeps sanitizer-only dependencies out of the
release workspace while calling the same production crates. The toolchain is
pinned because libFuzzer instrumentation requires nightly Rust.

Generate valid seed inputs and run the bounded smoke campaign:

```sh
rustup toolchain install nightly-2026-08-28 --profile minimal --component rust-src
cargo +nightly-2026-08-28 install cargo-fuzz --version 0.13.2 --locked
cargo xtask fuzz-smoke
```

An exact driver can instead be selected with `NUIF_CARGO_FUZZ`; the command
rejects every version except 0.13.2. `NUIF_FUZZ_RUNS` changes the per-target
run count within the enforced 1–1,000,000 range.

The nested dependency graph has its own lock file and policy because the
bundled LLVM libFuzzer runtime adds the OSI-approved NCSA license to the
non-shipping test graph. Audit it with:

```sh
cargo deny --manifest-path fuzz/Cargo.toml --config fuzz/deny.toml check
```

For a longer local campaign:

```sh
cargo +nightly-2026-08-28 fuzz run codec_roundtrip \
  target/fuzz-corpus/codec_roundtrip -- \
  -max_len=1048576 -timeout=10 -rss_limit_mb=2048 -use_value_profile=1
```

The five targets have deliberately separate contracts:

- `codec_roundtrip`: arbitrary text/CBOR bytes; accepted documents must reach
  canonical encode/decode fixpoints.
- `package_decode`: NUIF and Penpot archive parsers; accepted packages must
  deterministically re-encode and re-import.
- `resource_decoders`: bounded PNG and static-font inspection/decoding.
- `adapter_import`: UTF-8 HTML, SVG, DTCG, React and Svelte profile
  source import followed by export/import equivalence.
- `operation_sequence`: a byte choice stream becomes valid typed scalar
  operations and must preserve replay, inverse, codec and optional render
  relations.

Crash artifacts are generated under `fuzz/artifacts/` by cargo-fuzz and are not
committed until reviewed and converted into a named regression fixture.
