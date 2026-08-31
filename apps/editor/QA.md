# AI/headless QA contract

Status: items 1–8 and 10 have a profile-0 implementation through `nuif-api`, `nuif-testing`, `nuif` and `nuif-editor`; automatic minimized failure-fixture writing remains partial. The native-shell wiring has a deterministic AccessKit and CPU-render trial.

An automated QA client must be able to perform the following without synthetic mouse input:

1. create/open/save/canonicalize documents;
2. query entities by identity/type/name/relationship;
3. execute semantic transactions and capture inverse/replay logs;
4. evaluate layout at explicit contexts;
5. inspect resolved boxes, text diagnostics and accessibility semantics;
6. render deterministic snapshots;
7. diff canonical documents and resolved snapshots;
8. validate fidelity and extension-preservation assertions;
9. minimize a failing operation sequence into a reproducible fixture;
10. emit one machine-readable report containing inputs, versions, capabilities and artifacts.

GUI automation is reserved for testing shell wiring, focus, pointer/keyboard interactions and browser integration.

The headless client MUST apply the same bounded document reader as the CLI and MUST bound script bytes, line bytes and command count before retaining an operation log. `cargo xtask hostile-inputs` verifies document-ingestion boundaries; editor unit tests verify the limit-plus-one reader.

Headless and GUI sessions report the package capability negotiation result. If
any required capability is unavailable, selection and inspection may proceed,
but every semantic transaction and changed package save MUST fail atomically.
The no-op package copy remains byte-exact. This boundary is part of
`cargo xtask editor-hostile-inputs`.

`cargo xtask editor-trial` is the required complete-authoring trial. It starts from an explicit empty document identifier, dispatches only semantic/identity actions, compares canonical output bytes with the direct fixture generator, independently replays the logged patches, validates the result, and archives the full snapshot evidence used by automated and AI-driven iteration.

`cargo xtask editor-gui-trial` is the supplementary shell trial. It selects an entity and edits name, sizing, position, layout spacing and fill controls through AccessKit `Click` and `SetValue` requests, then locates resolved entities through the canvas widget and performs captured freeform move, north-west resize and responsive Stack reorder pointer drags in document coordinates. Movement commits one position operation; the leading-corner resize commits its anchored position and both fixed-size axes in one transaction; Stack/Flex reorder commits one same-parent `Move` based on resolved sibling centres. All are whole-pixel snapped where applicable, the expected final child order is asserted, and the reorder is exercised by the following undo/redo pair. The runner independently replays every patch, validates the output document, records the semantic tree, captures both the normal shell and the open File menu, requires every native and profile import/export route to be visible, and repeats the run to require identical canonical and pixel hashes. The shell unit tests additionally cover all eight handle geometries, Shift-proportional corner geometry, managed no-op/rejection boundaries, inactive-tool rejection, atomic resize validation, pixel grid/ruler defaults and exact export/import round trips for the declared SVG, HTML/CSS, DTCG, Penpot, static React JSX and static Svelte fixtures. No hard-coded screen coordinate identifies a document entity or handle; the runner derives pointer paths from resolved layout and the canvas transform.
