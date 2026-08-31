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

Portable interaction is represented as a separate bounded graph referencing stable entity/property identities. The initial graph vocabulary should cover events, state variables, conditions, value transforms, property writes, navigation/state transitions and animation triggers without embedding arbitrary general-purpose code.

Behavior capabilities are negotiated like extensions. Missing optional capabilities may degrade according to declared fallback; missing required capabilities prevent a claim of behavioral conformance.

## Host logic boundary

Application business logic, arbitrary network effects and unrestricted scripts are outside the core document model. Adapters may preserve references/bindings to host logic using extensions and provenance, but another implementation is not required to execute unknown host code.

## Inference

Behavior inferred from screenshots or static design states is never `lossless` solely because the generated result looks equivalent. Inference records MUST identify evidence, confidence and unresolved alternatives.
