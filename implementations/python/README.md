# Independent Python profile 0

This directory contains a second, mechanically independent implementation of the bounded NUIF v0 conformance path. It uses only Python's standard library and neither imports nor invokes a Rust workspace package.

The implementation reads and structurally validates canonical `nuif-text-0`, writes the canonical bytes, preserves an opaque unknown payload through an unrelated edit, evaluates the declared profile-0 stack/freeform/responsive layout, lowers explicit fidelity, and rasterizes the v0 solid rectangles and pinned Ahem text. The differential harness supplies reference artifacts; the implementation computes its own results before comparing boxes, decoded RGBA and fidelity.

Its scope is deliberately narrower than the full draft model. It supports the semantics exercised by the responsive-card fixture and returns a failure for visual operations outside that independent render subset. It is evidence for Gate G's v0 reproduction criterion, not a second general-purpose NUIF product.

Run its local unit tests with:

```sh
python3 -m unittest discover -s implementations/python/tests -p 'test_*.py'
```

The complete cross-implementation run is exposed through `cargo xtask gate-g`.
