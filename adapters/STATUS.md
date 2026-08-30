# Adapter implementation status

The adapter program separates an ecosystem's public interchange or source
surface from the subset for which NUIF can provide executable round-trip laws.
Research coverage does not imply an implemented conformance profile.

The inventory contains eleven targets and six executable profiles across
five target families. The remaining targets have no executable direction in
`adapters/index.json`.

This table is explanatory. `adapters/index.json` is the machine-audited target
inventory; `cargo xtask adapter-audit` checks its research, profile, crate and
gate references and writes `target/adapter-coverage-report.json`.

| Target | Primary integration surface | Executable status | Next bounded profile | Boundary |
| --- | --- | --- | --- | --- |
| HTML/CSS | DOM and CSS source | `nuif-html-css-0` and `nuif-html-css-v0` | Extend only with separately tested CSS/layout features | Arbitrary cascade, script and unmarked DOM are not imported |
| SVG | SVG 2 XML | `nuif-svg-0` | Add paths or transforms only under separately declared geometry laws | Paths, transforms, CSS cascade, paint servers, effects, animation, scripts and external resources are excluded |
| DTCG tokens | Design Tokens Format Module 2025.10 JSON | `nuif-dtcg-scalar-0` | Expand only after a token-model RFC and separate profile | Core tokens lack declared type, groups, aliases, descriptions, deprecation and token-local extensions |
| React | JSX source and React DOM properties | `nuif-react-jsx-0` | TSX or CSS-class support only under a separate grammar/runtime matrix | Components, hooks, spreads, control flow and runtime expressions require execution |
| Svelte | `.svelte` source and compiler AST | Research complete; no implementation | Static regular elements, literal text/attributes and profile-owned CSS spans | Runes, scripts, blocks, directives, preprocessors and dynamic components require execution |
| Penpot | `.penpot` v3 ZIP and JSON package | `nuif-penpot-v3-0` | Add compact pages only after the opt-in representation stabilizes | Components, libraries, interactions, media, paths, layout and compact pages are excluded |
| Figma | REST document JSON plus writable plugin API | Research complete; host report contract implemented; no live plug-in | One-page mapping in `adapters/figma/PROFILE-DRAFT.md` | `.fig` is not a public contract; live writes require user-run plug-in execution |
| Adobe UXP | Host-specific document APIs and `.ccx` packages | Research complete; host report contract implemented; no live package | InDesign page and basic page-item subset in `adapters/adobe/PROFILE-DRAFT.md` | UXP object models and mutation rules are product-specific; Illustrator is not in the retrieved UXP host contract |
| SwiftUI | Swift source and proposal–response layout runtime | Research complete; no implementation | Generated stack/text/shape subset with a pinned Apple toolchain | Arbitrary Swift and custom layouts are executable programs |
| Jetpack Compose | Kotlin source and constraint layout runtime | Research complete; no implementation | Generated row/column/text/shape subset with a pinned Android toolchain | Arbitrary Kotlin, state, modifier chains and subcomposition are executable programs |
| Flutter | Dart source and box-constraint runtime | Research complete; no implementation | Generated row/column/text/shape subset with a pinned Flutter toolchain | Arbitrary Dart, state, inherited widgets and custom render objects are executable programs |

## Implementation order

The SVG basic-shape, DTCG scalar-token and Penpot v3 profiles are implemented
because their bounded subsets map directly to the current model and run without
credentials or platform SDKs. Full DTCG coverage requires a token-model RFC.
Penpot's package path enforces ZIP resource limits and unknown-member retention
through one shared test contract. Figma and Adobe have
bounded draft profiles and the shared `HostAdapterReport` evidence envelope;
they remain non-integrated until a compiled plug-in and named live-host trial
exist.

React now uses the common byte-span correspondence contract for one marked
static intrinsic JSX subtree. Svelte requires the same retentive source-edit
discipline rather than whole-file generation. Native declarative UI targets begin as one-way lowerings with
foreign-runtime layout and screenshot comparisons. Bidirectional claims remain
out of scope until a static, profile-owned source subset has exact import and
edit-locality tests.

## Conformance requirements

Every implemented adapter profile must provide:

- a versioned profile document and machine-readable capability identifier;
- input byte, syntax-depth, entity/member and retained-data limits;
- import/export fixtures with canonical NUIF expected outputs;
- exact round trips for the declared subset and typed fidelity for every
  excluded property;
- correspondence records with foreign identity and property or source span;
- unknown-data preservation tests when the foreign format permits retention;
- repeated-output determinism, stale-correspondence rejection and atomic
  failure;
- foreign-validator or runtime evidence pinned by version when one exists;
- CLI, `xtask` and CI integration before the profile is listed as executable.

The supporting primary-source records are `svg`, `dtcg`,
`react-jsx-adapter-surface`, `svelte-source-adapter-surface`, `penpot`, `figma`,
`adobe-uxp-host-integration`, `swiftui-layout`, `jetpack-compose-layout` and
`flutter-layout` under `research/items/`.
