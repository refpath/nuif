import { createHash } from "node:crypto";
import { readFile, writeFile } from "node:fs/promises";
import * as Automerge from "@automerge/automerge";

const [inputPath, reportPath] = process.argv.slice(2);
if (!inputPath || !reportPath) {
  throw new Error("usage: node check.mjs input.json report.json");
}

const packageMetadata = JSON.parse(
  await readFile(new URL("./node_modules/@automerge/automerge/package.json", import.meta.url), "utf8")
);
if (packageMetadata.version !== "3.4.1") {
  throw new Error(`expected @automerge/automerge 3.4.1, loaded ${packageMetadata.version}`);
}

const input = JSON.parse(await readFile(inputPath, "utf8"));
if (input.schema_version !== 1 || input.profile !== "nuif-collab-tree-0") {
  throw new Error("unsupported NUIF structural oracle input");
}
if (typeof input.base_canonical_hash !== "string" || !input.base_canonical_hash.startsWith("nuif-cbor-0:sha256:")) {
  throw new Error("structural oracle input is not bound to a canonical base hash");
}

const commonBase = Automerge.from(
  { records: {} },
  { actor: "00000000000000000000000000000000" }
);
const replicaDocuments = Object.entries(input.replicas)
  .sort(([left], [right]) => left.localeCompare(right))
  .map(([replica, changes]) => {
    let document = Automerge.clone(commonBase, { actor: actorId(replica) });
    for (const change of changes) {
      document = Automerge.change(document, `record ${changeKey(change)}`, (draft) => {
        draft.records[changeKey(change)] = canonicalJson(change);
      });
    }
    return { replica, document };
  });

const forward = mergeDocuments(replicaDocuments.map(({ document }) => document));
const reverse = mergeDocuments(replicaDocuments.map(({ document }) => document).reverse());
const evenOdd = mergeDocuments([
  ...replicaDocuments.filter((_, index) => index % 2 === 0).map(({ document }) => document),
  ...replicaDocuments.filter((_, index) => index % 2 === 1).map(({ document }) => document)
]);
const duplicate = Automerge.merge(Automerge.clone(forward), Automerge.clone(reverse));
const loaded = Automerge.load(Automerge.save(forward));

const expected = [...input.expected_changes].sort(compareChanges);
const expectedJson = canonicalJson(expected);
const observations = [forward, reverse, evenOdd, duplicate, loaded].map(extractChanges);
const checks = {
  forward_exact: canonicalJson(observations[0]) === expectedJson,
  reverse_exact: canonicalJson(observations[1]) === expectedJson,
  alternate_merge_exact: canonicalJson(observations[2]) === expectedJson,
  duplicate_merge_idempotent: canonicalJson(observations[3]) === expectedJson,
  save_load_exact: canonicalJson(observations[4]) === expectedJson,
  record_count_exact: observations.every((changes) => changes.length === expected.length)
};
const passed = Object.values(checks).every(Boolean);
const report = {
  schema_version: 1,
  status: passed ? "passed" : "failed",
  oracle: "@automerge/automerge operation-set transport",
  version: packageMetadata.version,
  node_version: process.version,
  profile: input.profile,
  base_canonical_hash: input.base_canonical_hash,
  role: "Foreign convergent transport of immutable NUIF structural change records; NUIF remains the tree materializer.",
  expected_canonical_hash: input.expected_canonical_hash,
  replicas: replicaDocuments.map(({ replica }) => replica),
  records: expected.length,
  record_set_sha256: sha256(expectedJson),
  automerge_bytes: Automerge.save(forward).byteLength,
  checks
};
await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`);
console.log(`Automerge ${packageMetadata.version}: ${expected.length} structural records, status ${report.status}`);
if (!passed) process.exitCode = 1;

function mergeDocuments(documents) {
  if (documents.length === 0) throw new Error("oracle input contains no replica documents");
  let merged = Automerge.clone(documents[0]);
  for (const document of documents.slice(1)) {
    merged = Automerge.merge(merged, Automerge.clone(document));
  }
  return merged;
}

function extractChanges(document) {
  return Object.entries(document.records)
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([, value]) => JSON.parse(value))
    .sort(compareChanges);
}

function compareChanges(left, right) {
  return left.id.counter - right.id.counter || left.id.replica.localeCompare(right.id.replica);
}

function changeKey(change) {
  return `${change.id.counter.toString().padStart(20, "0")}:${change.id.replica}`;
}

function actorId(replica) {
  return createHash("sha256").update(`nuif:${replica}`).digest("hex").slice(0, 32);
}

function canonicalJson(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (value !== null && typeof value === "object") {
    return `{${Object.keys(value)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`)
      .join(",")}}`;
  }
  return JSON.stringify(value);
}

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}
