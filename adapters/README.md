# Adapters

External ecosystems are peers around NUIF, not parents of its canonical model.

The first executable adapter is the deliberately bounded [`nuif-html-css-0`](html-css/PROFILE.md) retentive profile. It records concrete byte spans, round-trips its declared container/text/token subset exactly, applies mapped changes without regenerating comments or unmapped regions, and rejects every unsupported semantic change with target/property fidelity.

The follow-on [`nuif-html-css-v0`](html-css/V0-PROFILE.md) profile carries the complete responsive-card model, including responsive rules, component/instance identity, unknown kinds and opaque extensions. Its automated editor bridge proves semantic editor output can patch retained source and return through the CLI to byte-identical canonical NUIF. Path rendering, instance materialization, unknown visuals and arbitrary HTML/CSS remain explicitly outside the target profile.

The separate [`nuif-web-accessibility-0`](html-css/ACCESSIBILITY-PROFILE.md)
projection lowers a ten-role, role-specific Boolean-state and five-relationship
subset into inert native HTML and ARIA. A pinned Playwright oracle compares the
computed roles, accessible names and supported states of the same eleven-node
fixture across Chromium, Firefox and WebKit while retaining host-tree
differences separately. It does not synthesize behavior or claim native
platform accessibility equivalence.

The one-way [`nuif-web-behavior-0`](html-css/BEHAVIOR-PROFILE.md) projection
composes that semantic mapping with the bounded state-machine sidecar. It maps
enabled native button/switch activation, `hidden` visibility and one polite
status announcement through a generated finite runtime authorized by its exact
CSP hash. Three Playwright engines reproduce every event's transition, state
and retained host effects. Authored JavaScript, behavior import, focus,
checkbox/radio mutation and assistive-technology speech remain excluded.

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

The [`nuif-penpot-v3-0`](penpot/PROFILE.md) profile maps one Penpot v3 package,
file, page and board with direct rectangle, ellipse and pinned literal-text
children. It retains unedited member payloads and unknown package data, returns
the original archive byte-for-byte on no-op synchronization, and applies ZIP,
expanded-data, member, compression and JSON resource limits before parsing.

The [`nuif-react-jsx-0`](react/PROFILE.md) profile extracts one directly
returned, marked intrinsic JSX subtree without executing JavaScript. It maps
fixed flex containers and literal pinned-font text through byte spans, retains
unrelated module source and rejects components, spreads, handlers and runtime
expressions.

The [`nuif-svelte-static-0`](svelte/PROFILE.md) profile maps one marked static
Svelte component made of regular containers and literal text. It patches the
same 21 semantic correspondences through the shared scalar planner, rejects
executable template constructs and checks every synchronized source against
the exact official Svelte compiler as a foreign oracle.

The [`nuif-figma-plugin-snapshot-0`](figma/SNAPSHOT-PROFILE.md) profile now
implements the credential-free pure mapping between normalized Plugin API
objects, canonical NUIF and a host mutation-plan tree. The compiled no-network
review shell in [`figma/plugin`](figma/plugin) consumes that exact schema,
requires confirmation before mutation and is checked against the Rust importer.
It covers a deliberately narrow visible/opaque fixed-size subset, repairs
portable identity deterministically and reports every declared Figma-only
property. The shell is static evidence, not a live Figma claim.

Canva now has the credential-free [`nuif-canva-design-editing-0`](canva/PROFILE-DRAFT.md)
normalized current-page mapper and CLI/gate evidence. Its pure mapping is
executable, while live Apps SDK mutation, undo and marketplace evidence remain
external. The remaining researched or externally bounded targets are Affinity,
Flutter, SwiftUI and Jetpack Compose. Figma and Canva retain bounded API-host
adapters and serializable host-object correspondence reports; Affinity has a
separate user-mediated SVG bridge draft. None has a corresponding live host
claim. Broader HTML/CSS, SVG and DTCG profiles remain
separate future work beyond the eleven executable profiles. Each adapter must
emit structured fidelity diagnostics and record provenance/correspondence
sufficient for later synchronization and minimal source patches where feasible.

[`STATUS.md`](STATUS.md) records the current primary integration surface,
implementation status, next bounded profile and exclusion boundary for every
advertised target. Research coverage and executable conformance are listed
separately.

[`index.json`](index.json) is the machine-readable counterpart. `cargo xtask
adapter-audit` requires all advertised targets to have a primary research
record, explicit target and per-profile directionality, a next bounded profile
and a non-empty boundary. Integrated entries additionally require each
profile's directions to be a subset of the target union plus crate, profile and
routed gate paths; non-integrated entries cannot claim executable directions. The
audit writes `target/adapter-coverage-report.json` and blocks the complete gate.

Vendor-specific semantics belong in namespaced extensions or adapter-local logic; they must not leak into the core merely because a vendor is popular.

Source and file-interchange adapters use `AdapterReport` and byte-span or
artifact correspondence. Plug-in/API hosts use `HostAdapterReport` and stable
host-object identifiers because they do not expose retained source bytes. See
ADRs 0008 and 0012 and `docs/HOST-INTEGRATION.md`.
