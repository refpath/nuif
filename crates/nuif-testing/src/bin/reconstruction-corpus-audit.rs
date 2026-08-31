use nuif_core::ResourceDigest;
use nuif_reconstruct::evaluation::EvaluationSuite;
use nuif_reconstruct::evaluation::corpus::{
    ArtifactDisclosure, ArtifactKind, CORPUS_MANIFEST_PROFILE, CollectionClass, ConsentBasis,
    CorpusArtifact, CorpusAudit, CorpusError, CorpusExample, CorpusManifest, CorpusSplit,
    LeakageGroups, Permission, PermittedUses, RightsRecord, SensitivityReview,
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
        eprintln!("reconstruction-corpus-audit: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let output = output_path()?;
    let manifest = fixture_manifest();
    let audit = manifest.audit().map_err(|error| error.to_string())?;

    let manifest_bytes = serde_json::to_vec(&manifest).map_err(|error| error.to_string())?;
    let decoded_manifest: CorpusManifest =
        serde_json::from_slice(&manifest_bytes).map_err(|error| error.to_string())?;
    let audit_bytes = serde_json::to_vec(&audit).map_err(|error| error.to_string())?;
    let decoded_audit: CorpusAudit =
        serde_json::from_slice(&audit_bytes).map_err(|error| error.to_string())?;

    let mut drifted_audit = audit.clone();
    drifted_audit.examples = drifted_audit.examples.saturating_add(1);

    let checks = vec![
        check(
            "typed_manifest_and_audit_round_trip",
            decoded_manifest == manifest
                && decoded_audit.validate_against(&decoded_manifest).is_ok(),
        ),
        check(
            "all_partitions_are_explicit",
            [
                CorpusSplit::Adaptation,
                CorpusSplit::Calibration,
                CorpusSplit::Validation,
                CorpusSplit::Test,
            ]
            .into_iter()
            .all(|split| audit.split_examples.get(&split).copied() == Some(1)),
        ),
        check(
            "every_family_dimension_is_split_isolated",
            leakage_dimensions_are_rejected(&manifest),
        ),
        check(
            "artifact_identity_is_split_isolated",
            artifact_leakage_is_rejected(&manifest),
        ),
        check(
            "split_requires_its_permitted_use",
            split_permissions_are_enforced(&manifest),
        ),
        check(
            "disclosure_does_not_replace_permission",
            disclosure_and_permission_are_independent(&manifest),
        ),
        check(
            "screenshot_suite_rejects_source_claim",
            screenshot_source_claim_is_rejected(&manifest),
        ),
        check(
            "private_capture_requires_withdrawal_policy",
            private_capture_without_withdrawal_is_rejected(),
        ),
        check(
            "near_duplicate_assignment_is_required",
            missing_near_duplicate_group_is_rejected(&manifest),
        ),
        check(
            "derived_audit_drift_is_rejected",
            matches!(
                drifted_audit.validate_against(&manifest),
                Err(CorpusError::AuditDrift)
            ),
        ),
    ];
    let passed = checks.iter().all(|check| check["passed"] == true);
    let artifact = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:reconstruction-corpus-integrity",
        "status": if passed { "passed" } else { "failed" },
        "source": source_identity(),
        "manifest": manifest,
        "audit": audit,
        "checks": checks,
        "non_claims": [
            "the deterministic records are synthetic policy fixtures, not a real reconstruction corpus",
            "the auditor validates declarations and digests; it does not interpret licenses or prove consent",
            "the auditor cannot detect an unrecorded family or near duplicate",
            "split isolation does not establish representativeness, sample independence, or statistical power",
            "withheld target metadata in this report is synthetic and does not demonstrate a private evaluator"
        ]
    });
    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut bytes = serde_json::to_vec_pretty(&artifact).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    fs::write(&output, bytes).map_err(|error| error.to_string())?;
    println!(
        "reconstruction corpus audit: {} checks, status {}",
        checks.len(),
        if passed { "passed" } else { "failed" }
    );
    if passed {
        Ok(())
    } else {
        Err(format!("report failed; inspect {}", output.display()))
    }
}

fn fixture_manifest() -> CorpusManifest {
    CorpusManifest {
        schema_version: 1,
        profile: CORPUS_MANIFEST_PROFILE.to_owned(),
        corpus_id: "synthetic-corpus-contract-1".to_owned(),
        snapshot: digest("synthetic corpus fixture snapshot v1"),
        dataset_card: digest("synthetic corpus fixture dataset card v1"),
        evaluator_artifact: digest("nuif reconstruction evaluator fixture v1"),
        examples: vec![
            synthetic_example("adaptation-1", CorpusSplit::Adaptation),
            synthetic_example("calibration-1", CorpusSplit::Calibration),
            source_example("validation-1", CorpusSplit::Validation),
            screenshot_example("test-1", CorpusSplit::Test),
        ],
    }
}

fn synthetic_example(id: &str, split: CorpusSplit) -> CorpusExample {
    example(
        id,
        split,
        EvaluationSuite::SyntheticExact,
        CollectionClass::ProjectSynthetic,
        ConsentBasis::ProjectGenerated,
        SensitivityReview::SyntheticNoPersonalData,
        Permission::Allowed,
        Permission::Allowed,
        None,
        false,
    )
}

fn source_example(id: &str, split: CorpusSplit) -> CorpusExample {
    example(
        id,
        split,
        EvaluationSuite::SourceBacked,
        CollectionClass::HostExport,
        ConsentBasis::ContractualAuthorization,
        SensitivityReview::HumanReviewedNoSensitiveData,
        Permission::Prohibited,
        Permission::Prohibited,
        Some(digest(&format!("{id}:withdrawal-policy"))),
        true,
    )
}

fn screenshot_example(id: &str, split: CorpusSplit) -> CorpusExample {
    example(
        id,
        split,
        EvaluationSuite::RealScreenshot,
        CollectionClass::PublicWeb,
        ConsentBasis::ContractualAuthorization,
        SensitivityReview::HumanReviewedNoSensitiveData,
        Permission::Prohibited,
        Permission::Prohibited,
        Some(digest(&format!("{id}:withdrawal-policy"))),
        false,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "the policy fixture keeps every rights and evidence choice visible at each call site"
)]
fn example(
    id: &str,
    split: CorpusSplit,
    suite: EvaluationSuite,
    collection: CollectionClass,
    consent_basis: ConsentBasis,
    sensitivity_review: SensitivityReview,
    adaptation: Permission,
    redistribution: Permission,
    withdrawal_policy_artifact: Option<ResourceDigest>,
    with_source: bool,
) -> CorpusExample {
    let mut inputs = vec![CorpusArtifact {
        kind: ArtifactKind::Screenshot,
        digest: digest(&format!("{id}:screenshot")),
        disclosure: ArtifactDisclosure::Public,
    }];
    if with_source {
        inputs.push(CorpusArtifact {
            kind: ArtifactKind::SourceDocument,
            digest: digest(&format!("{id}:source")),
            disclosure: ArtifactDisclosure::Restricted,
        });
    }
    CorpusExample {
        example_id: id.to_owned(),
        split,
        suite,
        collection,
        inputs,
        targets: vec![CorpusArtifact {
            kind: ArtifactKind::TargetDocument,
            digest: digest(&format!("{id}:target")),
            disclosure: if split == CorpusSplit::Test {
                ArtifactDisclosure::Withheld
            } else {
                ArtifactDisclosure::Restricted
            },
        }],
        rights: RightsRecord {
            license_expression: if collection == CollectionClass::ProjectSynthetic {
                "Apache-2.0 OR MIT"
            } else {
                "LicenseRef-Evaluation-Contract"
            }
            .to_owned(),
            evidence_artifact: digest(&format!("{id}:rights-evidence")),
            consent_basis,
            permitted_uses: PermittedUses {
                evaluation: Permission::Allowed,
                calibration: if split == CorpusSplit::Calibration {
                    Permission::Allowed
                } else {
                    Permission::Prohibited
                },
                adaptation,
                redistribution,
            },
            sensitivity_review,
            withdrawal_policy_artifact,
        },
        leakage: LeakageGroups {
            origin: format!("origin-{id}"),
            template: Some(format!("template-{id}")),
            components: BTreeSet::from([format!("component-{id}")]),
            fonts: BTreeSet::from([format!("font-{id}")]),
            resources: BTreeSet::from([format!("resource-{id}")]),
            generators: BTreeSet::from([format!("generator-{id}")]),
            near_duplicates: BTreeSet::from([format!("near-{id}")]),
        },
    }
}

fn leakage_dimensions_are_rejected(manifest: &CorpusManifest) -> bool {
    [
        "origin",
        "template",
        "component",
        "font",
        "resource",
        "generator",
        "near_duplicate",
    ]
    .into_iter()
    .all(|dimension| {
        let mut candidate = manifest.clone();
        let shared = "injected-shared-family".to_owned();
        set_group(&mut candidate.examples[0].leakage, dimension, &shared);
        set_group(&mut candidate.examples[3].leakage, dimension, &shared);
        matches!(
            candidate.audit(),
            Err(CorpusError::GroupLeakage {
                dimension: leaked,
                ..
            }) if leaked == dimension
        )
    })
}

fn set_group(groups: &mut LeakageGroups, dimension: &str, value: &str) {
    match dimension {
        "origin" => value.clone_into(&mut groups.origin),
        "template" => groups.template = Some(value.to_owned()),
        "component" => {
            groups.components.insert(value.to_owned());
        }
        "font" => {
            groups.fonts.insert(value.to_owned());
        }
        "resource" => {
            groups.resources.insert(value.to_owned());
        }
        "generator" => {
            groups.generators.insert(value.to_owned());
        }
        "near_duplicate" => {
            groups.near_duplicates.insert(value.to_owned());
        }
        _ => unreachable!(),
    }
}

fn artifact_leakage_is_rejected(manifest: &CorpusManifest) -> bool {
    let mut candidate = manifest.clone();
    candidate.examples[3].inputs[0].digest = candidate.examples[0].inputs[0].digest.clone();
    matches!(candidate.audit(), Err(CorpusError::ArtifactLeakage { .. }))
}

fn split_permissions_are_enforced(manifest: &CorpusManifest) -> bool {
    let mut forbidden = manifest.clone();
    forbidden.examples[0].rights.permitted_uses.adaptation = Permission::Prohibited;
    let adaptation_rejected = matches!(
        forbidden.audit(),
        Err(CorpusError::UseNotPermitted {
            usage: "adaptation",
            ..
        })
    );
    let mut uncalibrated = manifest.clone();
    uncalibrated.examples[1].rights.permitted_uses.calibration = Permission::Prohibited;
    let calibration_rejected = matches!(
        uncalibrated.audit(),
        Err(CorpusError::UseNotPermitted {
            usage: "calibration",
            ..
        })
    );
    let mut unevaluated = manifest.clone();
    unevaluated.examples[3].rights.permitted_uses.evaluation = Permission::Prohibited;
    adaptation_rejected
        && calibration_rejected
        && matches!(
            unevaluated.audit(),
            Err(CorpusError::UseNotPermitted {
                usage: "evaluation",
                ..
            })
        )
}

fn disclosure_and_permission_are_independent(manifest: &CorpusManifest) -> bool {
    let mut candidate = manifest.clone();
    candidate.examples[0].targets[0].disclosure = ArtifactDisclosure::Withheld;
    candidate.audit().is_ok()
}

fn screenshot_source_claim_is_rejected(manifest: &CorpusManifest) -> bool {
    let mut candidate = manifest.clone();
    candidate.examples[3].inputs.push(CorpusArtifact {
        kind: ArtifactKind::SourceDocument,
        digest: digest("injected screenshot-only source"),
        disclosure: ArtifactDisclosure::Restricted,
    });
    matches!(candidate.audit(), Err(CorpusError::SuiteEvidence(_)))
}

fn private_capture_without_withdrawal_is_rejected() -> bool {
    let mut manifest = fixture_manifest();
    let mut private = screenshot_example("private-1", CorpusSplit::Test);
    private.collection = CollectionClass::PrivateAuthenticated;
    private.rights.consent_basis = ConsentBasis::ExplicitOptIn;
    private.rights.sensitivity_review = SensitivityReview::HumanReviewedRestricted;
    private.rights.withdrawal_policy_artifact = None;
    manifest.examples = vec![private];
    matches!(manifest.audit(), Err(CorpusError::InvalidRights(_)))
}

fn missing_near_duplicate_group_is_rejected(manifest: &CorpusManifest) -> bool {
    let mut candidate = manifest.clone();
    candidate.examples[0].leakage.near_duplicates.clear();
    matches!(candidate.audit(), Err(CorpusError::InvalidGroups(_)))
}

fn digest(value: &str) -> ResourceDigest {
    ResourceDigest::from_sha256_hex(format!("{:x}", Sha256::digest(value.as_bytes())))
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
    let mut output = PathBuf::from("target/reconstruction-corpus-audit-report.json");
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--output" => {
                output = arguments
                    .next()
                    .ok_or_else(|| "--output requires a path".to_owned())?
                    .into();
            }
            "--help" | "-h" => {
                return Err("usage: reconstruction-corpus-audit [--output <json>]".to_owned());
            }
            unknown => return Err(format!("unknown argument {unknown:?}")),
        }
    }
    Ok(output)
}
