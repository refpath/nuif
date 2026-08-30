"use strict";

const crypto = require("node:crypto");
const fs = require("node:fs");
const path = require("node:path");

function fail(message) {
  throw new Error(message);
}

function expect(condition, message) {
  if (!condition) fail(message);
}

function jsonBytes(value) {
  return Buffer.from(JSON.stringify(value), "utf8");
}

function parseBytes(bytes) {
  return JSON.parse(Buffer.from(bytes).toString("utf8"));
}

function sha256(bytes) {
  return crypto.createHash("sha256").update(bytes).digest("hex");
}

function rejectedCode(action, code) {
  try {
    action();
  } catch (error) {
    return String(error.message).includes(code);
  }
  return false;
}

function main() {
  if (process.argv.length !== 7) {
    fail("usage: smoke.cjs binding.js input.nuif.json output.nuif.json patch.json report.json");
  }
  const [bindingPath, inputPath, outputPath, patchPath, reportPath] = process.argv.slice(2);
  const nuif = require(path.resolve(bindingPath));
  const input = fs.readFileSync(inputPath);
  const capabilities = parseBytes(nuif.capabilities());
  expect(capabilities.api_profile === "nuif-wasm-api-0", "wrong API profile");
  expect(capabilities.authorities.length === 0, "binding unexpectedly declares host authority");
  expect(
    nuif.apiVersion() === `${capabilities.api_profile}/${capabilities.binding_version}`,
    "wrong binding version",
  );

  const document = new nuif.NuifDocument(input, "nuif-text-0");
  const validation = parseBytes(document.validationReport());
  expect(validation.status === "passed" && validation.errors === 0, "fixture did not validate");
  expect(document.entityCount === 8, "fixture entity count changed");
  const before = document.canonicalHash();
  const textNoopExact = Buffer.compare(Buffer.from(document.exportBytes("nuif-text-0")), input) === 0;
  expect(textNoopExact, "canonical text no-op changed bytes");

  const patch = {
    base_revision: before,
    transactions: [{
      id: 9001,
      operations: [{
        op: "rename",
        entity: "00000000000000000000000000000020",
        name: "WASM conformance card",
      }],
    }],
  };
  const patchBytes = jsonBytes(patch);
  fs.writeFileSync(patchPath, patchBytes);
  const after = document.applyPatch(patchBytes);
  expect(after !== before, "patch did not change the canonical hash");
  expect(document.canUndo() && !document.canRedo(), "history state after apply is wrong");

  const staleRejected = rejectedCode(
    () => document.applyPatch(patchBytes),
    "NUIF_PATCH_APPLY_FAILED",
  );
  expect(document.canonicalHash() === after, "stale patch failure mutated the document");
  const malformedRejected = rejectedCode(
    () => document.applyPatch(Buffer.from("{", "utf8")),
    "NUIF_PATCH_DECODE_FAILED",
  );
  expect(document.canonicalHash() === after, "malformed patch failure mutated the document");
  const limitRejected = rejectedCode(
    () => document.applyPatch(Buffer.alloc(capabilities.limits.patch_bytes + 1, 0x20)),
    "NUIF_PATCH_LIMIT_EXCEEDED",
  );
  expect(document.canonicalHash() === after, "excessive patch failure mutated the document");

  const undo = parseBytes(document.undo());
  expect(undo.canonical_hash === before, "undo did not restore the opening hash");
  expect(document.canRedo(), "redo was not made available");
  const redo = parseBytes(document.redo());
  expect(redo.canonical_hash === after, "redo did not restore the edited hash");
  const output = Buffer.from(document.exportBytes("nuif-text-0"));
  fs.writeFileSync(outputPath, output);

  const cbor = Buffer.from(document.exportBytes("nuif-cbor-0"));
  const cborDocument = new nuif.NuifDocument(cbor, "nuif-cbor-0");
  expect(cborDocument.canonicalHash() === after, "CBOR binding round trip changed the hash");
  const encodingRejected = rejectedCode(
    () => document.exportBytes("automatic"),
    "NUIF_ENCODING_UNSUPPORTED",
  );

  const checks = {
    validation_passed: validation.status === "passed" && validation.errors === 0,
    text_noop_exact: textNoopExact,
    patch_changed_hash: after !== before,
    stale_patch_atomic: staleRejected && document.canonicalHash() === after,
    malformed_patch_atomic: malformedRejected && document.canonicalHash() === after,
    patch_limit_atomic: limitRejected && document.canonicalHash() === after,
    undo_redo_exact: undo.canonical_hash === before && redo.canonical_hash === after,
    cbor_hash_exact: cborDocument.canonicalHash() === after,
    unsupported_encoding_typed: encodingRejected,
    no_host_authority: capabilities.authorities.length === 0,
  };
  const passed = Object.values(checks).every(Boolean);
  fs.writeFileSync(reportPath, JSON.stringify({
    schema_version: 1,
    experiment: "nuif:experiment:wasm-cross-surface",
    status: passed ? "passed" : "failed",
    api_profile: capabilities.api_profile,
    binding_version: capabilities.binding_version,
    runtime: process.version,
    before_hash: before,
    after_hash: after,
    input_bytes: input.length,
    output_bytes: output.length,
    cbor_bytes: cbor.length,
    wasm_bytes: fs.statSync(path.join(path.dirname(path.resolve(bindingPath)), "nuif_bg.wasm")).size,
    output_sha256: sha256(output),
    checks,
  }, null, 2) + "\n");
  cborDocument.free();
  document.free();
  if (!passed) fail("WebAssembly smoke report failed");
}

main();
