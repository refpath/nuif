---
id: nuif:research:chromium-source-backed-ui-capture
kind: standard
status: reviewed
title: Chromium DevTools Protocol as a source-backed UI observation surface
source:
  url: https://chromedevtools.github.io/devtools-protocol/tot/DOMSnapshot/
  repository: https://github.com/ChromeDevTools/devtools-protocol
  authors: [Chromium Project]
  published_at: continuously versioned protocol
  license: BSD-3-Clause implementation repository
retrieved_at: 2026-08-30
tags: [browser, capture, dom, css, network, accessibility, screenshot, provenance]
confidence: 0.97
claims: [nuif:claim:source-inference-separation, nuif:claim:authored-resolved]
relations:
  - type: extends
    target: nuif:research:tree-sitter
    note: Adds resolved browser observations that static source parsing cannot supply.
  - type: related_to
    target: nuif:research:reverse-layout-inference
    note: Multi-viewport browser observations constrain later layout inference.
  - type: related_to
    target: nuif:research:accessibility-semantics
links:
  spec: [spec/04-layout.md, spec/09-provenance-and-fidelity.md, spec/11-security.md, spec/13-semantics-accessibility-and-behavior.md]
  adr: []
  rfc: [rfcs/0003-authored-resolved-provenance.md]
  code: [adapters/html-css/PROFILE.md]
  experiments: [nuif:experiment:browser-source-capture]
---

# Summary

The Chrome DevTools Protocol (CDP) exposes complementary browser observations:
DOM and layout snapshots, computed and matched CSS, platform-font usage,
stylesheet text, response bodies, accessibility trees, screenshots and browser
environment emulation. Together these provide much stronger evidence than a
screenshot, but they still describe one browser execution under one context;
they do not reveal arbitrary application intent or guarantee access to local
font bytes.

NUIF should add a dedicated, pinned browser-capture adapter. It should not turn
the current Tree-sitter source-synchronization adapter into a browser runtime.
Static source retention and resolved browser capture are separate evidence
channels that may be correlated through provenance.

## Evidence

- CDP `DOMSnapshot.captureSnapshot` returns flattened documents including
  iframes, template contents, imported documents and flattened shadow trees,
  with requested computed styles. It can include DOM rectangles, inline text
  boxes, paint order and blended background/text colors.
- CDP CSS exposes `getComputedStyleForNode`, `getMatchedStylesForNode`,
  `getStyleSheetText`, media queries, pseudo-state forcing and
  `getPlatformFontsForNode`. The last operation reports platform-font usage,
  not the original local font file bytes.
- CDP Network exposes request/response events and `getResponseBody`; downloaded
  image, stylesheet and web-font response bytes can therefore be captured while
  the request remains available to the protocol session.
- CDP Accessibility exposes full or partial accessibility trees, adding
  browser-computed role/name/state evidence distinct from the DOM tree.
- CDP Page exposes screenshots and an MHTML snapshot. Emulation exposes device
  metrics, media features, locale, timezone and related execution context.
- CSS Font Loading Level 3 defines `document.fonts.ready` as a readiness signal
  after font loading and layout operations complete; it does not make local
  font files distributable or directly retrievable.

## Mechanism

A capture run pins browser build and records a `CaptureContext`: operating
system, viewport, device-pixel ratio, page scale, locale, timezone, color
scheme, reduced-motion preference, font environment, pseudo states, scroll
positions, navigation URL and a deterministic settling policy. The policy waits
for the load milestone, network quiescence bounded by a timeout,
`document.fonts.ready`, and an explicit animation-freeze point.

For each declared viewport and state the adapter collects:

1. original HTML/CSS response bodies when available;
2. DOMSnapshot nodes, layout, inline boxes, computed-style whitelist and paint
   order;
3. matched rules and stylesheet text needed for source correspondence;
4. downloaded resource bodies, final URLs, response media types and hashes;
5. platform-font usage and font readiness;
6. the accessibility tree;
7. a reference screenshot and its exact capture parameters.

Canvas, WebGL, video and worklet output are observation boundaries: capture a
bounded raster/frame and retain source/provenance where accessible, but do not
pretend the pixels are an editable semantic reconstruction. Cookies,
authorization headers, form secrets, storage and credentials are excluded from
export. Captured scripts are inert source resources and are never executed by a
NUIF reader.

## NUIF relevance

**Borrow** CDP as the first source-backed Web observation port because its
layout, style, resource and accessibility domains can be pinned to a browser
build and replayed as fixture evidence.

**Adapt** observations into typed NUIF provenance and fidelity. Multiple
viewports/states constrain responsive inference; authored source spans remain
separate from resolved browser values.

**Reject** “DOM equals design intent,” capture under unspecified host settings,
credential export, automatic external fetch during NUIF load, and lossless
classification for canvas/video/script behavior based only on a frozen frame.

## Open questions

- Which CDP protocol revision should be pinned independently of the Chrome for
  Testing build used by layout differential tests?
- Which computed-style subset is sufficient to reconstruct the first browser
  profile without making snapshots unbounded?
- How can cross-origin iframes and opaque responses report unavailable evidence
  without silently flattening them into screenshots?
- Which interaction states can be captured reproducibly without executing
  untrusted navigation actions?
