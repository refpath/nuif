# Web behavior projection profile 0

Status: executable bounded research profile (`nuif-web-behavior-0`). This is a
one-way host lowering from `nuif-behavior-state-machine-0` plus
`nuif-web-accessibility-0` to native HTML activation and observable DOM/ARIA
effects. It is not a JavaScript interchange format and does not import behavior
from HTML.

## Admitted mapping

The complete source behavior program must first pass its own identifier, type,
graph, capability and resource checks. The document must also pass the web
accessibility projection. This web profile then admits:

- `activate` only on enabled native `button` elements or the button-backed
  `switch` role;
- `visibility(Boolean)` as `target.hidden = !value`;
- one non-empty `announcement(String)` per transition through an unfocused
  `role="status"`, `aria-live="polite"`, `aria-atomic="true"` region;
- ordered guards, sequential set/toggle/effect actions and run-to-completion
  state changes exactly as defined by the source profile.

Every enabled button/switch in the projected document is bound, including a
control with no matching transition. This preserves the source profile's
observable unmatched-event no-op. Checkbox and radio activation are excluded
because their native checked-state mutation has no corresponding action in the
current behavior model. Disabled transition sources fail before output because
native disabled controls do not produce the required activation.
Visibility effects that can hide any admitted event source, including through
an ancestor, also fail because a later abstract activation could no longer be
produced by the native host.

At most one announcement may occur in a transition. Repeated effects of the
same kind and target in one transition also fail. These rules prevent multiple
abstract effects from being coalesced into a single host observation within one
browser task.

## Generated runtime and authority

The output contains one fixed interpreter template plus the validated behavior
program encoded as JSON data. JSON embedding escapes `<`, `>`, `&`, U+2028 and
U+2029, so behavior strings cannot terminate the script element. The runtime
uses no `eval`, `Function`, dynamic import, request API, timer, event-handler
attribute or authored code. A document-level Content Security Policy permits
the exact UTF-8 script body by its generated SHA-256 hash and otherwise denies
scripts, connections, images, fonts, forms and base-URL changes.

An HTTP `Content-Security-Policy` response header remains preferable when a host
serves the output; the generated `meta` policy makes the self-contained fixture
enforceable and testable. Hosts embedding the body into their own page must
merge policies themselves rather than assuming this document policy transfers.

Native `commandfor` was considered. Its built-ins address popovers and dialogs;
custom commands still require a script listener and do not represent the
profile's state, guards, variables, visibility and announcement effects. A
small generated interpreter is therefore the narrower exact mapping.

## Foreign browser oracle

`cargo xtask gate-web-behavior` generates the same two-state, five-event fixture
used by the independent trace gate, computes its reference Rust trace, and
drives separate pointer and keyboard sequences through exact Playwright 1.62.1
Chromium, Firefox and WebKit engines. The keyboard sequence alternates Enter
and Space. After every event it compares selected transition, target state,
retained visibility, live-region text and stable announcement target. It also
retains status/body ARIA snapshots, browser versions, Node/OS/architecture and
runtime errors.

The current macOS/arm64 trial passes Chromium 151.0.7922.34, Firefox 153.0 and
WebKit 26.5 for all five events. Hosted Linux evidence is produced by CI and is
not inferred from workflow configuration.

Artifacts:

- `target/web-behavior-static-report.json`;
- `target/web-behavior-report.json`;
- `target/web-behavior-fixture.html`;
- `target/web-behavior-expected.json`.

The gate observes browser DOM state and the browser accessibility tree. It does
not establish branded-browser behavior, screen-reader speech/timing, native
platform UI, focus choreography, checkbox/radio semantics, navigation,
animation, timers, networking, filesystem access or arbitrary host business
logic. Those require separate profiles and oracles.
