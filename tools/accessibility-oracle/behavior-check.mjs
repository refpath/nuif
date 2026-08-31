import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { pathToFileURL } from "node:url";
import { chromium, firefox, webkit } from "playwright";

const [htmlPath, expectedPath, staticReportPath, reportPath] = process.argv
  .slice(2);
if (!htmlPath || !expectedPath || !staticReportPath || !reportPath) {
  throw new Error(
    "usage: node behavior-check.mjs <fixture.html> <expected.json> <static-report.json> <report.json>",
  );
}

const expected = JSON.parse(await fs.readFile(expectedPath, "utf8"));
const staticReport = JSON.parse(await fs.readFile(staticReportPath, "utf8"));
if (
  expected.profile !== "nuif-web-behavior-0" || staticReport.status !== "passed"
) {
  throw new Error(
    "web behavior oracle inputs do not identify a passing bounded profile",
  );
}
if (expected.events.length !== expected.expected.traces.length) {
  throw new Error(
    "web behavior oracle event and reference-trace counts differ",
  );
}

const engines = [
  ["chromium", chromium],
  ["firefox", firefox],
  ["webkit", webkit],
];
const modalities = ["pointer", "keyboard"];
const fixtureUrl = pathToFileURL(path.resolve(htmlPath)).href;
const visibilityTargets = new Set(
  expected.expected.traces.flatMap((trace) =>
    trace.effects
      .filter((effect) => effect.kind === "visibility")
      .map((effect) => effect.target)
  ),
);
const results = [];

for (const [engine, browserType] of engines) {
  const browser = await browserType.launch({ headless: true });
  try {
    for (const modality of modalities) {
      const page = await browser.newPage({ locale: "en-US" });
      const runtimeErrors = [];
      page.on("pageerror", (error) => runtimeErrors.push(error.message));
      page.on("console", (message) => {
        if (message.type() === "error") runtimeErrors.push(message.text());
      });
      await page.goto(fixtureUrl, { waitUntil: "load" });
      const mismatches = [];
      const observations = [];
      const initialProfile = await page.locator("html").getAttribute(
        "data-nuif-behavior-profile",
      );
      const initialState = await page.locator("html").getAttribute(
        "data-nuif-behavior-state",
      );
      if (
        initialProfile !== expected.profile ||
        initialState !== expected.expected.traces[0].from_state
      ) {
        mismatches.push({
          category: "runtime-initialization",
          expected_profile: expected.profile,
          observed_profile: initialProfile,
          expected_state: expected.expected.traces[0].from_state,
          observed_state: initialState,
        });
      }
      if ((await page.getByRole("status").count()) !== 1) {
        mismatches.push({ category: "status-live-region" });
      }
      const visible = new Map();
      for (const target of visibilityTargets) visible.set(target, true);
      let announcement = "";
      let announcementTarget = null;

      for (let index = 0; index < expected.events.length; index += 1) {
        const event = expected.events[index];
        const trace = expected.expected.traces[index];
        if (event.kind !== "activate") {
          mismatches.push({
            index,
            category: "unsupported-fixture-event",
            kind: event.kind,
          });
          continue;
        }
        const source = page.locator(`[data-nuif-id="${event.source}"]`);
        if ((await source.count()) !== 1) {
          mismatches.push({
            index,
            category: "missing-or-duplicate-source",
            source: event.source,
          });
          continue;
        }
        if (modality === "pointer") {
          await source.click();
        } else {
          await source.press(index % 2 === 0 ? "Enter" : "Space");
        }
        for (const effect of trace.effects) {
          if (effect.kind === "visibility") {
            visible.set(effect.target, effect.value.value);
          } else if (effect.kind === "announcement") {
            announcement = effect.value.value;
            announcementTarget = effect.target;
          }
        }
        const observedState = await page.locator("html").getAttribute(
          "data-nuif-behavior-state",
        );
        const observedTransition = await page
          .locator("html")
          .getAttribute("data-nuif-behavior-last-transition");
        if (
          observedState !== trace.to_state ||
          observedTransition !== (trace.transition ?? "")
        ) {
          mismatches.push({
            index,
            category: "transition-or-state",
            expected_transition: trace.transition,
            observed_transition: observedTransition,
            expected_state: trace.to_state,
            observed_state: observedState,
          });
        }
        const visibility = [];
        for (const [target, expectedVisible] of visible) {
          const targetNode = page.locator(`[data-nuif-id="${target}"]`);
          const observedVisible = await targetNode.evaluate((node) =>
            !node.hidden
          );
          visibility.push({
            target,
            expected: expectedVisible,
            observed: observedVisible,
          });
          if (observedVisible !== expectedVisible) {
            mismatches.push({
              index,
              category: "visibility-effect",
              target,
              expected: expectedVisible,
              observed: observedVisible,
            });
          }
        }
        const status = page.getByRole("status");
        const observedAnnouncement = await status.textContent();
        const observedAnnouncementTarget = await status.getAttribute(
          "data-nuif-announcement-target",
        );
        if (
          observedAnnouncement !== announcement ||
          observedAnnouncementTarget !== announcementTarget
        ) {
          mismatches.push({
            index,
            category: "announcement-effect",
            expected: announcement,
            observed: observedAnnouncement,
            expected_target: announcementTarget,
            observed_target: observedAnnouncementTarget,
          });
        }
        observations.push({
          index,
          source: event.source,
          transition: observedTransition,
          state: observedState,
          visibility,
          announcement: observedAnnouncement,
          announcement_target: observedAnnouncementTarget,
          status_aria_snapshot: await status.ariaSnapshot(),
        });
      }
      if (runtimeErrors.length > 0) {
        mismatches.push({
          category: "browser-runtime-errors",
          errors: runtimeErrors,
        });
      }
      results.push({
        engine,
        modality,
        version: browser.version(),
        status: mismatches.length === 0 ? "passed" : "failed",
        events: expected.events.length,
        body_aria_snapshot: await page.locator("body").ariaSnapshot(),
        observations,
        runtime_errors: runtimeErrors,
        mismatches,
      });
      await page.close();
    }
  } finally {
    await browser.close();
  }
}

const baseline = JSON.stringify(results[0]?.observations);
const hostDifferences = results
  .slice(1)
  .filter((result) => JSON.stringify(result.observations) !== baseline)
  .map((result) => ({
    category: "host-observation-difference",
    baseline_engine: results[0].engine,
    baseline_modality: results[0].modality,
    compared_engine: result.engine,
    compared_modality: result.modality,
  }));
const passed = results.length === engines.length * modalities.length &&
  results.every((result) => result.status === "passed");
const report = {
  schema_version: 1,
  experiment: "nuif:experiment:web-behavior-mapping",
  status: passed ? "passed" : "failed",
  profile: expected.profile,
  source_profile: expected.source_profile,
  oracle: {
    name: "Playwright",
    version: "1.62.1",
    authority: "foreign-browser-native-activation-dom-and-aria-observation",
    operating_system: `${os.platform()} ${os.release()}`,
    architecture: os.arch(),
    node: process.version,
  },
  fixture: {
    html: htmlPath,
    expected: expectedPath,
    static_report: staticReportPath,
    events: expected.events.length,
    event_sources: expected.event_sources,
    modalities,
  },
  required_subset: [
    "native enabled button and switch pointer plus alternating Enter/Space keyboard activation",
    "ordered guarded transition and state selection",
    "visibility effects through the HTML hidden property",
    "announcement text and stable target attribution through one polite status region",
    "exact CSP-hash-authorized finite runtime without browser errors",
  ],
  engines: results,
  host_differences: hostDifferences,
  non_claims: [
    "Playwright browsers are pinned engines rather than branded browser releases",
    "ARIA status-tree exposure does not prove screen-reader speech or announcement timing",
    "checkbox radio disabled-control focus navigation timers and animation are outside the profile",
    "the adapter is one-way and does not import behavior from HTML or JavaScript",
  ],
};
await fs.mkdir(path.dirname(reportPath), { recursive: true });
await fs.writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`);
if (!passed) {
  throw new Error(`web behavior oracle failed; inspect ${reportPath}`);
}
console.log(
  `web behavior oracle: ${engines.length} engines, ${modalities.length} modalities, ${expected.events.length} events, status passed`,
);
