import { mkdtemp, readdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { build } from "esbuild";

const root = resolve(import.meta.dirname, "..");
const output = await mkdtemp(join(tmpdir(), "nuif-canva-tests-"));
await build({
  entryPoints: [resolve(root, "test/protocol.test.ts"), resolve(root, "test/host.test.ts")],
  outdir: output,
  bundle: true,
  format: "esm",
  platform: "node",
  target: "node24",
  legalComments: "none"
});
const tests = (await readdir(output))
  .filter((name) => name.endsWith(".test.js"))
  .sort()
  .map((name) => resolve(output, name));
if (tests.length !== 2) throw new Error(`expected two compiled test files, observed ${tests.length}`);
const result = spawnSync(process.execPath, ["--test", ...tests], { stdio: "inherit" });
await rm(output, { recursive: true, force: true });
if (result.error !== undefined) throw result.error;
if (result.status !== 0) process.exit(result.status ?? 1);
