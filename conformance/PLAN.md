# Conformance plan

NUIF conformance is split into independently testable profiles.

## Required suites

1. `model` — stable IDs, containment, graph references, cycle rules.
2. `canonicalization` — deterministic text/binary representation and hashes.
3. `extensions` — used/required negotiation and opaque preservation.
4. `layout` — authored→resolved fixtures across viewport/context matrices.
5. `render` — normative geometry/paint/text behavior with declared tolerances.
6. `operations` — patch replay, inversion, preconditions and deterministic results.
7. `merge` — three-way semantic conflicts and move/reorder cases.
8. `provenance` — correspondence retention and fidelity diagnostics.
9. `adapter` — import/export loss reports and foreign-extension preservation.
10. `security` — parser depth/size limits, malicious assets and renderer budgets.

## Test techniques

- golden structural fixtures;
- property-based tests for operation sequences;
- fuzz parsers/codecs and path geometry;
- differential layout checks against browser/Taffy where semantics match;
- metamorphic tests such as encode→decode→encode stability;
- deterministic operation replay;
- visual snapshots with perceptual thresholds only where exact pixels are not normative.

A test result must include implementation version, capability profile, fixture ID and evaluation context. Foreign-reference results additionally include exact oracle versions, generator source revision, raw per-engine observations, a fixture-local measured bound and typed classifications for every divergence.
