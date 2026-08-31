import {
  MAX_SNAPSHOT_BYTES,
  ProfileError,
  SNAPSHOT_PROFILE,
  assertMutationPlan,
  summarizePlanReport,
  utf8Length,
  type MainToUiMessage,
  type PluginMutationPlan,
  type UiToMainMessage
} from "./protocol";

const exportButton = element<HTMLButtonElement>("export-selection");
const planInput = element<HTMLInputElement>("plan-file");
const applyButton = element<HTMLButtonElement>("apply-plan");
const confirmation = element<HTMLInputElement>("confirm-plan");
const summary = element<HTMLElement>("plan-summary");
const status = element<HTMLElement>("status");
let pendingPlan: PluginMutationPlan | null = null;

exportButton.addEventListener("click", () => {
  showStatus("Reading the selected frame…", "working");
  send({ type: "export-selection" });
});

planInput.addEventListener("change", async () => {
  pendingPlan = null;
  confirmation.checked = false;
  applyButton.disabled = true;
  summary.textContent = "No mutation plan loaded.";
  const file = planInput.files?.[0];
  if (file === undefined) return;
  try {
    if (file.size > MAX_SNAPSHOT_BYTES) throw new ProfileError("NUIF_FIGMA_MESSAGE_LIMIT", "File exceeds 16 MiB");
    const source = await file.text();
    if (utf8Length(source) > MAX_SNAPSHOT_BYTES) throw new ProfileError("NUIF_FIGMA_MESSAGE_LIMIT", "File exceeds 16 MiB");
    pendingPlan = assertMutationPlan(JSON.parse(source) as unknown);
    const counts = countNodes(pendingPlan.snapshot.root);
    const evidence = summarizePlanReport(pendingPlan.report);
    summary.textContent = `${counts.nodes} nodes · ${counts.text} text · ${evidence.fidelityEntries} lossless fidelity entries · ${evidence.correspondences} correspondences`;
    showStatus("Plan validated locally. Review the summary and confirm before applying.", "ok");
  } catch (error) {
    showError(error);
  }
});

confirmation.addEventListener("change", () => {
  applyButton.disabled = pendingPlan === null || !confirmation.checked;
});

applyButton.addEventListener("click", () => {
  if (pendingPlan === null || !confirmation.checked) return;
  applyButton.disabled = true;
  showStatus("Applying one host transaction…", "working");
  send({ type: "apply-plan", plan: pendingPlan });
});

window.addEventListener("message", (event: MessageEvent<unknown>) => {
  const envelope = event.data;
  if (envelope === null || typeof envelope !== "object" || !("pluginMessage" in envelope)) return;
  const message = (envelope as { pluginMessage: MainToUiMessage }).pluginMessage;
  switch (message.type) {
    case "ready":
      if (message.profile !== SNAPSHOT_PROFILE) showStatus(`Profile mismatch: ${message.profile}`, "error");
      else showStatus("Ready. Export one selected frame or load a reviewed mutation plan.", "ok");
      break;
    case "snapshot":
      download("selection.figma-snapshot.json", `${JSON.stringify(message.snapshot, null, 2)}\n`);
      showStatus("Snapshot downloaded. Convert it with the NUIF CLI to get canonical NUIF and a fidelity report.", "ok");
      break;
    case "apply-result":
      confirmation.checked = false;
      showStatus(`Created ${message.nodes_created} nodes as ${message.root_id}.`, "ok");
      break;
    case "error":
      showStatus(`${message.code}: ${message.message}`, "error");
      break;
  }
});

function send(message: UiToMainMessage): void {
  parent.postMessage({ pluginMessage: message }, "*");
}

function countNodes(root: PluginMutationPlan["snapshot"]["root"]): { nodes: number; text: number } {
  let nodes = 0;
  let text = 0;
  const pending = [root];
  while (pending.length !== 0) {
    const node = pending.pop();
    if (node === undefined) break;
    nodes += 1;
    if (node.kind === "TEXT") text += 1;
    pending.push(...node.children);
  }
  return { nodes, text };
}

function download(name: string, contents: string): void {
  const url = URL.createObjectURL(new Blob([contents], { type: "application/json" }));
  const link = document.createElement("a");
  link.href = url;
  link.download = name;
  link.click();
  URL.revokeObjectURL(url);
}

function showError(error: unknown): void {
  if (error instanceof ProfileError) showStatus(`${error.code}: ${error.message}`, "error");
  else showStatus(error instanceof Error ? error.message : String(error), "error");
}

function showStatus(message: string, kind: "ok" | "working" | "error"): void {
  status.textContent = message;
  status.dataset.kind = kind;
}

function element<T extends HTMLElement>(id: string): T {
  const value = document.getElementById(id);
  if (value === null) throw new Error(`Missing UI element ${id}`);
  return value as T;
}
