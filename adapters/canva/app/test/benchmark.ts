import { writeFile } from "node:fs/promises";
import { performance } from "node:perf_hooks";
import process from "node:process";
import type { DesignEditing } from "@canva/design";
import { applyPlanToSession, preflightHostImport } from "../src/host";
import { ProfileError, assertMutationPlan, type CanvaElement, type CanvaMutationPlan } from "../src/protocol";
import { mutationPlan, rectangle } from "./fixtures";

const output = process.argv[2];
if (output === undefined) throw new Error("missing benchmark output path");

const sizes = [1, 128, 1_024, 4_096, 16_384];
const parseMeasurements = [];
const preflightMeasurements = [];
for (const size of sizes) {
  const plan = sizedPlan(size);
  const bytes = JSON.stringify(plan);
  const iterations = size <= 128 ? 40 : size <= 1_024 ? 12 : size <= 4_096 ? 4 : 2;
  parseMeasurements.push(
    measure(`parse_validate_${size}`, iterations, () => assertMutationPlan(JSON.parse(bytes) as unknown), bytes.length)
  );
  preflightMeasurements.push(
    measure(`host_preflight_${size}`, iterations, () => preflightHostImport(plan, emptyPage()), bytes.length)
  );
}

const applyPlan = sizedPlan(1_024);
const applyDurations: number[] = [];
for (let iteration = 0; iteration < 5; iteration += 1) {
  const session = mockSession();
  const started = performance.now();
  const result = await applyPlanToSession(applyPlan, session.value);
  applyDurations.push(performance.now() - started);
  if (result.elementsCreated !== 1_024 || result.syncs !== 1 || session.syncs() !== 1) {
    throw new Error("mock apply benchmark violated the one-sync transaction contract");
  }
}

const hostilePlan = sizedPlan(128);
hostilePlan.page.elements[127]!.id = hostilePlan.page.elements[0]!.id;
let hostileRejected = 0;
const hostile = measure(
  "duplicate_id_rejection_128",
  40,
  () => {
    try {
      assertMutationPlan(hostilePlan);
    } catch (error) {
      if (error instanceof ProfileError && error.code === "NUIF_CANVA_DUPLICATE_HOST_ID") {
        hostileRejected += 1;
        return;
      }
      throw error;
    }
    throw new Error("duplicate host ID was accepted");
  },
  JSON.stringify(hostilePlan).length
);
const hostileMeasuredRejections = hostileRejected - 3;
if (hostileMeasuredRejections !== 40) throw new Error("hostile benchmark did not reject every measured plan");

const report = {
  schema_version: 1,
  status: "passed",
  profile: "nuif-canva-design-editing-0",
  scope: "local JavaScript protocol, host preflight, and mock transaction timing; no live Canva latency",
  environment: {
    node: process.version,
    platform: process.platform,
    architecture: process.arch
  },
  thresholds: "informational until repeated CI and live-host calibration establish budgets",
  parse_validate: parseMeasurements,
  host_preflight: preflightMeasurements,
  mock_apply_1024: summarize("mock_apply_1024", applyDurations, JSON.stringify(applyPlan).length),
  hostile_rejection: { ...hostile, rejected: hostileMeasuredRejections, expected: 40 },
  memory: {
    heap_used_bytes_after_suite: process.memoryUsage().heapUsed,
    rss_bytes_after_suite: process.memoryUsage().rss
  },
  live_host: { status: "not_run", required_for_session_latency_budget: true }
};
await writeFile(output, `${JSON.stringify(report, null, 2)}\n`);

function sizedPlan(size: number): CanvaMutationPlan {
  const elements = Array.from({ length: size }, (_, index) => {
    const raw = (index + 0x20).toString(16).padStart(32, "0");
    return rectangle({ id: `nuif:${raw}`, x: index % 320, y: Math.floor(index / 320) });
  });
  return mutationPlan(elements);
}

function measure(
  name: string,
  iterations: number,
  operation: () => unknown,
  inputBytes: number
): ReturnType<typeof summarize> {
  for (let warmup = 0; warmup < Math.min(iterations, 3); warmup += 1) operation();
  const durations = [];
  for (let iteration = 0; iteration < iterations; iteration += 1) {
    const started = performance.now();
    operation();
    durations.push(performance.now() - started);
  }
  return summarize(name, durations, inputBytes);
}

function summarize(name: string, durations: number[], inputBytes: number) {
  const sorted = [...durations].sort((left, right) => left - right);
  const total = durations.reduce((sum, duration) => sum + duration, 0);
  return {
    name,
    input_bytes: inputBytes,
    iterations: durations.length,
    median_ms: percentile(sorted, 0.5),
    p95_ms: percentile(sorted, 0.95),
    max_ms: sorted.at(-1) ?? 0,
    operations_per_second: total === 0 ? null : (durations.length * 1_000) / total
  };
}

function percentile(sorted: number[], quantile: number): number {
  return sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * quantile))] ?? 0;
}

function emptyPage(): DesignEditing.Page {
  return pageWithInsert(() => undefined);
}

function mockSession() {
  let syncCount = 0;
  const value = {
    page: pageWithInsert((_state) => ({})),
    helpers: {
      elementStateBuilder: {
        createRectElement: (opts: unknown) => opts,
        createShapeElement: (opts: unknown) => opts
      }
    },
    sync: async () => {
      syncCount += 1;
    }
  } as unknown as DesignEditing.CurrentPageSession;
  return { value, syncs: () => syncCount };
}

function pageWithInsert(insert: (state: unknown) => unknown): DesignEditing.Page {
  const background = {
    mediaContainer: { ref: undefined, set: () => undefined },
    colorContainer: { ref: { type: "solid", color: "#ffffff" }, set: () => undefined }
  };
  return {
    type: "absolute",
    id: "benchmark-page",
    locked: false,
    dimensions: { width: 320, height: 200 },
    background,
    elements: {
      count: () => 0,
      toArray: () => [],
      forEach: () => undefined,
      filter: () => [],
      insertAfter: (_reference: unknown, state: unknown) => insert(state)
    }
  } as unknown as DesignEditing.Page;
}
