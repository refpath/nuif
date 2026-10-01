---
name: research-register
description: Edit or review technical prose in research records, specifications, RFCs, ADRs, implementation documentation and explanatory code comments. Use when drafting or changing substantive claims, terminology or evidence, including the prose review before delivery. Does not apply to ordinary chat, commit subjects or edits confined to code or configuration values.
---

# Research register

## Inputs and output

Use the requested text or affected files, their owning guidance and the available
source or implementation evidence. Produce the requested prose edits, or findings
in chat when the task is a read-only review. Identify material claims that remain
unverified without creating a separate report.

## Workflow

1. Read the [writing register](../../../CONTRIBUTING.md#writing-register). For
   research records, also read [the corpus requirements](../../../research/README.md)
   and [the record schema](../../../research/schema/research-item.schema.json).
   For semantic proposals or specification changes,
   consult [governance](../../../GOVERNANCE.md).
2. Trace changed claims to their primary source locators, implementation or
   actual test results. Preserve the distinction between a source statement,
   repository interpretation, a proposed behavior and an observed result.
   Narrow unsupported claims or identify the missing evidence.
3. When prose concerns model identity, layout, rendering, operations, testing or
   the editor, read the relevant section of [terminology](references/terminology.md)
   and check the linked owner for the exact definition or profile boundary.
4. Edit within the requested scope. Replace promotional or metaphorical wording
   with the mechanism, property or measured result. Remove repeated claims and
   stock transitions while preserving facts, citations, identifiers, numbers,
   code blocks and uncertainty. Use imperative steps for operational guidance.
5. Review the result in context for meaning, grammar, terminology and evidence.
   Check that the change preserves limitations and does not present planned
   profiles as implemented. Resolve material ambiguities from the owning source
   rather than treating a phrase match as proof of a writing defect.
