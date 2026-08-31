---
id: nuif:research:web-behavior-host-lowering
kind: synthesis
status: verified
title: Finite browser lowering for portable behavior effects
source:
  url: https://html.spec.whatwg.org/multipage/interaction.html#the-hidden-attribute
  authors: [WHATWG, W3C Accessible Rich Internet Applications Working Group, W3C Web Application Security Working Group, Microsoft Playwright maintainers]
  published_at: 2026-08-30
  license: WHATWG and W3C document terms; Playwright documentation license
retrieved_at: 2026-08-31
tags: [behavior, html, aria, csp, browser, playwright, security, host-adapter]
confidence: 0.99
claims: [nuif:claim:multi-level-ir]
relations:
  - type: depends_on
    target: nuif:research:behavior-portability-state-machines
    note: The host lowering consumes the already bounded deterministic trace semantics rather than defining another state machine.
  - type: depends_on
    target: nuif:research:accessibility-semantics
    note: Native event sources and the live region require a validated semantic web projection.
  - type: related_to
    target: nuif:research:open-ui
    note: Native invoker commands were evaluated as a smaller declarative alternative but cover a different component-level surface.
links:
  spec: [spec/13-semantics-accessibility-and-behavior.md]
  adr: []
  rfc: []
  code: [crates/nuif-html/src/behavior.rs, crates/nuif-testing/src/bin/web-behavior-mapping.rs, tools/accessibility-oracle/behavior-check.mjs, xtask/src/main.rs]
  experiments: [nuif:experiment:web-behavior-mapping]
---

# Summary

A browser adapter can lower the first NUIF behavior profile without accepting
authored JavaScript. The narrow mapping is enabled native-button activation,
the HTML `hidden` property for visibility, and an ARIA `status` live region for
advisory announcements. The state machine remains validated data interpreted by
one fixed generated runtime. An exact CSP hash grants that runtime—and no other
inline script—authority inside the self-contained output.

Native HTML invoker commands are not a replacement for this profile. Their
built-in vocabulary covers popover and dialog actions. A custom command still
requires script and does not carry NUIF state, ordered guards, typed variables
or abstract effects. The smallest faithful result is therefore a finite
interpreter, not a general code generator or a misuse of popover state.

## Evidence

- The HTML Living Standard defines `hidden` for all HTML elements; the hidden
  state is not rendered. This is a direct Boolean host operation rather than a
  CSS-cascade approximation. Locator: HTML §6.1, lines 92–96 of the retrieved
  multipage interaction document,
  https://html.spec.whatwg.org/multipage/interaction.html#the-hidden-attribute,
  retrieved 2026-08-31.
- The HTML button `commandfor`/`command` surface has built-ins for showing,
  hiding and toggling popovers and closing or showing dialogs. The standard's
  custom-command example installs a JavaScript `command` event listener. This
  supports using native commands when their component semantics are exact, but
  not claiming that they encode a general guarded state machine. Locator: HTML
  button element, `commandfor` and examples, retrieved lines 98–175,
  https://html.spec.whatwg.org/dev/form-elements.html#the-button-element,
  retrieved 2026-08-31.
- WAI-ARIA 1.2 defines `status` as advisory live-region information, says it
  should not receive focus as a result of status change, and gives it implicit
  `aria-live=polite` and `aria-atomic=true`. This is the correct first target
  for non-urgent NUIF announcement effects. Locator: WAI-ARIA 1.2 `status` role,
  lines 4471–4480,
  https://www.w3.org/TR/wai-aria-1.2/#status,
  retrieved 2026-08-31.
- CSP Level 3 permits policy delivery through an early `meta` element, while
  preferring the response header for served resources. Hash sources use
  SHA-256/384/512 plus base64, and a matching hash can allow an inline script
  without `unsafe-inline`. The inline body is hashed after UTF-8 encoding.
  Locators: CSP3 §§2.3.1, 3.1, 3.3, 4.2 and 8.4,
  https://www.w3.org/TR/CSP3/,
  retrieved 2026-08-31.
- Playwright documents locators as its auto-waiting/retry unit and recommends
  user-facing roles or explicit test contracts over DOM-structure chains. It
  exposes ARIA snapshots computed from browser accessibility trees. Stable
  `data-nuif-id` is the adapter's explicit test contract; snapshots provide a
  foreign host observation rather than a source-string assertion. Locators:
  https://playwright.dev/docs/locators#introduction and
  https://playwright.dev/docs/aria-snapshots#aria-snapshots,
  retrieved 2026-08-31.
- The HTML activation model says non-click manual activation, including
  keyboard or voice input, fires a `click` event at an element with activation
  behavior. Playwright's `locator.press()` focuses the element and produces a
  key sequence, so separate Enter/Space runs test the native route without
  adding key handlers to the adapter. Locators:
  https://html.spec.whatwg.org/multipage/interaction.html#activation-behavior-of-elements
  and https://playwright.dev/docs/input#keys-and-shortcuts,
  retrieved 2026-08-31.

## Mechanism

`nuif-web-behavior-0` composes the behavior and accessibility profiles. It
rejects checkbox/radio transition sources because a native click also mutates
checked state that the source behavior vocabulary cannot currently express. It
rejects disabled transition sources because native disabled controls do not
produce the promised activation. All enabled button/switch entities receive a
listener so a valid activation with no transition remains an observable no-op.
It also rejects a visibility effect that can hide one of those controls through
the control itself or an ancestor; otherwise later source events could exist in
the abstract trace but not in the native host.

The program JSON is embedded only after `<`, `>`, `&`, U+2028 and U+2029 are
escaped. The static runtime contains no evaluation, dynamic import, request,
timer or handler-attribute surface. Its meta CSP denies all resource classes and
allows the one exact script body. A serving host should deliver and merge its
own response-header policy; the self-contained policy is not assumed to survive
embedding in another document.

One transition may emit at most one non-empty announcement, and may not repeat
the same effect/target pair. A browser can otherwise coalesce multiple
run-to-completion effects into one terminal DOM observation, producing a false
equivalence claim.

## NUIF relevance

The Rust stage validates unsafe embedding strings and every web-specific
refusal, computes the exact script/CSP digests and produces the reference run.
The Playwright stage performs separate five-event pointer and alternating
Enter/Space keyboard sequences in each pinned engine and checks state, selected
transition, retained `hidden` values, live-region text, target attribution,
runtime errors and ARIA snapshots after every event. The first
macOS/arm64 run passes Chromium 151.0.7922.34, Firefox 153.0 and WebKit 26.5.

This proves one browser-host mapping. It does not prove screen-reader speech,
focus order, native UI behavior, checkbox/radio semantics, authored-script
round trips, broader events/effects or a wire-format decision.

## Open questions

- Add a portable checked/pressed-state effect before admitting checkbox/radio
  activation, then test keyboard and pointer traces separately.
- Run assistive-technology announcement timing experiments before claiming
  more than DOM and browser accessibility-tree exposure.
- Define focus effects and focus-restoration laws before adding dialog or
  popover lowering.
- Compare a second host family against the same abstract effects before moving
  the behavior sidecar into a canonical schema proposal.
