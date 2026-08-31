import assert from "node:assert/strict";
import test from "node:test";
import { normalizeSelection } from "../src/normalize";
import { ProfileError, assertSnapshot } from "../src/protocol";
import { fixtureSnapshot, mockNode } from "./fixtures";

test("normalizes the exact frame subset deterministically", () => {
  const first = fixtureSnapshot();
  const second = fixtureSnapshot();
  assert.deepEqual(first, second);
  assert.equal(first.root.kind, "FRAME");
  assert.equal(first.root.children[0]?.kind, "RECTANGLE");
  assert.equal(first.root.children[1]?.text?.font_sha256, "f0a92cd0cc45735591c9b5b1fa8aecd5194e8dc518895ca22af94a46c23550dc");
  assert.deepEqual(first.root.children[0]?.fill, { red: 0.1, green: 0.2, blue: 0.9, alpha: 0.75 });
  assert.equal(first.nuif_document_id, "00000000-0000-0000-0000-000000000001");
  assert.deepEqual(assertSnapshot(first), first);
});

test("reports active host-only properties instead of silently flattening them", () => {
  const child = mockNode("RECTANGLE", {
    id: "3:2",
    effects: [{ type: "DROP_SHADOW" }],
    rotation: 15,
    boundVariables: { width: { type: "VARIABLE_ALIAS", id: "VariableID:1" } }
  });
  const root = mockNode("FRAME", { id: "3:1", children: [child] });
  const snapshot = normalizeSelection(root, {
    apiVersion: "1.0.0",
    pageId: "1:0",
    pageName: "Page"
  });
  assert.deepEqual(snapshot.root.children[0]?.unsupported_properties, ["boundVariables", "effects", "rotation"]);
});

test("fails closed for node kinds outside the subset", () => {
  const vector = mockNode("VECTOR", { id: "4:2" });
  const root = mockNode("FRAME", { id: "4:1", children: [vector] });
  assert.throws(
    () =>
      normalizeSelection(root, {
        apiVersion: "1.0.0",
        pageId: "1:0",
        pageName: "Page"
      }),
    (error: unknown) => error instanceof ProfileError && error.code === "NUIF_FIGMA_NODE_KIND"
  );
});

test("attributes affine and partial-ellipse semantics that basic geometry cannot carry", () => {
  const ellipse = mockNode("ELLIPSE", {
    id: "5:2",
    relativeTransform: [[1, 0.25, 10], [0, 1, 20]],
    arcData: { startingAngle: 0, endingAngle: Math.PI, innerRadius: 0.5 }
  });
  const root = mockNode("FRAME", { id: "5:1", children: [ellipse] });
  const snapshot = normalizeSelection(root, { apiVersion: "1.0.0", pageId: "1:0", pageName: "Page" });
  assert.deepEqual(snapshot.root.children[0]?.unsupported_properties, ["arcData", "relativeTransform"]);
});
