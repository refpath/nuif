# Risk register and impossibility boundaries

## Fundamental boundaries

1. **Rendered output is underdetermined.** Pixels/boxes cannot uniquely reveal whether layout came from flex, grid, constraints, absolute positioning or runtime code. Imported foreign content must mark inferred intent.
2. **Arbitrary program behavior is not serializable as UI structure.** NUIF does not promise to recover arbitrary JavaScript/Swift/Dart application logic.
3. **Text is environment-sensitive.** Font files, shaping versions, fallback and rasterization can differ. Exact conformance requires pinned inputs; portability must classify substitution separately.
4. **Platform-native controls differ.** Semantic equivalence may be possible while exact visuals/behavior are platform-specific.
5. **Effects/shaders can exceed a portable core.** Extensions may be preserved without being renderable.
6. **Standard complexity can kill adoption.** IFC/STEP demonstrate the cost of excessive semantic scope.
7. **A single reference implementation can accidentally become the spec.** Independent implementation is a standards gate.

## Containment strategies

- explicit fidelity reports and inference confidence;
- authored + resolved snapshots instead of pretending either alone is canonical truth;
- opaque extension preservation;
- deterministic capability/evaluation contexts;
- small core + profiles;
- reference implementation backed by normative conformance fixtures;
- fuzzing/resource budgets for all untrusted inputs;
- early independent implementation and adapter experiments.

## Thesis falsifiers

The project should rethink its architecture if ordinary source round trips require broad regeneration, if unknown extension preservation cannot survive routine edits, if the layout vocabulary becomes a vendor-property dump, or if a second implementation cannot reproduce v0 behavior from specification + fixtures alone.
