# Profile-0 render fixtures

`profile-zero-v1.json` defines the exact solid-paint CPU subset by value: encoded-sRGB color, scaled rectangle edge inclusion, Zeno 0.3.3 grayscale ellipse masks and integer source-over composition on an opaque-white RGBA8 target.

`cargo xtask gate-d-render` repeats every scene and PNG, compares committed SHA-256 baselines, rejects out-of-range colors and verifies property-attributed fidelity for path, image, instance, unknown-kind and document/entity extension data. The recorded rectangle and ellipse hashes reproduce on macOS/aarch64, Linux/aarch64 and Linux/x86_64. This matrix does not imply equality on untested platforms.

The fixture does not claim support for gradients, strokes, arbitrary paths, images, masks, effects or component-instance materialization. Those inputs are outside render profile 0 and remain explicit in fidelity reports.
