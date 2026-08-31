import { openDesign } from "@canva/design";
import { applyPlanToSession, normalizeCurrentPage, preflightHostImport } from "./host";
import { MAX_PLAN_BYTES, ProfileError, assertMutationPlan, summarizePlanReport, type CanvaMutationPlan } from "./protocol";

let loadedPlan: CanvaMutationPlan | null = null;

document.body.innerHTML = `
  <main>
    <header>
      <p class="eyebrow">NUIF review shell</p>
      <h1>Current page interchange</h1>
      <p>Export a bounded snapshot or review an exact geometry plan before applying it.</p>
    </header>
    <section aria-labelledby="export-title">
      <h2 id="export-title">Export</h2>
      <p>Supports unlocked, fixed pages containing the bounded rectangle, ellipse, and group subset.</p>
      <button id="export" type="button">Export current page</button>
    </section>
    <section aria-labelledby="import-title">
      <h2 id="import-title">Import</h2>
      <label class="file-label" for="plan">Choose NUIF Canva plan</label>
      <input id="plan" type="file" accept="application/json,.json" />
      <output id="summary" aria-live="polite">No plan loaded.</output>
      <label class="confirm"><input id="confirm" type="checkbox" disabled /> I reviewed this plan and want to apply it to the empty current page.</label>
      <button id="apply" type="button" disabled>Apply once</button>
    </section>
    <output id="status" class="status" aria-live="assertive"></output>
  </main>
`;

const style = document.createElement("style");
style.textContent = `
  :root { color-scheme: light; font-family: Canva Sans, ui-sans-serif, system-ui, sans-serif; color: #0d1216; background: #fff; }
  * { box-sizing: border-box; }
  body { margin: 0; }
  main { display: grid; gap: 16px; padding: 20px; max-width: 560px; }
  header, section { display: grid; gap: 10px; }
  section { border: 1px solid #d7d9db; border-radius: 10px; padding: 16px; }
  h1, h2, p { margin: 0; }
  h1 { font-size: 22px; line-height: 1.2; }
  h2 { font-size: 16px; }
  p, output, label { font-size: 14px; line-height: 1.45; }
  .eyebrow { color: #6b7175; font-size: 12px; font-weight: 700; letter-spacing: .04em; text-transform: uppercase; }
  button, .file-label { border: 0; border-radius: 8px; padding: 10px 14px; font: inherit; font-weight: 650; cursor: pointer; width: fit-content; }
  button { color: #fff; background: #7d2ae8; }
  button:disabled { cursor: not-allowed; opacity: .45; }
  .file-label { color: #24282b; background: #eef0f2; }
  input[type=file] { position: absolute; inline-size: 1px; block-size: 1px; overflow: hidden; clip-path: inset(50%); }
  .confirm { display: flex; gap: 8px; align-items: flex-start; }
  .status { min-height: 22px; font-weight: 600; }
  .status[data-error=true] { color: #b42318; }
`;
document.head.append(style);

const exportButton = required<HTMLButtonElement>("export");
const planInput = required<HTMLInputElement>("plan");
const confirmation = required<HTMLInputElement>("confirm");
const applyButton = required<HTMLButtonElement>("apply");
const summary = required<HTMLOutputElement>("summary");
const status = required<HTMLOutputElement>("status");

exportButton.addEventListener("click", () => void run(exportCurrentPage));
planInput.addEventListener("change", () => void run(loadSelectedPlan));
confirmation.addEventListener("change", updateApplyState);
applyButton.addEventListener("click", () => void run(applyLoadedPlan));

async function exportCurrentPage(): Promise<void> {
  setBusy(true);
  let snapshot: ReturnType<typeof normalizeCurrentPage> | undefined;
  await openDesign({ type: "current_page" }, async (session) => {
    snapshot = normalizeCurrentPage(session.page);
  });
  if (snapshot === undefined) throw new ProfileError("NUIF_CANVA_SESSION", "Canva did not return a current page");
  downloadJson("nuif-canva-current-page.json", snapshot);
  setStatus(`Exported ${countElements(snapshot.elements)} elements.`, false);
}

async function loadSelectedPlan(): Promise<void> {
  loadedPlan = null;
  confirmation.checked = false;
  confirmation.disabled = true;
  updateApplyState();
  const file = planInput.files?.[0];
  if (file === undefined) {
    summary.value = "No plan loaded.";
    return;
  }
  if (file.size > MAX_PLAN_BYTES) throw new ProfileError("NUIF_CANVA_MESSAGE_LIMIT", "Plan exceeds 16 MiB");
  const plan = assertMutationPlan(JSON.parse(await file.text()) as unknown);
  const report = summarizePlanReport(plan.report);
  loadedPlan = plan;
  confirmation.disabled = false;
  summary.value = `${countElements(plan.page.elements)} elements · ${report.fidelityEntries} fidelity entries · ${report.correspondences} correspondences`;
  setStatus("Plan validated. Host compatibility is checked without mutation when you apply.", false);
}

async function applyLoadedPlan(): Promise<void> {
  if (loadedPlan === null || !confirmation.checked) {
    throw new ProfileError("NUIF_CANVA_CONFIRMATION", "Load and explicitly confirm a plan first");
  }
  const plan = loadedPlan;
  setBusy(true);
  await openDesign({ type: "current_page" }, async (session) => {
    preflightHostImport(plan, session.page);
    const result = await applyPlanToSession(plan, session);
    setStatus(`Applied ${result.elementsCreated} elements with ${result.syncs} sync.`, false);
  });
  loadedPlan = null;
  planInput.value = "";
  confirmation.checked = false;
  confirmation.disabled = true;
  summary.value = "No plan loaded.";
  updateApplyState();
}

async function run(operation: () => Promise<void>): Promise<void> {
  setStatus("", false);
  try {
    await operation();
  } catch (error) {
    const message = error instanceof Error ? error.message : "Unknown Canva operation failure";
    setStatus(message, true);
  } finally {
    setBusy(false);
  }
}

function setBusy(busy: boolean): void {
  exportButton.disabled = busy;
  planInput.disabled = busy;
  confirmation.disabled = busy || loadedPlan === null;
  applyButton.disabled = busy || loadedPlan === null || !confirmation.checked;
}

function updateApplyState(): void {
  applyButton.disabled = loadedPlan === null || !confirmation.checked;
}

function setStatus(message: string, error: boolean): void {
  status.value = message;
  status.dataset.error = String(error);
}

function downloadJson(name: string, value: unknown): void {
  const blob = new Blob([`${JSON.stringify(value, null, 2)}\n`], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.download = name;
  link.href = url;
  link.click();
  URL.revokeObjectURL(url);
}

function countElements(elements: readonly { children: readonly unknown[] }[]): number {
  return elements.reduce(
    (count, element) => count + 1 + countElements(element.children as readonly { children: readonly unknown[] }[]),
    0
  );
}

function required<T extends HTMLElement>(id: string): T {
  const element = document.getElementById(id);
  if (element === null) throw new Error(`Missing UI element ${id}`);
  return element as T;
}
