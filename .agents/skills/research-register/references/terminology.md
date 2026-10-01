# Terminology

Preferred terms with their field of origin. Use the left column; do not substitute the right column.

## Document model

| Term | Origin | Do not write |
|------|--------|--------------|
| containment tree | scene-graph literature | hierarchy tree, node tree (unless quoting a source) |
| relationship graph, typed relation | graph theory | link web, connection map |
| entity, entity identity, `EntityId` | database/ECS literature | node id, object id (for NUIF entities) |
| stable identity | distributed systems | permanent id, magic id |
| authored intent, authored value | NUIF spec | design intent, source of truth |
| resolved value, resolved snapshot, evaluation context | compiler/layout literature | computed stuff, runtime values |
| lowering | compiler literature (MLIR) | translating down, flattening (flattening is a specific lowering that discards structure) |
| lifting, reconciliation | bidirectional-transformation literature | reverse sync, smart import |
| composition arc, opinion, layer stack | OpenUSD | override chain |
| variant, override, instance, definition | component systems (USD, Unity prefabs) | fork, clone |
| extension, dialect, namespace | glTF, MLIR | plugin data, custom junk |
| opaque preservation | NUIF spec | passthrough, blob keeping |
| fidelity class (`lossless`, `representable`, `approximated`, `preserved_unrenderable`, `unsupported`) | NUIF spec | quality level |
| provenance, correspondence record | data-provenance literature | source mapping, link back |
| design token | DTCG | variable (unless quoting Figma), style constant |

## Layout

| Term | Origin | Do not write |
|------|--------|--------------|
| layout family (`freeform`, `stack`, `flex`, `grid`, `constraint`) | NUIF spec | layout mode |
| intrinsic size, min-content, max-content, fit-content | CSS Sizing Level 3 | natural size, hug (Figma vocabulary is adapter-only) |
| flex base size, hypothetical main size, free space | CSS Flexbox Level 1 §9 | flex math |
| track sizing algorithm | CSS Grid Level 2 §12 | grid math |
| proposal–response layout | SwiftUI documentation | negotiation layout |
| linear constraint system, Cassowary | Badros, Borning 2001 | constraint magic |
| box, bounding box, transform | graphics | rect (in prose; `Rect` as a type name is fine) |
| evaluation context (viewport, scale factor, locale, writing direction, theme, fonts) | NUIF spec | environment, device profile |

## Rendering and text

| Term | Origin | Do not write |
|------|--------|--------------|
| render scene, draw command, render target | graphics | display list (acceptable if quoting), render stuff |
| renderer backend, render delegate | Hydra/Vello | rendering plugin |
| reference rasterization, CPU reference path | conformance practice | golden render |
| anti-aliasing, coverage, subpixel positioning, hinting | rasterization literature | smoothing |
| perceptual difference metric (ꟻLIP, SSIM), tolerance, baseline | image-quality literature | fuzzy match (except when citing WPT/WebRender syntax) |
| shaping, glyph run, cluster, advance | HarfBuzz/OpenType | text layout (ambiguous), font rendering |
| font fallback, font substitution | text stack literature | font swap |

## Operations and synchronization

| Term | Origin | Do not write |
|------|--------|--------------|
| semantic operation, transaction, patch, precondition, inverse operation | NUIF protocol | action, command (except when discussing the command pattern) |
| replay, deterministic replay | distributed systems | rerun |
| canonicalization, canonical form, canonical hash | RFC 8785, RFC 8949 §4.2 | normalization (use only for Unicode/number normalization steps) |
| three-way merge, structural merge, tree differencing, edit script | Chawathe 1996, GumTree | smart merge |
| commutation (of patches) | Darcs/Pijul patch theory | reordering |
| conflict-free replicated data type (CRDT), operational transformation (OT), last-writer-wins register, fractional index | distributed systems | live sync tech |
| lens, well-behaved lens, GetPut, PutGet | Foster 2007 | two-way binding |
| bidirectional transformation (BX) | BX community | round-trip magic |
| minimal patch, edit locality | program synthesis / BX | surgical edit |

## Testing

| Term | Origin | Do not write |
|------|--------|--------------|
| test oracle | software testing | checker, truth |
| metamorphic relation, metamorphic testing | Chen 1998 | invariant test (use "invariant" for state predicates) |
| differential testing | McKeeman 1998 | cross-check |
| property-based testing, strategy, shrinking | QuickCheck, proptest | fuzz (reserve for coverage-guided fuzzing) |
| coverage-guided fuzzing, fuzz target, corpus | libFuzzer/AFL | random testing |
| deterministic simulation testing, seed | FoundationDB/TigerBeetle | chaos testing (different technique) |
| delta debugging, test-case reduction | Zeller 2002 | minimization (acceptable as a verb: "minimize") |
| characterization test, snapshot test, reference image, expected output | testing literature | golden file (acceptable when citing Skia "golden master") |
| fixture, evaluation matrix | testing | test data |
| round trip, idempotence, fixpoint | mathematics | loop, stability |
| model-based testing, reference model | testing | twin |
| conformance suite, conformance profile, capability profile | standards practice | compliance pack |
| headless, in-process automation surface | tooling | AI hooks, agent mode |

## Editor

| Term | Origin | Do not write |
|------|--------|--------------|
| canvas, viewport, layers panel, properties panel (inspector), toolbar | design-tool UI | workspace bits |
| direct manipulation | Shneiderman 1983 | drag stuff |
| marquee selection, deep selection, snapping, smart guides | design-tool UI | box select |
| immediate mode, retained mode | GUI literature | frame-based, object-based |
| accessibility tree, role, action | WAI-ARIA, AccessKit | a11y tree (in prose) |
| harness, event injection | UI testing | fake input |
| test editor, reference editor | NUIF | product, app |

## Governance

| Term | Origin | Do not write |
|------|--------|--------------|
| draft specification, normative, informative | standards practice | the standard (until promoted) |
| RFC, ADR | repository process | proposal doc |
| independent implementation | standards practice | second impl |
| research record, claim, question, experiment | repository schema | note, idea |
