# AI/headless QA contract

Status: items 1–8 and 10 have a profile-0 implementation through `nuif-api`, `nuif-testing`, `nuif` and `nuif-editor`; automatic minimized failure-fixture writing remains partial.

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

`cargo xtask editor-trial` is the required complete-authoring trial. It starts from an explicit empty document identifier, dispatches only semantic/identity actions, compares canonical output bytes with the direct fixture generator, independently replays the logged patches, validates the result, and archives the full snapshot evidence used by automated and AI-driven iteration.
