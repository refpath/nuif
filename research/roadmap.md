# Continuous research roadmap

Research is maintained as a graph, not a bibliography. Each source record must support or challenge explicit NUIF claims and link to experiments, RFCs, ADRs, spec sections and code.

## Active research fronts

1. **Canonical model** — test tree+graph hybrid, stable identity, composition and provenance against real design-system fixtures.
2. **Layout** — compare CSS-compatible algorithms, proposal/response layouts, linear constraints and freeform design constraints; measure lowering loss.
3. **Rendering** — define normative visual semantics while keeping renderer implementation replaceable; build deterministic CPU reference paths for conformance where GPU variance is unacceptable.
4. **Text** — pin Unicode data, shaping inputs and font assets for reproducible tests; explicitly separate semantic text from shaped glyph caches.
5. **Synchronization** — prototype retentive correspondence maps and semantic source patches for HTML/Svelte.
6. **Extensions** — test opaque-preservation through implementations that cannot interpret a dialect.
7. **Collaboration** — compare operation log + CRDT profiles without polluting the canonical saved document.
8. **Serialization** — benchmark canonical text, deterministic CBOR and schema-based binary encodings on partial loading, unknown data and Git workflows.
9. **Governance** — track W3C/Khronos-style extension/IP processes and define a neutral migration path.
10. **Testing methodology** — seed-driven trial loops, metamorphic relations, reduction and report schema; evidence in `docs/whitepaper/11-cross-industry-patterns.md` and `conformance/HARNESS.md`.
11. **Reference editor** — headless-testable shell whose accessibility tree carries entity identity; ADR 0006 and `apps/editor/UI-SPEC.md`.

## Update policy

- Add new evidence as a new research item or source revision; do not silently rewrite history.
- Mark superseded records and link `supersedes` / `contradicts` relations.
- Record source commit/tag/version where available.
- Automated synthesis must record the generated-at date and source IDs used.
- Claims become specification requirements only through RFC/ADR/spec review.
