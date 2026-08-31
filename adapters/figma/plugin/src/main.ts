import { applyMutationPlan } from "./apply";
import { normalizeSelection } from "./normalize";
import {
  MAX_DEPTH,
  MAX_NODES,
  MAX_SNAPSHOT_BYTES,
  MAX_STRING_BYTES,
  MAX_TEXT_UTF16,
  ProfileError,
  SNAPSHOT_PROFILE,
  type MainToUiMessage,
  type UiToMainMessage
} from "./protocol";

figma.showUI(__html__, { width: 420, height: 520, themeColors: true });
post({
  type: "ready",
  profile: SNAPSHOT_PROFILE,
  limits: {
    message_bytes: MAX_SNAPSHOT_BYTES,
    nodes: MAX_NODES,
    depth: MAX_DEPTH,
    text_utf16: MAX_TEXT_UTF16,
    combined_string_bytes: MAX_STRING_BYTES
  }
});

figma.ui.onmessage = async (value: unknown) => {
  try {
    const message = parseUiMessage(value);
    if (message.type === "export-selection") {
      const selection = figma.currentPage.selection;
      if (selection.length !== 1) {
        throw new ProfileError("NUIF_FIGMA_SELECTION", "Select exactly one FRAME before exporting");
      }
      const root = selection[0];
      if (root === undefined) throw new ProfileError("NUIF_FIGMA_SELECTION", "Selection is empty");
      const snapshot = normalizeSelection(root, {
        apiVersion: figma.apiVersion,
        ...(figma.fileKey === undefined ? {} : { fileKey: figma.fileKey }),
        pageId: figma.currentPage.id,
        pageName: figma.currentPage.name,
        documentId: root.getSharedPluginData("nuif", "document_id")
      });
      post({ type: "snapshot", snapshot });
      return;
    }
    const result = await applyMutationPlan(message.plan);
    post({ type: "apply-result", root_id: result.rootId, nodes_created: result.nodesCreated });
  } catch (error) {
    if (error instanceof ProfileError) post({ type: "error", code: error.code, message: error.message });
    else post({ type: "error", code: "NUIF_FIGMA_HOST_ERROR", message: errorMessage(error) });
  }
};

function parseUiMessage(value: unknown): UiToMainMessage {
  if (value === null || typeof value !== "object") {
    throw new ProfileError("NUIF_FIGMA_MESSAGE", "UI message must be an object");
  }
  const message = value as Record<string, unknown>;
  if (message.type === "export-selection" && Object.keys(message).length === 1) {
    return { type: "export-selection" };
  }
  if (message.type === "apply-plan" && Object.keys(message).length === 2 && "plan" in message) {
    return { type: "apply-plan", plan: message.plan };
  }
  throw new ProfileError("NUIF_FIGMA_MESSAGE", "UI message shape is invalid");
}

function post(message: MainToUiMessage): void {
  figma.ui.postMessage(message);
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
