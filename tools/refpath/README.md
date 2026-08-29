# Refpath integration

This directory will contain deterministic extraction/index tooling for feeding NUIF research and project metadata into Refpath/refpath-cloud.

The intended graph contains nodes for research sources, atomic claims, experiments, specifications, RFCs, ADRs, code symbols/modules, fixtures, conformance assertions, and releases. Typed edges include `supports`, `contradicts`, `extends`, `implements`, `inspired_by`, `compares_to`, `supersedes`, `depends_on`, `validated_by`, and `implemented_by`.

Stable IDs in front matter and schemas are authoritative graph identities. Indexing must not depend on embeddings or natural-language inference for relationships that the repository can state explicitly.
