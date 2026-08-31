# NUIF behavior state-machine profile 0

Status: executable research sidecar. `nuif-behavior-state-machine-0` is not
part of the canonical semantic `Document` and does not establish the final
behavior schema. Its first experimental transport is the separately profiled
content-addressed package resource below.

## Model and execution

The profile is a flat deterministic state machine keyed by stable NUIF entity
identifiers. External `activate` events are accepted only from entities whose
semantic role is `button`, `checkbox`, `radio` or `switch`. In the active
state, transitions are examined in authored order. The first matching
event/guard executes its actions sequentially and changes the active state; an
unmatched event is a no-op. One external event always runs to completion before
the next starts.

State values are bounded Booleans and strings. Actions can set a value, toggle
a Boolean, or emit one of two abstract effects:

- `visibility`, carrying a Boolean for a stable target entity;
- `announcement`, carrying a bounded string for a stable target entity.

The runtime emits effects as data and does not directly mutate the document or
call a host API. The separate `nuif-web-behavior-0` adapter now maps the first
admitted subset to native browser activation, DOM visibility and a status live
region. Native and presentation adapters still require their own fidelity
contracts.

## Capabilities and limits

Each used effect capability is declared `required` or `optional_noop`. Missing
required capabilities reject runtime construction before any action executes.
An unavailable optional capability follows the profile's declared no-op
fallback and is recorded in the trace. Silent fallback is not permitted.

The static envelope admits at most 128 states, 1,024 transitions, 4,096 total
actions, 64 actions per transition, 128 variables and 64 capabilities. One run
accepts at most 4,096 external events. Identifiers and strings have explicit
byte limits. Unknown fields, entities, states, variables, capabilities, value
types, unreachable states and incompatible activation sources fail closed.

Timers, internal event queues, parallel states, floating-point or integer
arithmetic, navigation, animation, document mutation, filesystem/network
effects and arbitrary scripts are outside profile 0.

## Differential oracle

`cargo xtask gate-behavior` executes the same fixture through the Rust
reference runtime and a separately written JavaScript interpreter under pinned
Node in CI. It compares complete event, transition, state, variable, emitted
effect and skipped-optional traces for both full and required-only capability
sets. It separately requires a missing required capability to fail before
execution.

Artifacts:

- `target/behavior-portability-fixture.json`;
- `target/behavior-portability-static-report.json`;
- `target/behavior-portability-report.json`.

The JavaScript oracle is a second implementation of this profile. It is not a
browser DOM adapter or native UI runtime. Browser host mapping is tested
separately by `cargo xtask gate-web-behavior`; neither gate is evidence for
excluded behavior.

## Package attachment

`nuif-behavior-package-resource-0` stores one program as canonical CBOR in one
embedded `source` resource with provisional media type
`application/nuif-behavior+cbor`. The normal package manifest records its size,
SHA-256 digest and digest-derived blob path and declares
`nuif-behavior-state-machine-0` as required. No new `Document` field or ZIP
member family is introduced. The API is behind the opt-in Cargo feature
`package`, keeping state-machine-only consumers independent of the package and
codec dependency stack.

```rust
let digest = nuif_behavior::attach_behavior(&mut package, &program)?;
let bytes = package.encode()?;

let package = nuif_package::NuifPackage::decode(&bytes)?;
package.require_capabilities(&host_capabilities)?;
let attachment = nuif_behavior::attached_behavior(&package)?;
```

Generic package decode verifies and preserves the resource without executing
it. `attached_behavior` is the explicit opt-in that checks exact cardinality,
descriptor policy, canonical CBOR and every entity reference against the
package document. Runtime construction remains a later operation requiring a
caller-supplied set of effect capabilities. The behavior digest identifies the
program bytes; the complete package hash binds those bytes to the delivered
document.

`cargo xtask gate-behavior-package` records the Rust attachment checks and an
independent Python standard-library ZIP inspection in:

- `target/behavior-package-fixture.nuif`;
- `target/behavior-package-expected.json`;
- `target/behavior-package-static-report.json`;
- `target/behavior-package-report.json`.

`cargo xtask gate-behavior` runs this attachment gate before the independent
Rust/Node trace gate.
