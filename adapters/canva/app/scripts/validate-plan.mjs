import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { build } from "esbuild";

const [inputPath, outputPath] = process.argv.slice(2);
if (inputPath === undefined || outputPath === undefined) {
  throw new Error("usage: npm run validate-plan -- INPUT.json OUTPUT.json");
}
const root = resolve(import.meta.dirname, "..");
const temporary = await mkdtemp(join(tmpdir(), "nuif-canva-plan-"));
const bundle = resolve(temporary, "validate-plan.mjs");
await build({
  entryPoints: [resolve(root, "test/validate-plan.ts")],
  outfile: bundle,
  bundle: true,
  format: "esm",
  platform: "node",
  target: "node24",
  legalComments: "none"
});
const result = spawnSync(process.execPath, [bundle, resolve(inputPath), resolve(outputPath)], { stdio: "inherit" });
await rm(temporary, { recursive: true, force: true });
if (result.error !== undefined) throw result.error;
if (result.status !== 0) process.exit(result.status ?? 1);
