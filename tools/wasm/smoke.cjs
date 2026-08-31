"use strict";

const crypto = require("node:crypto");
const fs = require("node:fs");
const path = require("node:path");
const { isDeepStrictEqual } = require("node:util");

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
  if (process.argv.length !== 12) {
    fail("usage: smoke.cjs binding.js input.nuif.json output.nuif.json patch.json report.json input.nuif output.nuif capability-input.nuif variable-font.nuif variable-font-report.json");
  }
  const [
    bindingPath,
    inputPath,
    outputPath,
    patchPath,
    reportPath,
    packageInputPath,
    packageOutputPath,
    capabilityPackagePath,
    variableFontPackagePath,
    variableFontReportPath,
  ] = process.argv.slice(2);
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

  const emptyCapabilities = jsonBytes([]);
  const packageInput = fs.readFileSync(packageInputPath);
  const packageDocument = nuif.NuifDocument.fromPackage(packageInput);
  expect(packageDocument.packageMode === "portable", "package mode was not retained");
  expect(packageDocument.canonicalHash() === before, "package document hash differs from bare input");
  const emptyPackageReport = parseBytes(
    packageDocument.packageCapabilityReport(emptyCapabilities),
  );
  const packageNoopExact = Buffer.compare(
    Buffer.from(packageDocument.exportPackage("portable")),
    packageInput,
  ) === 0;
  expect(packageNoopExact, "no-op package export changed bytes");
  const packageAfter = packageDocument.applyPatch(patchBytes);
  expect(packageAfter === after, "package patch produced a different canonical hash");
  const packageOutput = Buffer.from(packageDocument.exportPackage("portable"));
  fs.writeFileSync(packageOutputPath, packageOutput);

  const capabilityPackage = fs.readFileSync(capabilityPackagePath);
  const structuralCapabilityDocument = nuif.NuifDocument.fromPackage(capabilityPackage);
  const missingReport = parseBytes(
    structuralCapabilityDocument.packageCapabilityReport(emptyCapabilities),
  );
  const capabilityPackageNoopExact = Buffer.compare(
    Buffer.from(structuralCapabilityDocument.exportPackage("portable")),
    capabilityPackage,
  ) === 0;
  const structuralCapabilityHash = structuralCapabilityDocument.canonicalHash();
  const structuralCapabilityMutationRejected = rejectedCode(
    () => structuralCapabilityDocument.applyPatch(patchBytes),
    "NUIF_PACKAGE_CAPABILITIES_REQUIRED",
  );
  const missingCapabilityRejected = rejectedCode(
    () => structuralCapabilityDocument.requirePackageCapabilities(emptyCapabilities),
    "NUIF_PACKAGE_CAPABILITIES_UNAVAILABLE",
  );
  const requiredCapabilities = jsonBytes(missingReport.required);
  const supportedCapabilityDocument = nuif.NuifDocument.fromPackageWithCapabilities(
    capabilityPackage,
    requiredCapabilities,
  );
  const supportedReport = parseBytes(
    supportedCapabilityDocument.packageCapabilityReport(requiredCapabilities),
  );
  supportedCapabilityDocument.requirePackageCapabilities(requiredCapabilities);
  const atomicCapabilityLoadRejected = rejectedCode(
    () => nuif.NuifDocument.fromPackageWithCapabilities(capabilityPackage, emptyCapabilities),
    "NUIF_PACKAGE_CAPABILITIES_UNAVAILABLE",
  );
  const malformedCapabilitySetRejected = rejectedCode(
    () => structuralCapabilityDocument.packageCapabilityReport(Buffer.from("{", "utf8")),
    "NUIF_CAPABILITY_SET_DECODE_FAILED",
  );

  const variableFontPackage = fs.readFileSync(variableFontPackagePath);
  const variableFontExpected = JSON.parse(fs.readFileSync(variableFontReportPath, "utf8"));
  const variableCapability = "nuif-opentype-variable-truetype-single-0";
  const variableStructural = nuif.NuifDocument.fromPackage(variableFontPackage);
  const variableMissing = parseBytes(
    variableStructural.packageCapabilityReport(emptyCapabilities),
  );
  const variableSnapshotRejected = rejectedCode(
    () => variableStructural.snapshotReport(640, 96),
    "NUIF_PACKAGE_CAPABILITIES_REQUIRED",
  );
  const variableSupported = jsonBytes([variableCapability]);
  const variableDocument = nuif.NuifDocument.fromPackageWithCapabilities(
    variableFontPackage,
    variableSupported,
  );
  const variableObserved = parseBytes(variableDocument.snapshotReport(640, 96));
  const variableRun = variableObserved.scene.commands.find(
    (command) => command.command === "text",
  )?.run;

  const checks = {
    package_contract_declared:
      capabilities.containers.includes("nuif-package-0") &&
      capabilities.operations.includes("load_package") &&
      capabilities.operations.includes("require_package_capabilities") &&
      capabilities.limits.package_bytes === 80 * 1024 * 1024 &&
      capabilities.limits.required_capabilities === 256,
    validation_passed: validation.status === "passed" && validation.errors === 0,
    text_noop_exact: textNoopExact,
    patch_changed_hash: after !== before,
    stale_patch_atomic: staleRejected && document.canonicalHash() === after,
    malformed_patch_atomic: malformedRejected && document.canonicalHash() === after,
    patch_limit_atomic: limitRejected && document.canonicalHash() === after,
    undo_redo_exact: undo.canonical_hash === before && redo.canonical_hash === after,
    cbor_hash_exact: cborDocument.canonicalHash() === after,
    unsupported_encoding_typed: encodingRejected,
    package_noop_exact: packageNoopExact,
    package_patch_hash_exact: packageAfter === after,
    package_empty_requirements_supported:
      emptyPackageReport.fully_supported === true &&
      emptyPackageReport.required.length === 0,
    package_capability_missing_exact:
      missingReport.fully_supported === false &&
      JSON.stringify(missingReport.missing_required) ===
        JSON.stringify(["nuif-behavior-state-machine-0"]),
    package_capability_required_before_use:
      missingCapabilityRejected && atomicCapabilityLoadRejected,
    package_structural_mutation_read_only:
      structuralCapabilityMutationRejected &&
      structuralCapabilityDocument.canonicalHash() === structuralCapabilityHash,
    package_capability_resource_preservation: capabilityPackageNoopExact,
    package_capability_exact_support:
      supportedReport.fully_supported === true &&
      supportedReport.missing_required.length === 0,
    malformed_capability_transport_typed: malformedCapabilitySetRejected,
    variable_font_capability_exact:
      variableMissing.fully_supported === false &&
      isDeepStrictEqual(variableMissing.missing_required, [variableCapability]),
    variable_font_snapshot_requires_capability: variableSnapshotRejected,
    variable_font_snapshot_matches_cli: isDeepStrictEqual(
      variableObserved,
      variableFontExpected,
    ),
    variable_font_coordinates_retained:
      variableRun?.font?.sha256 ===
        "0afd77effc877ff84fa7995a58c396c124514855f8084056846b54b8cb76f3ce" &&
      variableRun?.variation_coordinates?.length === 2,
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
    package_input_bytes: packageInput.length,
    package_output_bytes: packageOutput.length,
    package_output_sha256: sha256(packageOutput),
    capability_package_bytes: capabilityPackage.length,
    variable_font: {
      package_bytes: variableFontPackage.length,
      canonical_hash: variableObserved.canonical_hash,
      raster_sha256: variableObserved.raster.rgba_sha256,
      coordinates: variableRun?.variation_coordinates,
    },
    wasm_bytes: fs.statSync(path.join(path.dirname(path.resolve(bindingPath)), "nuif_bg.wasm")).size,
    output_sha256: sha256(output),
    checks,
  }, null, 2) + "\n");
  supportedCapabilityDocument.free();
  variableDocument.free();
  variableStructural.free();
  structuralCapabilityDocument.free();
  packageDocument.free();
  cborDocument.free();
  document.free();
  if (!passed) fail("WebAssembly smoke report failed");
}

main();
