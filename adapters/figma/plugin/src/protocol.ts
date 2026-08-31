export const SNAPSHOT_PROFILE = "nuif-figma-plugin-snapshot-0";
export const SNAPSHOT_SCHEMA_VERSION = 1;
export const PINNED_FONT_NAME = "Ahem";
export const PINNED_FONT_STYLE = "Regular";
export const PINNED_FONT_SHA256 =
  "f0a92cd0cc45735591c9b5b1fa8aecd5194e8dc518895ca22af94a46c23550dc";

export const MAX_SNAPSHOT_BYTES = 16 * 1024 * 1024;
export const MAX_NODES = 16_384;
export const MAX_DEPTH = 64;
export const MAX_TEXT_UTF16 = 4_096;
export const MAX_STRING_BYTES = 256 * 1024;
export const MIN_NODE_DIMENSION = 0.01;

export type HostNodeKind = "FRAME" | "GROUP" | "RECTANGLE" | "ELLIPSE" | "TEXT";
export type HostLayoutMode = "NONE" | "HORIZONTAL" | "VERTICAL";
export type HostAxisAlign = "MIN" | "CENTER" | "MAX";

export interface SolidPaint {
  red: number;
  green: number;
  blue: number;
  alpha: number;
}

export interface HostLayout {
  mode: HostLayoutMode;
  item_spacing: number;
  padding_top: number;
  padding_right: number;
  padding_bottom: number;
  padding_left: number;
  primary_axis_align: HostAxisAlign;
  counter_axis_align: HostAxisAlign;
}

export interface HostText {
  characters: string;
  font_family: string;
  font_style: string;
  font_sha256: string;
  font_size: number;
  line_height: number;
}

export interface SnapshotNode {
  id: string;
  name: string;
  kind: HostNodeKind;
  visible: boolean;
  opacity: number;
  x: number;
  y: number;
  width: number;
  height: number;
  fill: SolidPaint | null;
  layout: HostLayout;
  text: HostText | null;
  nuif_entity_id: string | null;
  unsupported_properties: string[];
  children: SnapshotNode[];
}

export interface PluginSnapshot {
  schema_version: number;
  host_application_version: string;
  host_api_version: string;
  host_document_id: string;
  host_document_revision: string | null;
  page_id: string;
  page_name: string;
  nuif_document_id: string | null;
  root: SnapshotNode;
}

export interface PluginMutationPlan {
  schema_version: number;
  profile: string;
  snapshot: PluginSnapshot;
  report: unknown;
}

export interface PlanReportSummary {
  fidelityEntries: number;
  correspondences: number;
}

export type MainToUiMessage =
  | { type: "ready"; profile: string; limits: Record<string, number> }
  | { type: "snapshot"; snapshot: PluginSnapshot }
  | { type: "apply-result"; root_id: string; nodes_created: number }
  | { type: "error"; code: string; message: string };

export type UiToMainMessage =
  | { type: "export-selection" }
  | { type: "apply-plan"; plan: unknown };

export class ProfileError extends Error {
  readonly code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = "ProfileError";
    this.code = code;
  }
}

export function emptyLayout(): HostLayout {
  return {
    mode: "NONE",
    item_spacing: 0,
    padding_top: 0,
    padding_right: 0,
    padding_bottom: 0,
    padding_left: 0,
    primary_axis_align: "MIN",
    counter_axis_align: "MIN"
  };
}

export function utf8Length(value: string): number {
  let bytes = 0;
  for (let index = 0; index < value.length; index += 1) {
    const code = value.charCodeAt(index);
    if (code <= 0x7f) bytes += 1;
    else if (code <= 0x7ff) bytes += 2;
    else if (code >= 0xd800 && code <= 0xdbff && index + 1 < value.length) {
      const next = value.charCodeAt(index + 1);
      if (next >= 0xdc00 && next <= 0xdfff) {
        bytes += 4;
        index += 1;
      } else bytes += 3;
    } else bytes += 3;
  }
  return bytes;
}

export function assertMutationPlan(value: unknown): PluginMutationPlan {
  let encoded: string | undefined;
  try {
    encoded = JSON.stringify(value);
  } catch {
    throw new ProfileError("NUIF_FIGMA_MESSAGE", "Mutation plan must be serializable JSON");
  }
  if (encoded === undefined) throw new ProfileError("NUIF_FIGMA_MESSAGE", "Mutation plan must be a JSON object");
  if (utf8Length(encoded) > MAX_SNAPSHOT_BYTES) {
    throw new ProfileError("NUIF_FIGMA_MESSAGE_LIMIT", "Mutation plan exceeds 16 MiB");
  }
  const plan = record(value, "plan");
  exactKeys(plan, ["schema_version", "profile", "snapshot", "report"], "plan");
  if (plan.schema_version !== SNAPSHOT_SCHEMA_VERSION || plan.profile !== SNAPSHOT_PROFILE) {
    throw new ProfileError("NUIF_FIGMA_PLAN_PROFILE", "Mutation plan profile marker is invalid");
  }
  const snapshot = assertSnapshot(plan.snapshot, true);
  summarizePlanReport(plan.report);
  return {
    schema_version: SNAPSHOT_SCHEMA_VERSION,
    profile: SNAPSHOT_PROFILE,
    snapshot,
    report: plan.report
  };
}

export function summarizePlanReport(value: unknown): PlanReportSummary {
  const report = record(value, "report");
  exactKeys(
    report,
    [
      "schema_version",
      "profile",
      "direction",
      "host_application",
      "host_api_version",
      "host_document_revision",
      "canonical_hash",
      "fidelity",
      "correspondences",
      "unmapped_host_data_preserved"
    ],
    "report"
  );
  if (
    report.schema_version !== SNAPSHOT_SCHEMA_VERSION ||
    report.profile !== SNAPSHOT_PROFILE ||
    report.direction !== "import" ||
    report.unmapped_host_data_preserved !== true
  ) {
    throw new ProfileError("NUIF_FIGMA_REPORT_PROFILE", "Plan report is not an exact import report for this profile");
  }
  nonemptyString(report.host_application, "report.host_application");
  nonemptyString(report.host_api_version, "report.host_api_version");
  optionalString(report.host_document_revision, "report.host_document_revision");
  nonemptyString(report.canonical_hash, "report.canonical_hash");
  if (!Array.isArray(report.fidelity) || !Array.isArray(report.correspondences)) {
    throw new ProfileError("NUIF_FIGMA_REPORT_ARRAY", "Plan report fidelity and correspondences must be arrays");
  }
  for (const value of report.fidelity) {
    const entry = record(value, "fidelity entry");
    exactKeys(entry, ["target", "pointer", "status"], "fidelity entry");
    string(entry.pointer, "fidelity pointer");
    record(entry.target, "fidelity target");
    const status = record(entry.status, "fidelity status");
    exactKeys(status, ["class"], "lossless fidelity status");
    if (status.class !== "lossless") {
      throw new ProfileError("NUIF_FIGMA_LOSSY_REPORT", "Mutation plan report contains non-lossless fidelity");
    }
  }
  return { fidelityEntries: report.fidelity.length, correspondences: report.correspondences.length };
}

export function assertSnapshot(value: unknown, mutationPlan = false): PluginSnapshot {
  const snapshot = record(value, "snapshot");
  exactKeys(
    snapshot,
    [
      "schema_version",
      "host_application_version",
      "host_api_version",
      "host_document_id",
      "host_document_revision",
      "page_id",
      "page_name",
      "nuif_document_id",
      "root"
    ],
    "snapshot"
  );
  if (snapshot.schema_version !== SNAPSHOT_SCHEMA_VERSION) {
    throw new ProfileError("NUIF_FIGMA_SNAPSHOT_PROFILE", "Snapshot schema_version must equal 1");
  }
  const header = {
    schema_version: SNAPSHOT_SCHEMA_VERSION,
    host_application_version: nonemptyString(snapshot.host_application_version, "host_application_version"),
    host_api_version: nonemptyString(snapshot.host_api_version, "host_api_version"),
    host_document_id: nonemptyString(snapshot.host_document_id, "host_document_id"),
    host_document_revision: optionalString(snapshot.host_document_revision, "host_document_revision"),
    page_id: nonemptyString(snapshot.page_id, "page_id"),
    page_name: nonemptyString(snapshot.page_name, "page_name"),
    nuif_document_id: optionalString(snapshot.nuif_document_id, "nuif_document_id")
  };
  const state = {
    nodes: 0,
    strings:
      utf8Length(header.host_application_version) +
      utf8Length(header.host_api_version) +
      utf8Length(header.host_document_id) +
      utf8Length(header.host_document_revision ?? "") +
      utf8Length(header.page_id) +
      utf8Length(header.page_name) +
      utf8Length(header.nuif_document_id ?? ""),
    hostIds: new Set<string>()
  };
  const root = assertNode(snapshot.root, 0, state, mutationPlan);
  if (root.kind !== "FRAME") {
    throw new ProfileError("NUIF_FIGMA_ROOT_KIND", "Mutation plan root must be a FRAME");
  }
  return { ...header, root };
}

function assertNode(
  value: unknown,
  depth: number,
  state: { nodes: number; strings: number; hostIds: Set<string> },
  mutationPlan: boolean
): SnapshotNode {
  if (depth >= MAX_DEPTH) throw new ProfileError("NUIF_FIGMA_DEPTH_LIMIT", "Tree exceeds 64 levels");
  state.nodes += 1;
  if (state.nodes > MAX_NODES) throw new ProfileError("NUIF_FIGMA_NODE_LIMIT", "Tree exceeds 16,384 nodes");
  const node = record(value, `node at depth ${depth}`);
  exactKeys(
    node,
    [
      "id",
      "name",
      "kind",
      "visible",
      "opacity",
      "x",
      "y",
      "width",
      "height",
      "fill",
      "layout",
      "text",
      "nuif_entity_id",
      "unsupported_properties",
      "children"
    ],
    `node at depth ${depth}`
  );
  const id = nonemptyString(node.id, "node.id");
  if (state.hostIds.has(id)) throw new ProfileError("NUIF_FIGMA_DUPLICATE_HOST_ID", `Duplicate host id ${id}`);
  state.hostIds.add(id);
  const name = string(node.name, "node.name");
  const kind = enumValue(node.kind, ["FRAME", "GROUP", "RECTANGLE", "ELLIPSE", "TEXT"] as const, "node.kind");
  if (mutationPlan && kind === "GROUP") {
    throw new ProfileError("NUIF_FIGMA_PLAN_GROUP", "Mutation plans must lower containers to FRAME nodes");
  }
  const visible = boolean(node.visible, "node.visible");
  const opacity = finite(node.opacity, "node.opacity");
  const x = finite(node.x, "node.x");
  const y = finite(node.y, "node.y");
  const width = nonnegative(node.width, "node.width");
  const height = nonnegative(node.height, "node.height");
  if (width < MIN_NODE_DIMENSION || height < MIN_NODE_DIMENSION) {
    throw new ProfileError("NUIF_FIGMA_NODE_SIZE", "Node dimensions must be at least 0.01");
  }
  if (opacity < 0 || opacity > 1) throw new ProfileError("NUIF_FIGMA_OPACITY", "Opacity must be in 0..=1");
  const fill = node.fill === null ? null : assertPaint(node.fill);
  const layout = assertLayout(node.layout);
  const text = node.text === null ? null : assertText(node.text);
  const nuifEntityId = optionalString(node.nuif_entity_id, "node.nuif_entity_id");
  if (!Array.isArray(node.unsupported_properties)) {
    throw new ProfileError("NUIF_FIGMA_PROPERTY_LIST", "unsupported_properties must be an array");
  }
  const unsupported = node.unsupported_properties.map((item) => {
    const property = string(item, "unsupported property");
    if (!/^[A-Za-z0-9_.-]{1,128}$/.test(property)) {
      throw new ProfileError("NUIF_FIGMA_PROPERTY_NAME", `Invalid unsupported property ${property}`);
    }
    return property;
  });
  if (!Array.isArray(node.children)) throw new ProfileError("NUIF_FIGMA_CHILDREN", "children must be an array");
  const children = node.children.map((child) => assertNode(child, depth + 1, state, mutationPlan));
  if (kind !== "FRAME" && kind !== "GROUP" && children.length !== 0) {
    throw new ProfileError("NUIF_FIGMA_LEAF_CHILD", "Leaf nodes cannot contain children");
  }
  if (kind === "GROUP" && (fill !== null || !layoutEquals(layout, emptyLayout()))) {
    throw new ProfileError("NUIF_FIGMA_GROUP_STYLE", "GROUP fill and layout must be empty");
  }
  if (kind === "TEXT" ? text === null : text !== null) {
    throw new ProfileError("NUIF_FIGMA_TEXT_SHAPE", "Only TEXT nodes may carry required text metadata");
  }
  if (mutationPlan && (!visible || opacity !== 1 || unsupported.length !== 0)) {
    throw new ProfileError("NUIF_FIGMA_LOSSY_PLAN", "Mutation plan must be visible, opaque and free of host-only properties");
  }
  state.strings +=
    utf8Length(id) +
    utf8Length(name) +
    unsupported.reduce((sum, property) => sum + utf8Length(property), 0) +
    utf8Length(nuifEntityId ?? "") +
    (text === null
      ? 0
      : utf8Length(text.characters) +
        utf8Length(text.font_family) +
        utf8Length(text.font_style) +
        utf8Length(text.font_sha256));
  if (state.strings > MAX_STRING_BYTES) {
    throw new ProfileError("NUIF_FIGMA_STRING_LIMIT", "Combined strings exceed 256 KiB");
  }
  return {
    id,
    name,
    kind,
    visible,
    opacity,
    x,
    y,
    width,
    height,
    fill,
    layout,
    text,
    nuif_entity_id: nuifEntityId,
    unsupported_properties: unsupported,
    children
  };
}

function assertLayout(value: unknown): HostLayout {
  const layout = record(value, "layout");
  exactKeys(
    layout,
    [
      "mode",
      "item_spacing",
      "padding_top",
      "padding_right",
      "padding_bottom",
      "padding_left",
      "primary_axis_align",
      "counter_axis_align"
    ],
    "layout"
  );
  const result: HostLayout = {
    mode: enumValue(layout.mode, ["NONE", "HORIZONTAL", "VERTICAL"] as const, "layout.mode"),
    item_spacing: nonnegative(layout.item_spacing, "layout.item_spacing"),
    padding_top: nonnegative(layout.padding_top, "layout.padding_top"),
    padding_right: nonnegative(layout.padding_right, "layout.padding_right"),
    padding_bottom: nonnegative(layout.padding_bottom, "layout.padding_bottom"),
    padding_left: nonnegative(layout.padding_left, "layout.padding_left"),
    primary_axis_align: enumValue(layout.primary_axis_align, ["MIN", "CENTER", "MAX"] as const, "layout.primary_axis_align"),
    counter_axis_align: enumValue(layout.counter_axis_align, ["MIN", "CENTER", "MAX"] as const, "layout.counter_axis_align")
  };
  if (result.mode === "NONE" && !layoutEquals(result, emptyLayout())) {
    throw new ProfileError("NUIF_FIGMA_LAYOUT_NONE", "NONE layout must use zero/default metrics");
  }
  if (result.mode !== "NONE" && result.primary_axis_align !== "MIN") {
    throw new ProfileError("NUIF_FIGMA_LAYOUT_PACKING", "Auto layout must use packed MIN alignment");
  }
  return result;
}

function assertPaint(value: unknown): SolidPaint {
  const paint = record(value, "paint");
  exactKeys(paint, ["red", "green", "blue", "alpha"], "paint");
  const result = {
    red: finite(paint.red, "paint.red"),
    green: finite(paint.green, "paint.green"),
    blue: finite(paint.blue, "paint.blue"),
    alpha: finite(paint.alpha, "paint.alpha")
  };
  if (Object.values(result).some((channel) => channel < 0 || channel > 1)) {
    throw new ProfileError("NUIF_FIGMA_PAINT_CHANNEL", "Paint channels must be in 0..=1");
  }
  return result;
}

function assertText(value: unknown): HostText {
  const text = record(value, "text");
  exactKeys(text, ["characters", "font_family", "font_style", "font_sha256", "font_size", "line_height"], "text");
  const result = {
    characters: string(text.characters, "text.characters"),
    font_family: string(text.font_family, "text.font_family"),
    font_style: string(text.font_style, "text.font_style"),
    font_sha256: string(text.font_sha256, "text.font_sha256"),
    font_size: positive(text.font_size, "text.font_size"),
    line_height: positive(text.line_height, "text.line_height")
  };
  if (result.characters.length > MAX_TEXT_UTF16) {
    throw new ProfileError("NUIF_FIGMA_TEXT_LIMIT", "Text exceeds 4,096 UTF-16 code units");
  }
  if (
    result.font_family !== PINNED_FONT_NAME ||
    result.font_style !== PINNED_FONT_STYLE ||
    result.font_sha256 !== PINNED_FONT_SHA256
  ) {
    throw new ProfileError("NUIF_FIGMA_FONT_IDENTITY", "Text requires the pinned Ahem Regular identity");
  }
  return result;
}

function record(value: unknown, label: string): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new ProfileError("NUIF_FIGMA_OBJECT", `${label} must be an object`);
  }
  return value as Record<string, unknown>;
}

function exactKeys(value: Record<string, unknown>, expected: string[], label: string): void {
  const actual = Object.keys(value).sort();
  const wanted = [...expected].sort();
  if (actual.length !== wanted.length || actual.some((key, index) => key !== wanted[index])) {
    throw new ProfileError("NUIF_FIGMA_FIELDS", `${label} fields do not match the profile`);
  }
}

function string(value: unknown, label: string): string {
  if (typeof value !== "string") throw new ProfileError("NUIF_FIGMA_STRING", `${label} must be a string`);
  return value;
}

function nonemptyString(value: unknown, label: string): string {
  const result = string(value, label);
  if (result.trim() === "") throw new ProfileError("NUIF_FIGMA_EMPTY_STRING", `${label} must not be empty`);
  return result;
}

function optionalString(value: unknown, label: string): string | null {
  return value === null ? null : string(value, label);
}

function boolean(value: unknown, label: string): boolean {
  if (typeof value !== "boolean") throw new ProfileError("NUIF_FIGMA_BOOLEAN", `${label} must be a boolean`);
  return value;
}

function finite(value: unknown, label: string): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new ProfileError("NUIF_FIGMA_NUMBER", `${label} must be finite`);
  }
  return value;
}

function nonnegative(value: unknown, label: string): number {
  const result = finite(value, label);
  if (result < 0) throw new ProfileError("NUIF_FIGMA_NEGATIVE", `${label} must be non-negative`);
  return result;
}

function positive(value: unknown, label: string): number {
  const result = finite(value, label);
  if (result <= 0) throw new ProfileError("NUIF_FIGMA_NON_POSITIVE", `${label} must be positive`);
  return result;
}

function enumValue<const T extends readonly string[]>(value: unknown, choices: T, label: string): T[number] {
  if (typeof value !== "string" || !choices.includes(value)) {
    throw new ProfileError("NUIF_FIGMA_ENUM", `${label} is outside the profile`);
  }
  return value as T[number];
}

function layoutEquals(left: HostLayout, right: HostLayout): boolean {
  return Object.keys(left).every((key) => left[key as keyof HostLayout] === right[key as keyof HostLayout]);
}
