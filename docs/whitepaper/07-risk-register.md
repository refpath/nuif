# Risk register and impossibility boundaries

## Fundamental boundaries

1. **Rendered output is underdetermined.** Pixels/boxes cannot uniquely reveal whether layout came from flex, grid, constraints, absolute positioning or runtime code. Imported foreign content must mark inferred intent.
2. **Arbitrary program behavior is not serializable as UI structure.** NUIF does not promise to recover arbitrary JavaScript/Swift/Dart application logic.
3. **Text is environment-sensitive.** Font files, shaping versions, fallback and rasterization can differ. Exact conformance requires pinned inputs; portability must classify substitution separately.
4. **Platform-native controls differ.** Semantic equivalence may be possible while exact visuals/behavior are platform-specific.
5. **Effects/shaders can exceed a portable core.** Extensions may be preserved without being renderable.
6. **Standard complexity can kill adoption.** IFC/STEP demonstrate the cost of excessive semantic scope.
7. **A single reference implementation can accidentally become the spec.** Independent implementation is a standards gate.
8. **Resource bytes carry legal and security constraints.** Exact font/image
   preservation does not imply permission to redistribute or safe decoding.
9. **Visual metrics are gameable.** A flat screenshot can look exact while
   discarding editability, semantics, accessibility and responsive behavior.
10. **Model confidence can be misleading.** Raw probability is not calibrated
    correctness and cannot upgrade inferred evidence into source truth.
11. **Capture can leak secrets.** Browser/network/accessibility observations may
    expose credentials, personal data or proprietary content unless collection,
    export, retention and training are separately bounded.

## Containment strategies

- explicit fidelity reports and inference confidence;
- authored + resolved snapshots instead of pretending either alone is canonical truth;
- opaque extension preservation;
- deterministic capability/evaluation contexts;
- small core + profiles;
- reference implementation backed by normative conformance fixtures;
- fuzzing/resource budgets for all untrusted inputs;
- early independent implementation and adapter experiments.
- stable asset IDs separated from resource digests, locators and provenance;
- verify resource size/digest before decoding and never fetch implicitly;
- typed operation output for models, atomic validation and finite correction loops;
- structural/text/resource/edit-task metrics alongside visual diagnostics;
- calibrated decision-level confidence, alternatives and abstention;
- private captures default to local processing, no retention and no training.

## Current implementation risks

- **The macOS graphics fork is a maintenance boundary.** The editor pins
  `refpath/xilem` commit `1b96eb8`; its parent `eabfe0a` moves the active
  renderer from wgpu 28 and metal-rs to wgpu 29 and the objc2 Metal bindings.
  The fork changes public API call sites and does not patch the Objective-C
  blocks ABI. Each fork update requires the editor tests, reverse dependency
  trace, and macOS Metal window smoke test recorded in
  `nuif:research:macos-metal-block-future-incompatibility`. A separate
  `refpath/metal-rs` `move-to-block2` branch exists only for review; NUIF does
  not depend on the deprecated binding or that experimental patch.
- **Release signing is credential-bound.** The editor packaging gate builds,
  archives and smoke-tests an unsigned host package. Platform signing and
  notarisation require release credentials and remain separate from source
  conformance.
- **Portable resources are only narrowly implemented.** The deterministic
  package, RGBA8 PNG and static single-face TrueType subsets have executable
  gates, while CPU profile 0 remains unchanged. RFC 0010 cannot be accepted
  until broad media/font matrices, the configured Linux/Windows/macOS jobs
  produce passing hosted evidence, external reproduction, calibrated aggregate
  budgets and interoperability review pass.
- **Capture and reconstruction are proposed, not implemented.** RFC 0011 and
  specification 14 define evidence/fidelity ceilings and provider boundaries.
  No current release claims browser-capture or screenshot-reconstruction
  accuracy.

## Thesis falsifiers

The project should rethink its architecture if ordinary source round trips require broad regeneration, if unknown extension preservation cannot survive routine edits, if the layout vocabulary becomes a vendor-property dump, or if a second implementation cannot reproduce v0 behavior from specification + fixtures alone. The resource/reconstruction path should additionally narrow if package bytes cannot reproduce across writers, correction loops improve pixels by deleting semantics, confidence cannot support useful risk/coverage, or tuned models do not beat the untuned tool-assisted baseline.
