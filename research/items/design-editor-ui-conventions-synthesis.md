---
id: nuif:research:design-editor-ui-conventions-synthesis
kind: synthesis
status: reviewed
title: Shared and vendor-specific layout conventions across design editors
source:
  url: https://www.copyright.gov/circs/circ33.pdf
  authors: [NUIF research]
  published_at: "2026-08-29"
  license: NUIF research licensing per docs/whitepaper/08-governance-and-standardization.md; Circular 33 is a US government work; other cited sources retain their own licences
retrieved_at: 2026-08-29
tags: [synthesis, editor-ui, conventions, figma, penpot, sketch, adobe-xd, framer, canva, copyright, trademark]
confidence: 0.8
claims: [nuif:claim:semantic-automation]
relations:
  - type: compares_to
    target: nuif:research:figma-ui3-editor-layout
    note: Figma UI3 row and the bottom-toolbar divergence.
  - type: compares_to
    target: nuif:research:penpot-editor-ui-and-automation
    note: Penpot row.
  - type: related_to
    target: nuif:research:figma
    note: Figma treated strictly as an adapter and layout reference.
  - type: related_to
    target: nuif:research:penpot
    note: Open-source editor reference.
  - type: related_to
    target: nuif:research:naming
    note: Complements the naming/trademark reconnaissance with UI-pattern boundaries.
  - type: related_to
    target: nuif:research:figma-tools-and-keyboard-shortcuts
    note: Shortcut conventions are treated as methods of operation.
links:
  spec: []
  adr: []
  rfc: []
  code: [apps/editor/README.md, apps/editor/ARCHITECTURE.md]
  experiments: []
---

# Summary

Six editors were compared from their vendor documentation: Figma UI3, Penpot, Sketch (Mac), Adobe XD (maintenance mode since 2023), Framer and Canva. All six place structural navigation (pages, layers, assets) in a left panel, the canvas in the centre and selection-dependent properties in a right panel or inspector; all six expose creation tools in a toolbar. The toolbar position is the main variable: Sketch, Penpot and Canva use the top; Adobe XD uses a left vertical strip; Framer and Figma UI3 float it at the bottom. Property panels are sectioned by concern (position/size, layout, appearance, fill, stroke, effects, export) in every editor that documents sections. The shared composition is therefore a genre convention, and the bottom floating toolbar is a recent Figma choice (shared with Framer) rather than a universal one.

Legal framing, recorded briefly and not as advice: the US Copyright Office states that ideas, methods, systems and "format" or "layout" are not copyrightable, while names and logos may be protected by trademark; the First Circuit held a menu command hierarchy to be an uncopyrightable "method of operation" (Lotus v. Borland), and the Ninth Circuit held that Apple "cannot get patent-like protection for the idea of a graphical user interface" (Apple v. Microsoft). A test editor that reproduces spatial arrangement, section taxonomy and shortcuts, but no icons, logos, names or copied artwork, stays within that documented boundary.

## Evidence

Retrieval date for all locators: 2026-08-29. Adobe and Canva pages could not be retrieved in full (timeouts / bot protection); those rows rely on search excerpts of the named vendor pages and are marked accordingly.

- Figma UI3: five regions (navigation bar, left sidebar, canvas, right sidebar, toolbar); toolbar at the bottom; Design/Prototype tabs on the right; sections Position, Auto layout, Layout, Appearance, Fill, Stroke, Effects, Export. https://help.figma.com/hc/en-us/articles/15297425105303-Explore-design-files; https://www.figma.com/blog/behind-our-redesign-ui3/ ("a slim new toolbar at the bottom of the canvas"); https://www.figma.com/blog/our-approach-to-designing-ui3/ ("Toolbars will float at the bottom of all Figma products").
- Penpot: toolbar at the top; Pages and Layers left; Design/Prototype/Inspect right; design groups size and position, layout/constraints, opacity and blend, fill, stroke, shadow, blur, text, export, interactions. https://help.penpot.app/user-guide/first-steps/the-interface/; https://help.penpot.app/user-guide/designing/layers/.
- Sketch: Toolbar top; Layer List left; Canvas centre; Inspector right ("design properties for any layers you've selected"); Minimap bottom right; Cmd . toggles the interface. https://www.sketch.com/docs/designing/the-interface/; https://www.sketch.com/docs/designing/the-interface/the-toolbar/ (toolbar "at the top of the Mac app window", contextual).
- Adobe XD (search excerpts of vendor pages; full page not retrieved): workspace elements include Design/Prototype/Share modes, Property Inspector, Pasteboard, Artboard, Plugins, Layers, Libraries, Toolbar; Layers panel via Cmd Y / Ctrl Y or a toolbar icon; the toolbar is described as a vertical strip on the left in vendor material (unverified in retrieved text). https://helpx.adobe.com/xd/help/workspace-basics.html; https://helpx.adobe.com/xd/help/layers.html.
- Adobe XD status: "Adobe XD continues to be in maintenance mode" and Adobe is "not investing in ongoing development or shipping new features" (search excerpt of the vendor support page; page not retrieved). https://helpx.adobe.com/support/xd.html.
- Framer: "The canvas controls are located in the bottom toolbar" with selection, pan, comment tools and a zoom menu "on the right side"; Space + drag pans; Cmd/Ctrl + and − zoom. https://www.framer.com/help/articles/how-to-use-the-canvas/. The Actions menu was "placed it in the Toolbar" in May 2023. https://www.framer.com/updates/may-update-2023. Left layers panel and right properties panel are stated in Framer Academy excerpts (lesson bodies not retrieved). https://www.framer.com/academy/lessons/framer-fundamentals-framer-interface.
- Canva (search excerpts; pages blocked by bot protection): a left side panel for elements, text, uploads and apps; a contextual toolbar above the design; layers reached via Position on the toolbar or a Layers tab in the side panel. https://www.canva.com/help/finding-and-arranging-layers/; https://www.canva.com/help/glow-up-variantb/.
- Copyright Office Circular 33, "Ideas, Methods, and Systems": copyright excludes "any idea, procedure, process, system, method of operation, concept, principle, or discovery"; "Layout and Design": the Office "will not accept a claim to copyright in 'format' or 'layout'"; "Names, Titles, Short Phrases": names and slogans are uncopyrightable but "may be protectable under federal or state trademark laws". https://www.copyright.gov/circs/circ33.pdf — pages 1–3.
- Lotus Development Corp. v. Borland International, 49 F.3d 807 (1st Cir. 1995): the menu command hierarchy is "an uncopyrightable 'method of operation'" under 17 U.S.C. § 102(b); "methods of operation" are "the means by which a user operates something". Full text mirrored at https://www.bitlaw.com/source/cases/copyright/Lotus.html — Part II.D.
- Apple Computer, Inc. v. Microsoft Corp., 35 F.3d 1435 (9th Cir. 1994): "Apple cannot get patent-like protection for the idea of a graphical user interface, or the idea of a desktop metaphor"; GUIs are dissected because "copyright protection extends only to protectable elements of expression". Full text at https://law.resource.org/pub/us/case/reporter/F3/035/35.F3d.1435.93-16883.93-16869.93-16867.html.
- Unverified: Adobe XD toolbar orientation and exact Property Inspector sectioning; Canva panel names beyond the excerpts; Framer properties-panel section names; whether Sketch's toolbar can be hidden independently of the whole interface.

## Mechanism

Comparison table (positions as documented; "excerpt" marks rows built from vendor search excerpts only):

| Editor | Left panel | Canvas | Right panel | Toolbar position | Property sectioning | Status |
|---|---|---|---|---|---|---|
| Figma UI3 | File tab: pages + layers; Assets; Tools | infinite, rulers, zoom menu top-right | Design / Prototype tabs | bottom, floating, collapsible | Position, Auto layout, Layout, Appearance, Fill, Stroke, Effects, Selection colors, Export | verified |
| Penpot | Pages, Layers (Alt L); Assets (Alt I) | infinite viewport, rulers | Design / Prototype / Inspect; palettes | top, horizontal | size/position, layout, constraints, layer, fill, stroke, shadow, blur, text, export, interactions | verified |
| Sketch (Mac) | Layer List (pages, frames, layers) | infinite canvas, minimap | Inspector | top (macOS toolbar), contextual | selection-dependent inspector | verified |
| Adobe XD | Layers (Cmd Y), Libraries, Plugins | pasteboard with artboards | Property Inspector | left, vertical (excerpt) | dimensions, appearance, design specs tab | excerpt; maintenance mode |
| Framer | Layers (sections, frames) | infinite canvas | Properties | bottom toolbar with zoom menu | size, content, design (excerpt) | partially verified |
| Canva | side panel (elements, text, uploads, apps); Layers tab | page-based canvas | contextual toolbar above design | top (contextual) | element-type dependent | excerpt |

Shared conventions: left panel holds document structure; canvas occupies the centre; right panel holds selection-dependent properties, sectioned by geometry, layout, appearance, paint, effects and export; creation tools live in a toolbar; hierarchical layer trees with visibility and lock; zoom control near the canvas edge.

Figma-specific or minority conventions: a floating, collapsible toolbar at the bottom of the canvas (shared only with Framer among the six); a mode switch (Design / Dev Mode) inside that toolbar; an actions/command palette in the toolbar (shared with Framer); optional property labels; minimize-UI behaviour that re-expands the right panel on selection.

Legal boundary as documented (three sentences): layouts, methods of operation and menu/command structures are outside copyright per Circular 33, Lotus and Apple; names, logos, icons and other original artwork remain protectable by trademark or copyright; the test editor therefore reproduces arrangement, taxonomy and bindings while using its own icon set, names and visual assets. This is a summary of public sources, not legal advice.

## NUIF relevance

**Borrow**
- The shared left-structure / centre-canvas / right-properties composition, because every compared editor uses it and it is documented as a genre convention.
- Concern-based property sectioning (geometry, layout, appearance, fill, stroke, effects, export) as the inspector taxonomy, mirrored in NUIF's property groups.
- Figma UI3's bottom floating toolbar and minimize-UI behaviour, since the test editor is specified to replicate the UI3 interaction model and the pattern is a method of operation rather than protectable expression.

**Adapt**
- Author an original icon set, palette and naming; do not reproduce Figma's icons, logo, product names or Help Center artwork (`nuif:research:naming`).
- Where editors disagree (toolbar position, ellipse/board shortcuts), document the chosen convention in the editor's own reference rather than presenting it as a Figma feature.
- Treat Adobe XD as a historical data point only, given its maintenance-mode status.

**Reject**
- Editor-specific product features that are not layout conventions: Figma Dev Mode, FigJam, AI actions, community marketplace, version history and branching UI, multiplayer cursors, comments, Canva's template and media browsers, Framer's CMS and publishing controls, Sketch's Components view and prototyping player beyond test needs. Reason: they are product scope, not conventions, and are outside the test editor's testing/import/export remit.

## Open questions

- Should the comparison be extended with Lunacy, Affinity Designer or Pencil-class editors to test whether the bottom-toolbar pattern is spreading?
- Are there jurisdictions outside the US where UI layout or "look and feel" receives stronger protection that would affect the test editor's distribution?
- Which Adobe XD and Canva documentation pages can be retrieved through an alternative channel to upgrade their rows from excerpt to verified?
