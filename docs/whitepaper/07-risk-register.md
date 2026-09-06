# Research limitations and implementation risks

## Representational limits

Rendered pixels do not determine a unique layout program. Source-backed
observations and inferred properties therefore require distinct provenance.
The reconstruction experiment can test whether an inferred document supports
specified edits or held-out viewports; it cannot prove recovery of unique
original source. The [reconstruction synthesis](12-resources-capture-and-reconstruction.md)
provides the relevant source records and evaluation contract.

Behavior portability is limited to declared operations and capabilities.
Arbitrary JavaScript, Swift or Dart application behavior is outside the current
model. Native controls, font substitution and rendering environments also
require separate fidelity judgments. These limits are recorded in the
[adapter inventory](../../adapters/STATUS.md) and
[text conformance research](../../research/items/text-rendering-reproducibility.md).

Resource preservation has two independent requirements: exact bytes and
authorized use. A valid digest or accepted font header establishes neither
redistribution permission nor decoder safety. Capture can expose private data;
the collection, export, retention and training policies are separate inputs.
The applicable implementation limits are defined in
[security](../../spec/11-security.md) and the
[resource and reconstruction chapter](12-resources-capture-and-reconstruction.md).

## Evidence and adoption risks

Generated round trips can preserve a shared exporter/importer mistake.
Independent expected values, separate implementations and externally authored
fixtures address different parts of this risk. None is replaced by increasing
the generated case count. [Research methods](00-foundation.md#research-method)
describe the distinction.

Visual similarity can conceal loss of structure, text, accessibility or
editability. Reconstruction evaluation therefore reports those properties
separately. Raw model probabilities remain uncalibrated until measured against
held-out outcomes; they cannot promote inferred values to source evidence.
See [confidence calibration](../../research/items/confidence-calibration-and-selective-prediction.md).

A larger semantic core requires more adapter mappings and conformance cases.
The project has not measured how this cost affects adoption. The response is
to delimit profiles and measure implementation effort, rather than infer an
adoption outcome from the complexity of IFC or STEP. External implementer and
workflow evidence is required by the [standards roadmap](../STANDARDS-ROADMAP.md).

## Implementation boundaries

| Boundary | Current evidence | Required continuation |
|---|---|---|
| Native editor forks | Reviewed Xilem and ui-events revisions, dependency traces and local semantic/render trials | Repeat host graphics checks for fork updates; establish parity before toolkit replacement |
| File output | Staged single-file replacement and injected-write failure tests | Separate multi-file transaction and concurrent-writer requirements if the workflow needs them |
| Release distribution | Unsigned source-built installation and package smoke tests | Platform signing and notarisation where required for the distribution channel |
| Portable resources | Named package, PNG and OpenType fixtures with independent local oracle comparisons | Hosted platform results, broader media matrices and external reproduction |
| Capture | A pinned loopback browser fixture and bounded capture contracts | Licensed real-page evaluation, additional hosts and state/resource combinations |
| Reconstruction | Typed proposals, synthetic metric tests and corpus-integrity checks | Executed OCR/model baselines, held-out accuracy and useful edit outcomes |

The fork evidence and alternatives are documented in the
[editor fork review](../../research/items/editor-fork-exit-review.md).
[File-output documentation](../../crates/nuif-cli/README.md#output-replacement)
states permission, symlink, hard-link and durability limits. Resource and capture
continuations are enumerated in the [experiment registry](../../research/experiments/index.yaml).

## Architecture revision criteria

The source-edit design requires revision if common edits within a declared
profile repeatedly require regeneration of unrelated source. Opaque preservation
requires revision if supported neighboring edits discard unknown payloads. The
specification requires revision if an independent implementation cannot derive
profile behavior from its requirements and fixtures without consulting Rust
internals. Each failure should produce a reduced case and a scoped decision,
rather than a compatibility exception without a semantic contract.

Reconstruction scope should narrow if gains in pixel similarity reduce
structural or edit-task quality, if confidence does not support useful
risk/coverage tradeoffs, or if adaptation fails to outperform the untuned
baseline under the same held-out data and budget. These are proposed decision
criteria; current synthetic results do not resolve them.
