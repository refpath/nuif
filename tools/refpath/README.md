# Refpath research ingestion contract

This directory will contain tooling that projects repository research/spec/code metadata into Refpath's research graph.

## Node classes

`source`, `paper`, `standard`, `repository`, `claim`, `question`, `experiment`, `rfc`, `adr`, `spec_section`, `fixture`, `crate`, `adapter`, `commit`.

## Edge classes

`supports`, `contradicts`, `extends`, `implements`, `inspired_by`, `compares_to`, `supersedes`, `depends_on`, `tests`, `specified_by`, `decided_by`, `evidenced_by`.

## Determinism

Stable IDs are supplied by front matter/spec identifiers rather than generated from prose. Importers should hash normalized source records to detect changes and maintain `supersedes` history instead of destructive replacement.

Code indexing is separate from research ingestion but joins through explicit `links.code` references and repository commit identity.
