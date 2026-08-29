---
id: nuif:research:hydra-render-delegate
kind: implementation
status: reviewed
title: Hydra scene index, render delegate abstraction, dirty tracking and image-diff testing
source:
  url: https://openusd.org/release/api/_page__hydra__getting__started__guide.html
  repository: https://github.com/PixarAnimationStudios/OpenUSD
  authors: [Pixar Animation Studios]
  published_at: "v26.08 (release branch ee47c679, 2026-07-20)"
  license: "TOST (Tomorrow Open Source Technology) license, Apache-2.0-derived"
retrieved_at: 2026-08-29
tags: [hydra, render-delegate, scene-index, dirty-bits, change-tracking, pluggable-backend, image-diff, golden-images, testing]
confidence: 0.9
claims: [nuif:claim:multi-level-ir, nuif:claim:sync-not-regenerate]
relations:
  - type: extends
    target: nuif:research:openusd
    note: Hydra is the imaging layer that consumes a composed USD stage.
  - type: supports
    target: nuif:research:renderers
    note: Concrete precedent for a stable scene abstraction with pluggable backends.
  - type: related_to
    target: nuif:research:vello
    note: Vello/wgpu would sit behind a NUIF render-scene boundary the way Storm sits behind Hydra.
  - type: compares_to
    target: nuif:research:mlir
    note: Filtering scene indices are rewrite passes over a scene IR.
links:
  spec: [spec/12-cli-api-and-automation.md, spec/00-conformance.md, spec/06-operations-and-patches.md]
  adr: [adrs/0003-reference-renderer.md]
  rfc: [rfcs/0004-headless-qa-contract.md]
  code: [crates/nuif-render, crates/nuif-api]
  experiments: [conformance/PLAN.md]
---

# Summary

Hydra is Pixar's imaging framework whose stated goal is "to decouple scene processing from rendering, and both from the application". Hydra 1.0 used three abstractions: `HdSceneDelegate` (adapter to a client scene graph), `HdRenderIndex` (a flattened representation of the scene that tracks changes through `HdChangeTracker` dirty bits and orchestrates prim sync) and `HdRenderDelegate` (the backend that creates rprims, sprims and bprims and owns a resource registry). Renderers are discovered at run time as `HdRendererPlugin` instances through the Plug system. Hydra 2.0 replaces scene delegates with `HdSceneIndexBase` (a queryable prim tree of nested data sources), replaces the render index with a graph of filtering scene indices, and replaces coarse dirty bits with hierarchical `HdDataSourceLocator` invalidation; legacy delegates and render delegates are wrapped by adapter classes.

Pixar tests Hydra at two levels. Core `hd` tests are C++ unit tests (`testHdSceneIndex`, `testHdDirtyBitsTranslator`, `testHdMergingSceneIndex`, `testHdDataSourceLocator`, ...) that use a recording observer to assert exact notification sequences. Imaging tests (`testUsdImagingGL*`) render a stage offscreen to PNG and compare against checked-in baselines using OpenImageIO `idiff` with per-test pixel and percentage thresholds passed from CMake through `cmake/macros/testWrapper.py`.

## Evidence

- Design goal and definitions of scene index, scene index observer, filtering scene index, scene index plugin — Hydra 2.0 Getting Started Guide, section "What is Hydra 2.0", https://openusd.org/release/api/_page__hydra__getting__started__guide.html (v26.08 docs, retrieved 2026-08-29).
- Mapping from 1.0 to 2.0: `HdSceneDelegate` → scene indices, `HdRenderIndex` → scene index graph via `HdSceneIndexPluginRegistry`, `HdRenderDelegate` → `HdRenderer`; adapters `HdRenderDelegateAdapterRenderer` and `HdRenderIndexAdapterSceneIndex`; env toggles `HD_ENABLE_SCENE_INDEX_EMULATION`, `USDIMAGINGGL_ENGINE_ENABLE_SCENE_INDEX` — same guide, section "How does Hydra 2.0 compare to Legacy Hydra 1.0?".
- `HdSceneIndexBase` interface: `GetPrim`, `GetChildPrimPaths`, `_SendPrimsAdded/Removed/Dirtied/Renamed`; invariant that add/remove notices must match traversal via `GetChildPrimPaths` — same guide, section "The Scene Index API"; `pxr/imaging/hd/sceneIndex.h` lines 48–121, 181–207.
- `HdSceneIndexObserver` entries: `AddedPrimEntry` acts as resync if the path exists; `RemovedPrimEntry` is a subtree; `DirtiedPrimEntry` carries `HdDataSourceLocatorSet` interpreted hierarchically; `PrimsRenamed` is an optimization over remove+add — same guide.
- Data source model: `HdContainerDataSource::GetNames/Get`, `HdVectorDataSource`, `HdSampledDataSource::GetValue(shutterOffset)` and `GetContributingSampleTimesForInterval` — same guide, section "Prim Data".
- `HdRenderIndex` documented as "a flattened representation of the client scene graph", tied to a single `HdRenderDelegate`, tracking changes via `HdChangeTracker`, orchestrating "syncing"; now "only used for emulation purposes" — `pxr/imaging/hd/renderIndex.h` lines 63–107; `SyncAll`, `GetChangeTracker`, `InsertRprim`, `InsertSceneIndex`, `_emulationSceneIndex`, `_mergingSceneIndex`, `_terminalSceneIndex` — lines 204–590.
- `HdChangeTracker` "Tracks changes from the HdSceneDelegate, providing invalidation cues to the render engine"; flags accumulate until the resource is next required — `pxr/imaging/hd/changeTracker.h` lines 6–14; `RprimDirtyBits` includes `Clean`, `InitRepr`, `Varying`, `DirtyPoints`, `DirtyTopology`, `DirtyPrimvar`, `DirtyTransform`, `DirtyVisibility`, `DirtyNormals`, `DirtyMaterialId`, `DirtyInstancer`, `DirtyRenderTag`, `AllDirty`; version counters `GetSceneStateVersion`, `GetRprimIndexVersion`, `GetVisibilityChangeCount`, `GetRenderTagVersion` — same header (retrieved via WebFetch 2026-08-29).
- `HdRenderDelegate` pure virtuals `GetSupportedRprimTypes/SprimTypes/BprimTypes`, `GetResourceRegistry`, `CreateRprim/Sprim/Bprim/Instancer`, `CreateRenderPass`, `CommitResources(HdChangeTracker*)`; optional `GetRenderSettingDescriptors`, `GetCapabilities`, `IsPauseSupported`, `IsParallelSyncEnabled` — `pxr/imaging/hd/renderDelegate.h` lines 106–551.
- `HdRenderParam` is "an opaque (to core Hydra) handle" passed to prims during sync — `renderDelegate.h` lines 43–45.
- `HdRendererPlugin`: "dynamically discovered and loaded at run-time using the Plug system", singleton per library, `IsSupported(reasonWhyNot)` — `pxr/imaging/hd/rendererPlugin.h` lines 23–54.
- `HdSceneDelegate` is "Adapter class providing data exchange with the client scene graph" — `pxr/imaging/hd/sceneDelegate.h` line 400–402.
- Core unit tests registered in `pxr/imaging/hd/CMakeLists.txt`: `testHdSceneIndex`, `testHdDirtyBitsTranslator`, `testHdDirtyList`, `testHdMergingSceneIndex`, `testHdDataSourceLocator`, `testHdDataSource`, `testHdSortedIds*`, `testHdTimeSampleArray`, `testHdExtCompDependencySort` (listing retrieved 2026-08-29).
- `testHdSceneIndex.cpp` defines `RecordingSceneIndexObserver` with `EventType_PrimAdded/Removed/Dirtied`, hashes events and compares event vectors and `GetChildPrimPaths` results via `_CompareValue` — `pxr/imaging/hd/testenv/testHdSceneIndex.cpp` lines 100–606.
- Image tests: `pxr_register_test` accepts `IMAGE_DIFF_COMPARE`, `WARN`, `WARN_PERCENT`, `HARD_WARN`, `FAIL`, `FAIL_PERCENT`, `HARD_FAIL`, `PERCEPTUAL`, `DIFF_COMPARE`, `EXPECTED_RETURN_CODE`, `TESTENV`, `ENV`, `PRE_COMMAND`, `POST_COMMAND` — `cmake/macros/Public.cmake` lines 772–784, 892–946.
- `testWrapper.py` `_imageDiff` shells out to `idiff` (`idiff.exe` on Windows) with `-warn`, `-warnpercent`, `-hardwarn`, `-fail`, `-failpercent`, `-hardfail`, `-p`; return codes 0 OK, 1 warning, 2 failure, 3 size mismatch, 4 file error; only 0 and 1 pass; failing pairs copied to `--failures-dir` — `cmake/macros/testWrapper.py` lines 199–260.
- Baseline lookup `_resolvePath` checks a `non-specific` subdirectory before the platform baseline directory; text diffs use system `diff --strip-trailing-cr` (`fc.exe` on Windows) — `testWrapper.py` lines 113–197.
- Concrete thresholds: `testUsdImagingGLBasicDrawing` uses `FAIL 0.2 FAIL_PERCENT 0.5 PERCEPTUAL`; `testUsdImagingGLInstancing_instancedCubes` uses `FAIL 0.01 FAIL_PERCENT 0.005 WARN 0.02 WARN_PERCENT 0.0025`; `testUsdImagingGLPurpose` compares four images with `FAIL 0.1 FAIL_PERCENT 10 PERCEPTUAL` — `pxr/usdImaging/usdImagingGL/CMakeLists.txt` (retrieved 2026-08-29).
- Test binaries `testUsdImagingGLBasicDrawing`, `...Highlight`, `...PickAndHighlight`, `...InstancePicking`, `...Resync`, `...SurfaceShader`, `...SublayerOperations`, `...Purpose`, `...PopOut`, `...TextureResync`; tests skipped on macOS, Windows, headless and static builds — same CMakeLists.
- Baselines are checked-in PNGs, e.g. `pxr/usdImaging/usdImagingGL/testenv/testUsdImagingGLBasicDrawing/baseline/testUsdImagingGLBasicDrawing.png`, `_refined.png`, `_shadersAnim_001.png`; the `usdImagingGL/testenv` tree holds 119 entries (listing retrieved 2026-08-29).
- `testUsdImagingGLBasicDrawing.cpp` selects the renderer plugin via `SetRendererPlugin(_GetRenderer())` and writes the color AOV with `WriteToFile(_engine.get(), HdAovTokens->color, imageFilePath)` — lines 84–307.

## Mechanism

Hydra 1.0 is a retained-mode pipeline. The application inserts prims into the render index by type id and path; the render delegate instantiates backend-specific `HdRprim`/`HdSprim`/`HdBprim` objects for the types it advertises. Scene edits do not rebuild the scene; the scene delegate marks dirty bits on the change tracker. When a task executes, the render index computes the set of prims needing sync, calls `Sync` on each with the delegate as data source and the dirty bits as the work list, then clears the bits. Version counters (scene state, index versions, visibility, render tags) let consumers detect coarse changes without walking prims. `CommitResources` gives the backend a barrier after sync. This is the sync-not-regenerate pattern: invalidation is fine-grained, pull-based and cleared on consumption.

Hydra 2.0 replaces the render index with a scene index graph. Each scene index is both a query surface (`GetPrim`, `GetChildPrimPaths`) and a notification source; filtering scene indices compose as a chain, each observing the previous. Invalidation is addressed by `HdDataSourceLocator` paths into nested container data sources and interpreted hierarchically, replacing the fixed dirty-bit vocabulary with an open, structured one. Sampled data sources expose time-varying values through shutter offsets and contributing sample times so renderers can reconstruct motion blur without a separate API. Emulation classes wrap old delegates and render delegates so both worlds interoperate during the transition.

Testing has an exact tier and a tolerance tier. The exact tier records observer events from a scene index under scripted mutations and compares them to expected sequences, and checks that traversal agrees with notifications (the documented invariant). The tolerance tier runs a renderer plugin offscreen, writes an AOV to disk and delegates comparison to `idiff` with declared per-pixel thresholds (`FAIL`), fraction-of-pixels thresholds (`FAIL_PERCENT`), an absolute per-pixel ceiling (`HARD_FAIL`) and an optional perceptual metric. Baselines are per-platform directories with a `non-specific` fallback, and thresholds are declared per test rather than globally.

## NUIF relevance

**Borrow**
- Separate a queryable scene abstraction from backends, with backends declaring supported prim types and capabilities, as `HdRenderDelegate::GetSupportedRprimTypes` and `GetCapabilities` do.
- Use pull-based, hierarchical invalidation (locator sets over nested data sources) between the NUIF resolved model and render backends instead of regenerating the render scene on each edit.
- Require the notification/traversal consistency invariant (adds and removes must equal a fresh traversal) and test it with a recording observer, as `testHdSceneIndex` does.
- Declare image-comparison tolerances per fixture (`FAIL`, `FAIL_PERCENT`, `HARD_FAIL`, perceptual) and keep per-platform baselines with a platform-neutral fallback directory.
- Treat the render-scene boundary as an emulation point: adapters allow old and new backends to coexist during migration, which NUIF should plan for around ADR 0003.

**Adapt**
- Hydra's shutter-offset sampling model maps to NUIF evaluation contexts (viewport, theme, state); NUIF should parametrize data-source queries by context rather than by time.
- Dirty bits are backend-facing; NUIF must additionally map invalidation back to authored entities for fidelity and provenance, which Hydra does not attempt.
- Hydra's baselines are opaque PNGs; NUIF conformance should also compare structured render plans (vector display lists) before rasterization so failures are attributable.
- `idiff` thresholds are chosen per test by hand; NUIF should derive tolerances from the normative text-rendering and anti-aliasing allowances in spec/05 and record them in fixtures.

**Reject**
- The Plug-system dynamic discovery of renderer plugins is not needed for NUIF conformance; backends can be static Rust trait implementations.
- Hydra skips image tests on macOS, Windows, headless and static builds; NUIF must run its reference renderer headlessly on all platforms because the headless QA contract is normative.
- The 1.0 fixed dirty-bit enum should not be copied; NUIF should start from locator-style structured invalidation.

## Open questions

- Which perceptual metric `idiff -p` implements and whether an equivalent is acceptable for text-heavy UI renders where small shifts are semantically significant.
- How to express NUIF evaluation-context dimensions in a locator scheme so a change to a token or breakpoint invalidates exactly the dependent resolved values.
- Whether NUIF render backends should expose a `CommitResources`-style barrier or rely on immutable snapshot handoff.
- How much of the 2.0 filtering scene index pattern maps onto NUIF lowering passes (authored → resolved → render scene) versus onto editor-side view transforms.
