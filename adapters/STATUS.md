# Adapter implementation status

The adapter program separates an ecosystem's public interchange or source
surface from the subset for which NUIF can provide executable round-trip laws.
Research coverage does not imply an implemented conformance profile.

This table is explanatory. `adapters/index.json` is the machine-audited target
inventory; `cargo xtask adapter-audit` checks its research, profile, crate and
gate references and writes `target/adapter-coverage-report.json`.

| Target | Primary integration surface | Executable status | Next bounded profile | Boundary |
| --- | --- | --- | --- | --- |
| HTML/CSS | DOM and CSS source | `nuif-html-css-0` and `nuif-html-css-v0` | Extend only with separately tested CSS/layout features | Arbitrary cascade, script and unmarked DOM are not imported |
| SVG | SVG 2 XML | `nuif-svg-0` | Add paths or transforms only under separately declared geometry laws | Paths, transforms, CSS cascade, paint servers, effects, animation, scripts and external resources are excluded |
| DTCG tokens | Design Tokens Format Module 2025.10 JSON | `nuif-dtcg-scalar-0` | Expand only after a token-model RFC and separate profile | Core tokens lack declared type, groups, aliases, descriptions, deprecation and token-local extensions |
| React | JSX source and React DOM properties | Research complete; no implementation | Static intrinsic JSX with literals and retained AST spans | Components, hooks, spreads, control flow and runtime expressions require execution |
| Svelte | `.svelte` source and compiler AST | Research complete; no implementation | Static regular elements, literal text/attributes and profile-owned CSS spans | Runes, scripts, blocks, directives, preprocessors and dynamic components require execution |
| Penpot | `.penpot` v3 ZIP and JSON package | Research complete; no implementation | One page with frame, rectangle, ellipse and text shapes | Package/resource limits and retentive unknown-member preservation precede components, libraries and interactions |
| Figma | REST document JSON plus writable plugin API | Research complete; no credential-free write profile | Checked-in REST fixtures for one page; separate plugin bridge for writes | `.fig` is not a public contract; live APIs require credentials, scopes, plan access and rate limits |
| SwiftUI | Swift source and proposal–response layout runtime | Research complete; no implementation | Generated stack/text/shape subset with a pinned Apple toolchain | Arbitrary Swift and custom layouts are executable programs |
| Jetpack Compose | Kotlin source and constraint layout runtime | Research complete; no implementation | Generated row/column/text/shape subset with a pinned Android toolchain | Arbitrary Kotlin, state, modifier chains and subcomposition are executable programs |
| Flutter | Dart source and box-constraint runtime | Research complete; no implementation | Generated row/column/text/shape subset with a pinned Flutter toolchain | Arbitrary Dart, state, inherited widgets and custom render objects are executable programs |

## Implementation order

The SVG basic-shape and DTCG scalar-token profiles are implemented because
their bounded subsets map directly to the current model and run without
credentials or platform SDKs. Full DTCG coverage requires a token-model RFC.
Penpot is the next package adapter after ZIP resource limits and unknown-member
retention have a shared test contract.

React and Svelte require a common retentive source-edit layer rather than whole
file generation. Native declarative UI targets begin as one-way lowerings with
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

The supporting primary-source records are `svg`, `dtcg`, `react-jsx-adapter-surface`,
`svelte-source-adapter-surface`, `penpot`, `figma`, `swiftui-layout`,
`jetpack-compose-layout` and `flutter-layout` under `research/items/`.
