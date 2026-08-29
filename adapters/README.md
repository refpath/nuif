# Adapters

External ecosystems are peers around NUIF, not parents of its canonical model.

The first executable adapter is the deliberately bounded [`nuif-html-css-0`](html-css/PROFILE.md) retentive profile. It records concrete byte spans, round-trips its declared container/text/token subset exactly, applies mapped changes without regenerating comments or unmapped regions, and rejects every unsupported semantic change with target/property fidelity.

The follow-on [`nuif-html-css-v0`](html-css/V0-PROFILE.md) profile carries the complete responsive-card model, including responsive rules, component/instance identity, unknown kinds and opaque extensions. Its automated editor bridge proves semantic editor output can patch retained source and return through the CLI to byte-identical canonical NUIF. Path rendering, instance materialization, unknown visuals and arbitrary HTML/CSS remain explicitly outside the target profile.

The [`nuif-svg-0`](svg/PROFILE.md) profile maps one surface, freeform groups,
rectangles, ellipses and literal pinned-font text to SVG 2 XML. It retains
UTF-8 spans for identity, geometry, paint and accessibility scalars, preserves
unmarked XML byte-for-byte during edits and rejects unsupported geometry,
paint and structure with typed fidelity.

The [`nuif-dtcg-scalar-0`](dtcg/PROFILE.md) profile maps flat DTCG 2025.10
boolean, string and number tokens while preserving NUIF integer/real identity
through namespaced metadata. It retains unknown extension bytes through the
same CLI synchronization contract as the source adapters. Its deliberately
narrow boundary precedes the token-model RFC required for groups, aliases and
composite types.

The remaining researched targets are Svelte, React, Penpot, Figma, Flutter,
SwiftUI and Jetpack Compose. Broader HTML/CSS, SVG and DTCG profiles remain
separate future work beyond the four executable profiles. Each adapter must
emit structured fidelity diagnostics and record provenance/correspondence
sufficient for later synchronization and minimal source patches where feasible.

[`STATUS.md`](STATUS.md) records the current primary integration surface,
implementation status, next bounded profile and exclusion boundary for every
advertised target. Research coverage and executable conformance are listed
separately.

[`index.json`](index.json) is the machine-readable counterpart. `cargo xtask
adapter-audit` requires all ten advertised targets to have a primary research
record, explicit directionality, a next bounded profile and a non-empty
boundary. Integrated entries additionally require crate, profile and routed
gate paths; non-integrated entries cannot claim executable directions. The
audit writes `target/adapter-coverage-report.json` and blocks the complete gate.

Vendor-specific semantics belong in namespaced extensions or adapter-local logic; they must not leak into the core merely because a vendor is popular.
