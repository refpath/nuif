# 03 — Components and composition

Status: draft.

A `Component` is a reusable authored definition with typed parameters and named slots. An `Instance` references a component and carries parameter values and non-destructive overrides.

Core parameter classes: boolean, number, string/text, enum, token reference, asset reference and content/slot value.

Variants are named parameter configurations, not duplicate component definitions.

Composition supports references to external NUIF libraries and ordered override layers. Themes and brands SHOULD be expressed as token/layer opinions rather than destructive copies.

DTCG-compatible design tokens are the default token interchange representation. NUIF bindings add stable token identity and resolved context values.
