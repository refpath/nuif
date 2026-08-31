import { createHash } from "node:crypto";
import { mkdir, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { build } from "esbuild";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const output = resolve(root, "dist");
await rm(output, { recursive: true, force: true });
await mkdir(output, { recursive: true });

const sourceFiles = (await readdir(resolve(root, "src"))).filter((name) => name.endsWith(".ts")).sort();
for (const name of sourceFiles) {
  const source = await readFile(resolve(root, "src", name), "utf8");
  if (/\b(fetch|WebSocket|EventSource|Worker|SharedWorker)\s*\(/.test(source) || /<iframe\b/i.test(source)) {
    throw new Error(`${name} introduces a forbidden network, worker, or nested-frame capability`);
  }
  if (/https?:\/\//.test(source)) throw new Error(`${name} contains a remote URL`);
}

await build({
  entryPoints: [resolve(root, "src/app.ts")],
  outfile: resolve(output, "app.js"),
  bundle: true,
  format: "iife",
  target: "es2020",
  legalComments: "none",
  minify: true,
  tsconfigRaw: { compilerOptions: { target: "ES2022" } }
});

const bundle = await readFile(resolve(output, "app.js"));
const packageLock = JSON.parse(await readFile(resolve(root, "package-lock.json"), "utf8"));
const declarationNormalization = JSON.parse(
  await readFile(resolve(root, "../../../target/canva-app-types/report.json"), "utf8")
);
const designLicense = await readFile(resolve(root, "node_modules/@canva/design/LICENSE.md"));
const errorLicense = await readFile(resolve(root, "node_modules/@canva/error/LICENSE.md"));
if (!designLicense.equals(errorLicense)) throw new Error("Canva SDK component licenses unexpectedly differ");
await writeFile(resolve(output, "CANVA-SDK-LICENSE.md"), designLicense);
const report = {
  schema_version: 1,
  status: "passed",
  profile: "nuif-canva-design-editing-0",
  review_bundle: true,
  live_ready: false,
  network_domains: [],
  workers: false,
  nested_frames: false,
  license_scope: "Canva Platform permitted apps only",
  toolchain: {
    canva_design: packageLock.packages["node_modules/@canva/design"].version,
    typescript: packageLock.packages["node_modules/typescript"].version,
    esbuild: packageLock.packages["node_modules/esbuild"].version
  },
  declaration_normalization: declarationNormalization,
  outputs: {
    "app.js": {
      bytes: bundle.length,
      sha256: createHash("sha256").update(bundle).digest("hex")
    },
    "CANVA-SDK-LICENSE.md": {
      bytes: designLicense.length,
      sha256: createHash("sha256").update(designLicense).digest("hex")
    }
  }
};
await writeFile(resolve(output, "build-report.json"), `${JSON.stringify(report, null, 2)}\n`);
console.log("built Canva no-network review bundle");
