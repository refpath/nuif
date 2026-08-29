---
id: nuif:research:unreal-asset-versioning-and-automation
kind: implementation
status: reviewed
title: Unreal Engine asset versioning, transactions, automation framework and asset diffing
source:
  url: https://dev.epicgames.com/documentation/en-us/unreal-engine/versioning-of-assets-and-packages-in-unreal-engine
  authors: [Epic Games]
  published_at: "Unreal Engine 5.8 documentation (retrieved 2026-08-29)"
  license: Proprietary documentation; engine source under Unreal Engine EULA (source-available, not open source)
retrieved_at: 2026-08-29
tags: [serialization, versioning, migration, undo, transaction, automation, screenshot-comparison, headless, diff, command-pattern, memento]
confidence: 0.84
claims: [nuif:claim:semantic-automation, nuif:claim:sync-not-regenerate, nuif:claim:opaque-preservation]
relations:
  - type: related_to
    target: nuif:research:unity-prefabs-and-yaml-merge
    note: Unity chose text serialization plus structural merge; Unreal keeps binary packages and per-class diff tooling.
  - type: related_to
    target: nuif:research:blender-dna-rna-and-headless
    note: Both use embedded version keys plus in-code migration; Blender additionally embeds the schema (SDNA), Unreal does not.
  - type: compares_to
    target: nuif:research:gltf
    note: FCustomVersion GUID registration is analogous to an extension registry with per-extension version numbers.
  - type: related_to
    target: nuif:research:structured-merge
    note: The Blueprint diff tool is a class-specific structural diff over graph nodes and pins.
  - type: supports
    target: nuif:research:content-addressed-versioning
    note: Unreal binds package identity to engine version and changelist, showing why schema version must be separate from content identity.
links:
  spec: [spec/06-operations-and-patches.md, spec/08-serialization.md, spec/12-cli-api-and-automation.md, spec/00-conformance.md]
  adr: [adrs/0004-serialization.md]
  rfc: [rfcs/0004-headless-qa-contract.md]
  code: [crates/nuif-cli, crates/nuif-render, crates/nuif-protocol]
  experiments: []
---

# Summary

Unreal Engine serializes assets through `FArchive` with three independent version streams: an Epic engine object version, a licensee object version, and any number of custom versions keyed by `FGuid` and registered at startup. Each `Serialize` implementation declares which custom versions it uses and branches on the stored value, which yields backward compatibility by construction; forward compatibility is refused (newer assets are hidden and references become null). Editor undo is a command-pattern transaction system: `FScopedTransaction` brackets a transaction, `UObject::Modify()` snapshots an object into the transaction buffer before mutation, and `UTransBuffer` keeps an undo/redo stack of `FTransaction` records bounded by memory. The Automation Framework runs C++-registered tests from the Session Frontend or headlessly through `-ExecCmds="Automation RunTests ...;Quit"` with `-nullrhi -unattended`, exports JSON/HTML reports, and includes screenshot comparison with per-channel tolerance, local and global error budgets and anti-aliasing tolerance. Commandlets provide a raw headless execution environment. Asset diffing exports a text form for generic tools and uses a Blueprint-specific graph diff for Blueprints.

## Evidence

- `FEngineVersion` carries major, minor, patch (`uint16`), changelist (`uint32`) and branch name; assets saved in a newer engine "will simply not show up in the Content Browser, and any references to them will be treated as null", with a data-loss risk on re-save. Epic docs, "Versioning of Assets and Packages in Unreal Engine", sections "Engine Version" and loading behaviour, retrieved 2026-08-29.
- Object-level versions: `EUnrealEngineObjectUE5Version` (Epic) and `EUnrealEngineObjectLicenseeUEVersion` (licensee). Custom versions: a `const FGuid` plus a global `FCustomVersionRegistration` object, e.g. `FCustomVersionRegistration GRegisterAnimationCustomVersion(FAnimationCustomVersion::GUID, FAnimationCustomVersion::LatestVersion, TEXT("AnimGraphVer"));`. Same page, section "Custom versions".
- `FArchive` accessors: `UEVer()`, `LicenseeUEVer()`, `UsingCustomVersion(FGuid)`, `CustomVer(FGuid)`; example override calls `Ar.UsingCustomVersion(FFrameworkObjectVersion::GUID)` then branches on `Ar.CustomVer(...) < FFrameworkObjectVersion::WheelOffsetIsFromWheel`. Same page, "Serialize override" example.
- Rule: the version associated with a registered `FGuid` "is assumed never to decrease", which is what lets the engine refuse newer assets while loading older ones. Same page.
- `UsingCustomVersion` "registers the custom version to the archive" and has no effect on loading archives; `CustomVer` queries a custom version and, when writing, requires prior registration. Epic API docs `FArchive::UsingCustomVersion` and `FArchive::CustomVer` (via search summary of the 5.6 API pages), retrieved 2026-08-29.
- `FScopedTransaction`: "Delineates a transactable block; [Begin()]s a transaction when entering scope, and [End()]s a transaction when leaving scope"; header `/Engine/Source/Editor/UnrealEd/Public/ScopedTransaction.h`; constructors take a session `FText` and `bShouldActuallyTransact`; `Cancel()` is reentrant; `Index` stores the transaction index. Epic API docs 5.8, `FScopedTransaction`, retrieved 2026-08-29.
- `UTransBuffer` is the "Transaction tracking system, manages the undo and redo buffer"; members `UndoBuffer: TArray<TSharedRef<FTransaction>>`, `UndoCount`, `MaxMemory` ("Maximum number of bytes the transaction buffer is allowed to occupy"), `ActiveCount`, `ActiveRecordCounts`; `End()` succeeds only when the action counter is 1. Epic API docs 5.8, `UTransBuffer`, retrieved 2026-08-29.
- `FTransaction` is "A single transaction, representing a set of serialized, undo-able changes to a set of objects", header `/Engine/Source/Editor/UnrealEd/Classes/Editor/Transactor.h`; inner `FObjectRecord`; methods `SaveObject`, `SaveArray`, `StoreUndo`, `Apply` ("Enacts the transaction"), `Finalize` ("try and work out what's changed"), `BeginOperation`/`EndOperation`. Epic API docs 5.8, `FTransaction`, retrieved 2026-08-29.
- `UObject::Modify(bool bAlwaysMarkDirty)`: if the engine is recording into the transaction buffer, saves a copy of the object into the buffer and marks the package dirty; returns whether the object was saved; `SaveToTransactionBuffer` is the underlying function. Epic API docs 5.1 `UObject::Modify` and `SaveToTransactionBuffer` (via search summary; the 5.8 API page is client-rendered and returned no body), retrieved 2026-08-29.
- Automation test categories: unit, feature, smoke ("complete within 1 second", run at every start), content stress, screenshot comparison; built in C++ in core modules, independent of the `UObject` environment; run from Window > Test Automation in the Session Frontend. Epic docs, "Automation Test Framework in Unreal Engine", retrieved 2026-08-29.
- Command-line forms: `-ExecCmds="Automation RunTest Test1+Test2;Quit"`, `... RunTest MySet.MySubSet;Quit`, `... RunTest Group:MyGroup;Quit`; `-ReportExportPath="<path>"` writes JSON plus HTML; `-ResumeRunTest` resumes an interrupted run. Epic docs 5.8, "Run Automation Tests in Unreal Engine", retrieved 2026-08-29.
- `-ExecCmds` "Execute the specified console commands"; `-unattended` disables dialogs for unmonitored runs; `-nullrhi` "Use null rendering hardware interface to run UE headless"; `-stdout`, `-BUILDMACHINE`, `-NoShaderCompile` are documented. Epic docs 5.8, "Unreal Engine Command-Line Arguments Reference", retrieved 2026-08-29.
- Commandlets "are executed in a 'raw' environment, in which the game isn't loaded ... no levels are loaded, and no actors exist"; entry point `virtual int32 Main(const FString& Params)`; flags `IsClient`, `IsEditor`, `IsServer`, `LogToConsole`; the name suffix `Commandlet` is appended automatically. Epic API docs 5.8, `UCommandlet`, retrieved 2026-08-29.
- Screenshot comparison stores results under `Saved/Automation/Comparisons`; the first run requires approving a ground-truth image through the Screenshot Browser, which creates a source-control changelist. Epic docs, "Screenshot Comparison Tool in Unreal Engine", retrieved 2026-08-29.
- `AutomationScreenshotOptions` properties: `resolution`, `delay`, `frame_delay`, `override_time_to` ("Sets Delta Time to 0"), `disable_noisy_rendering_features` (disables anti-aliasing, motion blur, screen-space reflections, eye adaptation, tonemapper, contact shadows), `disable_tonemapping`, `visualize_buffer`, `tolerance` (quick defaults, "we default to low"), `tolerance_amount` (per channel and brightness), `maximum_local_error`, `maximum_global_error`, `ignore_anti_aliasing` (search neighbouring pixels), `ignore_colors` (compare luminance only). Epic Python API 5.4, `unreal.AutomationScreenshotOptions`, retrieved 2026-08-29.
- Asset diffing: any asset can be exported to a readable text format and diffed with a user-configured external tool (`Diff Against Depot`, history-pair diff); Blueprints use a built-in graph/defaults diff. M. Noland, "Diffing Unreal Assets" (originally on the Unreal Engine blog, 2014-03-28), retrieved 2026-08-29.
- UE Diff Tool supports "Blueprints, Blueprint adjacent types"; unchanged nodes appear grey; red = present on left only, green = right only, cyan = changed, grey = moved nodes/comments; entry points `Diff > Depot` and `Diff Selected`. Epic docs 5.8, "UE Diff Tool in Unreal Engine", retrieved 2026-08-29.
- `DiffAssets` is exposed as an editor scripting function that "tries to diff two assets using class-specific tool", doing nothing if classes differ. Epic Blueprint API, `AssetTools/DiffAssets` (via search summary), retrieved 2026-08-29.

## Mechanism

Versioning. A package header records the engine version and a container of `(FGuid, int32)` custom versions that the writer touched. Registration is static: a global `FCustomVersionRegistration` inserts `(GUID, LatestVersion, FriendlyName)` into a process-wide registry at module load. During save, `Ar.UsingCustomVersion(GUID)` adds the registry's latest value to the archive's container; during load the container is populated from the file and `Ar.CustomVer(GUID)` returns the stored value, or a sentinel "before any version" when the file predates the GUID. Migration logic lives inline in `Serialize` as monotone `if (CustomVer < X)` branches; the invariant "never decreases" makes the branches a total order and allows the loader to refuse files whose stored value exceeds the registry's latest. There is no embedded schema; a reader that lacks the class or the GUID cannot interpret the bytes, hence the documented null-reference behaviour and re-save data loss.

Transactions. The transaction system is a memento-based command pattern. `FScopedTransaction` is a resource-acquisition-is-initialization (RAII) wrapper over `Begin`/`End` on the global transactor. Mutating editor code calls `Object->Modify()` before changing state; `Modify` serializes the object into the active `FTransaction` as an `FObjectRecord` (a serialized before-image) and marks the package dirty. `Finalize` diffs recorded objects to determine what changed; `Apply` re-serializes the saved state back into the objects (undo) and re-records the current state (redo), so the same record supports both directions. `UTransBuffer` holds `UndoBuffer` with `UndoCount` marking the redo frontier and trims by `MaxMemory`. Undo is therefore state-based (object snapshots), not operation-based; the granularity is the object, and correctness depends on every mutation path calling `Modify` first.

Automation. Tests are C++ classes registered by macros into a registry with flags (filter, priority, application context). A controller executes them in the editor, game, or commandlet process. The headless path is a normal executable invocation with `-nullrhi -unattended` plus `-ExecCmds="Automation RunTests <filter>;Quit"`; results are written by `-ReportExportPath` as JSON with HTML. Screenshot tests capture with deterministic settings (`disable_noisy_rendering_features`, fixed delta time), then compare against an approved ground-truth image using a two-level tolerance: per-pixel channel/brightness tolerance decides whether a pixel differs; `maximum_local_error` bounds the fraction of differing pixels inside sub-regions; `maximum_global_error` bounds the fraction over the whole image; `ignore_anti_aliasing` accepts a match in neighbouring pixels. Commandlets (`-run=<Name>`) provide the same process without world or client code loaded.

Diffing. Generic assets are diffed by exporting a text projection and delegating to an external tool; the projection is not the storage format. Blueprints are diffed structurally per graph, per node and per pin, with node matching sufficient to classify moved nodes separately from changed ones; the docs do not state the matching key (NUIF reading: node GUIDs in the graph model, unverified from retrieved sources).

## NUIF relevance

**Borrow**
- GUID-keyed, monotone, per-extension version numbers declared per serialized entity; NUIF `extensions_used`/`extensions_required` should carry a version integer per namespace with the same monotonicity invariant.
- The headless execution contract: one executable, no GPU (`-nullrhi`), no dialogs (`-unattended`), a filter expression, an exit-on-completion command and a machine-readable report path, which maps directly onto the QA contract in `apps/editor/QA.md` and spec/12.
- Two-level screenshot tolerance (per-pixel channel tolerance, local error budget, global error budget, anti-aliasing neighbourhood) as the model for NUIF deterministic snapshot comparison in conformance.
- Deterministic capture preconditions (fixed delta time, disabled temporal effects) as an explicit evaluation context for renders.

**Adapt**
- Replace before-image snapshots with inverse semantic operations; NUIF undo is operation-based (spec/06) so that undo logs double as patches and replay fixtures, which `FTransaction` records cannot.
- Keep `Modify()`-style pre-mutation hooks as an internal invariant of the editor's operation layer, but make the failure mode (mutation without a transaction) a conformance error surfaced by the CLI rather than silent.
- Export-to-text-then-diff should become diff-over-canonical-form: NUIF's `nuif-text-0` is the canonical form, not a projection.

**Reject**
- Refusing forward compatibility by hiding newer assets and nulling references; NUIF requires unknown data to be preserved as opaque extensions with fidelity records (RFC 0002), not dropped on re-save.
- Migration logic embedded inside per-class `Serialize` functions without an embedded schema; NUIF migrations must be declared operations (`migrate` command) that are testable independently of the loader.
- Class-specific diff tools that only exist for one asset family; NUIF diff must be generic over the typed document model.

## Open questions

- Whether newer-version assets are still hidden in UE 5.8 or now load with a warning; the retrieved page states the hide-and-null behaviour without a version qualifier.
- The exact node-matching key used by the Blueprint diff tool and how it handles node re-creation; no retrieved primary source specifies it.
- Whether `UTransBuffer` records dependent-object changes transitively or relies on each caller invoking `Modify` on every affected object.
