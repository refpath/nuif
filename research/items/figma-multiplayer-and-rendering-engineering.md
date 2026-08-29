---
id: nuif:research:figma-multiplayer-and-rendering-engineering
kind: article
status: reviewed
title: Figma engineering on multiplayer synchronization, fractional indexing and the custom renderer
source:
  url: https://www.figma.com/blog/how-figmas-multiplayer-technology-works/
  authors: [Evan Wallace, Alex Ringlein, Luke Anderson, Slava Kim, Laurel Woods, Jamie Wong]
  published_at: "2015-12-07 (design tool), 2016-09-28 (multiplayer), 2017-03-06 (ordered sequences), 2017-06-08 (WebAssembly), 2018-05-02 (Rust), 2019-10-16 (multiplayer technology), 2023-08-29 (performance testing), 2025-09-18 (WebGPU)"
  license: Proprietary blog content (Figma)
retrieved_at: 2026-08-29
tags: [collaboration, last-writer-wins, fractional-indexing, undo, reparenting, renderer, webgl, webgpu, tile-rendering, wasm, performance-testing]
confidence: 0.9
claims: [nuif:claim:collab-profile, nuif:claim:authored-resolved, nuif:claim:semantic-automation]
relations:
  - type: extends
    target: nuif:research:figma
    note: Adds the vendor's own description of the sync protocol and renderer behind the public node model.
  - type: related_to
    target: nuif:research:automerge-yjs
    note: Figma chose per-property last-writer-wins registers with server ordering instead of a general CRDT or operational transformation.
  - type: related_to
    target: nuif:research:structured-merge
    note: Property-level registers avoid merge conflicts by construction at the cost of never merging concurrent edits to one value.
  - type: related_to
    target: nuif:research:vello
    note: Figma's tile-based GPU renderer is a precedent for a retained, GPU-resident 2D scene renderer.
  - type: related_to
    target: nuif:research:renderers
    note: Figma rejected DOM, SVG and Canvas 2D for the same reasons that motivate a custom GPU renderer in NUIF.
links:
  spec: [spec/06-operations-and-patches.md, spec/10-collaboration-profile.md, spec/02-identity-and-properties.md]
  adr: [adrs/0005-collaboration-profile.md, adrs/0003-reference-renderer.md]
  rfc: []
  code: [crates/nuif-protocol, crates/nuif-render]
  experiments: []
---

# Summary

Figma's engineering posts describe the document as a two-level map `Map<ObjectID, Map<Property, Value>>` synchronized over a WebSocket to a per-document server process. Concurrency control is per-property last-writer-wins ordered by server arrival; the server is authoritative and rejects parent updates that would form a cycle; clients apply local edits optimistically and discard incoming values that conflict with unacknowledged local writes. Child order is a fractional index stored together with the parent link so both change atomically; indices are arbitrary-precision base-95 strings. Undo is per-user and rewrites redo history so that undo-copy-redo leaves the document unchanged. The editor is C++ compiled to WebAssembly with a custom tile-based WebGL renderer (and, since 2025, a WebGPU path with dynamic fallback), chosen over DOM, SVG and Canvas 2D for consistency and retained-mode performance. Performance is guarded by headless per-pull-request benchmarks with a 20 percent regression margin.

## Evidence

- Document structure: a single root, page objects beneath it, and per page "a hierarchy of objects"; conceptual model `Map<ObjectID, Map<Property, Value>>`. E. Wallace, "How Figma's multiplayer technology works", 2019-10-16, retrieved 2026-08-29.
- Conflict rule: servers "keep track of the latest value that any client has sent for a given property on a given object"; concurrent edits to the same property yield "the last value that was sent to the server"; text edits are whole-value ("either AB or BC but never ABC"). Same post.
- Operational transformation (OT) rejected as "very complicated and hard to implement correctly" for the requirements; design goal was to be "no more complex than necessary". Same post.
- Optimistic client rule: unacknowledged local changes are the best prediction, so clients "discard incoming changes from the server that conflict with unacknowledged property changes". Same post.
- Cycles: servers "reject parent property updates that would cause a cycle"; a client may transiently hold both an unacknowledged reparent and a conflicting server reparent; Figma temporarily removes the affected objects from the tree "until the server rejects the client's change". Same post.
- Ordering: position "is represented as a fraction between 0 and 1 exclusive"; insertion sets the average of neighbours; "The parent link and this position must both be stored as a single property so they update atomically". Same post.
- Undo: the principle "If you undo a lot, copy something, and redo back to the present ... the document should not change"; "an undo operation modifies redo history at the time of the undo, and likewise a redo operation modifies undo history". Same post; the principle is first stated in E. Wallace, "Multiplayer Editing in Figma", 2016-09-28, retrieved 2026-08-29.
- Offline: arbitrary offline editing; on reconnect the client "downloads a fresh copy of the document, reapplies any offline edits on top" and resumes over a new WebSocket. 2019 post.
- Architecture: "a separate process for each multiplayer document"; multiplayer server ported to Rust for lower latency and memory; "Serialization time is now over 10x faster"; problems listed with lifetimes, error stack traces, immature compression libraries and futures. E. Wallace, "Rust in Production at Figma", 2018-05-02, retrieved 2026-08-29.
- Fractional indexing detail: indices as strings with "averaging ... done using string manipulation"; leading `0.` omitted and full printable ASCII used ("base 95 instead of base 10"); the server assigns a unique position to the second of two identical inserts; index length grows with repeated edits; concurrent inserts may interleave. E. Wallace, "Realtime Editing of Ordered Sequences", 2017-03-06, retrieved 2026-08-29.
- Renderer rationale: HTML/SVG "often much slower than the 2D canvas API due to DOM access"; Canvas 2D "is an immediate mode API instead of a retained mode API so all geometry has to be re-uploaded ... every frame"; text layout "inconsistent between browsers"; missing features such as angular gradients; result is "a highly-optimized tile-based engine" in WebGL with masking, blurring, dithered gradients, blend modes, nested layer opacity; "a browser inside a browser" with own DOM, compositor and text layout; C++ via emscripten with compact 32-bit floats. E. Wallace, "Building a professional design tool on the web", 2015-12-07, retrieved 2026-08-29.
- WebAssembly: load time "improved by more than 3x ... regardless of document size"; wasm "parses around 20x faster than asm.js". E. Wallace, "WebAssembly cut Figma's load time by 3x", 2017-06-08, retrieved 2026-08-29.
- Renderer restructuring yielded up to 3x faster load, zoom and drag; tracked metrics are average frame time and maximum frame time. J. Wong, "Figma, faster", 2018-08-13, retrieved 2026-08-29.
- WebGPU: shipped in Chromium in 2023; enables compute shaders (planned blur optimisation) and avoids WebGL's "bug-prone global state"; "a dynamic fallback system" starts on WebGPU and falls back to WebGL on asynchronous test failure or mid-session failure. A. Ringlein, L. Anderson, "Figma Rendering: Powered by WebGPU", 2025-09-18, retrieved 2026-08-29.
- Performance continuous integration (CI): benchmarks run "in GPU-enabled virtual machines, in a headless Chromium process on every code change in every pull request" with "20% pass margin"; scenarios include local edits and "a stream of simulated multiplayer changes" (e.g. 100 simulated editors); a hardware lab handles precise cases; CPU profiles are captured per run; rendering prioritises local edits over remote changes. S. Kim, L. Woods, "Keeping Figma Fast", 2023-08-29, retrieved 2026-08-29.

## Mechanism

Synchronization. Each object is a set of registers keyed by property name. A client edit produces `(objectID, property, value)` messages applied locally at once and buffered as unacknowledged. The server serializes all messages per document, applies last-writer-wins per register, persists, and broadcasts. On receipt, a client applies a server value unless it holds an unacknowledged write to the same register, in which case the server value is dropped because the local write will arrive later in server order. Tree structure is a register too: `parent` and `position` are one composite value, so a reparent is a single register write and cannot be split by concurrent edits. Acyclicity is a server-side precondition on `parent` writes; the client's transient inconsistency is contained by detaching the involved subtree until the rejection arrives. Reconnection is state-based: reload, then replay the local unacknowledged log; there is no operation log merge beyond that.

Ordering. `position` is a string over a 95-symbol alphabet interpreted as a fraction in (0, 1). Insertion between `a` and `b` chooses the shortest string strictly between them (by averaging with string arithmetic); the server perturbs duplicates. This makes reorder a single-register write and avoids index shifts, at the cost of unbounded string growth and interleaving under concurrent insertion at the same gap.

Undo. Undo is per user and operates on the user's own history; undoing a property write re-writes the previous value as a new write (so the server sees it as a normal last-writer-wins update), and the redo stack is rewritten at undo time to reflect the state that the undo produced. The invariant is idempotence of undo-then-redo sequences with respect to the document.

Rendering. The document is retained in WebAssembly memory with compact numeric types. The renderer rasterizes into tiles on the GPU, so partial invalidation redraws only affected tiles and pan/zoom reuse cached tiles; text is laid out by an in-house engine to avoid cross-browser divergence. WebGPU adds compute passes; a runtime feature probe selects the backend and can downgrade mid-session. Performance regressions are caught by scripted scenarios in headless Chromium under GPU virtual machines.

## NUIF relevance

**Borrow**
- Property-level registers with server-ordered last-writer-wins as the default conflict policy for scalar authored properties in the collaboration profile, with the explicit consequence that concurrent edits to one value are not merged (spec/10).
- Parent and order stored as one atomic value; NUIF `move/reorder entity` should be one operation carrying both target parent and fractional position.
- Fractional indexing in a printable alphabet for sibling order in the operation log, with server-side deduplication of identical positions.
- Acyclicity as a precondition enforced at the authority, and a defined client-side containment strategy for transiently invalid states.
- Per-pull-request headless GPU benchmark scenarios with a fixed regression margin as a model for `nuif-render` and layout performance gates in conformance.

**Adapt**
- NUIF undo is defined as inverse semantic operations rather than value rewrites; Figma's "undo rewrites redo history" invariant should be stated as a testable property of the operation log rather than a client implementation detail.
- Offline reconciliation by reload-and-replay is acceptable only if replayed operations are the same typed operations as live ones and preconditions (spec/06) surface conflicts instead of silently overwriting.
- Tile-based GPU rendering is a renderer strategy, not a document property; NUIF's resolved layer must remain renderer-independent.

**Reject**
- Treating text content as a single register; NUIF text runs need sequence semantics or explicit conflict objects for concurrent edits.
- Undocumented wire protocol as an integration boundary; NUIF keeps the collaboration profile normative and materializable to canonical snapshots (ADR 0005).
- Renderer-specific text layout as the canonical result; NUIF resolved text diagnostics must be attributable to a declared shaping context, not to one engine's behaviour.

## Open questions

- How Figma handles register writes to deleted objects and whether tombstones are retained; not covered by the retrieved posts.
- Whether the fractional index ever gets rebalanced (reindexing all siblings) and how that interacts with concurrent edits.
- Whether the WebGPU path changes the tile strategy or only the blur/compute stages; the 2025 post gives no rendering detail.
