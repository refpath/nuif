import { createHash } from "node:crypto";
import { mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { build } from "esbuild";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const output = resolve(root, "dist");
const writeManifest = process.argv.includes("--manifest");
await rm(output, { recursive: true, force: true });
await mkdir(output, { recursive: true });

await build({
  entryPoints: [resolve(root, "src/main.ts")],
  outfile: resolve(output, "main.js"),
  bundle: true,
  format: "iife",
  target: "es2020",
  legalComments: "none",
  minify: true
});
const ui = await build({
  entryPoints: [resolve(root, "src/ui.ts")],
  bundle: true,
  format: "iife",
  target: "es2020",
  legalComments: "none",
  minify: true,
  write: false
});
const uiJavascript = ui.outputFiles[0]?.text;
if (uiJavascript === undefined) throw new Error("esbuild did not produce the UI bundle");
const template = await readFile(resolve(root, "src/ui.template.html"), "utf8");
const uiHtml = template.replace("/* NUIF_UI_BUNDLE */", uiJavascript.replaceAll("</script", "<\\/script"));
if (uiHtml === template) throw new Error("UI template marker is missing");
await writeFile(resolve(output, "ui.html"), uiHtml);

const manifestTemplate = await readFile(resolve(root, "manifest.template.json"), "utf8");
await writeFile(resolve(output, "manifest.template.json"), manifestTemplate);
if (writeManifest) {
  const pluginId = process.env.FIGMA_PLUGIN_ID;
  if (
    pluginId === undefined ||
    pluginId.length === 0 ||
    pluginId.length > 256 ||
    pluginId !== pluginId.trim() ||
    !/^[\x21-\x7e]+$/.test(pluginId) ||
    pluginId === "REPLACE_WITH_FIGMA_PLUGIN_ID"
  ) {
    throw new Error("FIGMA_PLUGIN_ID must be the non-empty printable ID assigned by Figma");
  }
  const manifest = JSON.parse(manifestTemplate);
  manifest.id = pluginId;
  await writeFile(resolve(output, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
}

const names = ["main.js", "ui.html", "manifest.template.json", ...(writeManifest ? ["manifest.json"] : [])];
const outputs = {};
for (const name of names) {
  const bytes = await readFile(resolve(output, name));
  if (/https?:\/\//.test(bytes.toString("utf8"))) throw new Error(`${name} contains a remote URL`);
  outputs[name] = { bytes: bytes.length, sha256: createHash("sha256").update(bytes).digest("hex") };
}
await writeFile(
  resolve(output, "build-report.json"),
  `${JSON.stringify(
    {
      schema_version: 1,
      status: "passed",
      profile: "nuif-figma-plugin-snapshot-0",
      live_ready: writeManifest,
      network_domains: [],
      outputs
    },
    null,
    2
  )}\n`
);
console.log(`built Figma review shell (${writeManifest ? "reviewer manifest" : "manifest template"})`);
