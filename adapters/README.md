# Adapters

External ecosystems are peers around NUIF, not parents of its canonical model.

Planned research adapters include SVG, HTML/CSS, Svelte, React, Penpot, Figma, Flutter, SwiftUI, and Jetpack Compose. Each adapter must emit structured fidelity diagnostics and record provenance/correspondence sufficient for later synchronization and minimal source patches where feasible.

Vendor-specific semantics belong in namespaced extensions or adapter-local logic; they must not leak into the core merely because a vendor is popular.
