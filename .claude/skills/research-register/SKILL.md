---
name: research-register
description: Mandatory writing register for every persisted text in this repository (README, docs/, spec/, rfcs/, adrs/, research/, apps/, conformance/, commit subjects). Technical research-paper prose with established terminology; no marketing, no invented brand terms, no AI writing patterns. Use before writing or editing any Markdown or YAML prose, and use `audit` mode when reviewing text for register violations.
user-invocable: true
argument-hint: "[audit | rewrite] [file or text]"
---

# Research register

All persisted prose in this repository is written as a technical research document. The register is derived from two prior skills (caveman: compression of filler; unslop: removal of AI writing patterns) and tightened for academic style. Compression applies to filler, never to grammar or precision.

Read `references/terminology.md` before writing about the model, layout, rendering, operations, testing or the editor. Read `references/taboo.md` when auditing.

## Modes

| Mode | Behaviour |
|------|-----------|
| write (default) | Produce text that satisfies every rule below. |
| `audit` | Report violations only: quoted fragment, rule, replacement. Change nothing. |
| `rewrite` | Two passes: (1) list violations, (2) emit corrected text preserving every fact, number, identifier, citation and code block. |

## Rules

### Register
1. Declarative sentences. One claim per sentence where possible. Present tense for facts; past tense for what a source did.
2. Third person. No "we think", "I", "you", "let's". Passive voice is acceptable when the agent is unknown or irrelevant.
3. Grammar stays complete: articles, conjunctions and verb forms are kept. Fragments are allowed only in tables and bullet lists.
4. Precision over brevity, brevity over decoration. Delete words that carry no information; never delete words that carry meaning (`not`, `only`, `except`, `must`, units, versions).
5. Modal verbs follow RFC 2119 / BCP 14 semantics in `spec/` (MUST, SHOULD, MAY). Outside `spec/`, avoid modal stacking ("could potentially", "might possibly").
6. Uncertainty is stated once, explicitly and quantitatively where possible (confidence value, "unverified", "n = 3 samples"). No hedging filler.

### Terminology
7. Use the established term from the field: computer graphics, programming languages, distributed systems, software testing, HCI, standards bodies. Prefer the term used by the primary source (spec, paper) over a paraphrase.
8. Never coin a branded or metaphorical name for a known concept. A "semantic lowering" is a lowering; a "smart merge" is a structural three-way merge; a "golden" is a reference image or expected output; an "AI QA agent" is an automated client of the headless API.
9. Acronyms: well-known technical acronyms are used bare (CSS, DOM, CRDT, SMT, GPU, API, CLI, CBOR, RFC). Any other acronym is expanded at first use. No invented abbreviations (cfg, impl, req, res).
10. Names of external projects, papers, RFCs and specifications are written as their owners write them (OpenUSD, glTF, HarfBuzz, Taffy, Vello, RFC 8949, CSS Flexible Box Layout Module Level 1).
11. Unit and number formatting: SI units with a space (`360 px`, `16 ms`), ranges with an en dash (`360–1440 px`), versions verbatim (`1.85.0`).

### Structure
12. Start with the subject, not with context about the world. No "In today's ...", no "As X becomes more ...".
13. Headings are nouns or noun phrases. No questions as headings, no "Final thoughts", "Key takeaways", "Conclusion" in short documents. Long papers may end with "Discussion" and "Limitations".
14. Lists are for enumerations with parallel structure. Do not number items unless order matters. Do not bold the first words of list items.
15. Tables carry comparable facts with sources; they are not decoration.
16. Every non-obvious factual claim carries a locator: URL plus section, page, theorem, file path, version or commit, and a retrieval date in research records.
17. No emoji, no exclamation marks, no rhetorical questions, no arrows (→) inside prose (arrows are allowed in diagrams and code blocks).
18. Bold is reserved for defined terms at first definition and for the labels **Borrow**, **Adapt**, **Reject** in research records. Italics are reserved for titles of works.

### Claims
19. Distinguish source statements from repository interpretation. In research records, the `# Summary` and `## Evidence` sections report the source; `## NUIF relevance` interprets.
20. Do not describe the project as a standard, industry standard or de facto standard. It is a draft specification with a reference implementation until independent implementations and conformance profiles exist.
21. Do not describe any component as complete, proven, production-ready or robust unless a linked fixture, test or proof exists in the repository.
22. Comparisons carry a metric and a source. "Faster", "smaller", "better" without both are removed.

### Forbidden phrase classes (see `references/taboo.md` for the full list)
23. Enthusiasm adjectives: powerful, seamless, robust, cutting-edge, game-changing, transformative, revolutionary, elegant, beautiful, blazing, world-class.
24. Metaphorical verbs: leverage, unlock, navigate, empower, supercharge, harness, delve.
25. Framing clichés: "at its core", "it's worth noting", "here's the thing", "the bottom line", "in other words", "at the end of the day", "not just X but Y", "X is more than Y".
26. Hedging clichés: "it remains to be seen", "arguably", "potentially" (unless quantified), "somewhat", "quite", "very", "really", "basically", "actually", "simply", "just".
27. Pleasantries and self-reference: "sure", "certainly", "happy to", "great question", "note that I", "as an AI".

## Validation checklist

Before finishing any persisted text:
- grep the text against `references/taboo.md`; zero matches;
- each acronym outside the bare list is expanded once;
- each factual claim has a locator or is marked unverified;
- the document does not call NUIF a standard;
- no heading is a question; no list item begins with bold;
- research records validate against `research/schema/research-item.schema.json` (`kind` and `status` from the enumerations; `retrieved_at` is a date).

## Boundaries

This register governs persisted text. Conversation replies to the maintainer follow `terse-chat`. Code comments follow the register but may use fragments. Commit subjects follow `git-commit` (single line, type prefix, imperative, no body).
