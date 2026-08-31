import {
  PINNED_FONT_NAME,
  PINNED_FONT_SHA256,
  PINNED_FONT_STYLE,
  ProfileError,
  SNAPSHOT_SCHEMA_VERSION,
  assertSnapshot,
  emptyLayout,
  type HostAxisAlign,
  type HostLayout,
  type HostNodeKind,
  type HostText,
  type PluginSnapshot,
  type SnapshotNode,
  type SolidPaint
} from "./protocol";

const NAMESPACE = "nuif";
type SupportedNode = FrameNode | GroupNode | RectangleNode | EllipseNode | TextNode;

export interface SnapshotEnvironment {
  apiVersion: string;
  fileKey?: string;
  pageId: string;
  pageName: string;
  documentId?: string;
  hostApplicationVersion?: string;
  hostDocumentRevision?: string;
}

export function normalizeSelection(root: SceneNode, environment: SnapshotEnvironment): PluginSnapshot {
  if (root.type !== "FRAME") {
    throw new ProfileError("NUIF_FIGMA_SELECTION", "Select exactly one FRAME before exporting");
  }
  const hostDocumentId = environment.fileKey?.trim() || `local:${root.id}`;
  const snapshot: PluginSnapshot = {
    schema_version: SNAPSHOT_SCHEMA_VERSION,
    host_application_version: environment.hostApplicationVersion?.trim() || "unreported-by-plugin-api",
    host_api_version: environment.apiVersion,
    host_document_id: hostDocumentId,
    host_document_revision: environment.hostDocumentRevision?.trim() || null,
    page_id: environment.pageId,
    page_name: environment.pageName,
    nuif_document_id: environment.documentId?.trim() || null,
    root: normalizeNode(root)
  };
  return assertSnapshot(snapshot);
}

function normalizeNode(node: SceneNode): SnapshotNode {
  const supported = requireSupportedNode(node);
  const kind = supported.type;
  const unsupported = new Set<string>();
  inspectCommonProperties(supported, unsupported);
  const fill = kind === "GROUP" ? null : normalizeFill(supported, unsupported);
  const layout = kind === "FRAME" ? normalizeLayout(supported, unsupported) : emptyLayout();
  const text = kind === "TEXT" ? normalizeText(supported) : null;
  if (kind === "TEXT") inspectTextProperties(supported, unsupported);
  const children = kind === "FRAME" || kind === "GROUP" ? supported.children.map(normalizeNode) : [];
  return {
    id: supported.id,
    name: supported.name,
    kind,
    visible: supported.visible,
    opacity: supported.opacity,
    x: supported.x,
    y: supported.y,
    width: supported.width,
    height: supported.height,
    fill,
    layout,
    text,
    nuif_entity_id: optionalSharedData(supported, "entity_id"),
    unsupported_properties: [...unsupported].sort(),
    children
  };
}

function requireSupportedNode(node: SceneNode): SupportedNode {
  switch (node.type) {
    case "FRAME":
    case "GROUP":
    case "RECTANGLE":
    case "ELLIPSE":
    case "TEXT":
      return node;
    default:
      throw new ProfileError("NUIF_FIGMA_NODE_KIND", `${node.type} node ${node.id} is outside the snapshot profile`);
  }
}

function normalizeFill(node: SupportedNode, unsupported: Set<string>): SolidPaint | null {
  const fills = property(node, "fills");
  const styleId = property(node, "fillStyleId");
  if (typeof styleId === "string" && styleId !== "") unsupported.add("fillStyleId");
  if (typeof fills === "symbol") {
    unsupported.add("fills.mixed");
    return null;
  }
  if (!Array.isArray(fills) || fills.length === 0) return null;
  if (fills.length !== 1) {
    unsupported.add("fills.multiple");
    return null;
  }
  const paint = fills[0] as Paint | undefined;
  if (paint === undefined || paint.type !== "SOLID" || paint.visible === false) {
    unsupported.add("fills");
    return null;
  }
  if (paint.blendMode !== undefined && paint.blendMode !== "NORMAL") unsupported.add("fills.blendMode");
  return {
    red: paint.color.r,
    green: paint.color.g,
    blue: paint.color.b,
    alpha: paint.opacity ?? 1
  };
}

function normalizeLayout(frame: FrameNode, unsupported: Set<string>): HostLayout {
  if (frame.layoutMode === "NONE") return emptyLayout();
  if (frame.layoutMode === "GRID") {
    unsupported.add("layoutMode.GRID");
    return emptyLayout();
  }
  if (frame.layoutWrap !== "NO_WRAP") unsupported.add("layoutWrap");
  if (frame.primaryAxisSizingMode !== "FIXED") unsupported.add("primaryAxisSizingMode");
  if (frame.counterAxisSizingMode !== "FIXED") unsupported.add("counterAxisSizingMode");
  if (frame.primaryAxisAlignItems !== "MIN") unsupported.add("primaryAxisAlignItems");
  let counterAxis: HostAxisAlign = "MIN";
  if (frame.counterAxisAlignItems === "CENTER") counterAxis = "CENTER";
  else if (frame.counterAxisAlignItems === "MAX") counterAxis = "MAX";
  else if (frame.counterAxisAlignItems !== "MIN") unsupported.add("counterAxisAlignItems");
  if (frame.itemReverseZIndex) unsupported.add("itemReverseZIndex");
  if (frame.strokesIncludedInLayout) unsupported.add("strokesIncludedInLayout");
  return {
    mode: frame.layoutMode,
    item_spacing: frame.itemSpacing,
    padding_top: frame.paddingTop,
    padding_right: frame.paddingRight,
    padding_bottom: frame.paddingBottom,
    padding_left: frame.paddingLeft,
    primary_axis_align: "MIN",
    counter_axis_align: counterAxis
  };
}

function normalizeText(text: TextNode): HostText {
  if (typeof text.fontName === "symbol" || typeof text.fontSize === "symbol" || typeof text.lineHeight === "symbol") {
    throw new ProfileError("NUIF_FIGMA_MIXED_TEXT", `Text node ${text.id} has mixed font or line-height metadata`);
  }
  if (text.lineHeight.unit !== "PIXELS") {
    throw new ProfileError("NUIF_FIGMA_LINE_HEIGHT", `Text node ${text.id} requires a pixel line height`);
  }
  const fontSha256 = optionalSharedData(text, "font_sha256");
  if (
    text.fontName.family !== PINNED_FONT_NAME ||
    text.fontName.style !== PINNED_FONT_STYLE ||
    fontSha256 !== PINNED_FONT_SHA256
  ) {
    throw new ProfileError(
      "NUIF_FIGMA_FONT_IDENTITY",
      `Text node ${text.id} requires Ahem Regular plus its exact shared-data SHA-256 marker`
    );
  }
  return {
    characters: text.characters,
    font_family: text.fontName.family,
    font_style: text.fontName.style,
    font_sha256: fontSha256,
    font_size: text.fontSize,
    line_height: text.lineHeight.value
  };
}

function inspectCommonProperties(node: SupportedNode, unsupported: Set<string>): void {
  if (node.locked) unsupported.add("locked");
  if (property(node, "isMask") === true) unsupported.add("isMask");
  if (property(node, "constrainProportions") === true) unsupported.add("constrainProportions");
  if (node.rotation !== 0) unsupported.add("rotation");
  if (node.blendMode !== "PASS_THROUGH" && node.blendMode !== "NORMAL") unsupported.add("blendMode");
  if (nonemptyArray(property(node, "strokes"))) unsupported.add("strokes");
  if (nonemptyArray(property(node, "effects"))) unsupported.add("effects");
  if (nonemptyRecord(property(node, "boundVariables"))) unsupported.add("boundVariables");
  if (property(node, "clipsContent") === true) unsupported.add("clipsContent");
  if (nonemptyArray(property(node, "layoutGrids"))) unsupported.add("layoutGrids");
  const gridStyleId = property(node, "gridStyleId");
  if (typeof gridStyleId === "symbol" || (typeof gridStyleId === "string" && gridStyleId !== "")) {
    unsupported.add("gridStyleId");
  }
  const overflowDirection = property(node, "overflowDirection");
  if (overflowDirection !== undefined && overflowDirection !== "NONE") unsupported.add("overflowDirection");
  const cornerRadius = property(node, "cornerRadius");
  if (typeof cornerRadius === "symbol" || (typeof cornerRadius === "number" && cornerRadius !== 0)) unsupported.add("cornerRadius");
  const reactions = property(node, "reactions");
  if (nonemptyArray(reactions)) unsupported.add("reactions");
  const constraints = property(node, "constraints");
  if (
    constraints !== undefined &&
    (property(constraints, "horizontal") !== "MIN" || property(constraints, "vertical") !== "MIN")
  ) {
    unsupported.add("constraints");
  }
  if (property(node, "layoutGrow") !== undefined && property(node, "layoutGrow") !== 0) unsupported.add("layoutGrow");
  if (property(node, "layoutAlign") !== undefined && property(node, "layoutAlign") !== "INHERIT") unsupported.add("layoutAlign");
  if (property(node, "layoutPositioning") !== undefined && property(node, "layoutPositioning") !== "AUTO") {
    unsupported.add("layoutPositioning");
  }
  const transform = property(node, "relativeTransform");
  if (!hasIdentityLinearTransform(transform)) unsupported.add("relativeTransform");
  if (node.type === "ELLIPSE") {
    const arc = node.arcData;
    if (arc.startingAngle !== 0 || Math.abs(arc.endingAngle - Math.PI * 2) > 1e-12 || arc.innerRadius !== 0) {
      unsupported.add("arcData");
    }
  }
}

function inspectTextProperties(node: TextNode, unsupported: Set<string>): void {
  if (node.textAlignHorizontal !== "LEFT") unsupported.add("textAlignHorizontal");
  if (node.textAlignVertical !== "TOP") unsupported.add("textAlignVertical");
  if (node.textAutoResize !== "NONE") unsupported.add("textAutoResize");
  if (node.textTruncation !== "DISABLED") unsupported.add("textTruncation");
  if (node.maxLines !== null) unsupported.add("maxLines");
  if (node.textCase !== "ORIGINAL") unsupported.add("textCase");
  if (node.textDecoration !== "NONE") unsupported.add("textDecoration");
  if (typeof node.letterSpacing === "symbol" || node.letterSpacing.value !== 0) unsupported.add("letterSpacing");
  if (node.paragraphIndent !== 0) unsupported.add("paragraphIndent");
  if (node.paragraphSpacing !== 0) unsupported.add("paragraphSpacing");
  if (node.listSpacing !== 0) unsupported.add("listSpacing");
  if (node.hangingPunctuation) unsupported.add("hangingPunctuation");
  if (node.hangingList) unsupported.add("hangingList");
  if (node.hyperlink !== null) unsupported.add("hyperlink");
  if (typeof node.textStyleId === "symbol" || node.textStyleId !== "") unsupported.add("textStyleId");
  if (node.characters.length !== 0) {
    const list = node.getRangeListOptions(0, node.characters.length);
    if (typeof list === "symbol" || list.type !== "NONE") unsupported.add("listOptions");
    const indentation = node.getRangeIndentation(0, node.characters.length);
    if (typeof indentation === "symbol" || indentation !== 0) unsupported.add("indentation");
  }
}

function optionalSharedData(node: BaseNode, key: string): string | null {
  const value = node.getSharedPluginData(NAMESPACE, key).trim();
  return value === "" ? null : value;
}

function property(value: unknown, key: string): unknown {
  if (value === null || (typeof value !== "object" && typeof value !== "function")) return undefined;
  return (value as Record<string, unknown>)[key];
}

function nonemptyArray(value: unknown): boolean {
  return Array.isArray(value) && value.length !== 0;
}

function nonemptyRecord(value: unknown): boolean {
  return value !== null && typeof value === "object" && Object.keys(value).length !== 0;
}

function hasIdentityLinearTransform(value: unknown): boolean {
  if (!Array.isArray(value) || value.length !== 2) return value === undefined;
  const first = value[0];
  const second = value[1];
  return (
    Array.isArray(first) &&
    Array.isArray(second) &&
    first.length === 3 &&
    second.length === 3 &&
    first[0] === 1 &&
    first[1] === 0 &&
    second[0] === 0 &&
    second[1] === 1
  );
}

export const sharedDataNamespace = NAMESPACE;
