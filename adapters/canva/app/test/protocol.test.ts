import assert from "node:assert/strict";
import test from "node:test";
import {
  MAX_ELEMENTS,
  PINNED_FONT_NAME,
  PINNED_FONT_SHA256,
  ProfileError,
  assertMutationPlan,
  summarizePlanReport,
  type CanvaElement
} from "../src/protocol";
import { mutationPlan, rectangle } from "./fixtures";

test("accepts the exact lossless Rust envelope", () => {
  const plan = assertMutationPlan(mutationPlan());
  assert.equal(plan.profile, "nuif-canva-design-editing-0");
  assert.deepEqual(summarizePlanReport(plan.report), { fidelityEntries: 1, correspondences: 1 });
});

test("rejects unknown fields and duplicate host IDs", () => {
  const unknown = mutationPlan() as unknown as Record<string, unknown>;
  unknown.extra = true;
  assert.throws(() => assertMutationPlan(unknown), hasCode("NUIF_CANVA_UNKNOWN_FIELD"));

  const duplicate = mutationPlan([rectangle(), rectangle()]);
  assert.throws(() => assertMutationPlan(duplicate), hasCode("NUIF_CANVA_DUPLICATE_HOST_ID"));
});

test("rejects forged lossless and malformed report claims", () => {
  const lossy = mutationPlan();
  const report = lossy.report as { fidelity: { status: { class: string; reason?: string } }[] };
  report.fidelity[0]!.status = { class: "unsupported", reason: "loss" };
  assert.throws(() => assertMutationPlan(lossy), hasCode("NUIF_CANVA_UNKNOWN_FIELD"));

  const hash = mutationPlan();
  (hash.report as { canonical_hash: string }).canonical_hash = "sha256:no";
  assert.throws(() => assertMutationPlan(hash), hasCode("NUIF_CANVA_CANONICAL_HASH"));
});

test("enforces element count, depth, text, and numeric bounds", () => {
  const excessive = mutationPlan(
    Array.from({ length: MAX_ELEMENTS + 1 }, (_, index) =>
      rectangle({ id: `host-${index.toString().padStart(5, "0")}` })
    )
  );
  assert.throws(() => assertMutationPlan(excessive), hasCode("NUIF_CANVA_ELEMENT_LIMIT"));

  const invalid = mutationPlan([rectangle({ opacity: Number.NaN })]);
  assert.throws(() => assertMutationPlan(invalid), hasCode("NUIF_CANVA_NUMBER"));

  let nested = rectangle({ id: "depth-64" });
  for (let depth = 63; depth >= 0; depth -= 1) {
    nested = rectangle({ id: `depth-${depth}`, kind: "group", fill: null, children: [nested] });
  }
  assert.throws(() => assertMutationPlan(mutationPlan([nested])), hasCode("NUIF_CANVA_DEPTH_LIMIT"));

  const text: CanvaElement = rectangle({
    id: "text-limit",
    kind: "text",
    fill: null,
    text: {
      characters: "x".repeat(4_097),
      font_family: PINNED_FONT_NAME,
      font_sha256: PINNED_FONT_SHA256,
      font_size: 16,
      line_height: 20
    }
  });
  assert.throws(() => assertMutationPlan(mutationPlan([text])), hasCode("NUIF_CANVA_TEXT_LIMIT"));
});

function hasCode(code: string): (error: unknown) => boolean {
  return (error) => error instanceof ProfileError && error.code === code;
}
