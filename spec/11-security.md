# 11 — Security and resource limits

Status: draft.

NUIF documents and extensions are untrusted input.

Implementations MUST bound decoded sizes, nesting depth, entity/relation counts, path segment counts, image/font sizes, decompression ratios and renderer resource allocations. Cyclic references MUST be detected where forbidden.

Fonts, images, SVG/imported data, adapters and plugins require sandbox-aware handling. Script/data-binding extensions are non-core and MUST NOT execute merely by opening a document.

Headless rendering MUST expose deterministic timeout/memory/resource budgets. GPU failures must not compromise process memory safety.
