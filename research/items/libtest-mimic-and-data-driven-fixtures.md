---
id: nuif:research:libtest-mimic-and-data-driven-fixtures
kind: synthesis
status: reviewed
title: libtest-mimic, datatest-stable, trybuild, expect-test and directory-driven fixture conventions (resvg, Taffy, rust-analyzer, Slint)
source:
  url: https://github.com/LukasKalbertodt/libtest-mimic
  authors: [Lukas Kalbertodt, nextest-rs, David Tolnay, Frank Rehberger, rust-analyzer team, Linebender (resvg), DioxusLabs (Taffy), Slint]
  published_at: "libtest-mimic 0.8.2 (2026-03-16); datatest-stable 0.3.3 (2026-03-31); trybuild 1.0.120 (2026-08-03); test-generator 0.3.1 (2022-12-08); expect-test 1.5.1 (2024-12-21)"
  license: MIT OR Apache-2.0 (libtest-mimic, datatest-stable, trybuild, test-generator, expect-test); resvg MIT OR Apache-2.0; Taffy MIT; rust-analyzer MIT OR Apache-2.0; Slint examples MIT
retrieved_at: 2026-08-29
tags: [testing, fixture, harness, libtest-mimic, datatest, expect-test, golden, data-driven, conformance, rust]
confidence: 0.88
claims: [nuif:claim:semantic-automation]
relations:
  - type: related_to
    target: nuif:research:rust-snapshot-property-fuzz-tooling
    note: nextest custom-harness rules and insta interplay.
  - type: related_to
    target: nuif:research:resvg-test-suite
    note: resvg fixture layout examined here.
  - type: related_to
    target: nuif:research:taffy-and-yoga-browser-generated-tests
    note: Taffy's browser-generated fixtures examined here.
  - type: related_to
    target: nuif:research:taffy
    note: Fixture regeneration protocol.
  - type: related_to
    target: nuif:research:golden-master-and-snapshot-testing
    note: expect-test and UPDATE_EXPECT are a golden-master mechanism.
links:
  spec: []
  adr: [adrs/0001-rust-reference-core.md]
  rfc: []
  code: [conformance/PLAN.md, conformance/README.md, conformance/fixtures, Cargo.toml, .github/workflows/ci.yml]
  experiments: []
---

# Summary

Cargo test targets with `harness = false` replace libtest with a user-supplied `main`. libtest-mimic re-implements libtest's CLI (filters, `--list`, `--skip`, `--exact`, `--ignored`, `--test-threads`, `--format`) so that a `main` can enumerate `Trial`s from a fixture directory at run time while remaining compatible with `cargo test` and cargo-nextest. datatest-stable wraps this in a `harness!` macro that maps one file per test. trybuild is the same idea specialised to compile-fail fixtures with `.stderr` references. expect-test provides inline or file-backed expectations updated in place with `UPDATE_EXPECT=1`. Large Rust projects converge on the same conventions: fixtures are files, expectations live beside inputs, regeneration is an explicit environment variable, references are pinned against font and renderer inputs, and generated tests are never edited by hand.

NUIF interpretation: `conformance/` should be a set of `harness = false` integration crates that enumerate fixture directories with libtest-mimic, produce one test per fixture (nextest then isolates each in a process), keep `input`, `expected` and `context` files together, and regenerate references only through one documented variable.

## Evidence

- Cargo: "The `harness` field indicates that the `--test` flag will be passed to `rustc`"; with `harness = false` "you are responsible for defining a `main()` function"; "Each integration test results in a separate executable binary, and `cargo test` will run them serially"; the working directory of each test is the package root. Locator: Cargo book "Cargo Targets" (`harness`, integration tests) and "cargo test" (working directory), retrieved 2026-08-29.
- libtest-mimic 0.8.2 (2026-03-16): usage `[[test]] name = "mytest" path = "tests/mytest.rs" harness = false`; `Arguments::from_args()`, `Trial::test(name, runner)`, `Trial::bench`, `Trial::ignorable_test`, `with_kind`, `with_ignored_flag`, `libtest_mimic::run(&args, tests).exit()`; run with `cargo test --test mytest`. Locator: `src/lib.rs` lines 13-52, 104-221; `CHANGELOG.md` "0.8.2".
- Supported arguments: `--include-ignored`, `--ignored`, `--test`, `--bench`, `--list`, `--nocapture` (no-op), `--show-output`, `--exact`, `--quiet`, `--test-threads`, `--logfile`, `--skip` (repeatable), `--color`, `--format`, positional filter; `Arguments::{is_ignored, is_filtered_out}` added in 0.8.2. Locator: `src/args.rs` lines 19-160; `CHANGELOG.md`.
- Known differences from libtest: "Output capture and `--nocapture`: simply not supported"; no `--format=junit`. Locator: `src/lib.rs` lines 54-71.
- `examples/tidy.rs` walks a directory recursively and creates `Trial::test(relative_path, move || check_file(&path)).with_kind("tidy")` per `.rs` file. Locator: `examples/tidy.rs`.
- nextest requires libtest-mimic 0.4.0 or 0.5.2+; a custom harness "MUST support being run with `--list --format terse`" printing `<TEST_NAME>: test` per line; datatest-stable is the reference example and with nextest "each test case is represented as a separate test, and is run as a separate process in parallel". Locator: nextest `site/src/docs/design/custom-test-harnesses.md`; datatest-stable `README.md`.
- datatest-stable 0.3.3 (2026-03-31), passively maintained, Rust ^1.72: `datatest_stable::harness! { { test = my_test, root = "path/to/fixtures", pattern = r".*" }, }`; test signatures `fn(&Path) -> Result<()>`, `fn(&Utf8Path) -> Result<()>`, `fn(&P, String)`, `fn(&P, Vec<u8>)`; `root` relative to the crate root; recursive traversal; `pattern` is a regex (fancy_regex); fixtures can be embedded with `include_dir!`. Locator: `README.md` "Usage"; `src/lib.rs` lines 44-210.
- trybuild 1.0.120 (2026-08-03): `TestCases::new().compile_fail("tests/ui/*.rs")` and `.pass(path)`; expected compiler output in adjacent `*.stderr`; on mismatch the actual output is written into a `wip` directory for manual promotion. Locator: `README.md` "Compile-fail tests", lines 97-134.
- test-generator 0.3.1 (2022-12-08, no release since): `#[test_resources("res/*/input.txt")]` generates one test per glob match; suggested layout `res/setN/{input.txt, expect.txt}`; requires `build.rs` to re-run on resource changes. Locator: `README.md`.
- expect-test 1.5.1 (2024-12-21): `expect![["..."]]` and `expect_file!["./path"]`; `Expect::assert_eq`, `assert_debug_eq`; `UPDATE_EXPECT=1` patches source files in place using `file!`, `line!`, `column!`; leading indentation is stripped. Locator: `src/lib.rs` lines 1-80.
- rust-analyzer: parser fixtures in `crates/parser/test_data/{lexer,parser}/{ok,err,inline}/NNNN_name.rs` with sibling `.rast` expectations; `tests.rs` iterates the directories and calls `expect_file![case.rast].assert_eq(&actual)`; multi-file fixtures use `//- /main.rs` metadata comments parsed by `test-utils::Fixture`; style guide requires minimal snippets, unindented raw strings, `cov_mark` marks, and forbids `#[should_panic]`. Locator: `crates/parser/src/tests.rs` lines 10-76; `crates/parser/test_data/parser/ok/0001_struct_item.{rs,rast}`; `crates/test-utils/src/fixture.rs` lines 1-40; `docs/book/src/contributing/style.md` "Minimal Tests", "Marked Tests", "`#[should_panic]`".
- resvg: fixtures are 200x200-viewBox SVGs where "Each test must test only a single issue", every element has an `id`, titles are unique; references are rendered at width 300 with pinned fonts (`--skip-system-fonts --use-fonts-dir tests/fonts` and explicit family mappings) and optimised with `oxipng -o 6 -Z`; `crates/resvg/tests/svg` mirrors `resvg-test-suite/svg`, while `tests/png` holds regression renders, not reference renders. Locator: `crates/resvg/tests/README.md`.
- resvg runner: `render_inner` reads `tests/<name>.svg` and `tests/<name>.png`, regenerates when `MAKE_REF` is set (invoking oxipng), writes side-by-side diff images to `tests/diffs/`, and counts pixels exceeding `DIFF_THRESHOLD`. Locator: `crates/resvg/tests/integration/main.rs` lines 36-176.
- Taffy: HTML fixtures in `test_fixtures/`; `scripts/gentest` downloads Chrome for Testing and ChromeDriver (`getchrome`), drives Chrome through WebDriver (`fantoccini`), and regenerates `tests/generated/` wholesale (`just gentest`), then runs `cargo fmt`; "You should not manually update the tests in `tests/generated`"; fixtures prefixed `x` are disabled; benchmarks are generated from the same fixtures; hand-written tests live in `tests/hand_written/` and share `tests/common` helpers (`new_test_tree`, measure functions). Locator: `CONTRIBUTING.md` lines 26-83; `scripts/gentest/src/main.rs` lines 11-67, 157; `tests/common/src/lib.rs`.
- Slint: `.slint` fixtures carry marker comments (`SLINT_SCALE_FACTOR=`, `BASE_THRESHOLD=`, `ROTATION_THRESHOLD=`), `build.rs` generates tests into `OUT_DIR` from `screenshots/cases`, fonts are pinned through `SLINT_DEFAULT_FONT`/`SLINT_FONT_PATH`, and `SLINT_CREATE_SCREENSHOTS=1` writes references; syntax tests regenerate with `SLINT_SYNTAX_TEST_UPDATE=1`; the test crates live in a separate `tests/` workspace. Locator: `tests/screenshots/{build.rs,testing.rs}`; `docs/testing.md`, master.

## Mechanism

Custom harness skeleton for a fixture directory (libtest-mimic 0.8):

```rust
// conformance/layout/tests/fixtures.rs   ([[test]] name = "fixtures", harness = false)
use libtest_mimic::{Arguments, Failed, Trial};

fn main() -> std::process::ExitCode {
    let args = Arguments::from_args();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures");
    let mut trials = Vec::new();
    for case in walk_case_dirs(&root) {                  // one directory per fixture
        let id = case.strip_prefix(&root).unwrap().display().to_string();
        let ignored = case.join("SKIP").exists();
        trials.push(Trial::test(format!("layout::{id}"), move || run_case(&case))
            .with_kind("layout").with_ignored_flag(ignored));
    }
    libtest_mimic::run(&args, trials).exit_code()
}

fn run_case(dir: &std::path::Path) -> Result<(), Failed> {
    let input = std::fs::read_to_string(dir.join("input.nuif"))?;
    let context: EvaluationContext = toml::from_str(&std::fs::read_to_string(dir.join("context.toml"))?)?;
    let actual = engine.layout(&decode(&input)?, &context)?;
    let actual_text = canonical_layout_text(&actual);
    let expected_path = dir.join("expected.layout.txt");
    if std::env::var_os("NUIF_UPDATE_EXPECT").is_some() { std::fs::write(&expected_path, &actual_text)?; return Ok(()); }
    let expected = std::fs::read_to_string(&expected_path)?;
    if expected != actual_text { return Err(format!("mismatch in {}", dir.display()).into()); }
    Ok(())
}
```

Properties of this design (from the sources): `--list --format terse` is provided by libtest-mimic, so nextest can run each `Trial` in its own process; `--skip`, `--exact` and the positional filter select fixtures by their path-derived names; `Trial::with_ignored_flag` implements `SKIP` markers without deleting fixtures; the working directory is the package root, so relative paths resolve; test names must be stable and unique because nextest keys results and JUnit cases by name.

Recommended layout for `conformance/` fixture crates (interpretation combining the sources):

```text
conformance/
  Cargo.toml                       # member of the workspace; [[test]] harness = false per suite
  README.md, PLAN.md
  fixtures/
    <suite>/                       # model | canonicalization | extensions | layout | render | operations | merge | provenance | adapter | security
      <fixture-id>/                # stable slug; directory name is the test name
        input.nuif                 # authored input (text form) or input.bin for binary-only cases
        context.toml               # evaluation context: viewport, scale, fonts, capability profile, tolerances
        expected.<kind>.txt|json   # canonical text/JSON expectation (layout boxes, diagnostics, canonical form)
        expected.png               # only for render suite; rendered by the CPU reference path, oxipng-optimised
        ops.json                   # operations suite: transaction list; inverse and replay derived, not stored
        meta.toml                  # title (unique), issue reference, tolerance overrides, disabled = true|false
    fonts/                         # pinned fonts referenced by context.toml; no system fonts
  tests/
    <suite>.rs                     # libtest-mimic harness; one Trial per fixture directory
  generated/                       # browser-differential cases regenerated by an xtask; never edited by hand
```

Regeneration protocol: a single variable (`NUIF_UPDATE_EXPECT=1`, mirroring `UPDATE_EXPECT`, `MAKE_REF`, `MASONRY_TEST_BLESS`, `SLINT_CREATE_SCREENSHOTS`) rewrites expectations; missing expectations fail and write `*.new` files; diff artifacts go to an ignored `diffs/` directory and are uploaded by CI; generated suites are regenerated wholesale by an `xtask` and committed separately.

## NUIF relevance

**Borrow**
- libtest-mimic as the harness for every `conformance/` suite, because it preserves `cargo test` filters and nextest process isolation without a code generator.
- The resvg fixture discipline (one issue per fixture, unique title, ids on every element, pinned fonts, lossless-compressed references) for the render and layout suites, because conformance/PLAN.md requires reproducible results with declared tolerances.
- rust-analyzer's sibling-expectation layout (`NNNN_name.rs` + `.rast`) and `UPDATE_EXPECT` semantics, because they keep inputs and expectations reviewable side by side in diffs.
- Taffy's "generated tests are never edited by hand" rule for browser-differential layout fixtures, because it separates oracle changes from engine changes.

**Adapt**
- datatest-stable's one-file-per-test model must become one-directory-per-test, because NUIF fixtures need `input`, `context` and `expected` files together and a fixture ID in the report (conformance/PLAN.md "fixture ID and evaluation context").
- expect-test inline expectations are appropriate for unit tests in crates, not for conformance fixtures, which must be language-neutral files usable by non-Rust implementations.
- trybuild's `wip` directory convention translates to `*.new` files next to expectations, matching egui and Masonry.

**Reject**
- test-generator, because it has had no release since 2022-12-08 and relies on `build.rs` re-run hints rather than run-time enumeration.
- Editing generated differential fixtures by hand, because both Taffy and Slint document that generated suites are overwritten wholesale.
- `#[should_panic]` and stdout capture as fixture assertions, because libtest-mimic does not capture output and rust-analyzer's style guide rejects `should_panic` in favour of explicit results.

## Open questions

- Whether nextest's per-process execution overhead is acceptable for thousands of small fixtures, or whether suites should batch fixtures per process for local runs and isolate only in CI.
- Whether conformance fixtures should be embedded with `include_dir!` for a self-contained `nuif conformance` CLI subcommand, in addition to directory enumeration.
- Whether a JSON test-result schema should be emitted by the harness directly (QA item 10), since libtest-mimic lacks `--format=junit` and nextest's JUnit only covers pass/fail/time.
