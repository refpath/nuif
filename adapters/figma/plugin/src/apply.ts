import {
  PINNED_FONT_NAME,
  PINNED_FONT_STYLE,
  ProfileError,
  assertMutationPlan,
  type HostLayout,
  type PluginMutationPlan,
  type SnapshotNode,
  type SolidPaint
} from "./protocol";
import { sharedDataNamespace } from "./normalize";

export interface ApplyResult {
  rootId: string;
  nodesCreated: number;
}

export async function applyMutationPlan(input: unknown): Promise<ApplyResult> {
  const plan = assertMutationPlan(input);
  await preflight(plan);
  const created: SceneNode[] = [];
  try {
    const root = createNode(plan.snapshot.root, created);
    if (root.type !== "FRAME") throw new ProfileError("NUIF_FIGMA_ROOT_KIND", "Created root was not a FRAME");
    root.setSharedPluginData(sharedDataNamespace, "document_id", plan.snapshot.nuif_document_id ?? "");
    root.setSharedPluginData(sharedDataNamespace, "profile", plan.profile);
    figma.currentPage.selection = [root];
    figma.viewport.scrollAndZoomIntoView([root]);
    figma.commitUndo();
    return { rootId: root.id, nodesCreated: created.length };
  } catch (error) {
    for (const node of [...created].reverse()) {
      if (!node.removed) node.remove();
    }
    throw error;
  }
}

async function preflight(plan: PluginMutationPlan): Promise<void> {
  if (containsText(plan.snapshot.root)) {
    const fonts = await figma.listAvailableFontsAsync();
    const present = fonts.some(
      ({ fontName }) => fontName.family === PINNED_FONT_NAME && fontName.style === PINNED_FONT_STYLE
    );
    if (!present) {
      throw new ProfileError(
        "NUIF_FIGMA_FONT_UNAVAILABLE",
        "Ahem Regular is not available in Figma. Install the pinned profile font before importing this plan."
      );
    }
    await figma.loadFontAsync({ family: PINNED_FONT_NAME, style: PINNED_FONT_STYLE });
  }
}

function containsText(node: SnapshotNode): boolean {
  return node.kind === "TEXT" || node.children.some(containsText);
}

function createNode(source: SnapshotNode, created: SceneNode[]): SceneNode {
  let node: SceneNode;
  switch (source.kind) {
    case "FRAME":
      node = figma.createFrame();
      break;
    case "RECTANGLE":
      node = figma.createRectangle();
      break;
    case "ELLIPSE":
      node = figma.createEllipse();
      break;
    case "TEXT":
      node = figma.createText();
      break;
    case "GROUP":
      throw new ProfileError("NUIF_FIGMA_PLAN_GROUP", "Mutation plans cannot contain GROUP nodes");
  }
  created.push(node);
  node.name = source.name;
  node.visible = true;
  node.opacity = 1;
  node.x = source.x;
  node.y = source.y;
  if (node.type === "FRAME") configureFrame(node, source.layout);
  if (node.type === "TEXT") configureText(node, source);
  configureFill(node, source.fill);
  node.resize(source.width, source.height);
  node.setSharedPluginData(sharedDataNamespace, "entity_id", source.nuif_entity_id ?? "");
  node.setSharedPluginData(sharedDataNamespace, "profile", planProfileMarker());
  if (node.type === "FRAME") {
    for (const childSource of source.children) {
      node.appendChild(createNode(childSource, created));
    }
    // Appending children can resize auto-layout frames; fixed plan dimensions win.
    node.resize(source.width, source.height);
  }
  return node;
}

function configureFrame(node: FrameNode, layout: HostLayout): void {
  node.layoutMode = layout.mode;
  if (layout.mode === "NONE") return;
  node.layoutWrap = "NO_WRAP";
  node.primaryAxisAlignItems = "MIN";
  node.counterAxisAlignItems = layout.counter_axis_align;
  node.primaryAxisSizingMode = "FIXED";
  node.counterAxisSizingMode = "FIXED";
  node.itemSpacing = layout.item_spacing;
  node.paddingTop = layout.padding_top;
  node.paddingRight = layout.padding_right;
  node.paddingBottom = layout.padding_bottom;
  node.paddingLeft = layout.padding_left;
  node.itemReverseZIndex = false;
  node.strokesIncludedInLayout = false;
}

function configureText(node: TextNode, source: SnapshotNode): void {
  if (source.text === null) throw new ProfileError("NUIF_FIGMA_TEXT_SHAPE", "TEXT metadata is missing");
  node.fontName = { family: source.text.font_family, style: source.text.font_style };
  node.characters = source.text.characters;
  node.fontSize = source.text.font_size;
  node.lineHeight = { unit: "PIXELS", value: source.text.line_height };
  node.textAutoResize = "NONE";
}

function configureFill(node: SceneNode, fill: SolidPaint | null): void {
  if (!("fills" in node)) {
    if (fill !== null) throw new ProfileError("NUIF_FIGMA_FILL_TARGET", `${node.type} cannot accept fills`);
    return;
  }
  node.fills =
    fill === null
      ? []
      : [
          {
            type: "SOLID",
            color: { r: fill.red, g: fill.green, b: fill.blue },
            opacity: fill.alpha,
            visible: true,
            blendMode: "NORMAL"
          }
        ];
}

function planProfileMarker(): string {
  return "nuif-figma-plugin-snapshot-0";
}
