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
11. `independent-reproduction` — a non-reference package parses/writes the fixture and independently reproduces resolved layout, raster and fidelity.

## Test techniques

- golden structural fixtures;
- property-based tests for operation sequences;
- fuzz parsers/codecs and path geometry;
- differential layout checks against browser/Taffy where semantics match;
- metamorphic tests such as encode→decode→encode stability;
- deterministic operation replay;
- visual snapshots with perceptual thresholds only where exact pixels are not normative.

A test result must include implementation version, capability profile, fixture ID and evaluation context. Foreign-reference results additionally include exact oracle versions, generator source revision, raw per-engine observations, a fixture-local measured bound and typed classifications for every divergence.

The implemented adapter suite covers `nuif-html-css-0` and the separately declared `nuif-html-css-v0` responsive-card profile with exact export/import, byte-local synchronization and property-attributed rejection. Arbitrary web source and other adapter targets remain non-conformant until separately declared profiles pass.

The implemented independent-reproduction suite covers the complete v0 fixture in canonical text and the declared profile-0 layout/render subset. The Python standard-library implementation computes its own boxes and pixels, and the harness compares decoded RGBA so PNG encoder behavior is not mistaken for render divergence. This is an in-repository mechanical reproduction, not evidence of external implementation provenance or standards adoption.
