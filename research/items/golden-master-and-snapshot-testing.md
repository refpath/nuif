---
id: nuif:research:golden-master-and-snapshot-testing
kind: synthesis
status: reviewed
title: Golden-master (characterization) and snapshot testing with insta, Jest and deterministic rendering baselines
source:
  url: https://insta.rs/docs/
  repository: https://github.com/mitsuhiko/insta
  authors: [Armin Ronacher, Michael Feathers, Christoph Nakazawa, Llewellyn Falco, Kent C. Dodds, Cruz, Rocha, Valente]
  published_at: "2016-07-27"
  license: insta Apache-2.0; Jest MIT; ApprovalTests Apache-2.0; JSS 2023 paper publisher controlled
retrieved_at: 2026-08-29
tags: [testing, snapshot-testing, characterization-test, golden-master, insta, jest, determinism, redaction, fonts, ahem]
confidence: 0.93
claims: [nuif:claim:semantic-automation, nuif:claim:opaque-preservation]
relations:
  - type: depends_on
    target: nuif:research:deterministic-simulation-testing
    note: Snapshot stability requires the same determinism controls that seeded simulation requires.
  - type: related_to
    target: nuif:research:differential-testing
    note: Browser-generated baselines are snapshots whose oracle is an alternative implementation.
  - type: related_to
    target: nuif:research:metamorphic-testing-graphics
    note: Snapshots capture one execution; metamorphic relations compare executions and need no baseline.
  - type: related_to
    target: nuif:research:harfbuzz-unicode
    note: Pinned fonts (Ahem) and shaping inputs are prerequisites for text-layout snapshots.
  - type: related_to
    target: nuif:research:encoding
    note: Canonical text profile snapshots are the primary human-reviewed artefact.
  - type: related_to
    target: nuif:research:vello
    note: Rendered-image snapshots need a deterministic CPU reference path.
  - type: related_to
    target: nuif:research:skia-gold-and-gm-tests
    note: Skia Gold is a triage workflow for image baselines at scale.
  - type: related_to
    target: nuif:research:flip-perceptual-difference-metric
    note: FLIP is a candidate perceptual comparator for image snapshots.
  - type: related_to
    target: nuif:research:ssim-and-classical-image-metrics
    note: Classical metrics are alternatives to per-channel pixel budgets.
links:
  spec: [spec/00-conformance.md, spec/08-serialization.md, spec/05-geometry-paint-text.md, spec/12-cli-api-and-automation.md]
  adr: []
  rfc: [rfcs/0004-headless-qa-contract.md]
  code: [crates/nuif-codec, crates/nuif-layout, crates/nuif-render, crates/nuif-cli]
  experiments: [conformance/PLAN.md, conformance/fixtures/v0-responsive-card/README.md]
---

# Summary

A characterization test records the observed behaviour of existing code as its oracle so that later changes are detected; it documents actual, not desired, behaviour (Feathers). Snapshot testing is the same idea automated: the first run stores the output, later runs diff against it. Jest 14 (2016) popularised the practice for UI trees and introduced the review-and-update workflow (`toMatchSnapshot`, `-u`, `--ci`). The Rust crate insta provides file and inline snapshots, serialised snapshot macros, redactions and filters for nondeterministic content, a `with_settings!` scope, an `INSTA_UPDATE` policy and the `cargo insta review` workflow. Grey-literature studies identify fragility, lack of context, large snapshots and blind approval as the main drawbacks. Deterministic visual baselines in browsers rely on the Ahem font, per-platform baselines, disabled animations and perceptual pixel thresholds.

For NUIF, snapshots are the storage form of the `canonicalization`, `layout` and `render` suites: canonical text, resolved box tables and images. The evidence below determines how to keep them deterministic and reviewable.

## Evidence

- Characterization testing documents "your system's actual behavior, not check for the behavior you wish your system had"; a production system "becomes its own specification". Feathers, https://michaelfeathers.silvrback.com/characterization-testing, retrieved 2026-08-29. Book: Working Effectively with Legacy Code, Prentice Hall 2004, ISBN 0131177052, chapter 13 (chapter text not retrieved).
- Wikipedia equates characterization test with "Golden Master Testing" and notes such tests verify observed behaviour, not correctness. https://en.wikipedia.org/wiki/Characterization_test, retrieved 2026-08-29.
- ApprovalTests: "Also known as Golden Master Tests or Snapshot Testing"; `*.approved.*` files are committed, `*.received.*` files are transitory; a reporter opens a diff tool on failure only. https://github.com/approvals/ApprovalTests.cpp/blob/master/doc/README.md and https://github.com/approvals/ApprovalTests.Net README, retrieved 2026-08-29.
- Jest 14.0 (2016-07-27) introduced `toMatchSnapshot()` with `react-test-renderer`, storing `pretty-format` output in `.snap` files and updating with `jest -u`. https://jestjs.io/blog/2016/07/27/jest-14, retrieved 2026-08-29.
- Jest docs: snapshots live in `__snapshots__/*.snap`; `--updateSnapshot`/`-u` re-records failing snapshots; `--ci` fails instead of writing new snapshots; as of Jest 20 snapshots are not written on CI without `--updateSnapshot`; `toMatchInlineSnapshot()`; property matchers such as `expect.any(Date)` are checked before the snapshot is written. https://jestjs.io/docs/snapshot-testing and https://jestjs.io/docs/cli, retrieved 2026-08-29.
- Jest best practices: treat snapshots as code and review them; resist regenerating snapshots instead of examining root causes (`no-large-snapshots` lint); tests must be deterministic (mock `Date.now()`); use descriptive names. https://jestjs.io/docs/snapshot-testing §Best Practices.
- insta 1.48.0 stores file snapshots as `snapshots/<module>__<name>.snap` next to the test; pending snapshots are `.snap.new` (inline: `.pending-snap`); header is YAML with `source`, `expression`, `input_file`, separated from the body by `---`; files are normalised to LF before diffing. https://docs.rs/insta/latest/insta/, https://insta.rs/docs/snapshot-types/, https://insta.rs/docs/snapshot-files/, retrieved 2026-08-29.
- Macros: `assert_snapshot!`, `assert_debug_snapshot!`, `assert_json_snapshot!`, `assert_compact_json_snapshot!`, `assert_compact_debug_snapshot!`, `assert_yaml_snapshot!`, `assert_ron_snapshot!`, `assert_csv_snapshot!`, `assert_toml_snapshot!`, `assert_binary_snapshot!` (experimental, compared byte for byte), `with_settings!`, `glob!`; features `yaml`, `json`, `ron`, `csv`, `toml`, `redactions`, `filters`, `glob`. https://docs.rs/insta/latest/insta/index.html#macros, retrieved 2026-08-29.
- Inline snapshots use a trailing `@"..."` argument; after review the tool rewrites it to `@r###"..."###`. https://insta.rs/docs/quickstart/ and https://insta.rs/docs/snapshot-types/.
- `INSTA_UPDATE`: `auto` (default: `no` on CI, `new` otherwise), `new` (write `.snap.new` pending review), `always` (write `.snap`, bypass review), `unseen` (`always` for new, `new` for existing), `no` (never write), `force` (rewrite even if passing). `INSTA_FORCE_PASS=1` lets tests pass to collect multiple snapshots; `INSTA_OUTPUT` ∈ {diff, summary, minimal, none}; `INSTA_WORKSPACE_ROOT` overrides cargo-based root detection. https://docs.rs/insta/latest/insta/ and https://insta.rs/docs/advanced/, retrieved 2026-08-29.
- `with_settings!({sort_maps => true}, { ... })`; `Settings` setters: `set_sort_maps` ("forceful sorting of maps before serialization"), `set_snapshot_path` (default `snapshots`), `set_prepend_module_to_snapshot`, `set_snapshot_suffix` (parameterised tests), `set_description`, `set_info`, `set_omit_expression`, `set_redactions`, `set_filters`, `set_strip_ansi_escape_codes`, `set_input_file`, `set_comparator`. https://insta.rs/docs/settings/ and https://docs.rs/insta/latest/insta/struct.Settings.html, retrieved 2026-08-29.
- Redactions (feature `redactions`) are a third macro argument `{ "selector" => replacement }` with selectors `.key`, `["key"]`, `[index]`, `[]`, `[start:end]`, `.*`, `.**`; helpers `insta::dynamic_redaction(|value, path| ...)`, `insta::sorted_redaction()` for unordered collections and `insta::rounded_redaction(3)` for floats. https://insta.rs/docs/redactions/, retrieved 2026-08-29.
- Filters (feature `filters`) are regex replacements applied to the string form, e.g. `(r"\b[[:xdigit:]]{32}\b", "[UID]")`, for content that is inherently textual. https://insta.rs/docs/filters/, retrieved 2026-08-29.
- `cargo insta` subcommands: `review` (alias `verify`), `accept`, `reject`, `test` with `--review`, `--accept`, `--accept-unseen`, `--check`, `--force-update-snapshots`, and `--unreferenced` ∈ {ignore, warn, reject, delete, auto}; `pending-snapshots`; `show`. https://insta.rs/docs/cli/ and `cargo-insta/src/cli.rs` on `master`, retrieved 2026-08-29.
- YAML is the recommended serialiser "because YAML is human readable and excellent at diffing because it is line based". https://insta.rs/docs/serializers/, retrieved 2026-08-29.
- Pitfalls: a grey-literature review of 50 documents finds fragility (28%), lack of context (22%), large snapshots (16%), manual verification (12%) and flakiness (6%); "blindly updating the test results" is the named failure mode; mitigations are code review (26%), treating snapshots as code (22%) and small snapshots (14%). Cruz, Rocha, Valente, "Snapshot testing in practice: Benefits and drawbacks", JSS 204 (2023) 111797, DOI 10.1016/j.jss.2023.111797, §2, §4.2 Table 3, §4.3 Table 4 (PDF https://homepages.dcc.ufmg.br/~mtov/pub/2023-jss-snapshot.pdf, retrieved 2026-08-29).
- Dodds (2017) quotes Searls: developers "will sooner just nuke the snapshot and record a fresh passing one", and states that snapshots beyond a few dozen lines suffer maintenance issues. https://kentcdodds.com/blog/effective-snapshot-testing, retrieved 2026-08-29.
- Playwright: screenshots differ across browsers and platforms "due to different rendering, fonts and more" and must be generated in the same environment; file names carry browser and platform suffixes; `toHaveScreenshot` defaults `animations: "disabled"`, `caret: "hide"`, `scale: "css"`, `threshold` 0.2 (YIQ perceived colour difference), optional `maxDiffPixels`, `maxDiffPixelRatio`, `mask`, `stylePath`. https://playwright.dev/docs/test-snapshots and https://playwright.dev/docs/api/class-pageassertions#page-assertions-to-have-screenshot-1, retrieved 2026-08-29.
- pixelmatch v5.3.0 `threshold` 0.1 default using YIQ colour difference (Kotsarenko and Ramos 2010) with anti-aliasing detection; the current README cites OKLab instead. https://github.com/mapbox/pixelmatch/blob/v5.3.0/README.md and https://github.com/mapbox/pixelmatch, retrieved 2026-08-29.
- WPT reftest fuzzy matching: `<meta name="fuzzy" content="maxDifference=15;totalPixels=300">` with inclusive ranges and per-reference prefixes. https://web-platform-tests.org/writing-tests/reftests.html, retrieved 2026-08-29.
- Ahem font: "well defined glyphs of precise sizes and shapes"; em-square exactly square; baseline 0.2em above bottom; X is a 1em square, p a 0.2em rectangle below baseline, É a 0.8em rectangle above, space transparent. https://web-platform-tests.org/writing-tests/ahem.html, retrieved 2026-08-29. Chromium: "Use the Ahem font to reduce the variance introduced by the platform's text rendering system"; pixel baselines are `-expected.png` with `platform/<PLATFORM-VERSION>` fallback chains. https://chromium.googlesource.com/chromium/src/+/main/docs/testing/writing_web_tests.md and web_test_baseline_fallback.md, retrieved 2026-08-29.
- expect-test 1.5.1 offers `expect![[...]]` inline snapshots updated with `UPDATE_EXPECT=1` and lists insta as the more complete alternative. https://docs.rs/expect-test, retrieved 2026-08-29.

## Mechanism

Snapshot assertion lifecycle (insta):

```
assert_*_snapshot!(name?, value, redactions?):
    text = serialize(value, format)                  # yaml/json/ron/csv/toml/debug/display
    text = apply_redactions(text, selectors)         # structured, before serialization
    text = apply_filters(text, regexes)              # string level
    path = snapshot_path / (module__name[suffix]).snap
    if exists(path) and body(path) == text: pass
    else match INSTA_UPDATE:
        no      -> fail
        new     -> write path.snap.new; fail (pending review)
        always  -> write path.snap; pass
        unseen  -> exists ? new : always
        force   -> write path.snap regardless
review: cargo insta review walks *.snap.new, shows diff, accept/reject moves or deletes
```

Determinism checklist for NUIF snapshots, with attribution:

1. Stable key ordering: `BTreeMap` in `nuif_core::Document` already sorts; use `sort_maps` for any `HashMap` (insta settings).
2. Float normalisation: `rounded_redaction(n)` or, preferably, round in the canonical encoder so that the snapshot equals the canonical text (insta redactions; `spec/08-serialization.md` numeric normalisation).
3. Redact or fix generated identifiers and timestamps; NUIF should instead generate IDs from the seed so no redaction is needed (Jest property matchers; JSS D14).
4. Pin fonts: ship Ahem or an equivalent metric-defined font for text fixtures (WPT Ahem; Chromium; Taffy and Yoga fixtures embed Ahem).
5. Fix viewport, scale factor, writing direction, locale and theme through `nuif_layout::EvaluationContext` (Playwright `scale`, `deviceScaleFactor`).
6. No animations or carets in rendered snapshots (Playwright defaults).
7. Per-platform image baselines only where the rasteriser is platform dependent; prefer a CPU reference path so one baseline suffices (Chromium fallback chains as the case to avoid).
8. Perceptual tolerance declared per fixture (`maxDifference;totalPixels`, YIQ/OKLab threshold), never global (WPT fuzzy; pixelmatch).
9. Never write snapshots on CI (`INSTA_UPDATE=no`, Jest `--ci`).
10. Keep snapshots small and named; review in code review; prune with `--unreferenced=delete` (Jest best practices; JSS Table 4; cargo-insta).

## NUIF relevance

**Borrow**

- insta file snapshots in YAML for canonical documents, resolved box tables and fidelity reports, with `with_settings!` suffixes for the 360/768/1440 viewport matrix (insta `set_snapshot_suffix`, `sort_maps`).
- The `INSTA_UPDATE=no` on CI plus `cargo insta review` workflow as the approval gate for the golden structural fixtures named in `conformance/PLAN.md`.
- Ahem-style metric fonts for text fixtures and WPT-style `maxDifference;totalPixels` fuzzy declarations for render fixtures where exact pixels are not normative.
- Snapshot header metadata (`description`, `info`) to carry implementation version, capability profile, fixture ID and evaluation context as required by `conformance/PLAN.md`.

**Adapt**

- Redactions should be unnecessary for canonical NUIF output; if a snapshot needs redaction, the encoder or the seed handling is nondeterministic and should be fixed instead.
- Image snapshots must record the comparison policy (metric, threshold, pixel budget) in the machine-readable report, not only in test code.
- Snapshot churn is a signal: the report should count snapshot updates per change so that over-approval is measurable (JSS 2023 fragility finding).

**Reject**

- Large whole-document snapshots of rendered DOM or HTML export output; export tests should snapshot the fidelity report and a normalised structural view, not raw formatter output.
- Platform-specific baseline fallback chains; NUIF should require a deterministic CPU reference renderer instead.

## Open questions

- Which of the canonical text profile or YAML `Debug` output should be the snapshot body, given that the canonical text is itself the normative artefact?
- Can `insta::Settings::set_comparator` host a tolerance-aware comparator for box tables so that resolved layouts are snapshotted with declared epsilon rather than exact text?
- How should image snapshots be stored in the repository without bloating history: content-addressed assets in the `.nuif` package or Git LFS?
