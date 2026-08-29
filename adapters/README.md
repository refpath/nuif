# Adapters

External ecosystems are peers around NUIF, not parents of its canonical model.

The first executable adapter is the deliberately bounded [`nuif-html-css-0`](html-css/PROFILE.md) retentive profile. It records concrete byte spans, round-trips its declared container/text/token subset exactly, applies mapped changes without regenerating comments or unmapped regions, and rejects every unsupported semantic change with target/property fidelity.

The follow-on [`nuif-html-css-v0`](html-css/V0-PROFILE.md) profile carries the complete responsive-card model, including responsive rules, component/instance identity, unknown kinds and opaque extensions. Its automated editor bridge proves semantic editor output can patch retained source and return through the CLI to byte-identical canonical NUIF. Path rendering, instance materialization, unknown visuals and arbitrary HTML/CSS remain explicitly outside the target profile.

Further research adapters include broader HTML/CSS, SVG, Svelte, React, Penpot, Figma, Flutter, SwiftUI, and Jetpack Compose. Each adapter must emit structured fidelity diagnostics and record provenance/correspondence sufficient for later synchronization and minimal source patches where feasible.

[`STATUS.md`](STATUS.md) records the current primary integration surface,
implementation status, next bounded profile and exclusion boundary for every
advertised target. Research coverage and executable conformance are listed
separately.

Vendor-specific semantics belong in namespaced extensions or adapter-local logic; they must not leak into the core merely because a vendor is popular.
