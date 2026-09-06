# Research coverage and evidence status

`research/coverage.yaml` maps architectural topics to source records, questions,
specification modules, decisions and experiments. The validator checks those
links and the required continuation fields. Registry validity establishes that
the research inventory is internally traceable. It does not establish that each
question is resolved or each cited result has been independently verified.

## Interpretation of status

The [audit policy](../../research/AUDIT.md) defines record verification and
profile gates. `covered` identifies a topic with the required evidence and
decision links. `experiment-required` identifies an unresolved empirical
question. `ongoing` identifies work requiring continued source review or
measurement. The [experiment registry](../../research/experiments/index.yaml)
records the continuation class, prerequisites and next action for open work.

A count of research records is an inventory measure. It is not a measure of
literature-search recall, evidence quality or beta readiness. The
[methods section](00-foundation.md#research-method) describes how local tests,
independent oracles and external evaluation differ.

## Implemented evidence and remaining limits

| Area | Executable evidence | Remaining limit |
|---|---|---|
| Source synchronization | Named adapter fixtures and 48 generated HTML cases | Independently authored documents, structural reconciliation and real workflow outcomes |
| Accessibility | Role/name/state comparison for the declared eleven-node web fixture | Native-platform mappings and screen-reader interaction |
| Behavior | Bounded state-machine traces and one-way web lowering | General application behavior and native controls |
| Packages | Deterministic bytes and independent local writers | Hosted platform evidence and external reproduction |
| Images | Named PNG decoder and composition matrices | Additional formats, colour metadata and host equivalence |
| Fonts | Static and variable TrueType package, shaping, metrics, outline and runtime fixtures | Broad OpenType coverage and cross-platform raster behavior |
| Variable-font bindings | Local Rust, CLI, Node/browser WASM, MCP and C snapshot comparison | Independent external implementation and broader host coverage |
| Live capture | Pinned loopback browser fixture and held-out viewport experiment | Licensed real pages, additional browsers and capture states |
| Screenshot reconstruction | Proposal contracts, synthetic evaluation and corpus-integrity checks | Executed OCR/model baselines and held-out accuracy |

The source workflow is specified in the [beta contract](../BETA.md).
Accessibility and behavior boundaries are listed in the
[adapter status document](../../adapters/STATUS.md). Package, font, capture and
reconstruction methods and source records are consolidated in the
[resource chapter](12-resources-capture-and-reconstruction.md). The
[implementation roadmap](../roadmap.md) identifies the corresponding commands
and artifact contracts.

## Continuation policy

New source evidence can supersede a decision without promoting its dependent
experiments. Passing one profile cannot close a broader question unless that
profile meets the question's stated acceptance conditions. A failed experiment
retains its counterexample and provenance. Scope changes require an updated
contract and explicit exclusions rather than retroactive reinterpretation of
a passing result.
