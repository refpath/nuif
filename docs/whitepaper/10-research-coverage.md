# Research coverage and continuous completeness

A research repository cannot truthfully claim to contain every paper that will ever be relevant. NUIF instead defines **operational completeness**: every planned architectural front must have an explicit status, evidence links, unresolved questions and an experiment/decision path.

`research/coverage.yaml` is the machine-readable coverage contract. It maps the founding research plan to research IDs, specification modules, RFCs/ADRs, code seams and experiments. The project can therefore identify gaps structurally rather than relying on prose search to infer that a topic was forgotten.

## Current state

All founding fronts are represented. The resource, browser-capture,
reconstruction-evaluation, adaptation/distillation and AI artifact-governance
fronts are now explicit rather than hidden inside “serialization” or
“inference.” Decisions that can safely be made from mature prior art are marked
`covered`. Questions whose answer would be premature without an implementation
are marked `experiment-required`. Areas whose evidence base will continuously
evolve—prior art, adapters, reconstruction and data/model governance—remain
`ongoing` by design.

This distinction is important: marking an open research problem as finished would be less rigorous than preserving it as a first-class graph node.

## Additional boundaries from the final sweep

- WAI-ARIA and accessibility API mappings support a semantic-role/state layer distinct from platform-specific accessibility trees.
- KHR_interactivity provides contemporary precedent for portable, capability-aware behavior graphs rather than arbitrary scripts embedded in visual nodes.
- ReverseORC and related layout-inference work show that multiple viewport observations materially improve recovery of responsive intent.
- Screenshot-to-code research continues to show that visual reconstruction is not equivalent to recovering authored layout or behavior.
- Merkle/content addressing is appropriate for immutable assets and snapshots but not for editable semantic identity.
- EPUB OCF and OCI descriptors support a narrow manifest-driven package with
  size/digest verification; NUIF now has an in-repository independent-writer
  fixture. A three-OS CI matrix now exercises the package, image and font
  gates, while successful hosted evidence and external reproduction remain
  open.
- OpenType and Fontations evidence now supports one executable static TrueType
  package baseline with a pinned HarfBuzz metadata oracle. The retired
  `ttf-parser` decision remains documented; broad font formats, portability
  outcomes and shaping/raster integration remain experiment-required. Warmed
  parser and packaged-validation allocation ceilings cover every accepted
  fixture.
- Browser source capture and screenshot reconstruction are different evidence
  lanes and cannot share a blanket `lossless` claim.
- Current screenshot-to-code work supports OCR/region/hierarchical and render-
  correction experiments, not a claim that authored UI recovery is solved.
- LoRA, quantized adaptation and distillation are conditional experiment
  techniques; evaluation, rights-cleared traces and artifact governance precede
  training.

The continuous-research process should periodically re-run topic searches, append or supersede research records, and update `research/coverage.yaml` only when new evidence or experiments change the status of a front.
