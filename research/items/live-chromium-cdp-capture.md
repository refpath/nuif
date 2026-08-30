---
id: nuif:research:live-chromium-cdp-capture
kind: synthesis
status: verified
title: Bounded live Chromium capture transport and executable baseline
source:
  url: https://chromedevtools.github.io/devtools-protocol/
  repository: https://github.com/ChromeDevTools/devtools-protocol
  authors: [Chromium DevTools contributors, W3C Browser Testing and Tools Working Group, Microsoft Playwright contributors, tungstenite contributors]
  published_at: continuously versioned protocol and implementations
  license: mixed specification and permissive implementation licenses
retrieved_at: 2026-08-31
tags: [browser, capture, chromium, cdp, websocket, playwright, webdriver, security, benchmark]
confidence: 0.98
claims: [nuif:claim:source-inference-separation, nuif:claim:authored-resolved, nuif:claim:bounded-untrusted-input]
relations:
  - type: extends
    target: nuif:research:chromium-source-backed-ui-capture
    note: Converts the proposed source-backed observation surface into a bounded live implementation and executable gate.
  - type: compares_to
    target: nuif:research:wasm-headless-execution
    note: Reuses the same exact Chrome for Testing installation while keeping capture transport separate from WebAssembly package testing.
  - type: related_to
    target: nuif:research:reverse-layout-inference
    note: The first live fixture measures held-out viewport prediction without claiming broad layout-intent recovery.
links:
  spec: [spec/11-security.md, spec/14-observation-capture-and-reconstruction.md]
  adr: []
  rfc: [rfcs/0011-observation-and-inference-provenance.md]
  code: [crates/nuif-capture/src/live.rs, crates/nuif-capture/src/lib.rs, crates/nuif-reconstruct/src/lib.rs, crates/nuif-testing/src/bin/live-browser-capture.rs, conformance/browser-oracle.lock.json, tools/browser/install-chrome-for-testing.sh, xtask/src/main.rs]
  experiments: [nuif:experiment:live-browser-source-capture]
---

# Summary

The first executable live-capture profile should use the already pinned Chrome
for Testing build through a small synchronous Chrome DevTools Protocol (CDP)
client. The decision is intentionally narrower than choosing a general browser
automation framework. NUIF needs exact DOM snapshot tables, a computed-style
whitelist, platform-font usage, response bodies, an accessibility tree and a
reference screenshot from one known Chromium build. A bounded loopback
WebSocket transport supplies those operations without adding a second browser
version authority or moving capture semantics into a framework object model.

This is not a general claim that raw CDP is better than Playwright or WebDriver
BiDi. Playwright is the stronger current choice for cross-browser application
testing. WebDriver BiDi is the stronger standards-track transport to watch for
portable remote control. Neither currently improves this profile's required
Chromium-specific evidence while preserving the repository's existing browser
pin and one-process Rust boundary.

## Evidence

- The CDP project documents JSON command/event domains and warns that the
  tip-of-tree protocol changes frequently without a backwards-compatibility
  guarantee. It also documents `DevToolsActivePort`, `/json/list` and the page
  WebSocket endpoint used here. NUIF therefore pins Chrome for Testing
  152.0.7977.64, records the reported product and protocol version, and treats a
  protocol change as a gate failure rather than accepting a floating schema.
  Locator: https://chromedevtools.github.io/devtools-protocol/, retrieved
  2026-08-31.
- `DOMSnapshot.captureSnapshot` returns a flattened DOM/layout table plus a
  requested computed-style whitelist. CSS exposes actual
  `getPlatformFontsForNode` usage, Network exposes bounded response bodies,
  Accessibility exposes the computed tree and Page exposes screenshots.
  Locators, retrieved 2026-08-31:
  https://chromedevtools.github.io/devtools-protocol/tot/DOMSnapshot/,
  https://chromedevtools.github.io/devtools-protocol/tot/CSS/,
  https://chromedevtools.github.io/devtools-protocol/tot/Network/,
  https://chromedevtools.github.io/devtools-protocol/tot/Accessibility/ and
  https://chromedevtools.github.io/devtools-protocol/tot/Page/.
- `Page.getResourceTree` and `Page.getResourceContent` are explicitly marked
  experimental by CDP. They are therefore only a pinned-build fallback after
  passive `Network.getResponseBody`, not a portable capture contract. The
  Fetch domain was also evaluated and rejected for this first profile: its
  response-stage mode pauses every matched request until the client continues
  it, so it changes the loading path being observed. Locators:
  https://chromedevtools.github.io/devtools-protocol/tot/Page/ and
  https://chromedevtools.github.io/devtools-protocol/tot/Fetch/, retrieved
  2026-08-31.
- Playwright versions require particular browser binaries and normally use its
  installation command/cache. That is useful when Playwright owns the test
  matrix, but NUIF already content-pins and installs the browser used by Gate C
  and the WebAssembly smoke test. Adding Playwright here would create a second
  browser compatibility and download lifecycle without replacing the CDP-only
  evidence calls. Reconsider it when Firefox/WebKit live capture becomes an
  implemented gate. Locator: https://playwright.dev/docs/browsers, retrieved
  2026-08-31.
- WebDriver BiDi is a W3C Working Draft on the Recommendation track with a
  Web-platform test suite and implementation report. It is the preferred
  portability watch path, but the current draft's remote-control/network/script
  surface does not replace Chromium's flattened DOM/layout snapshot or
  platform-font-use calls. A future adapter should share NUIF observation types
  rather than force CDP vocabulary into the standard transport. Locator:
  https://www.w3.org/TR/webdriver-bidi/, 29 June 2026 Working Draft, retrieved
  2026-08-31.
- Tungstenite provides a synchronous RFC 6455 client with explicit maximum
  frame/message configuration and no default TLS feature. That matches a
  sequential `ws://127.0.0.1` debugger socket. Tokio-tungstenite, chromiumoxide
  or headless_chrome would add async/runtime or browser-object layers without
  changing the evidence contract. TLS is deliberately absent because remote
  debugger endpoints are rejected. NUIF pins 0.29.0 rather than current 0.30.0:
  the upstream changelog says 0.30 adds rejection of non-compliant clients on
  the server side and updates Rand/SHA/MSRV. This adapter is exclusively a
  client to pinned Chrome; 0.29 preserves the needed API and removes four
  duplicate Digest-family version lines from the resolved graph. Locators:
  https://github.com/snapview/tungstenite-rs and
  https://github.com/snapview/tungstenite-rs/blob/master/CHANGELOG.md,
  retrieved 2026-08-31.

## Mechanism

`cargo xtask gate-j-live` installs or reuses the exact browser lock, then
accepts four complete isolated-profile captures against a bounded concurrent
keep-alive loopback fixture: 360 px, 768 px, held-out 900 px and a repeated
360 px run. An incomplete browser/network outcome is never filled in: the
harness records it and permits at most three fresh-profile attempts for that
viewport. The adapter:

1. creates a fresh temporary profile and accepts only its loopback debugger;
2. caps discovery HTTP; WebSocket frame, message and write-buffer bytes; event
   count and aggregate bytes; command count; connected capture time; DOM
   nodes; per-node and total font uses; resource count; per-resource bytes and
   total retained response bytes, with base64 length checked before decode;
3. enables only the required CDP domains and pins viewport/DPR, `en-US`, UTC,
   screen media, light color scheme and reduced motion;
4. waits for the lifecycle `load` event carrying the exact navigation loader
   ID, disables animation/transition/caret phases, fixes scroll at zero, waits
   two animation frames, awaits every image decode plus `document.fonts.ready`
   and waits two final frames so ready assets are reflected in layout/font-use
   evidence;
5. captures flattened DOM/layout/background style, computed accessibility,
   actual platform-font use and exact HTTP response bodies, using bounded
   Network response bodies first and the post-load Page resource tree/content
   cache as fallback; it drains pending events to a bounded quiet point and
   accepts a bounded PNG only after two consecutive screenshots are identical;
6. replaces opaque CDP backend IDs with deterministic preorder identities;
7. strips URL query/fragment data before a capture can be serialized and never
   requests cookies, storage or request-header fields from CDP; and
8. carries the structured runtime context into the canonical observation
   bundle.

The fixture injects independent query, cookie, storage, Authorization and
custom-header canaries after navigation. The server proves the query and
request values arrived; in-page code proves storage round-tripped and returns
the already-awaited probe response body. That body is rejected if it reflects
any canary, then retained under its same-origin URL. The gate scans serializable
capture, observation, proposal and package bytes for all five values. This is a
specific non-retention regression test, not a claim that arbitrary response
content is free of application secrets.

## Executable result

The gate requires exact repetition of raw capture, normalized observations,
proposal/package bytes and narrow-viewport screenshot. All five declared
response bodies must have exactly the expected SHA-256 set. The custom Ahem
font must be reported as an actually used downloaded font, main/button role and
name must occur in the accessibility tree, and screenshot dimensions must equal
the declared viewport. The report records per-viewport and total attempt counts;
only a complete exact fixture can be accepted, and three failed attempts are a
blocking gate error.

For the declared responsive fixture, linear geometry prediction fitted to the
360/768 px observations must have lower aggregate absolute error at 900 px than
copying the 360 px geometry as a one-screenshot freeform baseline. The machine
report stores both errors and every raw count. This result validates only the
fixture and falsifies a broken multi-viewport path; it is not evidence of broad
responsive-layout inference accuracy.

## Security and non-claims

An isolated profile avoids ambient browser cookies, storage and extensions.
The adapter owns no general login/profile import path. Response bodies remain
untrusted inert package resources and are not executed by package readers.
Captured pages still execute inside Chromium and may make network requests, so
the caller must authorize the target and apply environment/network policy.

Cross-origin opaque bodies, arbitrary application state, local host font bytes,
source-map correlation, canvas/WebGL semantics, video/worklet state and hostile
page determinism remain explicit omissions. Browser crashes, protocol drift and
budget violations fail the capture atomically. Cross-browser reproduction,
authenticated-site capture and a real licensed corpus remain future gates.

## Revisit conditions

Adopt Playwright or an equivalent higher-level runner when a real multi-engine
matrix is funded and its browser pins can become the single matrix authority.
Add WebDriver BiDi when its implemented domains can produce the portable
observation subset. Do not retain a raw CDP command merely because it exists;
every added domain must have a typed output, byte/cardinality ceiling, secret
policy and executable negative case.

## NUIF relevance

The result turns browser capture from a provider-input sketch into one real
ports-and-adapters consumer of the same observation, package and proposal
contracts used by later reconstruction work. Exact source/runtime evidence
stays distinguishable from inferred NUIF semantics, and the browser adapter
does not enter `nuif-core` or redefine the HTML source-synchronization profile.

## Open questions

- Which smallest matched-style and stylesheet/source-correspondence set is
  useful enough to add without making one capture unbounded?
- How should opaque frame/resource evidence be represented across CDP and
  WebDriver BiDi without treating missing source as screenshot equivalence?
- Which cross-OS differences remain after the exact browser/context pin, and
  which belong in environment compatibility rather than capture fidelity?
- Can a licensed real-page corpus preserve sensitive response bodies safely
  enough to evaluate the source-backed route outside synthetic fixtures?
