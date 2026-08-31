# Web accessibility projection profile 0

Status: executable bounded research profile (`nuif-web-accessibility-0`). This
profile projects portable NUIF semantics into inert native HTML and ARIA, then
compares computed role, accessible name and supported state exposure across
pinned Chromium, Firefox and WebKit test engines. It is not an arbitrary ARIA
serializer, a behavior runtime or a claim about branded browsers and native
platform accessibility APIs.

## Portable subset

The profile accepts at most 4,096 structurally valid entities and 8,192
relationships. Ten roles are admitted:

- `button`, `checkbox`, `radio` and `switch` for the bounded interactive
  semantic surface;
- `img`, `main`, `navigation`, `paragraph`, `region` and `group` for bounded
  content and landmark semantics.

Native HTML is preferred where its implicit role matches: `button`, checkbox
and radio `input`, `main`, `nav`, named `section` and `p`. `switch`, `img` and
`group` use explicit roles because there is no equivalent element in this
profile. Required-name, prohibited-name and role/state combinations are checked
before output. `switch` requires an explicit Boolean `checked` state. A direct
accessible name and a `labelled-by` relationship cannot both supply the same
node's name.

The Boolean state subset is deliberately role-specific:

- button: `disabled`, `expanded`, `pressed`;
- checkbox/radio: `checked`, `disabled`, `required`;
- switch: `checked`, `disabled`;
- non-widget roles: no state keys in profile 0.

Relationships map by stable entity identifier: `labelled-by` to
`aria-labelledby`, `described-by` to `aria-describedby`, `controls` to
`aria-controls`, `owns` to `aria-owns` and `flow-to` to `aria-flowto`.
Repeated relationship targets and unnamed `labelled-by` targets fail closed.
The source order of multiple relationship targets is retained because it can
affect accessible-name computation. `owns` additionally requires a directed,
acyclic, single-owner graph; multiple semantic parents and owned-tree cycles
are rejected before HTML is emitted.

Direct and referenced names are normalized to the whitespace form produced by
the accessible-name algorithm before they become oracle expectations. A name
that is empty after normalization fails closed rather than silently becoming
an unnamed control.

## Security and behavior boundary

Output contains no scripts, external URLs, event handlers or generated
application behavior. Native controls retain their built-in focus and state
exposure, but the profile does not invent activation results, navigation or
business logic. A correct accessibility tree is not evidence of keyboard-flow,
interaction or application behavior equivalence.

Unsupported roles, states, ambiguous names, malformed containment and
relationships outside the profile return typed errors without partial output.
The projection keeps `data-nuif-id` on every element so a foreign observation
can be attributed back to the stable NUIF entity.

## Foreign oracle

`cargo xtask gate-accessibility` generates one eleven-node fixture covering
every admitted role and Boolean state, installs the
exact Playwright 1.62.1 browser set and compares the required role/name/state
subset plus full ARIA snapshots across its Chromium, Firefox and WebKit
engines. The report records package, engine, Node, operating-system and
architecture versions. Required-subset mismatches are classified as semantic
loss; non-required tree differences are retained separately as host-tree
differences.

The current macOS/arm64 trial reports identical bounded snapshots from Chromium
151.0.7922.34, Firefox 153.0 and WebKit 26.5. Hosted Linux evidence is produced
by CI and must not be inferred merely from workflow configuration.

Artifacts:

- `target/accessibility-mapping-static-report.json`;
- `target/accessibility-mapping-report.json`;
- `target/accessibility-mapping-fixture.html`;
- `target/accessibility-mapping-expected.json`.

Descriptions as direct semantic strings, numeric/value states, live regions,
tables, trees, grids, composite-widget focus, keyboard interactions and native
Apple/Microsoft/Linux/mobile mappings require separate model and profile work.
