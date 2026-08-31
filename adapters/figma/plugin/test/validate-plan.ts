import { readFile, writeFile } from "node:fs/promises";
import { assertMutationPlan, summarizePlanReport } from "../src/protocol";

const [input, output] = process.argv.slice(2);
if (input === undefined || output === undefined) throw new Error("missing plan validation paths");
const plan = assertMutationPlan(JSON.parse(await readFile(input, "utf8")) as unknown);
const summary = summarizePlanReport(plan.report);
let nodes = 0;
const pending = [plan.snapshot.root];
while (pending.length !== 0) {
  const node = pending.pop();
  if (node === undefined) break;
  nodes += 1;
  pending.push(...node.children);
}
await writeFile(
  output,
  `${JSON.stringify(
    {
      schema_version: 1,
      status: "passed",
      profile: plan.profile,
      nodes,
      fidelity_entries: summary.fidelityEntries,
      correspondences: summary.correspondences
    },
    null,
    2
  )}\n`
);
