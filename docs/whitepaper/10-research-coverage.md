# Research coverage and continuous completeness

A research repository cannot truthfully claim to contain every paper that will ever be relevant. NUIF instead defines **operational completeness**: every planned architectural front must have an explicit status, evidence links, unresolved questions and an experiment/decision path.

`research/coverage.yaml` is the machine-readable coverage contract. It maps the founding research plan to research IDs, specification modules, RFCs/ADRs, code seams and experiments. Refpath can therefore identify gaps structurally rather than relying on NLP to infer that a topic was forgotten.

## Current state

All founding fronts are represented. Decisions that can safely be made from mature prior art are marked `covered`. Questions whose answer would be premature without an implementation are marked `experiment-required`. Areas whose evidence base will continuously evolve—prior art, adapters and inference/program synthesis—remain `ongoing` by design.

This distinction is important: marking an open research problem as finished would be less rigorous than preserving it as a first-class graph node.

## Additional boundaries from the final sweep

- WAI-ARIA and accessibility API mappings support a semantic-role/state layer distinct from platform-specific accessibility trees.
- KHR_interactivity provides contemporary precedent for portable, capability-aware behavior graphs rather than arbitrary scripts embedded in visual nodes.
- ReverseORC and related layout-inference work show that multiple viewport observations materially improve recovery of responsive intent.
- Screenshot-to-code research continues to show that visual reconstruction is not equivalent to recovering authored layout or behavior.
- Merkle/content addressing is appropriate for immutable assets and snapshots but not for editable semantic identity.

The continuous-research process should periodically re-run topic searches, append or supersede research records, and update `research/coverage.yaml` only when new evidence or experiments change the status of a front.
