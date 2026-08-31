# NUIF Canva review app

This is a compiled, no-network review shell for the bounded
`nuif-canva-design-editing-0` profile. It is a development artifact, not a
published Canva app and not evidence of live-host conformance.

## Build

Use the pinned Node range from `package.json`:

```sh
npm ci --ignore-scripts --no-audit --no-fund
npm run check
npm run benchmark -- ../../../target/canva-app-benchmark-report.json
```

The build compiles against the exact stable `@canva/design` package, produces
one JavaScript bundle and writes a hash-bearing `dist/build-report.json`. The
app has no backend, network allowlist, worker, nested frame, analytics or
authentication path.

The generated review artifact includes `CANVA-SDK-LICENSE.md`. Canva's SDK
license limits the bundle to permitted apps on the Canva Platform; it is not a
general-purpose NUIF browser package and must not be redistributed as one.

The pinned 2.12.0 package contains one invalid empty statement in its generated
ambient declaration. `prepare-types` verifies that exact source fragment,
removes it only in a generated type-check copy under `target/`, and records the
source and result hashes. The runtime build continues to resolve the untouched
official package. Any upstream declaration change makes this normalization
fail for review instead of silently rewriting new code.

The benchmark records parse/validate and host-preflight scaling through the
16,384-element profile maximum, a 1,024-element mock one-sync transaction and
duplicate-ID rejection. Results are informational until repeated CI runs and a
named live Canva trial establish enforceable latency budgets.

The resulting `dist/app.js` is the standalone JavaScript bundle used as the
app source in Canva's local development flow. Canva owns the app ID, intent,
origin and deployment metadata; none is invented or committed here.

## Review flow

Export reads one unlocked, fixed-size current page and downloads a normalized
JSON snapshot for the Rust adapter:

```sh
nuif import canva-design-editing-0 current-page.json document.nuif.json fidelity.json
```

Import starts outside Canva and emits the exact review envelope:

```sh
nuif export document.nuif.json canva-design-editing-plan-0 plan.json fidelity.json
```

Load the plan, inspect its summary, select the explicit confirmation, and then
apply. The complete envelope and lossless report are checked first. The live
host preflight is intentionally narrower than the pure mapping profile because
the public API does not expose stable element IDs, element names, exact text
height, or a portable font-file identity.

The first host mutation subset therefore accepts only unnamed, opaque,
unlocked rectangles and canonical ellipses on an empty page of exactly the
same dimensions. Text, groups, names, alpha, existing page content and every
unsupported property fail before insertion. Supported elements are inserted
in order and the session is synchronized once, producing one host undo step.

## Evidence boundary

Credential-free CI proves exact package type checking, fail-closed protocol
validation, pre-mutation rejection, a single-sync mock transaction,
deterministic bundling and Rust-plan interoperability. A reviewer-run Canva
trial is still required to establish actual insertion, session expiry,
cancellation and undo behavior in a named editor and Apps SDK version.

A marketplace candidate would additionally require product-owned localization,
the current Canva UI Kit and intent checklist, accessibility review, privacy
and data-use disclosures, developer verification, listing material and an
explicit owner-authenticated submission. Those are not properties of this
credential-free review bundle.
