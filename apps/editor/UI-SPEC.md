# Reference test editor: user-interface specification

Status: draft specification for the reference test editor. The editor replicates the spatial layout, tool set, property sections and keyboard bindings of the Figma design editor (UI3) as documented in `nuif:research:figma-ui3-editor-layout` and `nuif:research:figma-tools-and-keyboard-shortcuts`, without branding, icons, logos, typography or colour values taken from that product. The feature set is limited to what conformance testing, import and export require. Anything not listed here is out of scope; additions require an RFC.

Implementation status: the native application implements the complete interactive surface for currently executable profile-zero properties: native document import/save and PNG export; bounded import and reported export through the declared SVG, HTML/CSS, DTCG, Penpot, static React JSX and static Svelte adapter profiles; history; pages/layers/components browsing; canvas selection and insertion; subtree duplicate/delete; responsive evaluation-width presets; zoom and interface visibility; a default pixel grid and rulers; command routing; and atomic name, position, sizing-intent, stack/flex spacing/alignment, solid-fill and pinned-text inspection. Each document edit lowers to an invertible protocol operation and is covered by the AccessKit/replay trial. Foreign imports open a new session after confirmation rather than mutating the current document. Sections below whose data is not in profile zero remain specification targets, not inert or simulated controls—most notably multi-selection and direct manipulation, tree drag/reorder, Grid tracks, component authoring, in-editor token editing, advanced paints/effects, arbitrary foreign formats and non-PNG rendering export.

## Purpose and constraints

1. The editor is a client of `nuif-api`. Every gesture lowers to protocol operations before any document mutation (`apps/editor/ARCHITECTURE.md`).
2. The editor is a test instrument. Its state is NUIF state plus ephemeral selection, viewport and panel state.
3. Automated test iterations use the CLI and the in-process session driver, not the GUI. The GUI exists to author fixtures by hand, to inspect import and export results, and to prove that a human-authored fixture and a replayed operation log converge (roadmap phase 5 exit).
4. Layout conventions replicated here are user-interface conventions, not protected expression; names, marks, icons and visual assets are not reproduced (`nuif:research:design-editor-ui-conventions-synthesis`).

## Regions

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ [A] Top bar: document name · page selector · Minimize UI                     │
├───────────────┬───────────────────────────────────────────┬──────────────────┤
│ [B] Left      │ [C] Canvas                                │ [D] Right panel  │
│ panel         │  rulers · infinite canvas · zoom          │  zoom % · Export │
│  Pages        │  marquee · snapping · smart guides        │  ┌ Design ────┐  │
│  Layers       │  measurement overlay                      │  │ sections   │  │
│  Components   │      ┌───────────────────────────────┐    │  │ (resizable)│  │
│  (resizable)  │      │ [E] Toolbar (floating, bottom) │    │  └────────────┘  │
│               │      │ Move Hand Frame Shapes Pen Text│    │  Diagnostics     │
│               │      └───────────────────────────────┘    │                  │
└───────────────┴───────────────────────────────────────────┴──────────────────┘
```

Minimize UI collapses A and B; D reappears while a selection exists. Hide UI hides A, B, D and E. Panels B and D are resizable; widths persist per session only. Panel pixel widths are not specified by evidence and are chosen by implementation.

## Left panel [B]

- Pages: ordered list of surfaces (NUIF `Surface` entities); add, rename, reorder, delete.
- Layers: containment tree of the current page; expand and collapse; drag to reparent and reorder; rename inline; visibility and lock toggles; multi-selection synchronized with the canvas.
- Components: local component definitions of the document; drag to instantiate.

Excluded: team libraries, asset search, remote components.

## Canvas [C]

- Infinite canvas with independently toggled pixel rulers and a document-aligned background grid. The profile-zero authoring unit is `px` and is shown in the top bar, status bar and numeric inspector labels. The page background colour comes from the surface.
- Interaction grammar (bindings in the table below): marquee selection from empty space; additive and subtractive selection; hierarchical traversal (child, parent, next and previous sibling); deep selection; duplicate by modifier drag; pan by Space drag or Hand tool; zoom by modifier scroll and by shortcuts (fit, selection, 100 %).
- Snapping to objects and pixel grid with smart guides; suspended while the declared modifier is held. Measurement overlay to the hovered entity while the declared modifier is held.
- Every canvas gesture emits one transaction: move in freeform becomes a transform edit; drag inside a stack becomes a reorder; resize becomes a sizing-intent edit or a fixed size according to the parent family (`docs/whitepaper/03-protocol-and-portability.md`).

Excluded: comments, cursors of other users, presentation mode, prototype links.

## Toolbar [E]

Floating strip centred at the bottom of the canvas. Tools, in order, with their group menus:

| Group | Tools | Emits |
|---|---|---|
| Move | Move (`V`), Hand (`H`) | selection and transform transactions; Hand emits nothing |
| Region | Frame (`F`) | `Insert` of a `Container` |
| Shape | Rectangle (`R`), Ellipse (`O`), Line (`L`) | `Insert` of a `Shape` |
| Vector | Pen (`P`) | `Insert` of a `Shape(Path)` and path edits |
| Text | Text (`T`) | `Insert` of a `Text` entity |
| Actions | command palette (`Cmd/Ctrl K`) listing every operation and command by name | the selected operation |

The command palette is the keyboard route to every operation and to Import, Export, Validate and Snapshot. Excluded tools: Scale, Section, Slice, Polygon, Star, Arrow, Pencil, Comment, Dev Mode toggle, drawing and illustration tools, AI actions.

## Right panel [D]

One tab, Design. Sections appear in this order for a container with a stack or flex family; sections absent for the selected kind are hidden.

1. Header: entity name; Create component; Detach instance (for instances).
2. Position: alignment row (six alignments and two distributions for multi-selection); X, Y; rotation; flip horizontal and vertical; constraints (freeform children only).
3. Layout family: family selector (freeform, stack, flex, grid, constraint); flow direction and wrap; gap; padding uniform or per side; alignment grid. This section replaces the product's auto-layout section and exposes NUIF families directly.
4. Sizing: W, H; per-axis intent (fixed, intrinsic, fill, fit-content, percentage); min and max; aspect ratio; clip content. Values are plain numbers or token references (see Tokens).
5. Appearance: opacity; corner radius uniform or per corner; blend mode; visibility.
6. Fill: ordered list of solid colour and linear or radial gradient paints; image fill for `Image` entities.
7. Stroke: colour, weight, alignment, per-side, dash pattern, cap, join.
8. Effects: drop shadow, inner shadow, layer blur, background blur.
9. Typography (Text entities): font family from the pinned font set, weight, size, line height, letter spacing, alignment, text sizing behaviour.
10. Component (definitions): parameters of kind boolean, enum variant, text, instance swap. Instance: parameter values and override reset.
11. States: named interaction states (default, hover, pressed, focused, disabled) with per-state property overrides. This section exists because the v0 fixture requires state metadata; no prototype player is included.
12. Tokens: every numeric or colour control accepts a token reference chosen from the document's token set; a bound control shows the token name and a detach action. Token sets are edited in a dialog opened from the command palette (DTCG-compatible).
13. Export: format (PNG, SVG, PDF, `nuif-text-0`, `nuif-cbor-0`, adapter targets), scale, suffix; Export opens the fidelity report.

Diagnostics: a collapsible list under the sections showing validation, fidelity and layout diagnostics for the selection, produced by `nuif-api`; each entry links to the entity.

Excluded: Prototype and Inspect tabs, selection colours, styles library, variables modes UI, glass, noise and texture effects, video and pattern fills, layout guides, image cropping, boolean operations, masks, vector networks beyond path editing, version history, branching.

## Dialogs

- Import: the File menu selects NUIF, SVG, HTML/CSS, DTCG, Penpot, static React JSX or static Svelte. External input is size-bounded before parsing; text profiles require UTF-8 while the Penpot profile validates bounded ZIP members in memory. A confirmation dialog shows fidelity-class and correspondence counts before the imported document replaces the session as an unsaved document. Merge-into-document import is not implemented.
- Export: the File menu selects PNG, SVG, HTML/CSS, DTCG, Penpot, static React JSX or static Svelte. Each external adapter export writes the artefact and a sibling `.report.json`; a profile mismatch fails before the destination chooser and writes nothing.
- Tokens: token set editor.
- Evaluation context: viewport size presets (360, 768, 1440 px and custom), scale factor, locale, writing direction, theme; the canvas renders the selected context; multiple contexts can be shown side by side for a surface.
- Snapshot: writes the canonical document, the resolved snapshot for the current context and the CPU rasterization to a fixture directory in the harness format (`conformance/HARNESS.md`).

## Keyboard bindings

Bindings reproduce the documented product bindings where verified; entries marked U in the source record are chosen to match common expectation and are not claimed to be product-accurate.

| Action | macOS | Windows and Linux |
|---|---|---|
| Move, Hand, Frame, Rectangle, Ellipse, Line, Pen, Text | V, H, F, R, O, L, P, T | same |
| Command palette | Cmd K | Ctrl K |
| Add or remove stack layout | Shift A / Option Shift A | Shift A / Alt Shift A |
| Group / Ungroup / Frame selection | Cmd G / Shift Cmd G / Option Cmd G | Ctrl G / Shift Ctrl G / Ctrl Alt G |
| Create component / Detach instance | Option Cmd K / Option Cmd B | Ctrl Alt K / Ctrl Alt B |
| Duplicate / duplicate by drag | Cmd D / Option drag | Ctrl D / Alt drag |
| Copy / Paste | Cmd C / Cmd V | Ctrl C / Ctrl V |
| Undo / Redo | Cmd Z / Cmd Shift Z | Ctrl Z / Ctrl Shift Z, Ctrl Y |
| Select all / inverse | Cmd A / Cmd Shift A | Ctrl A / Ctrl Shift A |
| Select child / parent; next / previous sibling | Enter / Shift Enter; Tab / Shift Tab | same |
| Deep select; nested marquee; subtractive marquee | Cmd click; Cmd drag; Shift drag | Ctrl click; Ctrl drag; Shift drag |
| Nudge 1 px / 10 px | Arrow / Shift Arrow | same |
| Align left, right, top, bottom, centre horizontal, centre vertical | Option A, D, W, S, H, V | Alt A, D, W, S, H, V |
| Flip horizontal / vertical | Shift H / Shift V | same |
| Zoom in / out; fit; selection; 100 % | Cmd + / Cmd −; Shift 1; Shift 2; Shift 0 | Ctrl + / Ctrl −; Shift 1; Shift 2; Shift 0 |
| Rulers; pixel grid | Shift R; Cmd ' | Shift R; Ctrl ' |
| Minimize UI / Hide UI | Cmd Shift \ / Cmd \ | Ctrl Shift \ / Ctrl \ |
| Export | Shift Cmd E | Shift Ctrl E |
| Temporarily disable snapping; measure to hovered | hold Control; hold Option | hold Control; hold Alt |
| Pan | hold Space and drag | same |

## Automation surface

The editor binary accepts `--headless --script <file>` and either `--document <file>` or `--new-document <id>`, then runs a session script against the same `nuif-api` engine without creating a window (`nuif:research:blender-dna-rna-and-headless`, `nuif:research:unreal-asset-versioning-and-automation`). `--expect-document` makes byte-exact parity blocking; `--report` and `--snapshot-dir` write the operation log and canonical/context/layout/scene/CPU-raster artifacts. The JSONL script language contains editor commands and semantic accessibility actions sharing one session.

The `nuif-editor-automation` feature-gated binary drives the native Masonry tree in process. It dispatches AccessKit actions, captures the matching accessibility tree and CPU-rendered shell frame, replays the protocol log independently and emits a machine-readable artifact set. `cargo xtask editor-gui-trial` repeats that run and requires identical canonical and pixel hashes. No socket transport is implemented.

Widget identity: every widget bound to a document entity exposes the entity identifier in its accessibility node (`author_id`), and every control exposes a role and label, so a harness locates "the width control of entity X" by query and sets it through an accessibility `SetValue` action (`nuif:research:accesskit-semantic-ui-testing`). No test depends on pixel coordinates of widgets.

## Rendering and text

The canvas renders through `nuif-render`. Interactive rendering uses the Vello backend; snapshots and headless runs use the CPU reference backend so that editor snapshots and conformance references are produced by the same path. Fonts are limited to the pinned set shipped with fixtures; system fonts are not enumerated.

## Implementation stack

Decided in ADR 0006 (accepted): Rust-native shell on Masonry (pinned by git revision; Xilem not used) with Vello rendering and AccessKit, replacing the earlier Svelte 5 shell proposal for the reference editor. The canvas lowers NUIF `RenderScene` to Masonry's `imaging` command set; the CPU reference renderer in `nuif-render` remains the conformance oracle. Toolchain 1.98.0, MSRV 1.96. A browser build through WASM remains a later demonstration target, not the reference editor.
