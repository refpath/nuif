import { createHash } from "node:crypto";
import { readFile, writeFile } from "node:fs/promises";
import { VERSION, compile, parse } from "svelte/compiler";

const [reportPath, ...sourcePaths] = process.argv.slice(2);
if (!reportPath || sourcePaths.length === 0) {
  throw new Error("usage: node check.mjs report.json source.svelte [...]");
}
if (VERSION !== "5.57.0") {
  throw new Error(`expected Svelte 5.57.0, loaded ${VERSION}`);
}

const results = [];
for (const sourcePath of sourcePaths) {
  const source = await readFile(sourcePath, "utf8");
  const ast = parse(source, { filename: sourcePath, modern: true });
  if (ast.instance != null || ast.module != null || ast.css != null) {
    throw new Error(`${sourcePath}: script or component CSS escaped the profile`);
  }
  const counts = new Map();
  inspectFragment(ast.fragment, sourcePath, counts, true);
  if (counts.get("RegularElement") !== 2) {
    throw new Error(`${sourcePath}: expected two regular elements`);
  }
  const compiled = compile(source, {
    filename: sourcePath,
    generate: "client",
    css: "injected",
    dev: false
  });
  if (compiled.warnings.length !== 0) {
    throw new Error(`${sourcePath}: compiler warnings: ${JSON.stringify(compiled.warnings)}`);
  }
  results.push({
    path: sourcePath,
    source_sha256: sha256(source),
    source_bytes: Buffer.byteLength(source),
    ast_nodes: Object.fromEntries([...counts].sort(([a], [b]) => a.localeCompare(b))),
    compiled_javascript_sha256: sha256(compiled.js.code),
    compiled_javascript_bytes: Buffer.byteLength(compiled.js.code),
    warnings: compiled.warnings.length
  });
}

const report = {
  schema_version: 1,
  oracle: "official svelte/compiler",
  version: VERSION,
  node_version: process.version,
  parse_options: { modern: true },
  compile_options: { generate: "client", css: "injected", dev: false },
  status: "passed",
  sources: results
};
await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`);
console.log(`official Svelte ${VERSION}: ${results.length} static sources parsed and compiled`);

function inspectFragment(fragment, sourcePath, counts, topLevel) {
  let roots = 0;
  for (const node of fragment.nodes) {
    counts.set(node.type, (counts.get(node.type) ?? 0) + 1);
    if (node.type === "Text") {
      if (topLevel && node.data.trim() !== "") {
        throw new Error(`${sourcePath}: non-whitespace top-level text`);
      }
      continue;
    }
    if (node.type === "Comment") continue;
    if (node.type !== "RegularElement") {
      throw new Error(`${sourcePath}: executable or special node ${node.type}`);
    }
    roots += 1;
    if (node.name !== "div" && node.name !== "span") {
      throw new Error(`${sourcePath}: element ${node.name} is outside the profile`);
    }
    for (const attribute of node.attributes) {
      if (attribute.type !== "Attribute" || attribute.value === true) {
        throw new Error(`${sourcePath}: non-literal attribute ${attribute.type}`);
      }
      const parts = Array.isArray(attribute.value) ? attribute.value : [attribute.value];
      if (parts.some((part) => part.type !== "Text")) {
        throw new Error(`${sourcePath}: dynamic attribute ${attribute.name}`);
      }
    }
    inspectFragment(node.fragment, sourcePath, counts, false);
  }
  if (topLevel && roots !== 1) {
    throw new Error(`${sourcePath}: expected one top-level regular element, observed ${roots}`);
  }
}

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}
