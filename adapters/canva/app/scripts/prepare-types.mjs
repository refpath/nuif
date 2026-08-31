import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const sourcePath = resolve(root, "node_modules/@canva/design/index.d.ts");
const outputPath = resolve(root, "../../../target/canva-app-types/index.d.ts");
const reportPath = resolve(root, "../../../target/canva-app-types/report.json");
const source = await readFile(sourcePath, "utf8");
const invalid = "    export interface PageRefList extends ReadableList<PageRef> {\n    }\n        {};\n}";
const replacement = "    export interface PageRefList extends ReadableList<PageRef> {\n    }\n}";
const occurrences = source.split(invalid).length - 1;
if (occurrences !== 1) {
  throw new Error(`expected one known @canva/design ambient declaration defect, observed ${occurrences}`);
}
const normalized = source.replace(invalid, replacement);
await mkdir(resolve(outputPath, ".."), { recursive: true });
await writeFile(outputPath, normalized);
await writeFile(
  reportPath,
  `${JSON.stringify(
    {
      schema_version: 1,
      status: "passed",
      package: "@canva/design",
      version: "2.12.0",
      transformation: "remove one invalid empty statement after DesignEditing.PageRefList",
      source_sha256: createHash("sha256").update(source).digest("hex"),
      normalized_sha256: createHash("sha256").update(normalized).digest("hex")
    },
    null,
    2
  )}\n`
);
