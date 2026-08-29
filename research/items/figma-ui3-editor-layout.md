---
id: nuif:research:figma-ui3-editor-layout
kind: article
status: reviewed
title: Figma UI3 design editor layout, panels and Design-tab sections
source:
  url: https://help.figma.com/hc/en-us/articles/15297425105303-Explore-design-files
  authors: [Figma]
  published_at: "unknown"
  license: proprietary help-center and blog content (Figma); facts only recorded here
retrieved_at: 2026-08-29
tags: [figma, ui3, editor-ui, layout, properties-panel, layers-panel, toolbar, canvas, reference-editor]
confidence: 0.86
claims: [nuif:claim:semantic-automation]
relations:
  - type: extends
    target: nuif:research:figma
    note: Adds the editor-surface description that the node-model record omits.
  - type: related_to
    target: nuif:research:figma-tools-and-keyboard-shortcuts
    note: Tool and shortcut inventory for the same UI3 editor.
  - type: related_to
    target: nuif:research:figma-plugin-and-rest-api-as-automation-surface
    note: Programmatic surface behind the same panels.
  - type: compares_to
    target: nuif:research:penpot-editor-ui-and-automation
    note: Open-source editor with left/right sidebars and a top toolbar.
  - type: related_to
    target: nuif:research:design-editor-ui-conventions-synthesis
    note: Cross-editor comparison that places UI3 among shared conventions.
links:
  spec: [spec/12-cli-api-and-automation.md]
  adr: []
  rfc: [rfcs/0004-headless-qa-contract.md]
  code: [apps/editor/README.md, apps/editor/ARCHITECTURE.md, apps/editor/QA.md]
  experiments: []
---

# Summary

Figma's third interface generation ("UI3") was announced at Config 2024 and became the only interface on 30 April 2025 (Figma blog, "Making the move to UI3"). The Help Center describes a design file as five regions: navigation bar, left sidebar, canvas, right sidebar and toolbar. The toolbar is a single slim strip at the bottom of the canvas; the left sidebar carries file, pages, layers and assets; the right sidebar carries Design and Prototype tabs whose sections are Position, Auto layout, Layout, Appearance, Fill, Stroke, Effects, Selection colors and Export, with Typography, Component/Properties and Instance sections appearing contextually. Panels are fixed but resizable; the UI can be minimized (sidebars collapse, the right sidebar reappears on selection) or hidden entirely. Panel pixel widths, typographic metrics and the exact rendering order of some sections are not stated in any primary source retrieved and are marked unverified below.

NUIF interpretation: the layout is a stable, documented target for a test editor that reproduces spatial arrangement and interaction grammar without brand assets. The section list doubles as a checklist of property groups whose semantic operations the editor must expose to automation.

## Evidence

Each bullet is one claim, followed by its locator. Retrieval date for all locators: 2026-08-29.

- A design file has five regions, lettered A–E: navigation bar, left sidebar, canvas, right sidebar, toolbar. The toolbar "contains various creation tools, the quick actions menu, and switcher to switch between file modes". https://help.figma.com/hc/en-us/articles/15297425105303-Explore-design-files — region legend.
- The right sidebar "contains actions like sharing and exporting"; viewers see Comment and Properties tabs, editors see Design and Prototype tabs. Same article, region D.
- Canvas panning: hold Space and drag; zoom via keyboard or trackpad. Same article, region C.
- The course article names the same four working areas (toolbar, left sidebar/navigation panel, right sidebar/properties panel, canvas); Hand tool is H; zoom is Cmd/Ctrl + scroll. https://help.figma.com/hc/en-us/articles/30925881896727-FD4B-Navigate-Figma-Design-files — "Get to know the interface".
- Left sidebar content changes with the navigation-bar tab: File (Pages panel, Layers panel), Assets, Tools (plugins, widgets), Agents, and a Variables view. https://help.figma.com/hc/en-us/articles/360039831974-View-layers-and-assets-in-the-left-sidebar — tab list.
- Left sidebar width is user-adjustable by dragging its right edge; no pixel value is given. Same article, "adjust the width of the left sidebar".
- The file name sits at the top of the File tab; an "Edit file menu" opens next to it; a Find/Replace tool searches the file. Same article.
- Minimize UI: Cmd Shift \ (Mac) / Ctrl Shift \ (Windows). "the navigation bar and left sidebar remains minimized, while the right sidebar expands" when an object is selected and "minimizes again" on deselect. https://help.figma.com/hc/en-us/articles/41414918021271-Hide-or-minimize-the-UI — "Minimize the UI".
- Hide UI: Cmd \ / Ctrl \ conceals "the navigation bar, left and right sidebars, and the toolbar". Same article, "Hide the UI". Note: the blog post below states "Shift \" for minimize; the Help Center article is treated as authoritative and the discrepancy is recorded as unverified.
- UI3 headline changes: "Bottom toolbar"; "Flexible panels and modals" (resizable panels, horizontal scrolling); "Logical layout controls" (width, height, resizing, direction, alignment, spacing grouped in one section); Dev Mode toggle in the toolbar; "Actions menu for everything"; "Optional property labels" toggled from the dropdown next to the zoom percentage; UI2 retired 30 April 2025. https://www.figma.com/blog/making-the-move-to-ui3-a-guide-to-figmas-next-chapter/ — section headings as quoted.
- Redesign rationale and visual changes: "a slim new toolbar at the bottom of the canvas"; panels "resizable" and "collapsible"; component controls (variants, instances) given "top billing above attributes like color and size"; layout options "merged into a single panel"; inputs gained backgrounds, dropdowns gained borders, corners rounded; 200 redrawn icons. https://www.figma.com/blog/behind-our-redesign-ui3/ — published 2024-06-26.
- Design-process account: "Toolbars will float at the bottom of all Figma products"; the navigation panel "linearly lists the file name, branch name, and project name, followed by pages and layers"; after beta feedback "panels are fixed, but still resizable"; constraints "expanded by default"; auto layout controls "always show pixel values and resize mode". https://www.figma.com/blog/our-approach-to-designing-ui3/ — published 2024-10-10.
- Toolbar groups in order: Move tools (Move, Hand, Scale); Region tools (Frame, Section, Slice); Shape tools (Rectangle default; Rectangle, Line, Arrow, Ellipse, Polygon, Star, Image/video); Creation tools (Pen, Pencil); Text; Comment tools (Comment, Annotation, Measurement); Actions menu (AI tools, asset search, plugins, widgets, commands); Dev Mode toggle (Shift D); a Figma Draw button. https://help.figma.com/hc/en-us/articles/360041064174-Access-design-tools-from-the-toolbar — section headings. The article does not state the toolbar's screen position; position is taken from the blog posts and the file-overview article above.
- Actions menu shortcut: Cmd K (Mac) / Ctrl K (Windows). https://help.figma.com/hc/en-us/articles/23570416033943-Use-the-actions-menu-in-Figma-Design.
- Right sidebar tabs and property groups: Design and Prototype for editors; Comment and Properties for viewers; listed property groups include alignment/rotation/position, frame size, corner radius, constraints, layout guides, component properties, instance, auto layout, blend modes, text, fill, stroke, effects, export settings. With nothing selected the tab shows styles, local variables, canvas background colour and page export. A dropdown "next to the 100% zoom percentage" exposes "Property labels". https://help.figma.com/hc/en-us/articles/360039832014-Design-prototype-and-explore-layer-properties-in-the-right-sidebar.
- Zoom percentage is shown "in the top-right corner"; clicking it opens the Zoom/view options menu (zoom in/out, zoom to fit, pixel grid, snap to pixel grid, pixel preview, layout guides, multiplayer cursors). https://help.figma.com/hc/en-us/articles/360041065034-Adjust-your-zoom-and-view-options.
- Position section: alignment row (align left/right/top/bottom/horizontal centres/vertical centres, Option/Alt + A/D/W/S/H/V), X/Y measured from the top-left of the layer bounds, rotation field "at the top of the Design panel", flips via Shift H / Shift V, W/H fields with aspect-ratio lock. https://help.figma.com/hc/en-us/articles/360039956914-Adjust-alignment-rotation-position-and-dimensions.
- Constraints are opened "from the Position section of the right sidebar"; options Left/Right/Left and right/Center/Scale and Top/Bottom/Top and bottom/Center/Scale; not available for layers outside a frame or inside an auto-layout frame. https://help.figma.com/hc/en-us/articles/360039957734-Apply-constraints-to-define-how-layers-resize.
- Auto layout section controls: flow (vertical, horizontal with wrap, grid), gap (numeric or auto spacing), padding (uniform or per side), alignment, resizing (hug contents, fill container, fixed), min/max width and height. Shortcut Shift A. https://help.figma.com/hc/en-us/articles/360040451373-Guide-to-auto-layout.
- The controls sit under a right-panel section labelled "Auto layout"; removal via "Remove auto layout" or Option Shift A / Alt Shift A. https://help.figma.com/hc/en-us/articles/5731482952599-Toggle-on-auto-layout-in-designs.
- Frame properties: Frame tool F or A; frame presets list "in right sidebar" while the tool is active (Phone, Tablet, Desktop, Presentation, Watch, Paper, Social Media, Figma Community, Archive); "Clip content" hides children beyond the bounds; Layout guides; frame selection Option Cmd G / Ctrl Alt G. https://help.figma.com/hc/en-us/articles/360041539473-Frames-in-Figma-Design.
- Layout guides live in a "Layout guide" section; types uniform grid, column, row; visibility toggle Shift G. https://help.figma.com/hc/en-us/articles/360040450513-Create-layout-guides-with-rows-columns-and-grids. The zoom article above lists Ctrl G (Mac) / Ctrl Shift 4 (Windows) for the same toggle; the conflict is recorded as unverified.
- Text resizing (auto width, auto height, fixed) is in the Layout section; the "Typography" section holds text styles, font family, weight/style, size, line height, letter spacing, horizontal and vertical alignment, and a "Type settings" panel (text case, decoration, truncation, paragraph spacing, wrap). https://help.figma.com/hc/en-us/articles/360039956634-Explore-text-properties.
- Appearance section hosts the layer blend mode ("Apply blend mode in the Appearance section"); modes: Pass through, Normal, Darken, Multiply, Plus darker, Color burn, Lighten, Screen, Plus lighter, Color dodge, Overlay, Soft light, Hard light, Difference, Exclusion, Hue, Saturation, Color, Luminosity. https://help.figma.com/hc/en-us/articles/360040667874-Apply-blend-modes-to-layers-fills-and-effects.
- Corner radius: an "Independent corners" control opens a per-corner Corner radius panel; corner smoothing applies to the whole shape. https://help.figma.com/hc/en-us/articles/360050986854-Adjust-corner-radius-and-smoothing (search-snippet level; article body not retrieved).
- Stroke section controls in order: stroke fill, opacity, weight (px), position (inside/outside/center), individual per-side strokes, then advanced stroke settings: style (dashed, dotted, brush, dynamic), dash and gap, cap, join (miter, bevel, rounded), miter angle. https://help.figma.com/hc/en-us/articles/360049283914-Apply-and-adjust-stroke-properties.
- Effects section types: Glass, Drop shadow, Inner shadow, Layer blur, Background blur, Noise, Texture. Shadows expose X, Y, blur, spread, colour, and "Show drop shadows through transparent layers"; blurs are uniform or progressive; only one layer blur or background blur per layer. https://help.figma.com/hc/en-us/articles/360041488473-Apply-effects-to-layers.
- Selection colors section appears only "when your selection contains objects with mixed fills"; lists solid colours and gradients on fills and strokes grouped by variable, style and plain fill; a target icon selects all layers using a colour. https://help.figma.com/hc/en-us/articles/360042553434-View-and-adjust-colors-in-a-mixed-selection.
- Export section is "toward the bottom of the right sidebar" in Design mode with edit access, and under the Properties tab with view access; "Export" of all configured selections is Shift Cmd E / Shift Ctrl E. https://help.figma.com/hc/en-us/articles/360040028114-Export-from-Figma-Design.
- Export settings: formats PNG, JPG, SVG, PDF (PDF 1.7); scale presets 0.5x, 0.75x, 1x, 1.5x, 2x, 3x, 4x plus custom width/height via `w`/`h`; SVG and PDF export only at 1x; optional suffix; options include ignore overlapping layers, include bounding box (text), include "id" attribute, outline text, simplify stroke, image quality, image resampling, colour profile. https://help.figma.com/hc/en-us/articles/13402894554519-Export-formats-and-settings.
- Main component selection: a "Properties" section for creating Boolean, Instance swap, Text, Variant and Slot properties. https://help.figma.com/hc/en-us/articles/5579474826519-Explore-component-properties. The "Create component" button is in the right sidebar "next to the selection's name" (Option Cmd K / Ctrl Alt K). https://help.figma.com/hc/en-us/articles/360038663154-Create-components-to-reuse-in-designs.
- Instance selection: the instance menu in the properties panel offers "Detach instance" (Option Cmd B / Ctrl Alt B). https://help.figma.com/hc/en-us/articles/360038665754-Detach-an-instance-from-the-component.
- Vector selection: Enter enters vector edit mode; a secondary toolbar offers Variable width, Shape builder, Cut, Bend, Eraser, Lasso and Paint. https://help.figma.com/hc/en-us/articles/360039957634-Edit-vector-layers.
- Scale tool (K) replaces the Design tab contents with a scale multiplier, W/H fields and an anchor selector. https://help.figma.com/hc/en-us/articles/360040451453-Resize-layers-with-the-Scale-tool.
- Rulers appear along the top and left edges of the canvas; toggle Shift R; guides are dragged from rulers. https://help.figma.com/hc/en-us/articles/360040449713-Add-guides-to-the-canvas-or-frames.
- Unverified: exact pixel widths of the left and right sidebars; default sidebar widths; minimum widths. No primary source states them.
- Unverified: UI3 typography (font family, sizes) and control heights. The blog posts describe icon and input styling only.
- Unverified: the exact on-screen order of Design-tab sections; the order given in Mechanism is a reconstruction from the sources above (Position before Auto layout, Layout and Appearance from the blog descriptions; Export last from the export article).
- Unverified: Share button, Present/play control and collaborator avatars in the top-right area. The file-overview article states only that the right sidebar "contains actions like sharing and exporting".
- Unverified: page background colour control location (right sidebar with nothing selected per the right-sidebar article; the section name is not given).
- Unverified: the Help Center article "Navigating UI3" (article 23954856027159) returned HTTP 404 in en-us and es-419 and could not be used.
- Unverified: iOS/Android export presets; not mentioned in the export-settings article retrieved.

## Mechanism

Spatial model of a UI3 design file (reconstruction from the sources above; proportions not to scale):

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ [A] Navigation bar (File · Assets · Tools · Agents tabs; file name; Minimize) │
├───────────────┬───────────────────────────────────────────┬──────────────────┤
│ [B] Left      │ [C] Canvas                                │ [D] Right sidebar│
│ sidebar       │  rulers (top/left, Shift R)               │  zoom % ▾  share │
│  Pages        │  infinite; Space+drag pans; Cmd/Ctrl+     │  ┌Design┬Proto─┐ │
│  Layers       │  scroll zooms; marquee, smart guides,     │  │ sections… │  │
│  (resizable   │  snapping; Alt-hover measurements         │  │ (resizable)│ │
│   width)      │                                           │  └───────────┘  │
│  Assets tab   │      ┌───────────────────────────────┐    │                  │
│  (components) │      │ [E] Toolbar (floating, bottom) │    │                  │
│               │      │ Move▾ Region▾ Shape▾ Pen▾ T  C▾ │    │                  │
│               │      │ Actions(⌘K)  Dev Mode  Draw   │    │                  │
│               │      └───────────────────────────────┘    │                  │
└───────────────┴───────────────────────────────────────────┴──────────────────┘
Minimize UI (Cmd/Ctrl Shift \): A and B collapse; D reappears while a layer is selected.
Hide UI (Cmd/Ctrl \): A, B, D and E all hidden.
```

Design tab for a frame with auto layout selected (order reconstructed; see unverified note):

1. Header row: layer name, "Create component" button, property-label toggle available from the zoom menu.
2. Position: alignment row; X, Y; rotation; flip horizontal/vertical; Constraints (hidden for auto-layout children; expanded by default per the UI3 design post).
3. Auto layout: flow (vertical, horizontal, wrap, grid); gap; padding (uniform / per side); alignment grid; advanced settings.
4. Layout: W, H; resizing per axis (hug, fill, fixed); min/max width and height; Clip content; Layout guide (uniform grid, column, row).
5. Appearance: opacity (placement inferred, unverified); corner radius with Independent corners and smoothing; blend mode; visibility.
6. Fill: fill list (solid, gradient, image, video, pattern), colour picker with styles and variables.
7. Stroke: colour, opacity, weight, position, per-side strokes, advanced (style, dash/gap, cap, join, miter angle).
8. Effects: Glass, Drop shadow, Inner shadow, Layer blur, Background blur, Noise, Texture.
9. Selection colors: present only for mixed-fill selections.
10. Export: format, scale, suffix, per-format options; page export when nothing is selected.

Contextual variants:

- Text layer: Typography section (text style, family, weight, size, line height, letter spacing, alignment, type settings); text resizing controls in Layout.
- Vector layer: Enter opens vector edit mode with a secondary tool strip; Fill/Stroke/Effects remain.
- Main component: Properties section (Boolean, Instance swap, Text, Variant, Slot) placed above appearance attributes.
- Instance: instance menu with swap, reset, detach and "Go to main component"; component property controls.
- Multiple objects: alignment/distribution row; Selection colors when fills differ; mixed values otherwise (mixed-value rendering: unverified).
- Nothing selected: styles, local variables, canvas background colour, page export.
- Scale tool active: scale multiplier, W/H, anchor.

## NUIF relevance

**Borrow**
- Five-region composition (navigation, left structure panel, centre canvas, right properties panel, bottom floating toolbar) as the test editor's shell, because it is documented and stable since April 2025.
- The Design-tab section taxonomy (Position, Auto layout, Layout, Appearance, Fill, Stroke, Effects, Export) as the property-group vocabulary for the editor's inspector and for the query surface in `spec/12-cli-api-and-automation.md`.
- Minimize/hide UI behaviour, resizable panels and the selection-driven reappearance of the properties panel, because they exercise shell state without touching document state (`apps/editor/ARCHITECTURE.md`).
- Contextual section switching by selection type (text, vector, component, instance, mixed) as test fixtures for selection-dependent inspector queries.

**Adapt**
- Replace Figma-specific section names where NUIF semantics differ (for example, "Auto layout" becomes the NUIF responsive-container model from `spec/04-layout.md`), keeping the spatial slot but not the vendor vocabulary.
- Keep the toolbar order but expose every tool as a semantic operation first, so pointer gestures translate into protocol operations before mutation.
- Present property labels always on (Figma's optional labels) to make screenshots and accessibility-tree assertions deterministic.

**Reject**
- Comments, annotations and measurement tools of the Comment group; multiplayer cursors and avatars; Dev Mode and its Inspect/codegen panels; FigJam, Slides, Buzz and Sites modes; the Actions menu's AI features and asset search; plugin/widget marketplace; Agents tab; version history and branching UI; presentation/prototype player beyond what a test needs; Figma Draw illustration tools (Glass, Noise, Texture effects, brush strokes); Community and library publishing. Reason: none of these are required for testing, import or export, and several depend on network services.

## Open questions

- Which section order does the UI3 Design tab render for a plain frame, and does Auto layout appear as a collapsed row inside Layout or as its own section? Requires an in-app check or a source not yet retrieved.
- What are the default and minimum widths of the sidebars, and does the right sidebar width persist per file or per user?
- How should the test editor render "mixed" values for multi-selection without copying Figma's exact presentation?
- Should the test editor implement the Scale tool (K) given that scale is a derived transform in NUIF rather than a property?
