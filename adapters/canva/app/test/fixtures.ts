import type { CanvaElement, CanvaMutationPlan, SolidColor } from "../src/protocol";

export const blue: SolidColor = { red: 0.2, green: 0.4, blue: 0.8, alpha: 1 };

export function rectangle(overrides: Partial<CanvaElement> = {}): CanvaElement {
  return {
    id: "nuif:00000000000000000000000000000020",
    kind: "rectangle",
    name: null,
    visible: true,
    locked: false,
    opacity: 1,
    rotation: 0,
    x: 16,
    y: 16,
    width: 160,
    height: 80,
    fill: blue,
    text: null,
    unsupported_properties: [],
    children: [],
    ...overrides
  };
}

export function mutationPlan(elements: CanvaElement[] = [rectangle()]): CanvaMutationPlan {
  return {
    schema_version: 1,
    profile: "nuif-canva-design-editing-0",
    page: {
      schema_version: 1,
      host_application_version: "2.12.0",
      host_api_version: "2",
      host_document_id: "nuif-doc:00000000000000000000000000000001",
      host_document_revision: null,
      page_id: "nuif-page:00000000000000000000000000000010",
      page_name: null,
      width: 320,
      height: 200,
      background: { red: 1, green: 1, blue: 1, alpha: 1 },
      elements
    },
    report: {
      schema_version: 1,
      profile: "nuif-canva-design-editing-0",
      direction: "export",
      host_application: "Canva Design 2.12.0",
      host_api_version: "2",
      host_document_revision: null,
      canonical_hash: `nuif-cbor-0:sha256:${"a".repeat(64)}`,
      fidelity: elements.map((element) => ({
        target: { kind: "entity", id: element.id.slice(5) },
        pointer: `/entities/${element.id.slice(5)}`,
        status: { class: "lossless" }
      })),
      correspondences: elements.map((element) => ({
        target: { kind: "entity", id: element.id.slice(5) },
        host_object_id: element.id,
        host_property: null
      })),
      unmapped_host_data_preserved: false
    }
  };
}
