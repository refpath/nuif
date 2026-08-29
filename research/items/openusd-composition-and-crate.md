---
id: nuif:research:openusd-composition-and-crate
kind: implementation
status: reviewed
title: OpenUSD composition strength ordering, crate binary format, flattening and validation
source:
  url: https://github.com/PixarAnimationStudios/OpenUSD/tree/release
  repository: https://github.com/PixarAnimationStudios/OpenUSD
  authors: [Pixar Animation Studios, Alliance for OpenUSD]
  published_at: "v26.08 (release branch ee47c679, 2026-07-20)"
  license: "TOST (Tomorrow Open Source Technology) license, Apache-2.0-derived"
retrieved_at: 2026-08-29
tags: [composition, layers, variants, payloads, instancing, binary-format, deduplication, lazy-loading, flattening, validation, versioning]
confidence: 0.9
claims: [nuif:claim:multi-level-ir, nuif:claim:authored-resolved, nuif:claim:opaque-preservation]
relations:
  - type: extends
    target: nuif:research:openusd
    note: Deepens the composition summary with strength ordering, the crate encoding, the flatten lowering and the validation framework.
  - type: compares_to
    target: nuif:research:alembic
    note: A flattened stage is the USD analogue of an Alembic cache; USD keeps arcs, Alembic never has them.
  - type: related_to
    target: nuif:research:mlir
    note: Flattening is an explicit lowering from composed to non-composed scene description.
  - type: related_to
    target: nuif:research:encoding
    note: Crate is a concrete precedent for a deduplicating, versioned, random-access binary profile.
links:
  spec: [spec/03-components-and-composition.md, spec/08-serialization.md, spec/00-conformance.md, spec/07-extensions-and-dialects.md]
  adr: [adrs/0004-serialization.md]
  rfc: [rfcs/0001-multi-level-document-model.md, rfcs/0002-extension-preservation.md]
  code: [crates/nuif-codec, crates/nuif-core]
  experiments: [conformance/PLAN.md]
---

# Summary

OpenUSD composes a stage from layers connected by composition arcs. Opinions are resolved per layer stack in a fixed strength order, now spelled LIVERPS in the glossary (Local, Inherits, VariantSets, rElocates, References, Payloads, Specializes); Inherits and VariantSets recurse with Specializes excluded. Composition is non-destructive: authored opinions remain in their layers and the stage produces a composed view. Payloads are weaker than references and may be deferred at load; instancing shares composed prototypes for prims marked `instanceable` that carry direct arcs. A prim's `typeName` is plain metadata; unregistered type names compose and round-trip unchanged, and a `fallbackPrimTypes` layer-metadata dictionary lets older software map unknown types to known ones. Properties outside any schema are marked `custom`, a category the USD headers equate with Alembic `userProperties`.

The crate format (`.usdc`) is a versioned binary container with an eight-byte magic, a table of contents of six named sections (TOKENS, STRINGS, FIELDS, FIELDSETS, PATHS, SPECS), 64-bit value representations with inline/array/compressed flag bits, write-time deduplication tables for values and arrays, integer compression for structural sections, LZ4 for bulk data and mmap or pread access with zero-copy arrays. The software version is 0.15.0 but new files default to 0.8.0 and are upgraded only when a feature requires it; saving an existing file preserves its version. `usdcat --flatten` and `UsdStage::Flatten` lower a composed stage to a single arc-free layer, and the glossary states this loses sharing and multiplies data. Validation moved from a Python `complianceChecker` to a plugin-based `UsdValidation` framework with typed error severities and fixers; `usdchecker` is now a C++ front end over that framework.

## Evidence

- LIVERPS ordering and the recursion rule "we ignore Specializes arcs while recursing" — https://openusd.org/release/glossary.html#liverps-strength-ordering (v26.08 docs, retrieved 2026-08-29).
- Layer stack definition: "ordered set of layers resulting from the recursive gathering of all SubLayers of a Layer, plus the layer itself as first and strongest"; arcs target layer stacks, not layers — https://openusd.org/release/glossary.html#layer-stack.
- Sublayers are the arc that builds layer stacks and accept layer offsets — https://openusd.org/release/glossary.html#sublayers.
- Root layer stack: session layer plus root layer; edit targets can only target root-layer-stack prim specs — https://openusd.org/release/glossary.html#root-layer-stack.
- Payloads are recorded but not traversed under `UsdStage::InitialLoadSet::LoadNone`, and are weaker than references — https://openusd.org/release/glossary.html#payload.
- Instancing shares composed prims across instances and forfeits per-instance overrides beneath the instance root — https://openusd.org/release/glossary.html#instancing; `instanceable` requires a direct composition arc on the prim — https://openusd.org/release/glossary.html#instanceable.
- VariantSets are "a switchable reference"; a variant may contain arbitrary scene description and further arcs — https://openusd.org/release/glossary.html#variantset, #variant.
- Relocates arc: layer-metadata path mapping for non-destructive rename/reparent across arcs — https://openusd.org/release/glossary.html#relocates.
- All arcs except subLayers are list-editable (prepend, append, remove, reset) — https://openusd.org/release/glossary.html#composition-arcs.
- Flatten: text flattening "will generally produce extremely large files" because referenced assets are uniquely baked; crate mitigates via deduplication; `UsdStage::Flatten`, `usdcat --flatten`, `UsdUtilsFlattenLayerStack`, `usdcat --flattenLayerStack` — https://openusd.org/release/glossary.html#flatten.
- Toolset flag text for `usdcat --flatten`, `--flattenLayerStack`, `--usdFormat usda|usdc`, `usddiff -f`, `usdtree --flatten` — https://openusd.org/release/toolset.html.
- Crate glossary: `.usdc` is "losslessly, bidirectionally convertible to the .usda text format"; crate reads only a small index at open and defers big data; mmap or pread selectable at runtime — https://openusd.org/release/glossary.html#crate-file-format.
- Crate version history comment listing 0.0.1 through 0.15.0 (0.4.0 compressed structural sections, 0.5.0/0.6.0 compressed arrays, 0.7.0 64-bit array sizes, 0.8.0 payload list ops, 0.9.0 timecode, 0.10.0 pathExpression, 0.11.0 relocates, 0.12.0–0.15.0 splines and ArrayEdits) — `pxr/usd/sdf/crateFile.cpp` lines 384–411 at commit ee47c679.
- `OLDEST_SUPPORTED_VERSION "0.0.1"`, `OLDEST_CURRENT_VERSION "0.8.0"`, `DEFAULT_NEW_VERSION "0.8.0"`; env setting `USD_WRITE_NEW_USDC_FILES_AS_VERSION` documented as "saving edits to an existing file preserves its version" — `crateFile.cpp` lines 157–168.
- `RequestWriteVersionUpgrade(Version, reason)` promotes the write version only when a value type requires it (e.g. 0.8.0 for payload layer offsets, 0.9.0 timecode, 0.11.0 relocates, 0.12.0/0.13.0/0.15.0 splines) — `crateFile.cpp` lines 1062–1073, 1506–1642.
- `SdfFileVersion::CanRead` / `CanWrite` predicates — `pxr/usd/sdf/fileVersion.h` lines 76–83; `CrateFile::Version` is an alias of `SdfFileVersion` — `crateFile.h` line 268.
- Bootstrap struct: `uint8_t ident[8]; // "PXR-USDC"`, `uint8_t version[8]`, `int64_t tocOffset` — `crateFile.h` lines 479–484; `USDC_IDENT = "PXR-USDC"` — `crateFile.cpp` line 434.
- Section names TOKENS, STRINGS, FIELDS, FIELDSETS, PATHS, SPECS and `_KnownSections` — `crateFile.cpp` lines 266–275; writer emits them in that order then the bootstrap — lines 2897–2906.
- `ValueRep` bit layout: `_IsArrayBit = 1<<63`, `_IsInlinedBit = 1<<62`, `_IsCompressedBit = 1<<61`, `_IsArrayEditBit = 1<<60`, `_PayloadMask = (1<<48)-1` — `crateFile.h` lines 84–89.
- Index types `FieldIndex`, `FieldSetIndex`, `PathIndex`, `StringIndex`, `TokenIndex`; `Field {TokenIndex, ValueRep}`; `Spec {PathIndex, SdfSpecType, FieldSetIndex}` — `crateFile.h` lines 230–234, 507–562.
- Write-time deduplication tables `_valueDedup`, `_arrayDedup`, `_arrayEditDedup` keyed by value hash — `crateFile.cpp` lines 1090, 1700–1760, 1903–1907; times deduplicated by ValueRep — line 1343.
- Structural sections compressed with `Sdf_IntegerCompression` (`CompressToBuffer`) — `crateFile.cpp` lines 3057–3179, 3365–3382; bulk reps and token data compressed with `TfFastCompression` (LZ4 wrapper) — lines 3072–3074, 3413–3414.
- Access paths: `_MmapStream` (zero-copy arrays), `_PreadStream`; env settings `USDC_MMAP_PREFETCH_KB`, `USDC_ENABLE_ZERO_COPY_ARRAYS`, `USDC_USE_ASSET` — `crateFile.cpp` lines 171–190, 599–712, 2332–2420.
- Deprecation warning for files older than 0.8.0 (`PXR_USDC_EMIT_DEPRECATION_WARNINGS`) — `crateFile.cpp` lines 192–195.
- Tools `usddumpcrate` ("Write information about a usd crate (usdc) file") and `usdupdatecrate` exist under `pxr/usd/bin/` — repository listing at ee47c679; `usddumpcrate.py` line 33.
- Text format: `SdfUsdaFileFormat` with version token "1.0", `GetMinInputVersion`/`GetMaxOutputVersion`, `SaveToFile` "starting with the loaded layer's file version and upgrading as needed" — `pxr/usd/sdf/usdaFileFormat.h` lines 29, 69–123.
- `UsdPrim::GetTypeName` returns "the composed type name as authored and may not represent the full type"; `SetTypeName` writes `SdfFieldKeys->TypeName` metadata — `pxr/usd/usd/prim.h` lines 192–204.
- IsA schema membership derives from `typeName`; a prim subscribes to at most one IsA schema — https://openusd.org/release/glossary.html#isa-schema.
- Fallback prim types: `fallbackTypes` customData in `schema.usda`, `UsdStage::WriteFallbackPrimTypes`, and "prims with the unrecognized type name will be treated as having the effective schema type of the first recognized type in the list" — https://openusd.org/release/api/_usd__page__object_model.html (section "Fallback Prim Types").
- `UsdProperty::IsCustom`: the `custom` modifier "serves the same function as Alembic's 'userProperties'" for ad hoc client data outside any schema — `pxr/usd/usd/property.h` lines 179–185.
- UsdValidation framework: validators with metadata (name `pluginName:validatorName`, keywords, schemaTypes, isSuite), `UsdValidationContext` running validators in parallel, error types None/Error/Warn/Info, error sites, fixers — `pxr/usdValidation/usdValidation/README.md`; `enum class UsdValidationErrorType { None, Error, Warn, Info }` — `pxr/usdValidation/usdValidation/error.h` lines 37–42.
- `usdchecker` is C++ (`pxr/usdValidation/bin/usdchecker/usdchecker.cpp`): options `--includeKeywords`, `--noAssetChecks`, `-t, --strict` ("Return failure code even if only warnings are issued"), `--variantSets`, `--variants`, `--disableVariantValidationLimit`, `--rootPackageOnly`, `--skipVariants`, `--dumpRules`; default behaviour validates "all possible combinations of variant selections" — lines 56–140; `Warn` escalates to failure only under `strict` — lines 217–218.
- `pxr/usd/usdUtils/complianceChecker.py` no longer exists on the release branch (HTTP 404 at ee47c679, 2026-08-29); validators live under `pxr/usdValidation/{usdGeomValidators,usdShadeValidators,usdSkelValidators,usdUtilsValidators,...}`.

## Mechanism

Value resolution for "strongest wins" fields walks the prim index in LIVERPS order within each layer stack. Local opinions are consulted across the sublayer-expanded stack first. Inherits and VariantSets targets are then composed recursively with Specializes suppressed, so a specialized base can never override an inherited or variant opinion. Relocates remap remote paths into the local namespace before References and Payloads are followed. Specializes is consulted last. Because arcs target layer stacks and every arc except subLayers is a list op, a downstream layer can prepend, append, remove or reset arcs without touching upstream files. This yields the non-destructive property: the union of all authored opinions is preserved as data, and the composed result is a pure function of that data plus load state (payload inclusion, variant selections, session layer).

Type identity is metadata. `typeName` composes like any other field; the schema registry maps it to a `UsdPrimTypeInfo` when known. Unknown type names produce prims with `IsA` false for every schema but with all authored properties intact. `fallbackPrimTypes` is a forward-compatibility contract: the writer records substitutes so an older reader treats the prim as the first recognized fallback. Schema-less properties survive as `custom` properties. Together these define USD's opaque-preservation behaviour: preservation is structural (unknown tokens and fields round-trip through Sdf) rather than a byte-level blob.

Crate is index-first. The bootstrap at offset 0 holds the magic, a three-byte semantic version and the TOC offset; the TOC lists named sections with start and size. TOKENS and STRINGS are interned pools; PATHS is a compressed path tree; FIELDS pairs a token index with a 64-bit `ValueRep`; FIELDSETS are index runs terminated by a sentinel; SPECS map a path index and spec type to a field set. A `ValueRep` encodes the type enum, an inline flag (small values stored in the 48 payload bits), array and compressed flags, and otherwise a file offset. On write, every non-inlined value and array is hashed into deduplication tables so identical payloads share one offset, which is why the glossary claims crate flattening beats text flattening. Structural integer arrays use `Sdf_IntegerCompression`; bulk reps and token blocks use LZ4. Readers map or pread the file and materialize values on request; numeric arrays whose in-file layout matches memory are exposed zero-copy from the mapping. Versioning is monotone and feature-gated: a writer starts at the default (0.8.0) or the file's existing version and calls `RequestWriteVersionUpgrade` only when a value type demands a newer encoding; readers accept any version from 0.0.1 and warn below 0.8.0.

Flatten is an explicit lowering. `UsdStage::Flatten` evaluates composition (including load and variant state) and emits one layer with no arcs, unique namespaces per referenced instance, and resolved opinions; `UsdUtilsFlattenLayerStack` is a weaker lowering that collapses only the sublayer stack and keeps references, payloads and variants intact. Neither is invertible.

Validation is a registry of named validators with keyword and schema-type metadata; a context selects validators (optionally including ancestor schema types), runs them in parallel, and returns errors with severity, sites and optional fixers. The CLI enumerates variant combinations by default and maps `Warn` to a non-zero exit only in strict mode.

## NUIF relevance

**Borrow**
- Adopt a fixed, documented strength order for NUIF override sources (local, instance override, component variant, token theme, library reference) so resolution is a pure function of authored data, as LIVERPS makes USD composition deterministic.
- Adopt list-edit semantics (prepend, append, remove, reset) for relationship lists so downstream documents can non-destructively edit upstream composition, matching USD arcs.
- Adopt index-first binary layout with interned tokens, hashed value deduplication and a bootstrap-plus-TOC so the `nuif-cbor-0` successor can support lazy random access and cheap flattening.
- Adopt feature-gated, monotone encoding versions with a conservative default write version and "save preserves version", which is how crate avoids forcing upgrades on consumers.
- Adopt the two-tier lowering distinction (flatten layer stack versus flatten everything) as named NUIF lowering passes with fidelity records, since USD documents the losses of each.
- Adopt a validator registry with severity enum, sites, keywords and fixers as the model for NUIF `validate` diagnostics and auto-fix hooks.

**Adapt**
- USD's unknown-type preservation is structural (typeName as metadata plus `custom` properties); NUIF needs the same structural rule plus byte-level preservation for foreign extension payloads that have no NUIF value model.
- `fallbackPrimTypes` is a per-document forward-compatibility map; NUIF should generalize it to dialect-level fallback declarations attached to `extensions_used`.
- Payload-style deferred loading maps to NUIF component libraries and asset references, but NUIF must define load state as part of the evaluation context so resolved output is reproducible.
- Instancing's loss of per-instance overrides beneath the instance root is a precedent for NUIF instance override scoping; NUIF should keep overrides addressable but classify their cost.
- Variant combination validation in `usdchecker` is a model for NUIF responsive/theme context matrices, but NUIF should bound the matrix explicitly rather than via a hidden default limit.

**Reject**
- The 3D namespace model (prim paths, specifiers, kinds, purposes) is not adopted; NUIF identity is path-independent and semantic.
- Crate's reliance on mmap and platform I/O is unsuitable as a normative NUIF requirement; NUIF should specify the logical layout and keep transport choices implementation-defined.
- The absence of a per-prim source provenance record in the flattened output is a gap NUIF must not replicate; flattening in NUIF must emit correspondence records.

## Open questions

- Whether a NUIF text profile can guarantee lossless bidirectional conversion with a binary profile in the presence of opaque byte extensions, as usda and usdc do for USD values.
- How to define the analogue of `fallbackPrimTypes` for NUIF dialect constructs without allowing a fallback to silently change semantics.
- Whether NUIF should expose a `--strict` severity escalation or require explicit severity policies in capability profiles.
- How LIVERPS-style recursion rules translate to NUIF graphs with cycles disallowed but multiple relationship kinds coexisting.
