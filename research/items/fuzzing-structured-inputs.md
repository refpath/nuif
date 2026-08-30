---
id: nuif:research:fuzzing-structured-inputs
kind: synthesis
status: reviewed
title: Coverage-guided fuzzing of structured inputs (libFuzzer, AFL++, cargo-fuzz, arbitrary, grammar fuzzing, parser resource limits)
source:
  url: https://llvm.org/docs/LibFuzzer.html
  repository: https://github.com/rust-fuzz/cargo-fuzz
  authors: [LLVM Project, AFLplusplus contributors, rust-fuzz contributors, Cornelius Aschermann, Rohan Padhye, Google OSS-Fuzz, Skia contributors, image-rs contributors, resvg contributors]
  published_at: "2019-02-24"
  license: LLVM Apache-2.0 with LLVM exception; AFL++ Apache-2.0; cargo-fuzz and arbitrary MIT OR Apache-2.0; libfuzzer-sys (MIT OR Apache-2.0) AND NCSA; Skia BSD-3-Clause; resvg MIT OR Apache-2.0; Nautilus and Zest papers publisher controlled
retrieved_at: 2026-08-29
tags: [testing, fuzzing, coverage-guided, structure-aware, arbitrary, cargo-fuzz, grammar-fuzzing, resource-limits, parser, security]
confidence: 0.92
claims: [nuif:claim:opaque-preservation, nuif:claim:semantic-automation]
relations:
  - type: related_to
    target: nuif:research:property-based-testing-state-machines
    note: Structure-aware fuzzing and PBT share generator design; fuzzing replaces random sampling with a coverage-guided corpus.
  - type: related_to
    target: nuif:research:delta-debugging-and-test-case-reduction
    note: cargo fuzz tmin and libFuzzer -minimize_crash reduce bytes; structural reduction needs the generator.
  - type: related_to
    target: nuif:research:deterministic-simulation-testing
    note: Swarm testing and fuzz-target determinism rules apply to both.
  - type: supports
    target: nuif:research:webgpu-security
    note: Parser depth, size and allocation budgets are the CPU-side counterpart of renderer resource limits.
  - type: related_to
    target: nuif:research:svg
    note: usvg and Skia SVG fuzz targets show concrete limits for a vector document parser.
  - type: related_to
    target: nuif:research:encoding
    note: Deterministic CBOR and text decoders are the primary fuzz targets.
links:
  spec: [spec/11-security.md, spec/08-serialization.md, spec/07-extensions-and-dialects.md, spec/00-conformance.md]
  adr: []
  rfc: [rfcs/0002-extension-preservation.md, rfcs/0004-headless-qa-contract.md]
  code: [crates/nuif-codec, crates/nuif-core, crates/nuif-render, crates/nuif-layout, crates/nuif-testing, fuzz]
  experiments: [conformance/PLAN.md, conformance/HARNESS.md, research/experiments/index.yaml]
---

# Summary

Coverage-guided fuzzers (libFuzzer, AFL++) mutate a corpus of byte inputs and keep mutations that reach new coverage. For structured inputs, three approaches exist: custom mutators that parse, mutate and re-serialise (libFuzzer `LLVMFuzzerCustomMutator`, AFL++ `afl_custom_fuzz`, libprotobuf-mutator, Grammar-Mutator); grammar-based generators with tree mutation and coverage feedback (Nautilus); and parametric generators that decode the fuzzer's byte stream into a typed value so that byte mutations become structural mutations (Zest; Rust `arbitrary` with `cargo fuzz`). Parser fuzz targets must be deterministic, fast and bounded; libFuzzer enforces `-timeout`, `-rss_limit_mb` and `-malloc_limit_mb`, OSS-Fuzz flags inputs over about 25 seconds or 2.5 GB, and production parsers (usvg, roxmltree, serde_json, image, Skia) enforce nesting depth, node counts and allocation budgets in code.

For NUIF the codec, extension payload handling and path geometry are untrusted-input parsers under `spec/11-security.md`; the same `Arbitrary`-based generator can feed both fuzz targets and the trial-and-error loop.

## Evidence

- libFuzzer target contract: `extern "C" int LLVMFuzzerTestOneInput(const uint8_t *Data, size_t Size)`; the target "must tolerate any kind of input", "must not `exit()`", "must be as deterministic as possible", "must be fast", and ideally not modify global state; narrower targets are better. https://llvm.org/docs/LibFuzzer.html, §Fuzz Target, retrieved 2026-08-29.
- Coverage comes from SanitizerCoverage inline 8-bit counters via `-fsanitize=fuzzer`; mutations that reach a previously uncovered path are added to the corpus. Same page, §Corpus.
- Options: `-max_len` (0 = guess from corpus), `-len_control`, `-timeout` (default 1200 s), `-rss_limit_mb` (default 2048), `-malloc_limit_mb` (single allocation cap), `-dict`, `-use_value_profile`, `-jobs`/`-workers`, `-minimize_crash`, `-merge`. Same page, §Options.
- Structure-aware fuzzing: custom mutator `size_t LLVMFuzzerCustomMutator(uint8_t *Data, size_t Size, size_t MaxSize, unsigned int Seed)` parses per grammar, mutates, re-serialises; libprotobuf-mutator with `DEFINE_PROTO_FUZZER` uses protobuf as the intermediate format (SQLite example). https://github.com/google/fuzzing/blob/master/docs/structure-aware-fuzzing.md, retrieved 2026-08-29.
- AFL++: LTO instrumentation preferred; `-m` memory limit "highly recommend"; `-t` timeout; dictionaries via `-x` and `AFL_LLVM_DICT2FILE`; CMPLOG/Redqueen via `AFL_LLVM_CMPLOG=1` and `-c`; `afl-cmin` and `afl-tmin`. https://github.com/AFLplusplus/AFLplusplus/blob/stable/docs/fuzzing_in_depth.md, retrieved 2026-08-29.
- AFL++ custom mutator API: `afl_custom_init`, `afl_custom_fuzz`, `afl_custom_post_process`, `afl_custom_trim`, `afl_custom_havoc_mutation`, `afl_custom_queue_get`; `AFL_CUSTOM_MUTATOR_LIBRARY`, `AFL_CUSTOM_MUTATOR_ONLY`. https://github.com/AFLplusplus/AFLplusplus/blob/stable/docs/custom_mutators.md, retrieved 2026-08-29. Paper: Fioraldi, Maier, Eißfeldt, Heuse, WOOT 2020, §3.2.1 (havoc probability 6%), §3.2.2 Input-To-State (https://aflplus.plus/papers/aflpp-woot2020.pdf, retrieved 2026-08-29).
- Grammar-Mutator: AFL++ custom mutator for "highly-structured inputs" with JSON grammars, tree-based mutations (rules, random, random recursive, splicing) and tree-based trimming; `grammar_generator-<lang> 100 1000 ./seeds ./trees` creates 100 seeds of max tree size 1000. https://github.com/AFLplusplus/Grammar-Mutator README, retrieved 2026-08-29.
- cargo-fuzz: "a tool to invoke a fuzzer", libFuzzer via `libfuzzer-sys`, nightly required; commands `cargo fuzz init|add|list|run|tmin|cmin|coverage|fmt`; `cargo fuzz coverage` builds with `-Cinstrument-coverage` and writes `fuzz/coverage/<target>/coverage.profdata`; crash artefacts under `fuzz/artifacts/<target>/`. https://rust-fuzz.github.io/book/cargo-fuzz.html, /cargo-fuzz/tutorial.html, /cargo-fuzz/coverage.html and https://github.com/rust-fuzz/cargo-fuzz README, retrieved 2026-08-29. afl.rs alternative: `cargo afl build`, `cargo afl fuzz -i in -o out <bin>`, `fuzz!` macro (https://rust-fuzz.github.io/book/afl.html).
- `fuzz_target!` accepts `|data: &[u8]|` or `|input: T|` for `T: Arbitrary`, an `init:` block, and an optional `-> Corpus` return (`Corpus::Keep`/`Corpus::Reject`); inputs whose `Arbitrary` decoding fails are rejected. https://docs.rs/libfuzzer-sys/latest/libfuzzer_sys/macro.fuzz_target.html and https://rust-fuzz.github.io/book/cargo-fuzz/structure-aware-fuzzing.html, retrieved 2026-08-29.
- arbitrary 1.4.2: `trait Arbitrary<'a>: Sized { fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self>; fn arbitrary_take_rest(u: Unstructured<'a>) -> Result<Self>; fn size_hint(depth: usize) -> (usize, Option<usize>); fn try_size_hint(depth: usize) -> Result<(usize, Option<usize>), MaxRecursionReached> }`; `#[derive(Arbitrary)]` with the `derive` feature. https://docs.rs/arbitrary/latest/arbitrary/trait.Arbitrary.html and https://github.com/rust-fuzz/arbitrary README, retrieved 2026-08-29.
- `Unstructured` methods: `int_in_range` (not necessarily uniform; returns range start on empty data), `choose`, `choose_index`, `ratio`, `arbitrary_len` (uses element `size_hint`, takes lengths "from the end of the data"), `bytes`, `take_rest`, `arbitrary_iter`. https://docs.rs/arbitrary/latest/arbitrary/struct.Unstructured.html and `src/unstructured.rs`, retrieved 2026-08-29.
- Recursion behaviour: `size_hint::MAX_DEPTH = 20` guards only `size_hint` computation (`src/size_hint.rs` line 4, lines 38–47); `ArbitraryIter::next` draws a `bool` "keep going" flag that is false on exhausted data, so `Vec<T>` and recursive children terminate when bytes run out (`src/unstructured.rs`, `src/foreign/alloc/vec.rs`, `src/foreign/core/bool.rs`). Retrieved 2026-08-29. NUIF interpretation: depth must be bounded explicitly in `arbitrary` implementations.
- Grammar fuzzing: derivation trees with expansion phases bounded by `min_nonterminals`/`max_nonterminals` (https://www.fuzzingbook.org/html/GrammarFuzzer.html); greybox grammar fuzzing combines fragment or region mutation with coverage-guided seed selection (https://www.fuzzingbook.org/html/GreyboxGrammarFuzzer.html). Retrieved 2026-08-29.
- Nautilus: combining context-free grammars with feedback-driven fuzzing outperforms AFL "by an order of magnitude"; mutations on derivation trees (random with a configurable maximum subtree size, rules, random recursive 2^n repetitions, splicing) plus subtree and recursive minimisation; AFL-style 64 KB bitmap. Aschermann et al., NDSS 2019, DOI 10.14722/ndss.2019.23412, §IV.A–C, §V (PDF https://www.ndss-symposium.org/wp-content/uploads/2019/02/ndss2019_04A-3_Aschermann_paper.pdf, retrieved 2026-08-29).
- Zest: "converts random-input generators into deterministic parametric generators"; mutations in the untyped parameter domain map to structural mutations; every parameter sequence yields a syntactically valid input if the generator does; the algorithm tracks total and valid coverage and saves inputs that add valid coverage; the XML generator bounds `MAX_DEPTH` and `MAX_CHILDREN`. Padhye et al., ISSTA 2019, DOI 10.1145/3293882.3330576, Abstract, §3.1–3.2, Fig. 2 (PDF https://rohan.padhye.org/files/zest-issta19.pdf, retrieved 2026-08-29).
- OSS-Fuzz: default engines libfuzzer, afl, honggfuzz, centipede; seed corpus `<target>_seed_corpus.zip`, `<target>.dict`, `<target>.options` with `[libfuzzer]` keys such as `max_len`, `rss_limit_mb = 6000`, `timeout = 30`; inputs over "~25 seconds or more than 2.5GB RAM" are reported as timeout or OOM bugs; Rust projects build with `cargo fuzz build -O` on `base-builder-rust`. https://google.github.io/oss-fuzz/getting-started/new-project-guide/, /new-project-guide/rust-lang/, /faq/, retrieved 2026-08-29. Ideal integration: targets live in the project repository, run in regression CI, ship dictionaries and seed corpora, must not hang or exhaust memory instantly. https://google.github.io/oss-fuzz/advanced-topics/ideal-integration/.
- usvg parser limits (`crates/usvg/src/parser/svgtree/parse.rs`, `main`, retrieved 2026-08-29): `if depth > 1024 { return Err(Error::NodesLimitReached); }` in `parse_xml_node`; `if doc.nodes.len() > 1_000_000 { return Err(Error::NodesLimitReached); }`; `use` self-reference check `if link == node || link == origin`; `fix_recursive_patterns`, `fix_recursive_links`, `fix_recursive_fe_image` replace self-referential paint/clip/mask/filter links. roxmltree `ParsingOptions { allow_dtd, nodes_limit }` and `Error::EntityReferenceLoop` (depth limit 10, 255 references per reference). https://docs.rs/roxmltree/latest/roxmltree/, retrieved 2026-08-29. No fuzz directory exists in the resvg repository and no OSS-Fuzz `projects/resvg` entry was found.
- Skia: `fuzz/Fuzz.h` wraps the byte buffer with `next<T>()`, `nextRange(min, max)`, `exhausted()`; `FuzzCanvas(Fuzz*, SkCanvas*, int depth = 9)` returns when `depth <= 0` or the buffer is exhausted, draws up to 2000 ops and recurses with `depth - 1` into paints, image filters and pictures; `fuzz/oss_fuzz/FuzzSVG.cpp` rejects inputs over 30,000 bytes and renders the SVG DOM to a 128×128 surface. https://github.com/google/skia, `fuzz/Fuzz.h`, `fuzz/FuzzCanvasHelpers.h`, `fuzz/FuzzCanvasHelpers.cpp`, `fuzz/oss_fuzz/FuzzSVG.cpp`, `main`, retrieved 2026-08-29.
- image 0.25.10: `Limits { max_image_width, max_image_height, max_alloc }` with `max_alloc` default 512 MiB and `reserve`/`free` accounting; fuzz targets such as `fuzz/fuzzers/fuzzer_script_png.rs` call `image::load_from_memory_with_format(data, ImageFormat::Png)`. https://docs.rs/image/latest/image/struct.Limits.html and https://github.com/image-rs/image/tree/main/fuzz, retrieved 2026-08-29.
- serde_json: `remaining_depth: 128` with `check_recursion!` returning `RecursionLimitExceeded`; `disable_recursion_limit` requires the `unbounded_depth` feature and the docs recommend another stack-overflow guard. https://docs.rs/serde_json/latest/serde_json/struct.Deserializer.html#method.disable_recursion_limit, retrieved 2026-08-29.
- fontations: `cargo +nightly fuzz build -O --debug-assertions`; `fuzz_skrifa_outline.rs` iterates glyphs over size, location, hinting and memory variants; `helpers.rs` caps variation axes at 5. https://github.com/googlefonts/fontations/tree/main/fuzz, retrieved 2026-08-29.

## Mechanism

```
// Structure-aware target over the NUIF document model (synthesis; attributions inline)
#[derive(Arbitrary, Debug)]
struct FuzzDoc { root: FuzzNode, context: FuzzContext }

impl<'a> Arbitrary<'a> for FuzzNode {                    // manual impl: explicit depth bound (Zest Fig. 2; Skia depth = 9)
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> { gen_node(u, 0) }
}
fn gen_node(u, depth) -> Result<FuzzNode> {
    let n_children = if depth >= MAX_DEPTH { 0 } else { u.int_in_range(0..=MAX_CHILDREN)? };
    ...                                                 // arbitrary_len / arbitrary_iter stop on exhausted bytes
}

fuzz_target!(|doc: FuzzDoc| -> Corpus {                 // libfuzzer-sys; decoding failure auto-rejected
    let bytes = encode_canonical(&doc.into_document());  // bounded by construction
    if bytes.len() > MAX_INPUT { return Corpus::Reject; }
    let decoded = decode_with_limits(&bytes, Limits { depth: 256, entities: 100_000, alloc: 256 MiB });
    match decoded {
        Err(e) if e.is_limit() => Corpus::Keep,          // limits must fire, not overflow (usvg, serde_json)
        Err(_) => Corpus::Reject,
        Ok(d) => { assert_eq!(encode_canonical(&d), bytes); layout_with_budget(&d, &doc.context); Corpus::Keep }
    }
});
```

Run configuration: `cargo fuzz run codec_roundtrip -- -max_len=65536 -timeout=10 -rss_limit_mb=2048 -malloc_limit_mb=512 -dict=fuzz/nuif.dict -use_value_profile=1 -jobs=8`; `cargo fuzz cmin` after each campaign; `cargo fuzz tmin` on crashes; `cargo fuzz coverage` to compare corpus coverage with the property-based generator.

Budget classes to enforce inside NUIF parsers, with source examples: nesting depth (usvg 1024; serde_json 128; roxmltree entity depth 10), node or entity count (usvg 1,000,000; roxmltree `nodes_limit`), input size (Skia 30,000 bytes for SVG; libFuzzer `-max_len`), allocation (image `max_alloc` 512 MiB; libFuzzer `-malloc_limit_mb`), reference cycles (usvg `fix_recursive_*`), and time (libFuzzer `-timeout`; OSS-Fuzz 25 s).

Invariants: the target is deterministic for a given input; every limit produces a typed error rather than a panic or stack overflow; decoded documents re-encode to identical bytes; extension payloads decoded from arbitrary bytes are preserved verbatim.

## NUIF relevance

**Borrow**

- `cargo fuzz`/libFuzzer raw byte targets for every untrusted parser and a parametric byte choice stream for valid semantic operations, matching the `fuzz parsers/codecs and path geometry` technique in `conformance/PLAN.md`.
- Concrete limit values from usvg, serde_json and image as starting points for the bounds that `spec/11-security.md` requires (depth, entity count, allocation, cycle detection).
- libFuzzer's target contract (deterministic, no exit, no global state) as the acceptance rule for every headless engine entry point exposed through `spec/12-cli-api-and-automation.md`.
- Zest's valid-coverage feedback: keep corpus inputs that are valid documents and add coverage, reject structurally invalid ones so the corpus stays useful for the round-trip loop.

**Adapt**

- Depth bounding must be explicit in `Arbitrary` implementations because `arbitrary`'s `MAX_DEPTH` guards only size hints; NUIF should generate nodes with a depth parameter as in Zest and Skia.
- The same typed operation choice-stream mapper is reusable by deterministic trials and coverage-guided fuzzing; it delegates values and invariants to production operation types instead of serializing a second document model (Zest parametric generators; Hypothesis choice sequences).
- Extension payloads are opaque bytes and should be fuzzed for preservation, not for interpretation: the oracle is byte equality after round trip (`rfcs/0002-extension-preservation.md`).

**Reject**

- Grammar-mutator custom mutators (AFL++ Grammar-Mutator, libprotobuf-mutator) are unnecessary while the Rust model already provides a typed generator; they add a second grammar to maintain.
- A raw-byte-only campaign with no valid corpus. NUIF deliberately retains raw malformed parser inputs but regenerates valid canonical/package/resource/source seeds from production fixtures so mutations reach post-parse relations.

## Implemented decision

`fuzz/` is a standalone workspace pinned to nightly 2026-08-28,
cargo-fuzz 0.13.2 and libfuzzer-sys 0.4.13. Five targets separate codec,
package/archive, PNG/font, static-source-adapter and valid operation concerns.
Each owns a generated target-specific corpus under ignored `target/`; corpora
are not shared across incompatible input selectors. CPU rendering is sampled
only after valid typed operations, while GPU execution is excluded from the
security fuzzer because it has a separate nondeterministic process/device risk
boundary. `cargo xtask fuzz-smoke` applies 10-second, 512-MiB allocation and
2-GiB RSS limits, records every target, and CI runs 512 inputs per target under
AddressSanitizer. Format resource limits remain normative where declared by
`spec/11-security.md`; campaign limits are implementation test budgets rather
than wire-profile requirements.
