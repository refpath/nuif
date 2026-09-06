# Implementation language and runtime choice

## Rust reference core

[ADR 0001](../../adrs/0001-rust-reference-core.md) selects Rust for document
validation, operations, codecs, evaluation and language bindings. The existing
workspace demonstrates those interfaces through native, WebAssembly and C
consumers. This is implementation evidence for the selected stack, not a
comparative benchmark of programming languages.

Rust's ownership and type system support the implementation's memory-management
and typed-operation requirements. They do not prove semantic correctness or
eliminate unsafe behavior in dependencies. Bounded parsing, coverage-guided
fuzzing and independent oracle comparisons remain necessary. The
[toolchain policy](../../research/items/rust-toolchain-and-msrv-policy.md)
records the pinned compiler and minimum supported Rust version.

## Alternatives and replacement criteria

No equivalent NUIF core has been implemented in C++, Zig, Go or TypeScript for a
controlled comparison. Claims that these languages are categorically unsuitable
are therefore unwarranted. A replacement proposal requires evidence for the
actual workloads: hostile-input handling, canonical serialization, operation
replay, native/browser embedding and maintenance cost.

Target-native adapter code remains appropriate where a host exposes its API
through another language. The [binding decision](../../adrs/0011-sdk-and-foreign-bindings.md)
separates those host interfaces from the document model and does not make Rust
a conformance requirement.

## Dependency boundaries

The [dependency register](../../dependencies/index.json) records each direct
library's role, alternatives and rationale. Taffy supplies layout evaluation;
Harfrust and Skrifa supply shaping and font interpretation; Masonry and AccessKit
supply the native shell and its semantic test surface. Source adapters use
Tree-sitter where retained syntax is required. Each dependency remains behind
NUIF-owned types or an explicitly declared host boundary.

The [editor fork review](../../research/items/editor-fork-exit-review.md)
records the retained patches and a CPU-rendered egui alternative. A small
alternative-backend probe does not establish replacement parity for the editor.
Migration requires the same canvas, semantic-action, text, rendering and host
integration acceptance criteria as the current implementation.
