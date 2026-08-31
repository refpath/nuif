import { writeFile } from "node:fs/promises";
import { fixtureSnapshot } from "./fixtures";

const output = process.argv[2];
if (output === undefined) throw new Error("missing fixture output path");
await writeFile(output, `${JSON.stringify(fixtureSnapshot(), null, 2)}\n`);
