# Adapters

External ecosystems are peers around NUIF, not parents of its canonical model.

The first executable adapter is the deliberately bounded [`nuif-html-css-0`](html-css/PROFILE.md) retentive profile. It records concrete byte spans, round-trips its declared container/text/token subset exactly, applies mapped changes without regenerating comments or unmapped regions, and rejects every unsupported semantic change with target/property fidelity. It does not claim arbitrary HTML support or complete v0 responsive-card coverage.

Further research adapters include broader HTML/CSS, SVG, Svelte, React, Penpot, Figma, Flutter, SwiftUI, and Jetpack Compose. Each adapter must emit structured fidelity diagnostics and record provenance/correspondence sufficient for later synchronization and minimal source patches where feasible.

Vendor-specific semantics belong in namespaced extensions or adapter-local logic; they must not leak into the core merely because a vendor is popular.
