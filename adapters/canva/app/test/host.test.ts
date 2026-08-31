import assert from "node:assert/strict";
import test from "node:test";
import type { DesignEditing } from "@canva/design";
import { CANONICAL_ELLIPSE_PATH, applyPlanToSession, normalizeCurrentPage } from "../src/host";
import { ProfileError } from "../src/protocol";
import { blue, mutationPlan, rectangle } from "./fixtures";

test("applies preflighted geometry in order with exactly one sync", async () => {
  const plan = mutationPlan([
    rectangle(),
    rectangle({ id: "nuif:00000000000000000000000000000021", kind: "ellipse", fill: null, x: 200 })
  ]);
  const harness = hostHarness();
  const result = await applyPlanToSession(plan, harness.session);
  assert.deepEqual(result, { elementsCreated: 2, syncs: 1 });
  assert.equal(harness.syncs(), 1);
  assert.equal(harness.inserted.length, 2);
  assert.equal((harness.inserted[0] as { type: string }).type, "rect");
  assert.equal((harness.inserted[1] as { type: string; paths: { d: string }[] }).paths[0]!.d, CANONICAL_ELLIPSE_PATH);
  assert.equal(harness.backgroundColor(), "#ffffff");
});

test("rejects every narrowed host boundary before insertion or sync", async () => {
  const cases = [
    { plan: mutationPlan([rectangle({ name: "Named" })]), harness: hostHarness() },
    { plan: mutationPlan([rectangle({ kind: "group", children: [] })]), harness: hostHarness() },
    { plan: mutationPlan([rectangle({ fill: { ...blue, alpha: 0.5 } })]), harness: hostHarness() },
    { plan: mutationPlan(), harness: hostHarness({ existing: 1 }) },
    { plan: mutationPlan(), harness: hostHarness({ locked: true }) },
    { plan: mutationPlan(), harness: hostHarness({ unbounded: true }) }
  ];
  for (const item of cases) {
    await assert.rejects(() => applyPlanToSession(item.plan, item.harness.session), ProfileError);
    assert.equal(item.harness.inserted.length, 0);
    assert.equal(item.harness.syncs(), 0);
  }
});

test("normalizes the exact rectangle and canonical ellipse subset", () => {
  const rect = hostRect();
  const ellipse = hostShape();
  const page = hostPage({ sourceElements: [rect, ellipse] });
  const normalized = normalizeCurrentPage(page);
  assert.equal(normalized.page_name, null);
  assert.deepEqual(normalized.background, { red: 1, green: 1, blue: 1, alpha: 1 });
  assert.equal(normalized.elements[0]?.kind, "rectangle");
  assert.equal(normalized.elements[1]?.kind, "ellipse");
  assert.deepEqual(normalized.elements[0]?.fill, blue);
});

test("fails closed on text and noncanonical shapes", () => {
  const text = { ...hostCommon("text"), text: { readPlaintext: () => "hello" } };
  assert.throws(() => normalizeCurrentPage(hostPage({ sourceElements: [text] })), hasCode("NUIF_CANVA_TEXT_IDENTITY"));
  const shape = hostShape();
  shape.paths = readable([{ ...shape.paths.toArray()[0]!, d: "M 0 0 L 1 0 L 1 1 Z" }]);
  assert.throws(() => normalizeCurrentPage(hostPage({ sourceElements: [shape] })), hasCode("NUIF_CANVA_SHAPE"));
});

function hostHarness(options: { existing?: number; locked?: boolean; unbounded?: boolean } = {}) {
  const inserted: unknown[] = [];
  let syncCount = 0;
  let backgroundColor: string | undefined = "#ffffff";
  const sourceElements = Array.from({ length: options.existing ?? 0 }, () => hostRect());
  const page = hostPage({
    sourceElements,
    locked: options.locked,
    unbounded: options.unbounded,
    insert: (state) => {
      inserted.push(state);
      return state;
    },
    setBackground: (color) => {
      backgroundColor = color;
    }
  });
  const builder = {
    createRectElement: (opts: unknown) => ({ type: "rect", ...(opts as object) }),
    createShapeElement: (opts: unknown) => ({ type: "shape", ...(opts as object) })
  };
  const session = {
    page,
    helpers: { elementStateBuilder: builder },
    sync: async () => {
      syncCount += 1;
    }
  } as unknown as DesignEditing.CurrentPageSession;
  return { session, inserted, syncs: () => syncCount, backgroundColor: () => backgroundColor };
}

function hostPage(options: {
  sourceElements?: unknown[];
  locked?: boolean | undefined;
  unbounded?: boolean | undefined;
  insert?: (state: unknown) => unknown;
  setBackground?: (color: string | undefined) => void;
} = {}): DesignEditing.Page {
  const source = options.sourceElements ?? [];
  return {
    type: "absolute",
    id: "page-1",
    locked: options.locked ?? false,
    dimensions: options.unbounded === true ? undefined : { width: 320, height: 200 },
    background: {
      mediaContainer: { ref: undefined, set: () => undefined },
      colorContainer: {
        ref: { type: "solid", color: "#ffffff" },
        set: (state: { color: string } | undefined) => options.setBackground?.(state?.color)
      }
    },
    elements: {
      ...readable(source),
      insertAfter: (_reference: unknown, state: unknown) => options.insert?.(state) ?? state
    }
  } as unknown as DesignEditing.Page;
}

function hostRect(): Record<string, unknown> {
  return {
    ...hostCommon("rect"),
    fill: hostFill("#3366cc"),
    stroke: { weight: 0, colorContainer: { ref: { type: "solid", color: "#000000" }, set: () => undefined } }
  };
}

function hostShape(): Record<string, any> {
  return {
    ...hostCommon("shape"),
    viewBox: { top: 0, left: 0, width: 100, height: 100 },
    paths: readable([
      {
        d: CANONICAL_ELLIPSE_PATH,
        fill: {
          isMediaEditable: true,
          mediaContainer: { ref: undefined, set: () => undefined },
          colorContainer: { ref: { type: "solid", color: "#3366cc" }, set: () => undefined }
        },
        stroke: undefined
      }
    ])
  };
}

function hostCommon(type: string): Record<string, unknown> {
  return { type, locked: false, top: 16, left: 16, rotation: 0, transparency: 0, width: 160, height: 80 };
}

function hostFill(color: string) {
  return {
    mediaContainer: { ref: undefined, set: () => undefined },
    colorContainer: { ref: { type: "solid", color }, set: () => undefined }
  };
}

function readable<T>(values: T[]) {
  return {
    count: () => values.length,
    toArray: () => values,
    forEach: (callback: (value: T) => void) => values.forEach(callback),
    filter: (predicate: (value: T) => boolean) => values.filter(predicate)
  };
}

function hasCode(code: string): (error: unknown) => boolean {
  return (error) => error instanceof ProfileError && error.code === code;
}
