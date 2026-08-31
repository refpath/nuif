---
id: nuif:spec:semantics-accessibility-and-behavior
kind: specification
status: exploratory
---

# 13 — Semantics, accessibility and behavior

Status: exploratory draft.

## Semantic role layer

NUIF entities MAY carry semantic roles independently of visual entity kind. The
current wire model carries a portable role identifier, one direct accessible
name and Boolean state keys. Document relationships can express
`labelled-by`, `described-by`, `controls`, `owns` and `flow-to`. A direct
description string, non-Boolean value/state data and semantic relationship
cardinality rules require a future schema revision; adapters MUST NOT invent
them from visual geometry.

Adapters MUST map portable semantics to host accessibility facilities where supported and MUST report unsupported or approximated semantics. Visual appearance MUST NOT be treated as sufficient evidence of semantic role.

### Web accessibility projection profile 0

`nuif-web-accessibility-0` is an experimental bounded lowering to inert HTML
and ARIA. It admits at most 4,096 entities and 8,192 relationships. Its role set
is `button`, `checkbox`, `group`, `img`, `main`, `navigation`, `paragraph`,
`radio`, `region` and `switch`. Role-specific required/prohibited naming and
Boolean-state rules are fixed by the profile. A `switch` MUST carry `checked`;
unsupported or misplaced states MUST fail closed. A direct accessible name and
`labelled-by` MUST NOT compete on one entity. Direct and referenced names MUST
be whitespace-normalized for computed-name comparison and MUST NOT be empty
after normalization.

The five relationship kinds above lower to their corresponding ARIA IDREF
attributes using stable NUIF entity identifiers. Relationship order is retained
and duplicate targets fail closed. The `owns` graph MUST be acyclic and each
owned target MUST have at most one ARIA owner. Native HTML semantics are used
where the profile has an exact element; explicit ARIA is used only for `group`,
`img` and `switch`. Output contains no script, external URL, event handler or
synthesized host behavior.

The foreign oracle MUST record the exact test-engine and host versions, compare
computed role/name/state rather than source attributes alone and classify
required-subset loss separately from other host-tree differences. Browser-tree
agreement does not establish native platform API, keyboard interaction or
application behavior equivalence.

## Behavior graph

Portable interaction is represented as a separate bounded graph referencing stable entity/property identities. It MUST NOT embed arbitrary general-purpose code. The initial executable research profile is deliberately smaller than the eventual vocabulary of events, state, value transforms, property writes, navigation and animation triggers.

Behavior capabilities are negotiated like extensions. Missing optional capabilities may degrade according to declared fallback; missing required capabilities prevent a claim of behavioral conformance.

### Behavior state-machine profile 0

`nuif-behavior-state-machine-0` is an experimental sidecar and is not part of
the canonical wire model. It defines a flat deterministic state machine with a
single active state. Its only external event is `activate`, addressed to a
stable entity carrying role `button`, `checkbox`, `radio` or `switch`.
Transitions in the active state are evaluated in authored order; the first
exact event and equality-guard match executes its actions sequentially and
selects its target state. An unmatched event is a no-op. An event MUST complete
before the next external event is accepted.

Profile values are Boolean or bounded string. Actions may set a value, toggle a
Boolean or emit an abstract `visibility(Boolean)` or `announcement(String)`
effect to a stable entity. The reference runtime MUST NOT directly mutate the
document or invoke host APIs. Target adapters consume effects under separately
declared capability and fidelity contracts.

Every used effect capability MUST be declared `required` or `optional_noop`.
A missing required capability MUST reject runtime construction before actions
execute. An unavailable optional capability MUST emit no host effect and MUST
be recorded as skipped in the trace. Unknown or incompatible entities, states,
variables, value types, capabilities, unreachable states and over-limit graphs
MUST fail closed before execution.

The profile admits at most 128 states, 1,024 transitions, 4,096 total actions,
64 actions per transition, 128 variables, 64 capabilities and 4,096 external
events per run. Timers, internal events, parallel states, numeric computation,
navigation, animation, filesystem/network effects and scripts are excluded.
Conformance compares complete event, selected-transition, state, variable,
effect and skipped-capability traces. Final-state agreement alone is
insufficient.

## Host logic boundary

Application business logic, arbitrary network effects and unrestricted scripts are outside the core document model. Adapters may preserve references/bindings to host logic using extensions and provenance, but another implementation is not required to execute unknown host code. The state-machine sidecar MUST NOT be interpreted as authority to execute such bindings.

## Inference

Behavior inferred from screenshots or static design states is never `lossless` solely because the generated result looks equivalent. Inference records MUST identify evidence, confidence and unresolved alternatives.
