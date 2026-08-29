# Research corpus

`research/` is a machine-readable evidence corpus, not a loose notes folder. It is designed so Refpath can index sources, claims, relationships, experiments, decisions, and code links directly into a research graph.

## Required properties

Each research record has a stable ID, explicit kind/status, source identity, retrieval date, topical tags, evidence/claim links, confidence, and typed relations. Records should cite primary sources wherever possible and distinguish source statements from NUIF synthesis.

## Layout

- `schema/` — machine schemas for records and relations
- `items/` — one durable research record per primary source or synthesized subject
- `claims/` — atomic claims that can be supported, contradicted, or superseded
- `experiments/` — reproducible investigations with fixtures/results
- `maps/` — curated topic/prior-art maps

Do not encode important relationships only in prose. If Refpath should understand a relationship, record it structurally.
