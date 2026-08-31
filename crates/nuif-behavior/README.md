# NUIF behavior state-machine profile 0

Status: executable research sidecar. `nuif-behavior-state-machine-0` is not yet
part of the canonical NUIF wire model and does not establish the final behavior
schema. It exists to test a small portable execution contract before that
schema is frozen.

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
call a host API. A later web, native or presentation adapter can map admitted
effects under its own fidelity contract.

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
browser DOM adapter, a native UI runtime or evidence for excluded behavior.
