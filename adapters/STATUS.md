# Adapter implementation status

The adapter program separates an ecosystem's public interchange or source
surface from the subset for which NUIF can provide executable round-trip laws.
Research coverage does not imply an implemented conformance profile.

The inventory contains twelve targets and eleven executable profiles across
seven target families. The remaining targets have no executable direction in
`adapters/index.json`.

This table is explanatory. `adapters/index.json` is the machine-audited target
inventory; `cargo xtask adapter-audit` checks its research, profile, crate and
gate references and writes `target/adapter-coverage-report.json`.

| Target | Primary integration surface | Executable status | Next bounded profile | Boundary |
| --- | --- | --- | --- | --- |
| HTML/CSS | DOM and CSS source | `nuif-html-css-0`, `nuif-html-css-v0`, `nuif-web-accessibility-0` and `nuif-web-behavior-0` | Extend only with separately tested CSS/layout/focus/control-state features | Arbitrary cascade, authored scripts and unmarked DOM are not imported; behavior lowering is one-way and finite |
| SVG | SVG 2 XML | `nuif-svg-0` | Add paths or transforms only under separately declared geometry laws | Paths, transforms, CSS cascade, paint servers, effects, animation, scripts and external resources are excluded |
| DTCG tokens | Design Tokens Format Module 2025.10 JSON | `nuif-dtcg-scalar-0` | Expand only after a token-model RFC and separate profile | Core tokens lack declared type, groups, aliases, descriptions, deprecation and token-local extensions |
| React | JSX source and React DOM properties | `nuif-react-jsx-0` | TSX or CSS-class support only under a separate grammar/runtime matrix | Components, hooks, spreads, control flow and runtime expressions require execution |
| Svelte | `.svelte` source and compiler AST | `nuif-svelte-static-0` | Component CSS only after selector/cascade/scope-hash rules | Runes, scripts, blocks, directives, preprocessors, component CSS and dynamic components require execution or another profile |
| Penpot | `.penpot` v3 ZIP and JSON package | `nuif-penpot-v3-0` | Add compact pages only after the opt-in representation stabilizes | Components, libraries, interactions, media, paths, layout and compact pages are excluded |
| Figma | Normalized Plugin API snapshot/plan plus writable host | `nuif-figma-plugin-snapshot-0` mapping and compiled no-network review shell; no live host run | Assigned-ID reviewer run and live fixtures in `adapters/figma/PROFILE-DRAFT.md` | `.fig` is not a public contract; static evidence does not prove host writes, undo or persistence |
| Affinity | User-mediated SVG import/export in the desktop application | Research and existing `nuif-svg-0` bridge only; no live Affinity trial | Retained two-way SVG trial in `adapters/affinity/PROFILE-DRAFT.md` | No public document API or native `.af*` schema is claimed; native files are opaque and UI automation is non-conformant |
| Canva | Apps SDK Design Editing API; Connect APIs are a separate OAuth workflow | `nuif-canva-design-editing-0` mapper plus deterministic Canva-only review shell, cross-language plan validation, mock one-sync tests and maximum-profile measurements; no live host run | Reviewer-run current-page transaction with one-sync/one-undo evidence in `adapters/canva/PROFILE-DRAFT.md` | Live import is narrowed to empty-page unnamed opaque rectangles/ellipses; live mutation, preview APIs, Docs, native NUIF Connect I/O and marketplace approval are excluded |
| SwiftUI | Swift source and proposal–response layout runtime | Research complete; no implementation | Generated stack/text/shape subset with a pinned Apple toolchain | Arbitrary Swift and custom layouts are executable programs |
| Jetpack Compose | Kotlin source and constraint layout runtime | Research complete; no implementation | Generated row/column/text/shape subset with a pinned Android toolchain | Arbitrary Kotlin, state, modifier chains and subcomposition are executable programs |
| Flutter | Dart source and box-constraint runtime | Research complete; no implementation | Generated row/column/text/shape subset with a pinned Flutter toolchain | Arbitrary Dart, state, inherited widgets and custom render objects are executable programs |

## Implementation order

The SVG basic-shape, DTCG scalar-token and Penpot v3 profiles are implemented
because their bounded subsets map directly to the current model and run without
credentials or platform SDKs. Full DTCG coverage requires a token-model RFC.
Penpot's package path enforces ZIP resource limits and unknown-member retention
through one shared test contract. Figma's normalized snapshot and mutation-plan
mapping has exact Rust/CLI trials, while its host execution remains
uncertified. Affinity has a bounded interchange draft over the existing SVG
profile. Canva has the shared `HostAdapterReport` envelope and a compiled shell
that consumes Rust plans, validates an exact transport, rejects unsupported
host mutations before insertion and packages the Canva SDK license. None of
these vendor paths has live-host evidence yet.

Canva now has a pure normalized current-page mapper with deterministic
round-trip IDs, typed unsupported-property fidelity, strict resource limits,
maximum-profile timing and a deterministic no-network review artifact; the
gate deliberately records live host execution as not run. React and Svelte
now use the common byte-span correspondence contract for one marked static
subtree. Svelte additionally compiles direct and CLI output with
the exact official compiler. Native declarative UI targets begin as one-way lowerings with
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

The supporting primary-source records are `svg`, `dtcg`, `accessibility-semantics`,
`react-jsx-adapter-surface`, `svelte-source-adapter-surface`, `penpot`, `figma`,
`affinity-interchange-and-adoption`, `canva-apps-and-connect-adoption`,
`swiftui-layout`, `jetpack-compose-layout` and `flutter-layout` under
`research/items/`. The earlier `adobe-uxp-host-integration` record remains
historical prior art rather than an advertised target.
