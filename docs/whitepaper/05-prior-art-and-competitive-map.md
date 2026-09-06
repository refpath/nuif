# Prior-art comparison

The surveyed systems provide precedents for individual parts of authored
interface interchange. This comparison records their relevance to NUIF's
proposed scope. It is not a product ranking or an exhaustive novelty claim.
Each linked research record identifies the reviewed source and retrieval date.

| System and source record | Relevant mechanism | Boundary of the comparison |
|---|---|---|
| [Penpot](../../research/items/penpot.md) | Inspectable design documents and SVG mapping | A design-tool model does not by itself establish cross-runtime synchronization |
| [OpenPencil](../../research/items/openpencil.md) | Programmable editor, Figma codec and automation interfaces | Editor capabilities and neutral specification governance are separate questions |
| [Figma](../../research/items/figma.md) | Component and layout authoring semantics | Interoperability depends on the exposed vendor APIs and their versions |
| [W3C UI Specification Schema Community Group](../../research/items/ui-spec-schema-cg.md) | Proposed implementation-agnostic UI schema | The group closed in 2026; the cited record does not establish the cause |
| [Open UI](../../research/items/open-ui.md) | Control anatomy, states and accessibility | The reviewed scope concerns web controls |
| [SVG](../../research/items/svg.md) | Vector geometry and paint | NUIF component and authored-layout mappings require additional contracts |
| [Lottie and Rive](../../research/items/lottie-rive.md) | Animation and state-machine runtimes | Runtime delivery and general authoring interchange have different requirements |
| [DTCG](../../research/items/dtcg.md) | Design-token semantics | Tokens form one part of the proposed document model |
| [OpenUSD](../../research/items/openusd.md) | Layers, references and variants | UI composition requires a separately specified interpretation |
| [glTF](../../research/items/gltf.md) | Extension and capability declarations | The reviewed specification concerns runtime 3D assets |
| [MaterialX](../../research/items/materialx.md) | Typed renderer-independent graphs | Material graphs do not define UI document semantics |
| [MLIR](../../research/items/mlir.md) | Dialects and explicit lowering | Compiler infrastructure supplies a pattern, not a document interchange contract |
| [CSS](../../research/items/css-formatting.md) | Authored style and resolved formatting | Correspondence with non-web layout requires named profiles |
| [IFC](../../research/items/ifc.md) | Semantic interchange and domain profiles | NUIF has no measured adoption-cost comparison with this domain |

## Repository interpretation

The architecture adopts established geometry, token and text specifications
where their semantics match a declared profile. Composition, lowering,
capability declarations and source correspondence require adaptation to UI
editing. The [cross-industry synthesis](11-cross-industry-patterns.md) records
those decisions individually.

The proposed contribution is the integration of authored and resolved state,
stable semantic identity, opaque preservation, fidelity reporting and retentive
source edits. Evidence for that integration comes from bounded experiments.
Neither the number of surveyed systems nor the absence of an identical system
in this comparison proves novelty, usefulness or adoption potential.
