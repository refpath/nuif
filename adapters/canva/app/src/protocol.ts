export const CANVA_PROFILE = "nuif-canva-design-editing-0";
export const CANVA_SCHEMA_VERSION = 1;
export const CANVA_HOST_API_VERSION = "2";
export const PINNED_FONT_NAME = "Ahem";
export const PINNED_FONT_SHA256 =
  "f0a92cd0cc45735591c9b5b1fa8aecd5194e8dc518895ca22af94a46c23550dc";
export const MAX_PLAN_BYTES = 16 * 1024 * 1024;
export const MAX_ELEMENTS = 16_384;
export const MAX_DEPTH = 64;
export const MAX_TEXT_UTF16 = 4_096;
export const MAX_STRING_BYTES = 1_048_576;
export const MIN_ELEMENT_DIMENSION = 0.01;

export type CanvaElementKind = "group" | "rectangle" | "ellipse" | "text";

export interface SolidColor {
  red: number;
  green: number;
  blue: number;
  alpha: number;
}

export interface CanvaText {
  characters: string;
  font_family: string;
  font_sha256: string;
  font_size: number;
  line_height: number;
}

export interface CanvaElement {
  id: string;
  kind: CanvaElementKind;
  name: string | null;
  visible: boolean;
  locked: boolean;
  opacity: number;
  rotation: number;
  x: number;
  y: number;
  width: number;
  height: number;
  fill: SolidColor | null;
  text: CanvaText | null;
  unsupported_properties: string[];
  children: CanvaElement[];
}

export interface CanvaPage {
  schema_version: number;
  host_application_version: string;
  host_api_version: string;
  host_document_id: string;
  host_document_revision: string | null;
  page_id: string;
  page_name: string | null;
  width: number;
  height: number;
  background: SolidColor | null;
  elements: CanvaElement[];
}

export interface CanvaMutationPlan {
  schema_version: number;
  profile: string;
  page: CanvaPage;
  report: unknown;
}

export interface PlanReportSummary {
  fidelityEntries: number;
  correspondences: number;
}

export class ProfileError extends Error {
  readonly code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = "ProfileError";
    this.code = code;
  }
}

export function utf8Length(value: string): number {
  return new TextEncoder().encode(value).length;
}

export function assertMutationPlan(value: unknown): CanvaMutationPlan {
  let encoded: string | undefined;
  try {
    encoded = JSON.stringify(value);
  } catch {
    throw new ProfileError("NUIF_CANVA_MESSAGE", "Mutation plan must be serializable JSON");
  }
  if (encoded === undefined) throw new ProfileError("NUIF_CANVA_MESSAGE", "Mutation plan must be a JSON object");
  if (utf8Length(encoded) > MAX_PLAN_BYTES) {
    throw new ProfileError("NUIF_CANVA_MESSAGE_LIMIT", "Mutation plan exceeds 16 MiB");
  }
  const plan = record(value, "plan");
  exactKeys(plan, ["schema_version", "profile", "page", "report"], "plan");
  if (plan.schema_version !== CANVA_SCHEMA_VERSION || plan.profile !== CANVA_PROFILE) {
    throw new ProfileError("NUIF_CANVA_PLAN_PROFILE", "Mutation plan profile marker is invalid");
  }
  const page = assertPage(plan.page);
  summarizePlanReport(plan.report);
  return {
    schema_version: CANVA_SCHEMA_VERSION,
    profile: CANVA_PROFILE,
    page,
    report: plan.report
  };
}

export function assertPage(value: unknown): CanvaPage {
  const page = record(value, "page");
  exactKeys(
    page,
    [
      "schema_version",
      "host_application_version",
      "host_api_version",
      "host_document_id",
      "host_document_revision",
      "page_id",
      "page_name",
      "width",
      "height",
      "background",
      "elements"
    ],
    "page"
  );
  if (page.schema_version !== CANVA_SCHEMA_VERSION || page.host_api_version !== CANVA_HOST_API_VERSION) {
    throw new ProfileError("NUIF_CANVA_PAGE_PROFILE", "Page schema or Apps SDK major is invalid");
  }
  const header = {
    schema_version: CANVA_SCHEMA_VERSION,
    host_application_version: nonemptyString(page.host_application_version, "host_application_version"),
    host_api_version: CANVA_HOST_API_VERSION,
    host_document_id: nonemptyString(page.host_document_id, "host_document_id"),
    host_document_revision: optionalString(page.host_document_revision, "host_document_revision"),
    page_id: nonemptyString(page.page_id, "page_id"),
    page_name: optionalString(page.page_name, "page_name"),
    width: dimension(page.width, "page.width"),
    height: dimension(page.height, "page.height"),
    background: page.background === null ? null : assertColor(page.background, "page.background")
  };
  if (!Array.isArray(page.elements)) {
    throw new ProfileError("NUIF_CANVA_ELEMENT_LIST", "page.elements must be an array");
  }
  const state = {
    elements: 0,
    stringBytes:
      utf8Length(header.host_application_version) +
      utf8Length(header.host_document_id) +
      utf8Length(header.host_document_revision ?? "") +
      utf8Length(header.page_id) +
      utf8Length(header.page_name ?? ""),
    hostIds: new Set<string>()
  };
  const elements = page.elements.map((element, index) => assertElement(element, 0, `page.elements[${index}]`, state));
  if (state.stringBytes > MAX_STRING_BYTES) {
    throw new ProfileError("NUIF_CANVA_STRING_LIMIT", "Plan string data exceeds 1 MiB");
  }
  return { ...header, elements };
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
    report.schema_version !== CANVA_SCHEMA_VERSION ||
    report.profile !== CANVA_PROFILE ||
    report.direction !== "export" ||
    report.host_api_version !== CANVA_HOST_API_VERSION ||
    report.unmapped_host_data_preserved !== false
  ) {
    throw new ProfileError("NUIF_CANVA_REPORT_PROFILE", "Plan report is not an exact export report for this profile");
  }
  nonemptyString(report.host_application, "report.host_application");
  optionalString(report.host_document_revision, "report.host_document_revision");
  const canonicalHash = nonemptyString(report.canonical_hash, "report.canonical_hash");
  if (!/^nuif-cbor-0:sha256:[0-9a-f]{64}$/.test(canonicalHash)) {
    throw new ProfileError("NUIF_CANVA_CANONICAL_HASH", "Plan report canonical hash is invalid");
  }
  if (!Array.isArray(report.fidelity) || !Array.isArray(report.correspondences)) {
    throw new ProfileError("NUIF_CANVA_REPORT_ARRAY", "Report fidelity and correspondences must be arrays");
  }
  for (const value of report.fidelity) assertFidelity(value);
  for (const value of report.correspondences) assertCorrespondence(value);
  return { fidelityEntries: report.fidelity.length, correspondences: report.correspondences.length };
}

function assertElement(
  value: unknown,
  depth: number,
  pointer: string,
  state: { elements: number; stringBytes: number; hostIds: Set<string> }
): CanvaElement {
  if (depth >= MAX_DEPTH) throw new ProfileError("NUIF_CANVA_DEPTH_LIMIT", "Element tree exceeds 64 levels");
  state.elements += 1;
  if (state.elements > MAX_ELEMENTS) {
    throw new ProfileError("NUIF_CANVA_ELEMENT_LIMIT", "Element tree exceeds 16,384 elements");
  }
  const element = record(value, pointer);
  exactKeys(
    element,
    [
      "id",
      "kind",
      "name",
      "visible",
      "locked",
      "opacity",
      "rotation",
      "x",
      "y",
      "width",
      "height",
      "fill",
      "text",
      "unsupported_properties",
      "children"
    ],
    pointer
  );
  const id = nonemptyString(element.id, `${pointer}.id`);
  if (state.hostIds.has(id)) throw new ProfileError("NUIF_CANVA_DUPLICATE_HOST_ID", `Duplicate host id ${id}`);
  state.hostIds.add(id);
  const kind = enumValue(element.kind, ["group", "rectangle", "ellipse", "text"] as const, `${pointer}.kind`);
  const name = optionalString(element.name, `${pointer}.name`);
  const visible = boolean(element.visible, `${pointer}.visible`);
  const locked = boolean(element.locked, `${pointer}.locked`);
  const opacity = finite(element.opacity, `${pointer}.opacity`);
  if (opacity < 0 || opacity > 1) throw new ProfileError("NUIF_CANVA_OPACITY", `${pointer}.opacity must be in 0..=1`);
  const rotation = finite(element.rotation, `${pointer}.rotation`);
  const x = finite(element.x, `${pointer}.x`);
  const y = finite(element.y, `${pointer}.y`);
  const width = dimension(element.width, `${pointer}.width`);
  const height = dimension(element.height, `${pointer}.height`);
  const fill = element.fill === null ? null : assertColor(element.fill, `${pointer}.fill`);
  const text = element.text === null ? null : assertText(element.text, `${pointer}.text`);
  if (!Array.isArray(element.unsupported_properties)) {
    throw new ProfileError("NUIF_CANVA_PROPERTY_LIST", `${pointer}.unsupported_properties must be an array`);
  }
  const unsupportedProperties = element.unsupported_properties.map((value, index) => {
    const property = string(value, `${pointer}.unsupported_properties[${index}]`);
    if (!/^[A-Za-z0-9_.-]{1,128}$/.test(property)) {
      throw new ProfileError("NUIF_CANVA_PROPERTY_NAME", `Invalid unsupported property ${property}`);
    }
    return property;
  });
  if (!Array.isArray(element.children)) {
    throw new ProfileError("NUIF_CANVA_CHILD_LIST", `${pointer}.children must be an array`);
  }
  if (kind === "text" && text === null) {
    throw new ProfileError("NUIF_CANVA_TEXT_SHAPE", `${pointer} requires text metadata`);
  }
  if (kind !== "text" && text !== null) {
    throw new ProfileError("NUIF_CANVA_TEXT_SHAPE", `${pointer} cannot carry text metadata`);
  }
  if (kind !== "group" && element.children.length !== 0) {
    throw new ProfileError("NUIF_CANVA_CHILD_SHAPE", `${pointer} cannot carry children`);
  }
  state.stringBytes +=
    utf8Length(id) +
    utf8Length(name ?? "") +
    unsupportedProperties.reduce((sum, property) => sum + utf8Length(property), 0) +
    (text === null
      ? 0
      : utf8Length(text.characters) + utf8Length(text.font_family) + utf8Length(text.font_sha256));
  const children = element.children.map((child, index) =>
    assertElement(child, depth + 1, `${pointer}.children[${index}]`, state)
  );
  return {
    id,
    kind,
    name,
    visible,
    locked,
    opacity,
    rotation,
    x,
    y,
    width,
    height,
    fill,
    text,
    unsupported_properties: unsupportedProperties,
    children
  };
}

function assertColor(value: unknown, pointer: string): SolidColor {
  const color = record(value, pointer);
  exactKeys(color, ["red", "green", "blue", "alpha"], pointer);
  const result = {
    red: finite(color.red, `${pointer}.red`),
    green: finite(color.green, `${pointer}.green`),
    blue: finite(color.blue, `${pointer}.blue`),
    alpha: finite(color.alpha, `${pointer}.alpha`)
  };
  for (const [name, channel] of Object.entries(result)) {
    if (channel < 0 || channel > 1) throw new ProfileError("NUIF_CANVA_COLOR", `${pointer}.${name} must be in 0..=1`);
  }
  return result;
}

function assertText(value: unknown, pointer: string): CanvaText {
  const text = record(value, pointer);
  exactKeys(text, ["characters", "font_family", "font_sha256", "font_size", "line_height"], pointer);
  const characters = string(text.characters, `${pointer}.characters`);
  if (characters.length > MAX_TEXT_UTF16) {
    throw new ProfileError("NUIF_CANVA_TEXT_LIMIT", `${pointer}.characters exceeds 4,096 UTF-16 code units`);
  }
  const fontFamily = nonemptyString(text.font_family, `${pointer}.font_family`);
  const fontSha256 = nonemptyString(text.font_sha256, `${pointer}.font_sha256`);
  if (fontFamily !== PINNED_FONT_NAME || fontSha256 !== PINNED_FONT_SHA256) {
    throw new ProfileError("NUIF_CANVA_FONT", `${pointer} does not use the pinned profile font`);
  }
  const fontSize = positive(text.font_size, `${pointer}.font_size`);
  const lineHeight = positive(text.line_height, `${pointer}.line_height`);
  return {
    characters,
    font_family: fontFamily,
    font_sha256: fontSha256,
    font_size: fontSize,
    line_height: lineHeight
  };
}

function assertFidelity(value: unknown): void {
  const entry = record(value, "fidelity entry");
  exactKeys(entry, ["target", "pointer", "status"], "fidelity entry");
  assertTarget(entry.target, "fidelity target");
  string(entry.pointer, "fidelity pointer");
  const status = record(entry.status, "fidelity status");
  exactKeys(status, ["class"], "fidelity status");
  if (status.class !== "lossless") {
    throw new ProfileError("NUIF_CANVA_LOSSY_REPORT", "Mutation plan report contains non-lossless fidelity");
  }
}

function assertCorrespondence(value: unknown): void {
  const entry = record(value, "correspondence");
  exactKeys(entry, ["target", "host_object_id", "host_property"], "correspondence");
  assertTarget(entry.target, "correspondence target");
  nonemptyString(entry.host_object_id, "correspondence.host_object_id");
  optionalString(entry.host_property, "correspondence.host_property");
}

function assertTarget(value: unknown, pointer: string): void {
  const target = record(value, pointer);
  exactKeys(target, ["kind", "id"], pointer);
  if (target.kind !== "document" && target.kind !== "entity" && target.kind !== "asset" && target.kind !== "token") {
    throw new ProfileError("NUIF_CANVA_REPORT_TARGET", `${pointer}.kind is invalid`);
  }
  const id = nonemptyString(target.id, `${pointer}.id`);
  if (!/^[0-9a-f]{32}$/.test(id)) {
    throw new ProfileError("NUIF_CANVA_REPORT_TARGET", `${pointer}.id is not a canonical NUIF identifier`);
  }
}

function record(value: unknown, pointer: string): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new ProfileError("NUIF_CANVA_OBJECT", `${pointer} must be an object`);
  }
  return value as Record<string, unknown>;
}

function exactKeys(value: Record<string, unknown>, expected: readonly string[], pointer: string): void {
  const actual = Object.keys(value).sort();
  const wanted = [...expected].sort();
  if (actual.length !== wanted.length || actual.some((key, index) => key !== wanted[index])) {
    throw new ProfileError("NUIF_CANVA_UNKNOWN_FIELD", `${pointer} fields do not match the profile schema`);
  }
}

function string(value: unknown, pointer: string): string {
  if (typeof value !== "string") throw new ProfileError("NUIF_CANVA_STRING", `${pointer} must be a string`);
  return value;
}

function nonemptyString(value: unknown, pointer: string): string {
  const result = string(value, pointer);
  if (result.trim().length === 0) throw new ProfileError("NUIF_CANVA_STRING", `${pointer} must not be empty`);
  return result;
}

function optionalString(value: unknown, pointer: string): string | null {
  return value === null ? null : string(value, pointer);
}

function boolean(value: unknown, pointer: string): boolean {
  if (typeof value !== "boolean") throw new ProfileError("NUIF_CANVA_BOOLEAN", `${pointer} must be a boolean`);
  return value;
}

function finite(value: unknown, pointer: string): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new ProfileError("NUIF_CANVA_NUMBER", `${pointer} must be finite`);
  }
  return value;
}

function positive(value: unknown, pointer: string): number {
  const result = finite(value, pointer);
  if (result <= 0) throw new ProfileError("NUIF_CANVA_NUMBER", `${pointer} must be positive`);
  return result;
}

function dimension(value: unknown, pointer: string): number {
  const result = finite(value, pointer);
  if (result < MIN_ELEMENT_DIMENSION) {
    throw new ProfileError("NUIF_CANVA_DIMENSION", `${pointer} must be at least 0.01`);
  }
  return result;
}

function enumValue<const T extends readonly string[]>(value: unknown, allowed: T, pointer: string): T[number] {
  if (typeof value !== "string" || !allowed.includes(value)) {
    throw new ProfileError("NUIF_CANVA_ENUM", `${pointer} is invalid`);
  }
  return value as T[number];
}
