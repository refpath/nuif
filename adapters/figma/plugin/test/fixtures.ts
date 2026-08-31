import { normalizeSelection } from "../src/normalize";
import type { PluginSnapshot } from "../src/protocol";

type MockValues = Record<string, unknown>;

export function mockNode(type: string, values: MockValues = {}): SceneNode {
  const shared = (values.shared ?? {}) as Record<string, string>;
  const common: MockValues = {
    type,
    id: values.id ?? "2:1",
    name: values.name ?? type,
    visible: true,
    opacity: 1,
    x: 16,
    y: 24,
    width: 80,
    height: 40,
    rotation: 0,
    locked: false,
    isMask: false,
    constrainProportions: false,
    relativeTransform: [[1, 0, values.x ?? 16], [0, 1, values.y ?? 24]],
    blendMode: type === "FRAME" || type === "GROUP" ? "PASS_THROUGH" : "NORMAL",
    fills: [],
    fillStyleId: "",
    strokes: [],
    effects: [],
    boundVariables: {},
    clipsContent: false,
    cornerRadius: 0,
    reactions: [],
    constraints: { horizontal: "MIN", vertical: "MIN" },
    layoutGrow: 0,
    layoutAlign: "INHERIT",
    layoutPositioning: "AUTO",
    children: [],
    getSharedPluginData: (namespace: string, key: string) => (namespace === "nuif" ? (shared[key] ?? "") : "")
  };
  if (type === "FRAME") {
    Object.assign(common, {
      layoutMode: "NONE",
      layoutWrap: "NO_WRAP",
      primaryAxisSizingMode: "FIXED",
      counterAxisSizingMode: "FIXED",
      primaryAxisAlignItems: "MIN",
      counterAxisAlignItems: "MIN",
      itemReverseZIndex: false,
      strokesIncludedInLayout: false,
      itemSpacing: 0,
      paddingTop: 0,
      paddingRight: 0,
      paddingBottom: 0,
      paddingLeft: 0
    });
  }
  if (type === "ELLIPSE") {
    Object.assign(common, { arcData: { startingAngle: 0, endingAngle: Math.PI * 2, innerRadius: 0 } });
  }
  if (type === "TEXT") {
    Object.assign(common, {
      characters: "NUIF",
      fontName: { family: "Ahem", style: "Regular" },
      fontSize: 16,
      lineHeight: { unit: "PIXELS", value: 20 },
      textAlignHorizontal: "LEFT",
      textAlignVertical: "TOP",
      textAutoResize: "NONE",
      textTruncation: "DISABLED",
      maxLines: null,
      textCase: "ORIGINAL",
      textDecoration: "NONE",
      letterSpacing: { unit: "PIXELS", value: 0 },
      paragraphIndent: 0,
      paragraphSpacing: 0,
      listSpacing: 0,
      hangingPunctuation: false,
      hangingList: false,
      hyperlink: null,
      textStyleId: "",
      getRangeListOptions: () => ({ type: "NONE" }),
      getRangeIndentation: () => 0
    });
  }
  return Object.assign(common, values) as unknown as SceneNode;
}

export function fixtureSnapshot(): PluginSnapshot {
  const rectangle = mockNode("RECTANGLE", {
    id: "2:2",
    name: "Swatch",
    x: 20,
    y: 20,
    width: 120,
    height: 64,
    fills: [{ type: "SOLID", color: { r: 0.1, g: 0.2, b: 0.9 }, opacity: 0.75 }],
    shared: { entity_id: "00000000-0000-0000-0000-000000000003" }
  });
  const frame = mockNode("FRAME", {
    id: "2:1",
    name: "NUIF fixture",
    x: 480,
    y: 240,
    width: 320,
    height: 180,
    children: [
      rectangle,
      mockNode("TEXT", {
        id: "2:3",
        name: "Label",
        x: 20,
        y: 108,
        width: 120,
        height: 24,
        fills: [{ type: "SOLID", color: { r: 0, g: 0, b: 0 }, opacity: 1 }],
        shared: {
          entity_id: "00000000-0000-0000-0000-000000000004",
          font_sha256: "f0a92cd0cc45735591c9b5b1fa8aecd5194e8dc518895ca22af94a46c23550dc"
        }
      })
    ],
    fills: [{ type: "SOLID", color: { r: 1, g: 1, b: 1 }, opacity: 1 }],
    shared: {
      document_id: "00000000-0000-0000-0000-000000000001",
      entity_id: "00000000-0000-0000-0000-000000000002"
    }
  });
  return normalizeSelection(frame, {
    apiVersion: "1.0.0",
    fileKey: "credential-free-fixture",
    pageId: "1:0",
    pageName: "Fixture",
    documentId: "00000000-0000-0000-0000-000000000001",
    hostApplicationVersion: "unreported-by-plugin-api"
  });
}

export function losslessPlanReport(): unknown {
  return {
    schema_version: 1,
    profile: "nuif-figma-plugin-snapshot-0",
    direction: "import",
    host_application: "Figma Design fixture",
    host_api_version: "1.0.0",
    host_document_revision: null,
    canonical_hash: `nuif-cbor-0:sha256:${"0".repeat(64)}`,
    fidelity: [
      {
        target: { kind: "document", id: "00000000-0000-0000-0000-000000000001" },
        pointer: "/identity",
        status: { class: "lossless" }
      }
    ],
    correspondences: [],
    unmapped_host_data_preserved: true
  };
}
