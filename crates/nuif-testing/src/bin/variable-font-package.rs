use nuif_codec::canonical_hash;
use nuif_core::{
    Asset, AssetId, AssetKind, AssetPortability, Document, Entity, EntityId, EntityKind, FontAsset,
    ResourceDescriptor, ResourceDigest, ResourceRole,
};
use nuif_font::{
    OPENTYPE_VARIABLE_TRUETYPE_PROFILE, VariableFontInspection, inspect_opentype_variable_metadata,
    validate_variable_font_asset_candidate,
};
use nuif_package::{NuifPackage, PackageMode, ResourceResolver};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::PathBuf;

const FONT: &[u8] = include_bytes!(
    "../../../../conformance/font/fixtures/roboto-flex-mvar-subset/RobotoFlex-MVAR-subset.ttf"
);
const ASSET_ID: AssetId = AssetId::new(0xf1);

fn main() {
    if let Err(error) = run() {
        eprintln!("variable-font-package: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let output = output_path()?;
    let inspection =
        inspect_opentype_variable_metadata(FONT, 0).map_err(|error| error.to_string())?;
    let (package_trials, digest) = package_trials(&inspection)?;
    let policy_trials = policy_trials(&inspection, &digest);
    let passed = package_trials
        .iter()
        .chain(&policy_trials)
        .all(passed_trial);
    let report = report(&package_trials, &policy_trials, &digest, passed);
    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(
        &output,
        serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    println!(
        "variable font package candidate: {} package, {} policy trials, status {}",
        package_trials.len(),
        policy_trials.len(),
        if passed { "passed" } else { "failed" }
    );
    if passed {
        Ok(())
    } else {
        Err(format!("report failed; inspect {}", output.display()))
    }
}

fn package_trials(
    inspection: &VariableFontInspection,
) -> Result<(Vec<Value>, ResourceDigest), String> {
    let mut package = resource_only_package(PackageMode::Portable);
    let digest = package
        .add_embedded(FONT.to_vec(), "font/ttf", ResourceRole::Authoring, None)
        .map_err(|error| error.to_string())?;
    let candidate = candidate_asset(inspection, digest.clone());
    let validated = validate_variable_font_asset_candidate(&candidate, FONT)
        .map_err(|error| error.to_string())?;
    let original_document_hash = canonical_hash(&package.document).map_err(|e| e.to_string())?;
    let encoded = package.encode().map_err(|error| error.to_string())?;
    let decoded = NuifPackage::decode(&encoded).map_err(|error| error.to_string())?;
    let fixed = decoded.encode().map_err(|error| error.to_string())?;
    let mut edited = decoded.clone();
    add_unrelated_surface(&mut edited.document);
    let edited_bytes = edited.encode().map_err(|error| error.to_string())?;
    let edited = NuifPackage::decode(&edited_bytes).map_err(|error| error.to_string())?;
    let edited_document_hash = canonical_hash(&edited.document).map_err(|e| e.to_string())?;

    let mut bound = resource_only_package(PackageMode::Portable);
    let bound_digest = bound
        .add_embedded(FONT.to_vec(), "font/ttf", ResourceRole::Authoring, None)
        .map_err(|error| error.to_string())?;
    bound.document.assets.insert(ASSET_ID, candidate);
    let bound_error = bound.encode().err().map_or_else(
        || "candidate package was accepted".to_owned(),
        |error| error.to_string(),
    );

    let linked = linked_trials(&digest)?;
    let mut trials = vec![
        trial(
            "candidate_asset_metadata_and_policy_validate",
            validated.coordinates.len() == inspection.axes.len()
                && validated
                    .coordinates
                    .iter()
                    .all(|coordinate| coordinate.normalized_2_14 == 0),
            &json!({"axes": validated.coordinates.len()}),
        ),
        trial(
            "resource_only_package_byte_fixpoint",
            encoded == fixed,
            &json!({"package_bytes": encoded.len()}),
        ),
        trial(
            "exact_bytes_survive_unrelated_semantic_edit",
            edited.embedded(&digest) == Some(FONT)
                && original_document_hash != edited_document_hash,
            &json!({"font_bytes": FONT.len(), "package_bytes": edited_bytes.len()}),
        ),
        trial(
            "candidate_capability_is_declared",
            decoded
                .required_capabilities
                .contains(OPENTYPE_VARIABLE_TRUETYPE_PROFILE),
            &json!({"capability": OPENTYPE_VARIABLE_TRUETYPE_PROFILE}),
        ),
        trial(
            "typed_package_binding_remains_fail_closed",
            bound_digest == digest
                && bound_error.contains("variable, CFF, color, bitmap or SVG table"),
            &json!({"package_dispatch_enabled": false, "error": bound_error}),
        ),
    ];
    trials.extend(linked);
    Ok((trials, digest))
}

fn linked_trials(digest: &ResourceDigest) -> Result<Vec<Value>, String> {
    let mut package = resource_only_package(PackageMode::Authoring);
    package
        .add_linked(
            digest.clone(),
            FONT.len() as u64,
            "font/ttf",
            ResourceRole::Authoring,
            "https://example.invalid/RobotoFlex-MVAR-subset.ttf",
            None,
        )
        .map_err(|error| error.to_string())?;
    let encoded = package.encode().map_err(|error| error.to_string())?;
    let decoded = NuifPackage::decode(&encoded).map_err(|error| error.to_string())?;
    let without_resolver = decoded.resolve_resource(digest, None).is_err();
    let exact = decoded
        .resolve_resource(digest, Some(&mut BytesResolver(FONT.to_vec())))
        .map_err(|error| error.to_string())?;
    let mut corrupt = FONT.to_vec();
    corrupt[0] ^= 1;
    let corrupt_rejected = decoded
        .resolve_resource(digest, Some(&mut BytesResolver(corrupt)))
        .is_err();
    Ok(vec![
        trial(
            "linked_resolution_requires_explicit_resolver",
            without_resolver,
            &json!({}),
        ),
        trial(
            "digest_pinned_linked_bytes_validate",
            exact == FONT,
            &json!({"bytes": exact.len()}),
        ),
        trial(
            "linked_digest_mismatch_rejected",
            corrupt_rejected,
            &json!({}),
        ),
    ])
}

fn policy_trials(inspection: &VariableFontInspection, digest: &ResourceDigest) -> Vec<Value> {
    let baseline = candidate_asset(inspection, digest.clone());
    let mut cases = Vec::new();
    cases.push(mutated_rejection("missing_axis", &baseline, |font| {
        font.axes.pop_first();
    }));
    cases.push(mutated_rejection("unknown_axis", &baseline, |font| {
        font.axes.insert("TEST".to_owned(), 0.0);
    }));
    cases.push(mutated_rejection("out_of_range_axis", &baseline, |font| {
        let axis = font.axes.first_entry().expect("fixture axes");
        *axis.into_mut() = f64::MAX;
    }));
    cases.push(mutated_rejection("stale_family_name", &baseline, |font| {
        font.names = vec!["not Roboto Flex".to_owned()];
    }));
    cases.push(mutated_rejection("stale_coverage", &baseline, |font| {
        font.coverage.clear();
    }));
    cases.push(mutated_rejection(
        "wrong_decoder_profile",
        &baseline,
        |font| {
            font.policy_evidence
                .insert("font.decoder_profile".to_owned(), "other".to_owned());
        },
    ));
    cases.push(mutated_rejection("wrong_fs_type", &baseline, |font| {
        font.policy_evidence
            .insert("opentype.fs_type".to_owned(), "0xffff".to_owned());
    }));
    cases.push(mutated_rejection(
        "invalid_feature_tag",
        &baseline,
        |font| {
            font.features.insert("not-a-tag".to_owned(), 1);
        },
    ));
    cases.push(mutated_rejection("blank_license", &baseline, |font| {
        font.policy_evidence
            .insert("license.expression".to_owned(), " ".to_owned());
    }));
    cases.push(mutated_rejection(
        "review_not_approved",
        &baseline,
        |font| {
            font.policy_evidence
                .insert("license.embedding_review".to_owned(), "pending".to_owned());
        },
    ));
    let mut unavailable = baseline;
    unavailable.portability = AssetPortability::Unavailable;
    cases.push(trial(
        "unavailable_exact_resource_rejected",
        validate_variable_font_asset_candidate(&unavailable, FONT).is_err(),
        &json!({}),
    ));
    cases
}

fn candidate_asset(inspection: &VariableFontInspection, resource: ResourceDigest) -> Asset {
    Asset {
        schema_version: 1,
        id: ASSET_ID,
        name: Some("Roboto Flex variable package candidate".to_owned()),
        resource: Some(resource),
        portability: AssetPortability::Portable,
        kind: AssetKind::Font(FontAsset {
            face_index: 0,
            names: inspection.font.names.clone(),
            axes: inspection
                .axes
                .iter()
                .map(|axis| (axis.tag.clone(), f64::from(axis.default_16_16) / 65_536.0))
                .collect::<BTreeMap<_, _>>(),
            features: BTreeMap::new(),
            coverage: inspection.font.coverage.clone(),
            policy_evidence: BTreeMap::from([
                (
                    "font.decoder_profile".to_owned(),
                    OPENTYPE_VARIABLE_TRUETYPE_PROFILE.to_owned(),
                ),
                (
                    "opentype.fs_type".to_owned(),
                    format!("0x{:04x}", inspection.font.fs_type),
                ),
                ("license.expression".to_owned(), "OFL-1.1".to_owned()),
                ("license.embedding_review".to_owned(), "approved".to_owned()),
            ]),
        }),
    }
}

fn resource_only_package(mode: PackageMode) -> NuifPackage {
    let mut package = NuifPackage::new(Document::empty(EntityId::new(1)), mode);
    package.required_capabilities = BTreeSet::from([OPENTYPE_VARIABLE_TRUETYPE_PROFILE.to_owned()]);
    package
}

fn add_unrelated_surface(document: &mut Document) {
    let id = EntityId::new(2);
    let mut entity = Entity::new(id, EntityKind::Surface);
    entity.name = Some("unrelated package edit".to_owned());
    document.entities.insert(id, entity);
    document.roots.push(id);
}

fn mutated_rejection(name: &str, baseline: &Asset, mutate: impl FnOnce(&mut FontAsset)) -> Value {
    let mut asset = baseline.clone();
    let AssetKind::Font(font) = &mut asset.kind else {
        unreachable!();
    };
    mutate(font);
    trial(
        name,
        validate_variable_font_asset_candidate(&asset, FONT).is_err(),
        &json!({}),
    )
}

fn report(
    package_trials: &[Value],
    policy_trials: &[Value],
    digest: &ResourceDigest,
    passed: bool,
) -> Value {
    json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:variable-font-package-candidate",
        "status": if passed { "passed" } else { "failed" },
        "profile": OPENTYPE_VARIABLE_TRUETYPE_PROFILE,
        "fixture": {
            "bytes": FONT.len(),
            "digest": digest,
            "license_expression": "OFL-1.1",
            "embedding_review": "approved",
        },
        "package_trials": package_trials,
        "policy_trials": policy_trials,
        "summary": {
            "package": package_trials.len(),
            "policy": policy_trials.len(),
            "blocking_failures": package_trials.iter().chain(policy_trials).filter(|trial| !passed_trial(trial)).count(),
        },
        "boundary": {
            "asset_binding_deferred": true,
            "reference_runtime_enabled": false,
            "reason": "the package dispatcher still rejects typed variable-font assets until layout and rendering consume the same normalized coordinates",
        },
        "non_claims": [
            "resource-only package retention does not admit typed variable-font assets",
            "candidate asset validation does not establish layout rendering or cross-surface parity",
            "one OFL fixture does not establish a broad rights-reviewed corpus",
        ],
    })
}

fn trial(name: &str, passed: bool, evidence: &Value) -> Value {
    json!({"name": name, "status": if passed { "passed" } else { "failed" }, "evidence": evidence})
}

fn passed_trial(value: &Value) -> bool {
    value.get("status").and_then(Value::as_str) == Some("passed")
}

struct BytesResolver(Vec<u8>);

impl ResourceResolver for BytesResolver {
    fn resolve(&mut self, _: &ResourceDescriptor) -> Result<Vec<u8>, String> {
        Ok(self.0.clone())
    }
}

fn output_path() -> Result<PathBuf, String> {
    let mut arguments = env::args().skip(1);
    let mut output = PathBuf::from("target/variable-font-package-report.json");
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--output" => {
                output = PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| "--output requires a path".to_owned())?,
                );
            }
            "--help" | "-h" => {
                return Err("usage: variable-font-package [--output <json>]".to_owned());
            }
            _ => return Err(format!("unknown argument: {argument}")),
        }
    }
    Ok(output)
}
