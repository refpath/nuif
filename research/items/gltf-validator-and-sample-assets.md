---
id: nuif:research:gltf-validator-and-sample-assets
kind: implementation
status: reviewed
title: Khronos glTF Validator report format, sample-asset corpus and extension prefix registry as a conformance kit
source:
  url: https://github.com/KhronosGroup/glTF-Validator
  repository: https://github.com/KhronosGroup/glTF-Validator
  authors: [The Khronos Group, glTF-Validator contributors]
  published_at: "glTF-Validator 2.0.0-dev.3.10; glTF-Sample-Assets main; glTF extensions registry main (retrieved 2026-08-29)"
  license: "Apache-2.0 (validator); CC-BY-4.0 and CC0 per asset (sample assets)"
retrieved_at: 2026-08-29
tags: [conformance, validation, report-format, severity, sample-assets, corpus, extensions, registry, prefixes]
confidence: 0.93
claims: [nuif:claim:opaque-preservation, nuif:claim:semantic-automation]
relations:
  - type: extends
    target: nuif:research:gltf
    note: Adds the validator, the sample corpus and the prefix registry process to the core/extension model.
  - type: related_to
    target: nuif:research:gltf-interactivity
    note: KHR_interactivity is at Release Candidate in the registry and must be supported by the validator before ratification.
  - type: compares_to
    target: nuif:research:openusd-composition-and-crate
    note: Both projects ship a CLI validator with typed severities; glTF's report is machine-readable JSON with a schema.
  - type: related_to
    target: nuif:research:ui-spec-schema-cg
    note: Report schema and issue code table are precedents for NUIF diagnostic schemas.
links:
  spec: [spec/00-conformance.md, spec/07-extensions-and-dialects.md, spec/12-cli-api-and-automation.md]
  adr: []
  rfc: [rfcs/0002-extension-preservation.md, rfcs/0004-headless-qa-contract.md]
  code: [crates/nuif-cli]
  experiments: [conformance/PLAN.md, conformance/fixtures]
---

# Summary

The Khronos glTF ecosystem pairs a specification with three executable conformance artifacts. The glTF Validator (Dart, Apache-2.0) checks JSON schema conformance, reference validity, binary buffer contents, images, GLB container structure and a set of extensions, and writes a JSON report described by `docs/validation.schema.json`: `uri`, `mimeType`, `validatorVersion`, `validatedAt`, an `issues` block with counts (`numErrors`, `numWarnings`, `numInfos`, `numHints`), a `truncated` flag and `messages` each carrying `code`, `severity` (0 Error, 1 Warning, 2 Information, 3 Hint), `message` and either a JSON `pointer` or a GLB byte `offset`, plus an `info` block. `ISSUES.md` enumerates 159 codes across six categories (IoError, SchemaError, SemanticError, LinkError, DataError, GlbError). Unknown extensions are Information (`UNSUPPORTED_EXTENSION`), undeclared use is an Error (`UNDECLARED_EXTENSION`), and unknown properties are Warnings (`UNEXPECTED_PROPERTY`).

glTF-Sample-Assets is "a curated collection of glTF models that illustrate one or more features or capabilities of glTF"; each model has `metadata.json` (name, path, summary, tags, legal with SPDX license, screenshot) and is indexed in `Models/model-index.json` with tags `core`, `extension`, `testing`, `showcase`, `video`, `written`, `pbrtest`, `issues` and format variants (`glTF`, `glTF-Binary`, `glTF-Embedded`, `glTF-Draco`, `glTF-Quantized`, `glTF-KTX-BasisU`). CI runs the validator over every asset. The extension registry defines prefixes (`KHR` reserved for Khronos, `EXT` for multi-vendor, 99 registered vendor prefixes obtained by GitHub issue), naming rules, a five-stage status ladder (Proposal, Initial Draft, Review Draft, Release Candidate, Ratified) and the rule that extensions "can't remove existing glTF properties or redefine existing glTF properties".

## Evidence

- Report schema: `uri`, `mimeType` (`model/gltf+json` or `model/gltf-binary`), `validatorVersion` (semver), `validatedAt` (date-time), `issues.{numErrors,numWarnings,numInfos,numHints,messages,truncated}`, message `severity` enum 0–3 with descriptions Error/Warning/Information/Hint, `pointer` (json-pointer) or `offset` required — https://github.com/KhronosGroup/glTF-Validator/blob/main/docs/validation.schema.json (retrieved 2026-08-29).
- CLI: report written to `<asset_filename>.report.json`, recursive directory validation, "Shell return code will be non-zero if at least one error was found"; options `-o/--stdout`, `-r/--validate-resources`, `-t/--write-timestamp`, `-p/--absolute-path`, `-m/--messages`, `-a/--all`, `-c/--config`, `-h/--threads` — `README.md` (main).
- Config file: `max-issues`, `ignore` list, `only` list, `override` severity map with "0 - Error, 1 - Warning, 2 - Info, 3 - Hint" — `docs/config-example.yaml`.
- Implemented feature classes: JSON syntax and GLB correctness, schema properties, reference validity, Data URI, accessor values (NaN, invalid quaternions, indecomposable matrices), `accessor.min/max`, sparse accessors, animation I/O, image NPOT and unsupported features, extension validation for `EXT_texture_webp`, `KHR_animation_pointer` (partial), `KHR_lights_punctual`, `KHR_materials_anisotropy`, ... — `README.md` "Implemented features".
- Issue table: 159 codes; category sizes IoError 1, SchemaError 14, SemanticError 41, LinkError 52, DataError 35, GlbError 16 — `ISSUES.md` (main, retrieved 2026-08-29).
- Specific codes: `UNEXPECTED_PROPERTY` Warning (line 20), `INVALID_EXTENSION_NAME_FORMAT` Warning (42), `NON_RELATIVE_URI` Warning (73), `UNKNOWN_ASSET_MAJOR_VERSION` Error / `UNKNOWN_ASSET_MINOR_VERSION` Warning (78–79), `INCOMPLETE_EXTENSION_SUPPORT` Information (102), `UNDECLARED_EXTENSION` Error (128), `UNEXPECTED_EXTENSION_OBJECT` Error (129), `UNSUPPORTED_EXTENSION` Information (131), `UNUSED_OBJECT` Information (134) — `ISSUES.md`.
- Validator test corpus: `test/base/data/<category>/<case>.gltf` paired with `<case>.gltf.report.json` golden reports (e.g. `accessor/alignment.gltf.report.json`, `custom_property.gltf.report.json`); categories accessor, animation, asset, buffer, buffer_view, camera, glb, image, json, material, mesh, node, root, sampler, scene, skin, texture and `_data` variants; `test/ext` for extensions — repository listing (retrieved 2026-08-29).
- Latest tags `2.0.0-dev.3.10`, `2.0.0-dev.3.8`, `2.0.0-dev.3.7`; distribution via npm `gltf-validator`, hosted drag-and-drop tool at https://github.khronos.org/glTF-Validator — `README.md`, tags list.
- Sample assets purpose and lists: Showcase, Complete, Testing ("intended to be used for testing of viewers, converts, and other software systems"), Core Only, Video Tutorials, Written Tutorials, PBR tests, Issues — https://github.com/KhronosGroup/glTF-Sample-Assets/blob/main/README.md (retrieved 2026-08-29).
- Provided forms: separate-resource `.gltf`, embedded Data URI (to be avoided except for specific cases), binary `.glb` — same README, "Model Contents".
- `Models/model-index.json` entries with `label`, `name`, `screenshot`, `tags`, `variants` (e.g. `ABeautifulGame` with `glTF`, `glTF-Binary`, `glTF-Binary-KTX-ETC1S-Draco`; `AlphaBlendModeTest` tagged `core`, `testing`) — file head (retrieved 2026-08-29); 162 model directories under `Models/` (listing).
- Per-model `metadata.json` with `version: 2`, `legal[]` (license, licenseUrl, artist, year, owner, what, spdx), `tags`, `screenshot`, `name`, `path`, `summary`, `createReadme` — `Models/Box/metadata.json`, `Models/NegativeScaleTest/metadata.json`.
- Sample-assets CI installs validator 2.0.0-dev.3.10 and runs `./gltf_validator -r -a ./Models/`, uploading `**/*.report.json` — `.github/workflows/ci.yml` lines 1–30.
- The archived glTF-Sample-Models repository was replaced; unlicensed assets removed; `2.0` renamed `Models` — sample-assets README "Obsolete Interface".
- Prefix registry: `KHR` and `EXT` reserved; 99 registered vendor prefixes (ADOBE, AGI, AMZN, BLENDER, CESIUM, EPIC, GODOT, GOOGLE, MSFT, NV, OMI, UNITY, VRMC, ...); request "by submitting an issue on GitHub" with prefix and vendor name — https://github.com/KhronosGroup/glTF/blob/main/extensions/Prefixes.md and `extensions/README.md` lines 117–121 (retrieved 2026-08-29).
- Naming rules: uppercase prefix plus underscore, lowercase snake-case, recommended `<PREFIX>_<scope>_<feature>` — `extensions/README.md` lines 184–191.
- Status ladder table (Proposal, Initial Draft, Review Draft, Release Candidate, Ratified); Review Draft requires "At least one third party glTF implementation"; Release Candidate requires Sample Viewer and Validator support — `extensions/README.md` lines 66–75.
- "Extensions can't remove existing glTF properties or redefine existing glTF properties to mean something else"; `extensionsUsed` vs `extensionsRequired`; required iff "a typical glTF loader would fail to load the asset in the absence of support" — `extensions/README.md` lines 131–175.
- Extension schemas "should allow additional properties"; `extras` is the application-specific escape hatch distinct from extensions — `extensions/README.md` lines 178–215.
- Current in-progress KHR list includes `KHR_interactivity` (Release Candidate), `KHR_gaussian_splatting` (Release Candidate), `KHR_texture_procedurals` (Initial Draft) — `extensions/README.md` lines 45–57.

## Mechanism

The validator is a single-pass structural checker followed by link and data passes. Schema errors come from the JSON Schema of the core spec; semantic errors from cross-field constraints; link errors from index references between arrays (accessor → bufferView → buffer, node → mesh, ...); data errors from decoding buffers and images; GLB errors from container parsing. Every finding is a coded issue with a fixed severity and a locator (JSON pointer for the JSON tree, byte offset for GLB). Severities are policy, not structure: a YAML config can override any code's severity, ignore codes, or restrict to a subset, and a `max-issues` cap sets `truncated`. The exit code depends only on error count, so warnings never break a pipeline unless overridden to errors.

Extension handling encodes the ecosystem's preservation contract. Any extension object must be declared in `extensionsUsed` (else Error). An extension the validator does not implement is reported as Information, not failure, so vendor extensions pass validation by default; partially implemented extensions are flagged as `INCOMPLETE_EXTENSION_SUPPORT`. Unknown properties outside `extensions`/`extras` are Warnings. This makes "opaque but declared" the validated state, and "opaque and undeclared" the failing state.

The test corpus is golden-report based: every input asset has a checked-in expected report, so validator changes are detected as report diffs. The sample-asset corpus is separately tagged by purpose; the `testing` tag marks assets that exercise a feature edge (alpha modes, negative scale, NPOT textures) and the `core` tag marks assets that need no extension. CI validates the entire corpus, and the extension status ladder requires validator and sample-viewer support before Release Candidate, closing the loop between spec, corpus and checker.

Prefix registration is deliberately lightweight: a GitHub issue reserves a namespace, and promotion from vendor to `EXT` requires multiple implementations while `KHR` requires Khronos ratification and IP coverage. Naming rules make ownership and scope parseable from the identifier.

## NUIF relevance

**Borrow**
- Publish a JSON Schema for NUIF `validate` output with counts per severity, `truncated`, coded messages, and a locator (entity/property path or byte offset for binary profiles), mirroring `validation.schema.json`.
- Maintain a single issue-code table with fixed default severities and categories (schema, semantic, link, data, container), and require every diagnostic to cite a code.
- Make severity policy configurable (ignore, only, override, max-issues) while keeping exit status defined by error count, as the validator does.
- Treat "unknown but declared extension" as Information and "undeclared extension object" as Error, which is the executable form of NUIF's used/required rule.
- Build the conformance fixture corpus with per-asset `metadata.json` (SPDX license, tags, summary, variants) and a generated index; tag fixtures `core`, `extension`, `testing`, `issues`.
- Store golden validation reports alongside fixtures so validator regressions surface as diffs.
- Adapt the prefix registry model to NUIF's lowercase identifier grammar (`nuif.*`, `ext.*`, collision-resistant vendor namespaces) and adopt the status ladder that requires validator and reference-viewer support before ratification.

**Adapt**
- glTF's locator is a JSON pointer, which is path-based; NUIF locators must use stable semantic IDs with an optional path hint since NUIF identity is path-independent.
- The validator does not check preservation across a round trip; NUIF conformance must add a preservation suite (import → export → compare) that the glTF kit lacks.
- Format variants (`glTF-Binary`, `glTF-Embedded`, ...) map to NUIF profiles (`nuif-text-0`, `nuif-cbor-0`, package); NUIF should require every fixture in every profile with canonical-hash equality.
- `extras` as an untyped escape hatch conflicts with NUIF's requirement that extension data be namespaced; NUIF should route such data through a vendor extension instead.

**Reject**
- Dart as an implementation language is irrelevant; NUIF's validator is the Rust CLI.
- Severity-only categorization without fidelity classes is insufficient; NUIF diagnostics must also carry the fidelity class (`lossless` ... `unsupported`) and the responsible pass.
- The sample corpus's mixed CC-BY/CC0 licensing complicates redistribution in test suites; NUIF fixtures should be CC0 or project-licensed.

## Open questions

- Whether NUIF should reserve numeric severity values (0–3) for compatibility with tooling that consumes glTF-style reports.
- How to represent multi-profile locators (text line/column, CBOR byte offset, semantic ID) in one report entry without ambiguity.
- Whether the validator's "unsupported extension is Information" default is safe for NUIF where an unrenderable required extension must be a conformance failure at render time but not at parse time.
- How NUIF's extension status ladder should handle dialects that change lowering rules rather than add properties.
