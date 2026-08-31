use nuif_capture::{browser_capture_provider_manifest, screenshot_baseline_provider_manifest};
use nuif_core::ResourceDigest;
use nuif_reconstruct::provider::{
    ExecutionMode, InventoryFormat, PROVIDER_MANIFEST_PROFILE, ProviderArtifact,
    ProviderArtifactRole, ProviderCapability, ProviderError, ProviderIdentity, ProviderManifest,
    ProviderMaturity, SupplyChainInventory,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    if let Err(error) = run() {
        eprintln!("reconstruction-provider-manifest: {error}");
        std::process::exit(1);
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "the gate keeps the manifest, adversarial mutations, and emitted evidence in one reviewable path"
)]
fn run() -> Result<(), String> {
    let output = output_path()?;
    let screenshot = screenshot_baseline_provider_manifest();
    let browser = browser_capture_provider_manifest();
    let learned = learned_fixture();
    let screenshot_identity = screenshot.identity().map_err(|error| error.to_string())?;
    let browser_identity = browser.identity().map_err(|error| error.to_string())?;
    let learned_identity = learned.identity().map_err(|error| error.to_string())?;
    let encoded = learned.encode().map_err(|error| error.to_string())?;
    let decoded = ProviderManifest::decode(&encoded).map_err(|error| error.to_string())?;

    let mut drifted = learned.clone();
    drifted.artifacts[1].digest = digest("changed-model-weights");
    let drifted_identity = drifted.identity().map_err(|error| error.to_string())?;

    let mut no_inventory = learned.clone();
    no_inventory.inventory = None;
    let mut no_card = learned.clone();
    no_card.model_card = None;
    let mut duplicate = learned.clone();
    let duplicate_id = duplicate.artifacts[0].id.clone();
    duplicate.artifacts[1].id.clone_from(&duplicate_id);
    let mut two_implementations = learned.clone();
    two_implementations.artifacts[1].role = ProviderArtifactRole::Implementation;
    let invalid_identity = ProviderIdentity {
        kind: "proposal".to_owned(),
        manifest: ResourceDigest("sha256:not-a-digest".to_owned()),
    };

    let checks = vec![
        check(
            "development_baselines_are_manifest_bound",
            screenshot.validate().is_ok()
                && browser.validate().is_ok()
                && screenshot.maturity == ProviderMaturity::Development
                && browser.maturity == ProviderMaturity::Development
                && screenshot.inventory.is_none()
                && browser.inventory.is_none()
                && screenshot_identity.validate().is_ok()
                && browser_identity.validate().is_ok(),
        ),
        check(
            "learned_manifest_reaches_identity_fixpoint",
            decoded == learned && decoded.identity().ok() == Some(learned_identity.clone()),
        ),
        check(
            "artifact_change_changes_provider_identity",
            drifted_identity != learned_identity,
        ),
        check(
            "released_or_learned_provider_requires_inventory",
            matches!(
                no_inventory.validate(),
                Err(ProviderError::InvalidManifest(
                    "released or learned providers require a supply-chain inventory"
                ))
            ),
        ),
        check(
            "learned_provider_requires_model_card",
            matches!(
                no_card.validate(),
                Err(ProviderError::InvalidManifest(
                    "learned artifacts require a model card"
                ))
            ),
        ),
        check(
            "artifact_ids_are_unique",
            matches!(duplicate.validate(), Err(ProviderError::InvalidArtifact)),
        ),
        check(
            "exactly_one_implementation_is_required",
            matches!(
                two_implementations.validate(),
                Err(ProviderError::InvalidManifest(
                    "exactly one implementation artifact is required"
                ))
            ),
        ),
        check(
            "invalid_provider_identity_fails_closed",
            matches!(
                invalid_identity.validate(),
                Err(ProviderError::InvalidIdentity)
            ),
        ),
    ];
    let passed = checks.iter().all(|check| check["passed"] == true);
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:reconstruction-provider-manifest",
        "status": if passed { "passed" } else { "failed" },
        "source": source_identity(),
        "development_baselines": {
            "screenshot": { "manifest": screenshot, "identity": screenshot_identity },
            "browser": { "manifest": browser, "identity": browser_identity }
        },
        "synthetic_learned_fixture": {
            "manifest": learned,
            "identity": learned_identity,
            "canonical_cbor_sha256": sha256(&encoded)
        },
        "checks": checks,
        "non_claims": [
            "the learned-provider record uses synthetic digests and is not a released model",
            "the development screenshot and browser manifests contain no learned artifact or model-accuracy claim",
            "an inventory digest does not prove that an SPDX or CycloneDX document is complete or correct",
            "a model card and supply-chain inventory do not confer benchmark, format, safety, or legal status",
            "no remote provider is called and no model weights are loaded"
        ]
    });
    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut bytes = serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    fs::write(&output, bytes).map_err(|error| error.to_string())?;
    println!(
        "reconstruction provider manifest: {} checks, status {}",
        checks.len(),
        if passed { "passed" } else { "failed" }
    );
    if passed {
        Ok(())
    } else {
        Err(format!("report failed; inspect {}", output.display()))
    }
}

fn learned_fixture() -> ProviderManifest {
    ProviderManifest {
        schema_version: 1,
        profile: PROVIDER_MANIFEST_PROFILE.to_owned(),
        provider_id: "synthetic-learned-provider-fixture-1".to_owned(),
        kind: "reconstruction-proposal".to_owned(),
        maturity: ProviderMaturity::Released,
        capabilities: BTreeSet::from([
            ProviderCapability::Ocr,
            ProviderCapability::UiGrounding,
            ProviderCapability::Proposal,
            ProviderCapability::Correction,
        ]),
        execution_modes: BTreeSet::from([ExecutionMode::Local, ExecutionMode::Remote]),
        input_profiles: BTreeSet::from(["nuif-observations-0".to_owned()]),
        output_profiles: BTreeSet::from(["nuif-proposal-0".to_owned()]),
        artifacts: vec![
            artifact("implementation", ProviderArtifactRole::Implementation),
            artifact("model", ProviderArtifactRole::ModelWeights),
            artifact("processor", ProviderArtifactRole::Processor),
            artifact("adapter", ProviderArtifactRole::Adapter),
            artifact("quantization", ProviderArtifactRole::Quantization),
            artifact("prompt", ProviderArtifactRole::PromptTemplate),
            artifact("tools", ProviderArtifactRole::ToolConfiguration),
        ],
        model_card: Some(digest("synthetic-model-card")),
        inventory: Some(SupplyChainInventory {
            format: InventoryFormat::Spdx301,
            artifact: digest("synthetic-spdx-3.0.1-inventory"),
        }),
    }
}

fn artifact(id: &str, role: ProviderArtifactRole) -> ProviderArtifact {
    ProviderArtifact {
        id: id.to_owned(),
        role,
        digest: digest(&format!("synthetic-provider-artifact:{id}")),
        format: if role == ProviderArtifactRole::ModelWeights {
            "safetensors"
        } else {
            "opaque-test-fixture"
        }
        .to_owned(),
        version: "fixture-1".to_owned(),
    }
}

fn digest(value: &str) -> ResourceDigest {
    ResourceDigest::from_sha256_hex(sha256(value.as_bytes()))
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn check(name: &str, passed: bool) -> Value {
    json!({ "name": name, "passed": passed })
}

fn source_identity() -> Value {
    json!({
        "revision": command_text("git", &["rev-parse", "HEAD"]),
        "dirty": Command::new("git")
            .args(["diff", "--quiet", "--ignore-submodules", "HEAD", "--"])
            .status()
            .is_ok_and(|status| !status.success()),
        "toolchain": command_text("rustc", &["--version"]),
        "os": env::consts::OS,
        "architecture": env::consts::ARCH,
    })
}

fn command_text(program: &str, arguments: &[&str]) -> Option<String> {
    Command::new(program)
        .args(arguments)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_owned())
}

fn output_path() -> Result<PathBuf, String> {
    let mut arguments = env::args().skip(1);
    let mut output = PathBuf::from("target/reconstruction-provider-manifest-report.json");
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--output" => {
                output = arguments
                    .next()
                    .ok_or_else(|| "--output requires a path".to_owned())?
                    .into();
            }
            "--help" | "-h" => {
                return Err("usage: reconstruction-provider-manifest [--output <json>]".to_owned());
            }
            unknown => return Err(format!("unknown argument {unknown:?}")),
        }
    }
    Ok(output)
}
