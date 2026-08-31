use nuif_behavior::{
    BEHAVIOR_ATTACHMENT_PROFILE, BEHAVIOR_MEDIA_TYPE, BEHAVIOR_PROFILE, BehaviorAttachmentError,
    attach_behavior, attached_behavior, behavior_fixture,
};
use nuif_codec::{canonical_hash, encode_canonical_record};
use nuif_core::{Document, EntityId, ResourceRole};
use nuif_package::{MIME_TYPE, NuifPackage, PackageMode, digest};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    if let Err(error) = run() {
        eprintln!("behavior-package: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let paths = paths()?;
    let (document, program, _) = behavior_fixture();
    let document_hash = canonical_hash(&document).map_err(stringify)?;
    let mut package = NuifPackage::new(document.clone(), PackageMode::Portable);
    let plain_package_hash = package.package_hash().map_err(stringify)?;
    let behavior_digest = attach_behavior(&mut package, &program).map_err(stringify)?;
    let package_bytes = package.encode().map_err(stringify)?;
    let package_hash = package.package_hash().map_err(stringify)?;
    let decoded = NuifPackage::decode(&package_bytes).map_err(stringify)?;
    let attachment = attached_behavior(&decoded)
        .map_err(stringify)?
        .ok_or_else(|| "attached behavior disappeared during package decode".to_owned())?;
    let descriptor = decoded
        .resources
        .get(&behavior_digest)
        .ok_or_else(|| "behavior descriptor disappeared during package decode".to_owned())?;
    let behavior_bytes = decoded
        .embedded(&behavior_digest)
        .ok_or_else(|| "behavior bytes disappeared during package decode".to_owned())?;
    let behavior_path = descriptor_path(descriptor)?;
    let behavior_supported = BTreeSet::from([BEHAVIOR_PROFILE.to_owned()]);
    let supported_report = decoded.capability_report(&behavior_supported);
    let unsupported_report = decoded.capability_report(&BTreeSet::new());

    let checks = json!({
        "canonical_behavior_bytes": encode_canonical_record(&attachment.program).map_err(stringify)? == behavior_bytes,
        "document_hash_unchanged": canonical_hash(&decoded.document).map_err(stringify)? == document_hash,
        "package_hash_changed": package_hash != plain_package_hash,
        "package_byte_fixpoint": decoded.encode().map_err(stringify)? == package_bytes,
        "program_roundtrip_exact": attachment.program == program,
        "digest_roundtrip_exact": attachment.digest == behavior_digest,
        "manifest_requires_behavior_profile": decoded.required_capabilities.contains(BEHAVIOR_PROFILE),
        "package_capability_negotiation_accepts_exact_support": supported_report.fully_supported && decoded.require_capabilities(&behavior_supported).is_ok(),
        "package_capability_negotiation_reports_missing": !unsupported_report.fully_supported && unsupported_report.missing_required == behavior_supported && decoded.require_capabilities(&BTreeSet::new()).is_err(),
        "resource_is_embedded_source": descriptor.role == ResourceRole::Source && descriptor.derivation.is_none(),
        "resource_path_is_content_addressed": behavior_path == format!("blobs/sha256/{}", behavior_digest.sha256_hex().unwrap_or_default()),
        "absent_attachment_is_optional": attached_behavior(&NuifPackage::new(document.clone(), PackageMode::Portable)).map_err(stringify)?.is_none(),
        "capability_without_resource_rejected": capability_without_resource_rejected(document.clone()),
        "resource_without_capability_rejected": resource_without_capability_rejected(document.clone(), behavior_bytes),
        "linked_behavior_rejected": linked_behavior_rejected(document.clone(), behavior_bytes),
        "duplicate_behavior_rejected": duplicate_behavior_rejected(document.clone(), &program),
        "non_cbor_behavior_rejected": non_cbor_behavior_rejected(document.clone()),
        "document_rebinding_rejected": document_rebinding_rejected(package.clone()),
        "corrupt_package_rejected": corrupt_package_rejected(&package_bytes, behavior_bytes),
    });
    let passed = checks
        .as_object()
        .is_some_and(|checks| checks.values().all(|value| value == true));
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:behavior-package-resource",
        "status": if passed { "passed" } else { "failed" },
        "profile": BEHAVIOR_ATTACHMENT_PROFILE,
        "behavior_profile": BEHAVIOR_PROFILE,
        "media_type": BEHAVIOR_MEDIA_TYPE,
        "package_profile": nuif_package::PROFILE,
        "document_hash": document_hash,
        "behavior_digest": behavior_digest,
        "behavior_bytes": behavior_bytes.len(),
        "behavior_path": behavior_path,
        "package_bytes": package_bytes.len(),
        "package_hash": package_hash,
        "checks": checks,
        "boundaries": [
            "behavior remains outside the canonical Document schema",
            "generic package decode verifies and preserves the inert resource but does not execute it",
            "host capability authorization remains a separate runtime decision",
            "the media type and attachment profile are provisional research identifiers"
        ]
    });
    let expected = json!({
        "schema_version": 1,
        "profile": BEHAVIOR_ATTACHMENT_PROFILE,
        "mime_type": String::from_utf8_lossy(MIME_TYPE),
        "behavior_media_type": BEHAVIOR_MEDIA_TYPE,
        "behavior_digest": behavior_digest,
        "behavior_sha256": sha256(behavior_bytes),
        "behavior_bytes": behavior_bytes.len(),
        "behavior_path": behavior_path,
        "package_sha256": sha256(&package_bytes),
        "package_bytes": package_bytes.len(),
        "expected_members": decoded.resources.len() + 3,
    });
    write(&paths.package, &package_bytes)?;
    write_json(&paths.expected, &expected)?;
    write_json(&paths.report, &report)?;
    println!(
        "behavior package: {} bytes, {} checks, status {}",
        package_bytes.len(),
        checks.as_object().map_or(0, serde_json::Map::len),
        if passed { "passed" } else { "failed" }
    );
    if passed {
        Ok(())
    } else {
        Err(format!("report failed; inspect {}", paths.report.display()))
    }
}

fn capability_without_resource_rejected(document: Document) -> bool {
    let mut package = NuifPackage::new(document, PackageMode::Portable);
    package
        .required_capabilities
        .insert(BEHAVIOR_PROFILE.to_owned());
    matches!(
        attached_behavior(&package),
        Err(BehaviorAttachmentError::MissingResource)
    )
}

fn resource_without_capability_rejected(document: Document, bytes: &[u8]) -> bool {
    let mut package = NuifPackage::new(document, PackageMode::Portable);
    package
        .add_embedded(
            bytes.to_vec(),
            BEHAVIOR_MEDIA_TYPE,
            ResourceRole::Source,
            None,
        )
        .is_ok()
        && matches!(
            attached_behavior(&package),
            Err(BehaviorAttachmentError::MissingCapability)
        )
}

fn linked_behavior_rejected(document: Document, bytes: &[u8]) -> bool {
    let mut package = NuifPackage::new(document, PackageMode::Authoring);
    let added = package.add_linked(
        digest(bytes),
        bytes.len() as u64,
        BEHAVIOR_MEDIA_TYPE,
        ResourceRole::Source,
        "https://example.invalid/behavior.cbor",
        None,
    );
    package
        .required_capabilities
        .insert(BEHAVIOR_PROFILE.to_owned());
    added.is_ok()
        && matches!(
            attached_behavior(&package),
            Err(BehaviorAttachmentError::InvalidResourceDescriptor { .. })
        )
}

fn duplicate_behavior_rejected(
    document: Document,
    program: &nuif_behavior::BehaviorProgram,
) -> bool {
    let mut package = NuifPackage::new(document, PackageMode::Portable);
    if attach_behavior(&mut package, program).is_err() {
        return false;
    }
    let mut alternative = program.clone();
    "open".clone_into(&mut alternative.initial_state);
    package
        .add_embedded(
            match encode_canonical_record(&alternative) {
                Ok(bytes) => bytes,
                Err(_) => return false,
            },
            BEHAVIOR_MEDIA_TYPE,
            ResourceRole::Source,
            None,
        )
        .is_ok()
        && matches!(
            attached_behavior(&package),
            Err(BehaviorAttachmentError::MultipleResources { observed: 2 })
        )
}

fn non_cbor_behavior_rejected(document: Document) -> bool {
    let mut package = NuifPackage::new(document, PackageMode::Portable);
    if package
        .add_embedded(
            br#"{"script":"alert(1)"}"#.to_vec(),
            BEHAVIOR_MEDIA_TYPE,
            ResourceRole::Source,
            None,
        )
        .is_err()
    {
        return false;
    }
    package
        .required_capabilities
        .insert(BEHAVIOR_PROFILE.to_owned());
    matches!(
        attached_behavior(&package),
        Err(BehaviorAttachmentError::Codec(_))
    )
}

fn document_rebinding_rejected(mut package: NuifPackage) -> bool {
    package.document = Document::empty(EntityId::new(0xff));
    matches!(
        attached_behavior(&package),
        Err(BehaviorAttachmentError::Behavior(_))
    )
}

fn corrupt_package_rejected(package: &[u8], behavior: &[u8]) -> bool {
    let Some(offset) = package
        .windows(behavior.len())
        .position(|window| window == behavior)
    else {
        return false;
    };
    let mut corrupted = package.to_vec();
    corrupted[offset] ^= 1;
    NuifPackage::decode(&corrupted).is_err()
}

fn descriptor_path(descriptor: &nuif_core::ResourceDescriptor) -> Result<String, String> {
    match &descriptor.locator {
        nuif_core::ResourceLocator::Embedded { path } => Ok(path.clone()),
        nuif_core::ResourceLocator::Linked { .. } => {
            Err("behavior fixture unexpectedly used a linked locator".to_owned())
        }
    }
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "map_err supplies owned error values to this generic adapter"
)]
fn stringify(error: impl ToString) -> String {
    error.to_string()
}

struct Paths {
    package: PathBuf,
    expected: PathBuf,
    report: PathBuf,
}

fn paths() -> Result<Paths, String> {
    let mut package = PathBuf::from("target/behavior-package-fixture.nuif");
    let mut expected = PathBuf::from("target/behavior-package-expected.json");
    let mut report = PathBuf::from("target/behavior-package-static-report.json");
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        let value = arguments.next().ok_or_else(|| {
            "usage: behavior-package [--package <nuif>] [--expected <json>] [--report <json>]"
                .to_owned()
        })?;
        match argument.as_str() {
            "--package" => package = PathBuf::from(value),
            "--expected" => expected = PathBuf::from(value),
            "--report" => report = PathBuf::from(value),
            _ => return Err(format!("unknown argument {argument}")),
        }
    }
    Ok(Paths {
        package,
        expected,
        report,
    })
}

fn write(path: &PathBuf, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(stringify)?;
    }
    fs::write(path, bytes).map_err(stringify)
}

fn write_json(path: &PathBuf, value: &Value) -> Result<(), String> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(stringify)?;
    bytes.push(b'\n');
    write(path, &bytes)
}
