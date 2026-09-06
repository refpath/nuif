---
id: nuif:research:developer-beta-evidence-method
kind: synthesis
status: reviewed
title: Profile-specific developer beta evidence and source workflow evaluation
source:
  url: https://www.w3.org/TR/2005/REC-qaframe-spec-20050817/
  authors: [W3C Quality Assurance Working Group]
  published_at: 2005-08-17
  license: W3C document licence
retrieved_at: 2026-09-06
tags: [conformance, methodology, source-synchronization, evaluation]
confidence: 0.9
claims: []
relations:
  - type: extends
    target: nuif:research:metamorphic-testing-graphics
    note: Applies relation-based checks to retentive source edits and limits inference from generated coverage.
links:
  spec: [spec/00-conformance.md, spec/09-provenance-and-fidelity.md]
  adr: []
  rfc: []
  code: [crates/nuif-testing/src/bin/source-workflow.rs, crates/nuif-cli/tests/source_output.rs, xtask/src/main.rs]
  experiments: []
---

# Summary

The W3C QA Framework recommends explicit specification scope, implementation
classes, conformance labels and treatment of profiles and extensions. It
separates testable requirements from explanatory material. Locators:
sections 2–3, especially the scope, conformance-model and subdivision principles,
Recommendation dated 2005-08-17, retrieved 2026-09-06.

## Evidence

The linked Recommendation supplies specification-writing guidance. It does not
define NUIF's beta criteria, certify its tests or establish application usability.
The source is used to structure a bounded acceptance contract, not to imply
W3C review of NUIF.

## NUIF relevance

**Adapt** explicit profiles and acceptance assertions for one developer workflow:
source import, supported semantic editing, retentive synchronization and
fidelity inspection. `docs/BETA.md` records the contract and exclusions.

The generated corpus varies two HTML profiles, containment structure, document
size, text and foreign-source syntax. Its checks concern exact re-import,
no-op identity, repeated planning, fixpoint, successive edits, locality and typed
refusals. CLI subprocess cases additionally exercise file replacement and report
path separation. All case outcomes are reported under the recorded revision.

**Reject** interpreting successful generated round trips as independent semantic
validation. Exporter and importer share model assumptions. The corpus cannot
estimate production acceptance rates, browser visual equivalence or external
user task success. Those outcomes require separately selected documents,
independent oracles or external evaluation.

## Validation contract

`cargo xtask source-workflow` fails on any violated assertion. The full
verification run archives its report and hashes it in the manifest. The
acceptance contract requires external workflow evidence before beta promotion;
registry validation or a larger generated sample cannot satisfy that condition.
