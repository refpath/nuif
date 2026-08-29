# Prior art and competitive map

No surveyed system currently combines the whole NUIF thesis. Several solve important subsets.

| System | Strongest reusable idea | Gap relative to NUIF |
|---|---|---|
| Penpot | open inspectable design document; SVG mapping | shape-centric; not a cross-runtime synchronization standard |
| OpenPencil | programmable editor, Figma codec, DOM/CSS, CLI/MCP | editor ecosystem, not neutral standards governance |
| Figma | mature component/layout authoring semantics | proprietary canonical model and evolving vendor format |
| W3C UI Specification Schema CG | implementation-agnostic UI field/schema goal | closed 2026; schema approach lacked executable renderer/protocol proof |
| Open UI | component anatomy/states/accessibility research | web-control scope, not authored visual document exchange |
| SVG | vector geometry/paint interoperability | lacks high-level components/responsive authored layout |
| Lottie/Rive | portable animation and state-machine runtimes | animation/runtime focus rather than general UI authoring |
| DTCG | neutral token semantics | intentionally only tokens |
| OpenUSD | non-destructive layers/references/variants | 3D scene domain, not UI semantics |
| glTF | compact core + extension governance | delivery/runtime asset rather than authoring model |
| MaterialX | renderer-independent typed graph | material domain |
| MLIR | dialects/multi-level lowering | compiler infrastructure rather than document semantics |
| CSS | rigorous layout families and authored→formatting pipeline | web-specific cascade/DOM/runtime semantics |
| IFC/STEP | long-lived semantic interchange and profiles | complexity warns against over-generalizing the core |

## Directly borrow

Stable standards concepts: SVG geometry, DTCG token values, Unicode/OpenType text foundations, CSS-compatible algorithms for matching profiles, glTF-style capability declarations, OpenUSD-style composition principles, MLIR-style dialect/lowering discipline.

## Adapt

Retentive lenses → property/source correspondence; CRDTs → collaboration profile; WebRender/Vello/Skia → renderer boundary and conformance strategy; Tree-sitter → source-preserving adapter infrastructure.

## Invent/prove

The genuinely new integration is the combination of authored + resolved UI state, stable cross-tool semantic identity, opaque extension retention, fidelity accounting, bidirectional semantic patches and a native open editor whose internal state is the standard itself.
