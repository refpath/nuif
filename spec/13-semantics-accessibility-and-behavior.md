# 13 — Semantics, accessibility and behavior

Status: exploratory draft.

## Semantic role layer

NUIF entities MAY carry semantic roles independently of visual entity kind. Core semantics include a portable role identifier, accessible name/description sources, state/property values and semantic relationships such as labelled-by, described-by, controls, owns and flow/order relationships.

Adapters MUST map portable semantics to host accessibility facilities where supported and MUST report unsupported or approximated semantics. Visual appearance MUST NOT be treated as sufficient evidence of semantic role.

## Behavior graph

Portable interaction is represented as a separate bounded graph referencing stable entity/property identities. The initial graph vocabulary should cover events, state variables, conditions, value transforms, property writes, navigation/state transitions and animation triggers without embedding arbitrary general-purpose code.

Behavior capabilities are negotiated like extensions. Missing optional capabilities may degrade according to declared fallback; missing required capabilities prevent a claim of behavioral conformance.

## Host logic boundary

Application business logic, arbitrary network effects and unrestricted scripts are outside the core document model. Adapters may preserve references/bindings to host logic using extensions and provenance, but another implementation is not required to execute unknown host code.

## Inference

Behavior inferred from screenshots or static design states is never `lossless` solely because the generated result looks equivalent. Inference records MUST identify evidence, confidence and unresolved alternatives.
