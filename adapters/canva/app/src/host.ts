import type { DesignEditing } from "@canva/design";
import {
  CANVA_HOST_API_VERSION,
  CANVA_SCHEMA_VERSION,
  ProfileError,
  assertMutationPlan,
  type CanvaElement,
  type CanvaMutationPlan,
  type CanvaPage,
  type SolidColor
} from "./protocol";

export const CANVA_APPS_SDK_VERSION = "2.12.0";
export const CANONICAL_ELLIPSE_PATH = "M 50 0 A 50 50 0 1 1 50 100 A 50 50 0 1 1 50 0 Z";

export interface ApplyResult {
  elementsCreated: number;
  syncs: number;
}

type FixedAbsolutePage = DesignEditing.AbsolutePage & { readonly dimensions: DesignEditing.Dimensions };

export function normalizeCurrentPage(page: DesignEditing.Page): CanvaPage {
  const absolute = requireFixedUnlockedPage(page);
  const elements = absolute.elements
    .toArray()
    .map((element, index) => normalizeElement(element, `${String(absolute.id)}:${index}`, 0));
  return {
    schema_version: CANVA_SCHEMA_VERSION,
    host_application_version: `Apps SDK ${CANVA_APPS_SDK_VERSION}`,
    host_api_version: CANVA_HOST_API_VERSION,
    host_document_id: `canva-current-page:${String(absolute.id)}`,
    host_document_revision: null,
    page_id: String(absolute.id),
    page_name: null,
    width: absolute.dimensions.width,
    height: absolute.dimensions.height,
    background: normalizeFill(absolute.background, "page.background"),
    elements
  };
}

export async function applyPlanToSession(
  input: unknown,
  session: DesignEditing.CurrentPageSession
): Promise<ApplyResult> {
  const plan = assertMutationPlan(input);
  const page = preflightHostImport(plan, session.page);
  setPageBackground(page, plan.page.background);
  let created = 0;
  for (const element of plan.page.elements) {
    const state = createElementState(session.helpers.elementStateBuilder, element);
    const inserted = page.elements.insertAfter(undefined, state);
    if (inserted === undefined) {
      throw new ProfileError("NUIF_CANVA_INSERT_FAILED", `Canva did not insert ${element.id}`);
    }
    created += 1;
  }
  await session.sync();
  return { elementsCreated: created, syncs: 1 };
}

export function preflightHostImport(
  plan: CanvaMutationPlan,
  hostPage: DesignEditing.Page
): DesignEditing.AbsolutePage {
  const page = requireFixedUnlockedPage(hostPage);
  if (page.elements.count() !== 0) {
    throw new ProfileError("NUIF_CANVA_NONEMPTY_PAGE", "Import requires an empty current page");
  }
  if (page.dimensions.width !== plan.page.width || page.dimensions.height !== plan.page.height) {
    throw new ProfileError("NUIF_CANVA_PAGE_SIZE", "Current page dimensions do not match the plan exactly");
  }
  if (plan.page.page_name !== null) {
    throw new ProfileError("NUIF_CANVA_PAGE_NAME", "The Apps SDK does not expose a writable page name");
  }
  assertOpaqueColor(plan.page.background, "page.background");
  if (plan.page.background !== null && page.background === undefined) {
    throw new ProfileError("NUIF_CANVA_PAGE_BACKGROUND", "Current page does not expose a writable background");
  }
  for (const element of plan.page.elements) preflightElement(element);
  return page;
}

function requireFixedUnlockedPage(page: DesignEditing.Page): FixedAbsolutePage {
  if (page.type !== "absolute") {
    throw new ProfileError("NUIF_CANVA_PAGE_TYPE", "Only absolute Canva pages are supported");
  }
  if (page.locked) throw new ProfileError("NUIF_CANVA_LOCKED_PAGE", "The current page is locked");
  if (page.dimensions === undefined) {
    throw new ProfileError("NUIF_CANVA_UNBOUNDED_PAGE", "Whiteboards and other unbounded pages are unsupported");
  }
  return page as FixedAbsolutePage;
}

function normalizeElement(
  element: DesignEditing.AbsoluteElement | DesignEditing.GroupContentElement,
  hostId: string,
  depth: number
): CanvaElement {
  if (depth >= 64) throw new ProfileError("NUIF_CANVA_DEPTH_LIMIT", "Element tree exceeds 64 levels");
  const common = normalizeCommon(element, hostId);
  switch (element.type) {
    case "rect":
      if (element.stroke.weight !== 0) {
        throw new ProfileError("NUIF_CANVA_STROKE", `${hostId} has an unsupported rectangle stroke`);
      }
      return { ...common, kind: "rectangle", fill: normalizeFill(element.fill, `${hostId}.fill`), text: null, children: [] };
    case "shape": {
      const paths = element.paths.toArray();
      if (
        element.viewBox.top !== 0 ||
        element.viewBox.left !== 0 ||
        element.viewBox.width !== 100 ||
        element.viewBox.height !== 100 ||
        paths.length !== 1 ||
        paths[0]?.d !== CANONICAL_ELLIPSE_PATH ||
        paths[0].stroke !== undefined
      ) {
        throw new ProfileError("NUIF_CANVA_SHAPE", `${hostId} is not the canonical profile ellipse`);
      }
      return {
        ...common,
        kind: "ellipse",
        fill: normalizePathFill(paths[0].fill, `${hostId}.paths[0].fill`),
        text: null,
        children: []
      };
    }
    case "group":
      return {
        ...common,
        kind: "group",
        fill: null,
        text: null,
        children: element.contents
          .toArray()
          .map((child, index) => normalizeElement(child, `${hostId}.${index}`, depth + 1))
      };
    case "text":
      throw new ProfileError(
        "NUIF_CANVA_TEXT_IDENTITY",
        `${hostId} text cannot prove portable font-file identity and exact text-box height`
      );
    case "embed":
    case "unsupported":
      throw new ProfileError("NUIF_CANVA_ELEMENT_TYPE", `${hostId} has unsupported type ${element.type}`);
  }
}

function normalizeCommon(
  element: DesignEditing.AbsoluteElement | DesignEditing.GroupContentElement,
  id: string
): Omit<CanvaElement, "kind" | "fill" | "text" | "children"> {
  if (element.locked) throw new ProfileError("NUIF_CANVA_LOCKED_ELEMENT", `${id} is locked`);
  return {
    id,
    name: null,
    visible: true,
    locked: false,
    opacity: 1 - element.transparency,
    rotation: element.rotation,
    x: element.left,
    y: element.top,
    width: element.width,
    height: element.height,
    unsupported_properties: []
  };
}

function normalizeFill(fill: DesignEditing.Fill | undefined, pointer: string): SolidColor | null {
  if (fill === undefined) return null;
  if (fill.mediaContainer.ref !== undefined) {
    throw new ProfileError("NUIF_CANVA_MEDIA_FILL", `${pointer} contains image or video media`);
  }
  return normalizeColorFill(fill.colorContainer.ref, pointer);
}

function normalizePathFill(fill: DesignEditing.PathFill, pointer: string): SolidColor | null {
  if (fill.mediaContainer.ref !== undefined) {
    throw new ProfileError("NUIF_CANVA_MEDIA_FILL", `${pointer} contains image or video media`);
  }
  return normalizeColorFill(fill.colorContainer.ref, pointer);
}

function normalizeColorFill(fill: DesignEditing.ColorFill | undefined, pointer: string): SolidColor | null {
  if (fill === undefined) return null;
  if (fill.type !== "solid") throw new ProfileError("NUIF_CANVA_COLOR_FILL", `${pointer} is not a solid color`);
  return parseHex(fill.color, pointer);
}

function parseHex(value: string, pointer: string): SolidColor {
  if (!/^#[0-9a-f]{6}$/.test(value)) {
    throw new ProfileError("NUIF_CANVA_COLOR", `${pointer} is not a lowercase six-digit sRGB color`);
  }
  return {
    red: Number.parseInt(value.slice(1, 3), 16) / 255,
    green: Number.parseInt(value.slice(3, 5), 16) / 255,
    blue: Number.parseInt(value.slice(5, 7), 16) / 255,
    alpha: 1
  };
}

function preflightElement(element: CanvaElement): void {
  if (element.kind !== "rectangle" && element.kind !== "ellipse") {
    throw new ProfileError("NUIF_CANVA_LIVE_KIND", `Live import does not support ${element.kind}`);
  }
  if (
    element.name !== null ||
    !element.visible ||
    element.locked ||
    element.opacity !== 1 ||
    element.text !== null ||
    element.children.length !== 0 ||
    element.unsupported_properties.length !== 0
  ) {
    throw new ProfileError("NUIF_CANVA_LIVE_PROPERTY", `${element.id} contains a property the host cannot reproduce exactly`);
  }
  if (element.x < -32768 || element.x > 32767 || element.y < -32768 || element.y > 32767) {
    throw new ProfileError("NUIF_CANVA_HOST_POSITION", `${element.id} is outside Canva's position bounds`);
  }
  if (element.rotation < -180 || element.rotation > 180) {
    throw new ProfileError("NUIF_CANVA_HOST_ROTATION", `${element.id} is outside Canva's rotation bounds`);
  }
  assertOpaqueColor(element.fill, `${element.id}.fill`);
}

function assertOpaqueColor(color: SolidColor | null, pointer: string): void {
  if (color !== null && color.alpha !== 1) {
    throw new ProfileError("NUIF_CANVA_HOST_ALPHA", `${pointer} must be opaque for Canva solid fills`);
  }
}

function setPageBackground(page: DesignEditing.AbsolutePage, color: SolidColor | null): void {
  if (page.background === undefined) {
    if (color !== null) throw new ProfileError("NUIF_CANVA_PAGE_BACKGROUND", "Page background is unavailable");
    return;
  }
  page.background.mediaContainer.set(undefined);
  page.background.colorContainer.set(color === null ? undefined : { type: "solid", color: colorToHex(color) });
}

function createElementState(
  builder: DesignEditing.ElementStateBuilder,
  element: CanvaElement
): DesignEditing.InsertableElementState {
  const common = {
    top: element.y,
    left: element.x,
    rotation: element.rotation,
    transparency: 0,
    width: element.width,
    height: element.height
  };
  const fill = element.fill === null ? undefined : { colorContainer: { type: "solid" as const, color: colorToHex(element.fill) } };
  if (element.kind === "rectangle") {
    return fill === undefined ? builder.createRectElement(common) : builder.createRectElement({ ...common, fill });
  }
  if (element.kind === "ellipse") {
    const path = fill === undefined ? { d: CANONICAL_ELLIPSE_PATH } : { d: CANONICAL_ELLIPSE_PATH, fill };
    return builder.createShapeElement({
      ...common,
      viewBox: { top: 0, left: 0, width: 100, height: 100 },
      paths: [path]
    });
  }
  throw new ProfileError("NUIF_CANVA_LIVE_KIND", `Live import does not support ${element.kind}`);
}

function colorToHex(color: SolidColor): string {
  const byte = (value: number): string => Math.round(value * 255).toString(16).padStart(2, "0");
  return `#${byte(color.red)}${byte(color.green)}${byte(color.blue)}`;
}
