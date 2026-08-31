import assert from "node:assert/strict";
import test from "node:test";
import { ProfileError, assertMutationPlan, assertSnapshot, utf8Length } from "../src/protocol";
import { fixtureSnapshot, losslessPlanReport } from "./fixtures";

test("accepts the exact Rust mutation-plan envelope", () => {
  const snapshot = fixtureSnapshot();
  const plan = assertMutationPlan({
    schema_version: 1,
    profile: "nuif-figma-plugin-snapshot-0",
    snapshot,
    report: losslessPlanReport()
  });
  assert.equal(plan.snapshot.root.children.length, 2);
});

test("rejects duplicate host IDs before host mutation", () => {
  const snapshot = fixtureSnapshot();
  snapshot.root.children[0]!.id = snapshot.root.id;
  assert.throws(
    () => assertSnapshot(snapshot),
    (error: unknown) => error instanceof ProfileError && error.code === "NUIF_FIGMA_DUPLICATE_HOST_ID"
  );
});

test("rejects lossy mutation plans", () => {
  const snapshot = fixtureSnapshot();
  snapshot.root.visible = false;
  assert.throws(
    () =>
      assertMutationPlan({
        schema_version: 1,
        profile: "nuif-figma-plugin-snapshot-0",
        snapshot,
        report: losslessPlanReport()
      }),
    (error: unknown) => error instanceof ProfileError && error.code === "NUIF_FIGMA_LOSSY_PLAN"
  );
});

test("rejects a plan whose report is not entirely lossless", () => {
  const report = losslessPlanReport() as { fidelity: Array<{ status: { class: string } }> };
  report.fidelity[0]!.status.class = "unsupported";
  assert.throws(
    () =>
      assertMutationPlan({
        schema_version: 1,
        profile: "nuif-figma-plugin-snapshot-0",
        snapshot: fixtureSnapshot(),
        report
      }),
    (error: unknown) => error instanceof ProfileError && error.code === "NUIF_FIGMA_LOSSY_REPORT"
  );
});

test("rejects dimensions below the Figma resize minimum", () => {
  const snapshot = fixtureSnapshot();
  snapshot.root.children[0]!.width = 0;
  assert.throws(
    () => assertSnapshot(snapshot),
    (error: unknown) => error instanceof ProfileError && error.code === "NUIF_FIGMA_NODE_SIZE"
  );
});

test("counts UTF-8 bytes without relying on browser-only globals", () => {
  assert.equal(utf8Length("Ahem"), 4);
  assert.equal(utf8Length("å"), 2);
  assert.equal(utf8Length("🎨"), 4);
});

test("applies the combined-string limit to UTF-8 bytes and portable IDs", () => {
  const snapshot = fixtureSnapshot();
  snapshot.root.name = "å".repeat(131_072);
  assert.throws(
    () => assertSnapshot(snapshot),
    (error: unknown) => error instanceof ProfileError && error.code === "NUIF_FIGMA_STRING_LIMIT"
  );
});
