---
id: nuif:research:figma-tools-and-keyboard-shortcuts
kind: article
status: reviewed
title: Figma Design tools, keyboard shortcuts and canvas interactions relevant to a test editor
source:
  url: https://help.figma.com/hc/en-us/articles/360040328653-Use-Figma-products-with-a-keyboard
  authors: [Figma]
  published_at: "unknown"
  license: proprietary help-center content (Figma); facts only recorded here
retrieved_at: 2026-08-29
tags: [figma, keyboard-shortcuts, tools, direct-manipulation, selection, canvas, editor-ui]
confidence: 0.84
claims: [nuif:claim:semantic-automation]
relations:
  - type: extends
    target: nuif:research:figma
    note: Interaction grammar layered over the public node model.
  - type: related_to
    target: nuif:research:figma-ui3-editor-layout
    note: Shortcuts operate the toolbar and panels described there.
  - type: related_to
    target: nuif:research:figma-plugin-and-rest-api-as-automation-surface
    note: Programmatic equivalents of the same operations.
  - type: compares_to
    target: nuif:research:penpot-editor-ui-and-automation
    note: Penpot shares most bindings (R, E, T, Shift 0/1/2, Ctrl G/K/D).
links:
  spec: [spec/12-cli-api-and-automation.md, spec/06-operations-and-patches.md]
  adr: []
  rfc: [rfcs/0004-headless-qa-contract.md]
  code: [apps/editor/README.md, apps/editor/QA.md]
  experiments: []
---

# Summary

Figma publishes its shortcut inventory primarily through an in-app panel (Ctrl Shift ?) rather than a single reference article; the Help Center article on keyboard use documents keyboard navigation, the keyboard box-selection tool and toolbar focus, while individual feature articles document the tool and command bindings. This record consolidates the bindings that a NUIF test editor needs, each tied to the article that states it. Bindings that no retrieved primary source states (zoom to 100%, redo, Move tool V, Slice tool S, big-nudge key) are listed but marked unverified. Modifier mapping is Cmd/Option on macOS and Ctrl/Alt on Windows throughout the Help Center.

NUIF interpretation: the bindings define the pointer-and-keyboard layer that `apps/editor/QA.md` reserves for shell testing. Every binding below must have a semantic-operation equivalent so that the headless QA contract can execute the same action without synthetic input.

## Evidence

Retrieval date for all locators: 2026-08-29. "Snippet" marks claims verified only through the Help Center search excerpt of the named article rather than the full article body.

- Shortcut panel: Ctrl Shift ? on both platforms, also via Help and resources or the actions menu; the panel "appears along the bottom of your screen" with category tabs and a Layout tab for keyboard layout. https://help.figma.com/hc/en-us/articles/360040328653-Use-Figma-products-with-a-keyboard — "View keyboard shortcuts".
- Keyboard-only canvas: arrow keys pan when nothing is selected, Shift + arrows pan faster; Cmd/Ctrl + or − zooms; F6 (Mac) / Ctrl F6 (Windows) focuses the toolbar; keyboard box selection is Option Space / Ctrl Space; objects needing multiple clicks (lines, vector paths) cannot be inserted by keyboard. Same article.
- Hand tool H; Space held temporarily activates Hand; Cmd/Ctrl + scroll zooms. https://help.figma.com/hc/en-us/articles/30925881896727-FD4B-Navigate-Figma-Design-files; https://help.figma.com/hc/en-us/articles/360041064174-Access-design-tools-from-the-toolbar.
- Scale tool K. https://help.figma.com/hc/en-us/articles/360040451453-Resize-layers-with-the-Scale-tool.
- Frame tool F or A; frame selection Option Cmd G / Ctrl Alt G. https://help.figma.com/hc/en-us/articles/360041539473-Frames-in-Figma-Design.
- Section tool Shift S (snippet). https://help.figma.com/hc/en-us/articles/9771500257687-Organize-your-canvas-with-sections.
- Rectangle R, Line L, Arrow Shift L, Ellipse O; Polygon and Star have no listed shortcut; Shift constrains proportion, Option/Alt draws from centre. https://help.figma.com/hc/en-us/articles/360040450133-Shape-tools.
- Pen P; Escape leaves a path open and deselects. https://help.figma.com/hc/en-us/articles/360040450213-Vector-networks.
- Pencil Shift P, in the Creation tools menu. https://help.figma.com/hc/en-us/articles/4402723791511-Sketch-on-the-canvas-with-the-pencil-tool.
- Vector edit mode: select vector layers and press Enter. https://help.figma.com/hc/en-us/articles/360039957634-Edit-vector-layers.
- Text tool T; Dev Mode toggle Shift D. https://help.figma.com/hc/en-us/articles/360041064174-Access-design-tools-from-the-toolbar.
- Comment mode C. https://help.figma.com/hc/en-us/articles/360039825314-Guide-to-comments-in-Figma.
- Actions menu Cmd K / Ctrl K. https://help.figma.com/hc/en-us/articles/23570416033943-Use-the-actions-menu-in-Figma-Design.
- Add auto layout Shift A; remove Option Shift A / Alt Shift A. https://help.figma.com/hc/en-us/articles/5731482952599-Toggle-on-auto-layout-in-designs.
- Group Cmd G / Ctrl G; ungroup Shift Cmd G or Cmd Delete / Shift Ctrl G or Ctrl Backspace; frame Cmd Option G / Ctrl Alt G; double-click selects a layer inside a group. https://help.figma.com/hc/en-us/articles/360039832054-The-difference-between-frames-and-groups.
- Create component Option Cmd K / Ctrl Alt K. https://help.figma.com/hc/en-us/articles/360038663154-Create-components-to-reuse-in-designs.
- Detach instance Option Cmd B / Ctrl Alt B. https://help.figma.com/hc/en-us/articles/360038665754-Detach-an-instance-from-the-component.
- Duplicate Cmd D / Ctrl D; Option/Alt + drag duplicates; copy Cmd C, paste Cmd V; paste to replace; copy as PNG. https://help.figma.com/hc/en-us/articles/4409078832791-Copy-and-paste-objects.
- Option/Alt drag on an instance creates another instance; the click must be released before the modifier. https://help.figma.com/hc/en-us/articles/360039150173-Create-and-insert-component-instances — "Drag to copy".
- Selection: click; Shift click adds/removes; marquee on empty canvas; Cmd/Ctrl drag marquee selects nested layers; Shift drag removes; Cmd/Ctrl click deep-selects; Enter "Select Child", Shift Enter "Select Parent", Tab / Shift Tab next/previous sibling; Cmd/Ctrl A select all; Cmd/Ctrl Shift A select inverse; Option Cmd A / Ctrl Alt A select matching layers; Esc deselects; right-click "Select layer" submenu. https://help.figma.com/hc/en-us/articles/360040449873-Select-layers-and-objects.
- Nudge: small nudge 1, big nudge 10 "in resolution-independent points", set under Preferences > Nudge amount. https://help.figma.com/hc/en-us/articles/4404575206295-Set-small-and-big-nudge-values. Arrow keys apply the small nudge; the Shift + arrow binding for the big nudge is stated in the position article only by reference to "big nudge" and is marked unverified as a key binding. https://help.figma.com/hc/en-us/articles/360039956914-Adjust-alignment-rotation-position-and-dimensions.
- Align: Option/Alt + A (left), D (right), W (top), S (bottom), H (horizontal centres), V (vertical centres); flip Shift H / Shift V; Shift while rotating snaps to 15°. Same position article.
- Snapping: "Snap to objects" aligns centres and outer points; "Snap to pixel grid"; "Snap to geometry" in vector edit mode; settings under Preferences and the actions menu; hold Control to disable temporarily (snippet). Same position article.
- Measurement: select a layer, hold Option/Alt and hover another layer to show a red measurement line and distances between bounds (snippet). https://help.figma.com/hc/en-us/articles/360039956974-Measure-distances-between-layers.
- Zoom: Shift + / Shift − zoom in/out; Shift 1 zoom to fit; Shift 2 zoom to selection; pixel grid Cmd ' / Ctrl '; snap to pixel grid Cmd Shift ' / Ctrl Shift '; pixel preview Ctrl P / Ctrl Alt P; layout guides Ctrl G (Mac) / Ctrl Shift 4 (Windows); multiplayer cursors Option Cmd \ / Ctrl Alt \. https://help.figma.com/hc/en-us/articles/360041065034-Adjust-your-zoom-and-view-options.
- Layout guides toggle Shift G. https://help.figma.com/hc/en-us/articles/360040450513-Create-layout-guides-with-rows-columns-and-grids. Conflicts with the zoom article; both recorded.
- Rulers Shift R. https://help.figma.com/hc/en-us/articles/360040449713-Add-guides-to-the-canvas-or-frames.
- Minimize UI Cmd Shift \ / Ctrl Shift \; hide UI Cmd \ / Ctrl \. https://help.figma.com/hc/en-us/articles/41414918021271-Hide-or-minimize-the-UI.
- Export all configured selections Shift Cmd E / Shift Ctrl E. https://help.figma.com/hc/en-us/articles/360040028114-Export-from-Figma-Design.
- Undo Cmd Z / Ctrl Z (Help Center search excerpt only; the originating article was not identified, so the binding is snippet-level).
- Keyboard layouts (US, UK, German and others) can be selected so that shortcuts map to the physical keyboard; article located but not retrieved. https://help.figma.com/hc/en-us/articles/5665442977431-Select-keyboard-layout.
- Unverified: Move tool V (search excerpt only, article not identified).
- Unverified: Slice tool S (the slice article was located, https://help.figma.com/hc/en-us/articles/360040028394-Using-the-Slice-Tool, but the key was not visible in retrieved text).
- Unverified: zoom to 100% Shift 0 (documented for FigJam at https://help.figma.com/hc/en-us/articles/1500004414582-Pan-and-zoom-in-FigJam, not found for Figma Design).
- Unverified: redo Cmd Shift Z / Ctrl Shift Z and Ctrl Y; no Help Center locator found.
- Unverified: Shift + arrow as the big-nudge binding (see above); Cmd/Ctrl + / − listed as zoom in the keyboard article while Shift + / − listed in the zoom article; both are recorded.
- Unverified: place image Shift Cmd K / Shift Ctrl K.

## Mechanism

Consolidated binding table (Mac / Windows). V = verified from the article cited above; S = snippet-level; U = unverified.

| Action | macOS | Windows | Status |
|---|---|---|---|
| Move/select tool | V | V | U |
| Hand tool / temporary pan | H / hold Space | H / hold Space | V |
| Scale tool | K | K | V |
| Frame tool | F or A | F or A | V |
| Section tool | Shift S | Shift S | S |
| Slice tool | S | S | U |
| Rectangle / Ellipse / Line / Arrow | R / O / L / Shift L | same | V |
| Polygon / Star | none listed | none listed | V (absence) |
| Pen / Pencil | P / Shift P | P / Shift P | V |
| Text | T | T | V |
| Comment | C | C | V |
| Actions menu | Cmd K | Ctrl K | V |
| Dev Mode toggle | Shift D | Shift D | V |
| Add / remove auto layout | Shift A / Option Shift A | Shift A / Alt Shift A | V |
| Group / Ungroup | Cmd G / Shift Cmd G | Ctrl G / Shift Ctrl G | V |
| Frame selection | Option Cmd G | Ctrl Alt G | V |
| Create component | Option Cmd K | Ctrl Alt K | V |
| Detach instance | Option Cmd B | Ctrl Alt B | V |
| Duplicate / duplicate by drag | Cmd D / Option drag | Ctrl D / Alt drag | V |
| Copy / Paste | Cmd C / Cmd V | Ctrl C / Ctrl V | V |
| Undo | Cmd Z | Ctrl Z | S |
| Redo | Cmd Shift Z | Ctrl Shift Z, Ctrl Y | U |
| Select all / inverse / matching | Cmd A / Cmd Shift A / Option Cmd A | Ctrl A / Ctrl Shift A / Ctrl Alt A | V |
| Select child / parent | Enter / Shift Enter | same | V |
| Next / previous sibling | Tab / Shift Tab | same | V |
| Deep select | Cmd click | Ctrl click | V |
| Marquee (nested / subtract) | drag (Cmd drag / Shift drag) | drag (Ctrl drag / Shift drag) | V |
| Enter vector edit mode | Enter | Enter | V |
| Nudge small / big | Arrow (1) / Shift Arrow (10) | same | V values, U binding |
| Align L/R/T/B/HC/VC | Option A/D/W/S/H/V | Alt A/D/W/S/H/V | V |
| Flip H / V | Shift H / Shift V | same | V |
| Zoom in / out | Shift + / Shift −, Cmd + / − | Shift + / −, Ctrl + / − | V |
| Zoom to fit / selection / 100% | Shift 1 / Shift 2 / Shift 0 | same | V / V / U |
| Rulers / pixel grid / layout guides | Shift R / Cmd ' / Shift G (or Ctrl G) | Shift R / Ctrl ' / Shift G (or Ctrl Shift 4) | V (conflict noted) |
| Minimize UI / hide UI | Cmd Shift \ / Cmd \ | Ctrl Shift \ / Ctrl \ | V |
| Export | Shift Cmd E | Shift Ctrl E | V |
| Shortcut panel | Ctrl Shift ? | Ctrl Shift ? | V |
| Keyboard box selection | Option Space | Ctrl Space | V |
| Focus toolbar | F6 | Ctrl F6 | V |
| Measure to hovered layer | hold Option | hold Alt | S |
| Temporarily disable snapping | hold Control | hold Control | S |

Canvas interaction model as documented: marquee selection from empty canvas; additive and subtractive selection with Shift; hierarchical traversal by Enter/Shift Enter/Tab; deep select with the platform command key; duplication by modifier drag; smart-guide snapping to objects and pixel grid, suspended while Control is held; measurement overlay on modifier hover; pan by Space drag; zoom by command-key scroll.

## NUIF relevance

**Borrow**
- The full binding table as the default keymap of the test editor, because the bindings are widely shared with Penpot and other editors and are not protected expression (see the synthesis record).
- The hierarchical selection grammar (Enter, Shift Enter, Tab, deep select) as the canvas counterpart of NUIF's relationship queries in `spec/12-cli-api-and-automation.md`.
- Small/big nudge as configurable resolution-independent values, matching NUIF's authored coordinates.

**Adapt**
- Map each binding to a named semantic operation in `spec/06-operations-and-patches.md` so that the QA client can replay the same operation log without pointer input (`rfcs/0004-headless-qa-contract.md`).
- Resolve the two documented conflicts (layout-guides toggle; zoom modifier) by choosing one binding and documenting it in the editor's own shortcut panel.
- Provide a keyboard-layout switch only if the test matrix needs non-US layouts; otherwise fix a US layout to keep tests deterministic.

**Reject**
- Comment mode (C), Dev Mode toggle (Shift D), multiplayer cursor toggle, actions-menu AI entries, keyboard box-selection tool, Figma Draw tools and the Scale-tool K binding if scale is not a first-class NUIF operation. Reason: outside the testing/import/export scope or dependent on services the test editor will not have.

## Open questions

- Which primary source, if any, publishes the complete Figma Design shortcut list outside the in-app panel? The in-app panel could be transcribed under controlled conditions and recorded as an experiment.
- Should NUIF's editor treat Shift + arrow as "big nudge" or as "pan faster when nothing is selected", given that Figma overloads the key by selection state?
- How should conflicting bindings from different Help Center articles be reconciled in a conformance keymap fixture?
