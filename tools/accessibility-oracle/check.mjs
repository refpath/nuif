import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { pathToFileURL } from "node:url";
import { chromium, firefox, webkit } from "playwright";

const [htmlPath, expectedPath, staticReportPath, reportPath] = process.argv.slice(2);
if (!htmlPath || !expectedPath || !staticReportPath || !reportPath) {
  throw new Error(
    "usage: node check.mjs <fixture.html> <expected.json> <static-report.json> <report.json>",
  );
}

const expected = JSON.parse(await fs.readFile(expectedPath, "utf8"));
const staticReport = JSON.parse(await fs.readFile(staticReportPath, "utf8"));
if (expected.profile !== "nuif-web-accessibility-0" || staticReport.status !== "passed") {
  throw new Error("accessibility oracle inputs do not identify a passing bounded profile");
}

const engines = [
  ["chromium", chromium],
  ["firefox", firefox],
  ["webkit", webkit],
];
const relationshipAttributes = {
  "labelled-by": "aria-labelledby",
  "described-by": "aria-describedby",
  controls: "aria-controls",
  owns: "aria-owns",
  "flow-to": "aria-flowto",
};
const fixtureUrl = pathToFileURL(path.resolve(htmlPath)).href;
const results = [];

for (const [engine, browserType] of engines) {
  const browser = await browserType.launch({ headless: true });
  try {
    const page = await browser.newPage({ locale: "en-US" });
    await page.goto(fixtureUrl, { waitUntil: "load" });
    const nodes = [];
    const mismatches = [];
    for (const node of expected.nodes) {
      const selector = `[data-nuif-id="${node.entity}"]`;
      const element = page.locator(selector);
      if ((await element.count()) !== 1) {
        mismatches.push({
          entity: node.entity,
          category: "missing-or-duplicate-dom-node",
        });
        continue;
      }
      const baseOptions = node.accessible_name === null
        ? {}
        : { name: node.accessible_name, exact: true };
      const baseRoleCount = await page
        .getByRole(node.role, baseOptions)
        .and(element)
        .count();
      if (baseRoleCount !== 1) {
        mismatches.push({
          entity: node.entity,
          category: "computed-role-or-name",
          expected_role: node.role,
          expected_name: node.accessible_name,
        });
      }
      const stateOptions = { ...baseOptions };
      for (const state of ["checked", "disabled", "expanded", "pressed", "selected"]) {
        if (Object.hasOwn(node.states, state)) {
          stateOptions[state] = node.states[state];
        }
      }
      const stateRoleCount = await page
        .getByRole(node.role, stateOptions)
        .and(element)
        .count();
      if (baseRoleCount === 1 && stateRoleCount !== 1) {
        mismatches.push({
          entity: node.entity,
          category: "computed-state",
          expected_states: node.states,
        });
      }
      if (Object.hasOwn(node.states, "required")) {
        const required = await element.evaluate((candidate) => candidate.matches(":required"));
        if (required !== node.states.required) {
          mismatches.push({
            entity: node.entity,
            category: "native-required-state",
            expected: node.states.required,
            observed: required,
          });
        }
      }
      for (const [kind, targets] of Object.entries(node.relationships)) {
        const attribute = relationshipAttributes[kind];
        const expectedValue = targets.map((target) => `nuif-${target}`).join(" ");
        const observedValue = await element.getAttribute(attribute);
        if (observedValue !== expectedValue) {
          mismatches.push({
            entity: node.entity,
            category: "relationship-idref",
            relationship: kind,
            expected: expectedValue,
            observed: observedValue,
          });
        }
      }
      const ariaSnapshot = await element.ariaSnapshot();
      for (const targetId of node.relationships.owns ?? []) {
        const target = expected.nodes.find((candidate) => candidate.entity === targetId);
        const targetSignature = target.accessible_name === null
          ? `- ${target.role}`
          : `${target.role} "${target.accessible_name}"`;
        if (!ariaSnapshot.includes(targetSignature)) {
          mismatches.push({
            entity: node.entity,
            category: "computed-owns-tree",
            target: targetId,
          });
        }
      }
      nodes.push({
        entity: node.entity,
        aria_snapshot: ariaSnapshot,
      });
    }
    results.push({
      engine,
      version: browser.version(),
      status: mismatches.length === 0 ? "passed" : "failed",
      required_nodes: expected.nodes.length,
      observed_nodes: nodes.length,
      body_aria_snapshot: await page.locator("body").ariaSnapshot(),
      nodes,
      mismatches,
    });
  } finally {
    await browser.close();
  }
}

const baseline = results[0]?.body_aria_snapshot;
const hostDifferences = results
  .filter((result) => result.body_aria_snapshot !== baseline)
  .map((result) => ({
    category: "host-tree-difference",
    baseline_engine: results[0].engine,
    compared_engine: result.engine,
  }));
const passed = results.length === engines.length
  && results.every((result) => result.status === "passed");
const report = {
  schema_version: 1,
  experiment: "nuif:experiment:accessibility-mapping",
  status: passed ? "passed" : "failed",
  profile: expected.profile,
  oracle: {
    name: "Playwright",
    version: "1.62.1",
    authority: "foreign-browser-role-name-state-computation",
    operating_system: `${os.platform()} ${os.release()}`,
    architecture: os.arch(),
    node: process.version,
  },
  fixture: {
    html: htmlPath,
    expected: expectedPath,
    static_report: staticReportPath,
    semantic_nodes: expected.nodes.length,
  },
  required_subset: [
    "computed role",
    "accessible name including labelled-by",
    "checked disabled expanded pressed and required state",
    "stable relationship IDREFs and owned-tree exposure",
  ],
  engines: results,
  host_differences: hostDifferences,
  classification: {
    semantic_loss: results.flatMap((result) =>
      result.mismatches.map((mismatch) => ({ engine: result.engine, ...mismatch }))),
    host_tree_difference: hostDifferences,
  },
  non_claims: [
    "Playwright browsers are pinned test engines, not branded Chrome Firefox or Safari releases",
    "ARIA-tree agreement does not establish keyboard interaction or application behavior",
    "native macOS Windows Linux Android and iOS accessibility APIs are not compared",
  ],
};
await fs.mkdir(path.dirname(reportPath), { recursive: true });
await fs.writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`);
if (!passed) {
  throw new Error(`accessibility oracle failed; inspect ${reportPath}`);
}
console.log(
  `accessibility oracle: ${results.length} engines, ${expected.nodes.length} nodes, status passed`,
);
