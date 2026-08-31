---
id: nuif:research:behavior-portability-state-machines
kind: synthesis
status: verified
title: Bounded portable behavior through deterministic state-machine traces
source:
  url: https://www.w3.org/TR/scxml/
  authors: [W3C Voice Browser Working Group, Khronos 3D Formats Working Group]
  published_at: 2015-09-01
  license: W3C document license and Khronos specification terms
retrieved_at: 2026-08-31
tags: [behavior, state-machine, event, capability, deterministic, sandbox, differential-testing]
confidence: 0.99
claims: [nuif:claim:multi-level-ir]
relations:
  - type: related_to
    target: nuif:research:gltf-interactivity
    note: KHR_interactivity supplies the contemporary behavior-graph and optional-operation precedent; this synthesis chooses a smaller finite execution profile for NUIF's first oracle.
  - type: related_to
    target: nuif:research:accessibility-semantics
    note: Activation sources use portable semantic entity identity rather than inferred visual affordances.
links:
  spec: [spec/13-semantics-accessibility-and-behavior.md]
  adr: []
  rfc: []
  code: [crates/nuif-behavior/src/lib.rs, crates/nuif-testing/src/bin/behavior-portability.rs, tools/behavior-oracle/check.mjs, xtask/src/main.rs]
  experiments: [nuif:experiment:behavior-portability]
---

# Summary

Portable behavior should begin with deterministic event traces, not arbitrary
host scripts or an immediately universal visual-scripting graph. SCXML provides
the mature semantics needed for the first layer: external events, ordered
transition selection, guards, actions and run-to-completion execution.
KHR_interactivity provides a current asset-format precedent for typed variables,
explicit operations, bounded validation and no-op degradation of unsupported
optional operations. Its much broader operation graph is still a Release
Candidate and is intentionally Turing-complete, so copying it wholesale would
expand NUIF's safety and conformance surface before basic portability is proven.

The best first experiment is therefore a flat, bounded state-machine sidecar
that references stable NUIF entities and emits abstract effects as data. The
same authored program is executed by independent Rust and JavaScript runtimes;
complete traces, not merely final pixels, are the conformance observation.

## Evidence

- SCXML 1.0 is a W3C Recommendation. Its basic model selects transitions from
  an active state in response to events, permits conditions and resolves
  multiple matches by document order. Its interpreter principles require
  causality, deterministic behavior without external processors and
  run-to-completion before another external event is processed. This supports
  ordered guarded transitions and one-event-at-a-time traces. Locator:
  https://www.w3.org/TR/scxml/#Basic, and
  https://www.w3.org/TR/scxml/#AlgorithmforSCXMLInterpretation, retrieved
  2026-08-31.
- The current KHR_interactivity specification identifies itself as Release
  Candidate. It describes directed acyclic behavior graphs, strictly typed
  value sockets, retained custom variables, explicit static/dynamic resource
  limits and unsupported extension operations demoted to no-ops. It also says
  arbitrary scripting is not a design goal and acknowledges that its complete
  execution model is Turing-complete, requiring runtime limits. NUIF profile 0
  consequently adopts the type, capability and limit lessons while excluding
  internal loops, timers and general computation. Locator:
  https://raw.githubusercontent.com/KhronosGroup/glTF/refs/heads/main/extensions/2.0/Khronos/KHR_interactivity/Specification.adoc,
  sections Introduction, Concepts, Unsupported Operations and Limits,
  retrieved 2026-08-31.
- Khronos submitted KHR_interactivity for ratification on 16 July 2026. The
  announcement describes event/control/data/state nodes, custom variables,
  property writes and extension-defined capabilities; unavailable companion
  operations degrade to no-ops. Submission is not ratification, so NUIF must
  retain the Release Candidate status in its decision record. Locator:
  https://www.khronos.org/news/press/gltf-interactivity-extension-submitted-for-ratification,
  retrieved 2026-08-31.
- The Khronos interactivity test-asset repository currently reports 149
  self-checking cases and 831 sub-tests and recommends both manual and automated
  engine execution. This supports generated fixtures and machine-readable
  traces rather than prose-only behavior claims. Locator:
  https://github.com/KhronosGroup/glTF-Test-Assets-Interactivity/blob/main/Tests/Interactivity/README.md,
  retrieved 2026-08-31.

## Mechanism

`nuif-behavior-state-machine-0` admits `activate` events only from stable NUIF
entities carrying an activatable semantic role. The active state's transitions
are evaluated in authored order. The first exact event and equality-guard match
executes at most 64 sequential actions, then reaches its target state. There is
no internal event generation, recursion or asynchronous continuation, so every
accepted external event terminates under a statically known action budget.

The value surface is Boolean plus bounded string. State actions set or toggle
those values. Effects are abstract `visibility(Boolean)` and
`announcement(String)` records addressed to stable entities. A host declares
the effect capabilities it implements. Missing required capabilities reject
runtime construction; unavailable `optional_noop` effects are skipped and
recorded. This is an explicit fallback policy, not silent loss.

The Rust gate produces a five-event fixture and expected full/required-only
traces. A separately written Node interpreter validates and executes the same
program, then compares every selected transition, state, variable, effect and
skipped optional operation. The initial trial passes both capability runs and
the required-capability refusal probe on local Node 26.7.0; CI separately pins
Node 24.20.0 and records its hosted result rather than inferring it from the
workflow.

## NUIF relevance

Behavior should be a modular layer over stable document identity. It should not
make the canonical document an event log, grant scripts authority, or force
every reader to implement timers, networking and host business logic. The
sidecar experiment establishes semantics and test vectors before any wire
schema decision. Once multiple target adapters need the same model, a future
RFC can decide whether behavior becomes a package member, a canonical document
section or a namespaced extension.

The abstract-effect boundary also keeps target fidelity honest. A web adapter
now maps the bounded subset to native activation, DOM visibility and one status
region under `nuif-web-behavior-0`; a presentation runtime may map it to scene
visibility; a device profile may reject announcements. Those adapters share a
trace contract but retain independent capability and host-observation reports.

## Open questions

- Should the first wire proposal store behavior as a separately hashed package
  member or in the canonical semantic model?
- Which additional event kinds can be grounded in portable semantics rather
  than vendor input APIs?
- What numeric type and JSON encoding can preserve exact cross-language values
  without JavaScript precision loss?
- Which internal events, timers and animation triggers admit a static
  termination/budget rule strong enough for the next profile?
- What native UI adapter can observe the same effects without confusing host
  agreement with visual or assistive-technology equivalence?
