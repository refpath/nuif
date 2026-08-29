# Research corpus

`research/` is a machine-readable evidence corpus, not a loose notes folder. `AUDIT.md` records the current accuracy/alignment review and the gated plan that turns claims into falsifiable work.

## Evidence states

- `seed` — discovered, not fully reviewed;
- `reviewed` — source identity and relevance checked, but not every material claim is necessarily verified at its locator;
- `verified` — material claims are checked against primary locators and implementation claims have reproducible evidence;
- `superseded` and `rejected` — retained with typed relations so history is not silently rewritten.

Confidence and status are independent. A high-confidence reviewed record is not described as verified.

## Required properties

Each research record has a stable ID, source identity, retrieval date, tags, confidence, claim links, typed relations and repository links. Verified records additionally require `Summary`, `Evidence`, `Mechanism`, `NUIF relevance` and `Open questions` sections. Exact sections, pages, versions or commits support non-obvious source claims; synthesis is separated from source statements.

## Layout

- `items/` — one durable record per source or synthesized subject;
- `index.yaml` — architectural claims and topic-to-record map;
- `questions.yaml` — open and decided research questions;
- `experiments/index.yaml` — reproducible investigations and their executable artifacts;
- `coverage.yaml` — coverage status per architectural front;
- `schema/` — record schemas;
- `AUDIT.md` and `roadmap.md` — current audit and gated research process.

Important relationships must be structural. `tools/research/validate.sh` checks record schema, identifiers, claims, relations, topics, questions, experiments, coverage links and artifact paths. Network source-health checks are periodic rather than part of offline conformance.
