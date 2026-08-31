use nuif_api::{
    MAX_SESSION_RESOURCE_BYTES, MAX_SESSION_RESOURCES, MAX_SESSION_TOTAL_RESOURCE_BYTES, Session,
};
use nuif_codec::{DeterministicCbor, Encoder, canonical_hash, encode_canonical_record};
use nuif_core::{
    Asset, AssetId, AssetKind, AssetPortability, Document, EntityId, ImageAsset,
    ResourceDescriptor, ResourceDigest, ResourceLocator, ResourceRole,
};
use nuif_package::{
    MAX_CAPABILITY_BYTES, MAX_PACKAGE_BYTES, MAX_REQUIRED_CAPABILITIES, MAX_RESOURCE_BYTES,
    MAX_RESOURCES, MAX_TOTAL_RESOURCE_BYTES, MIME_TYPE, NuifPackage, PackageError, PackageMode,
    ResourceResolver,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, Stats, StatsAlloc};
use std::alloc::System;
use std::env;
use std::fs;
use std::io::{Cursor, Write as _};
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, System as ZipSystem, ZipWriter};

const PROFILE: &str = "nuif-package-0";
const SHARED_RESOURCE_TRIAL_BYTES: usize = 8 * 1024 * 1024;
const MAX_SHARED_HANDOFF_ALLOCATED_BYTES: usize = 1024 * 1024;
const MAX_SHARED_HANDOFF_RETAINED_BYTES: usize = 1024 * 1024;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

fn main() {
    if let Err(error) = run() {
        eprintln!("package-resources: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let (output, package_output) = output_paths()?;
    let started = Instant::now();
    let (package, resource, resource_bytes) = fixture()?;
    let semantic_hash = canonical_hash(&package.document).map_err(|error| error.to_string())?;
    let encoded = package.encode().map_err(|error| error.to_string())?;
    let package_hash = package.package_hash().map_err(|error| error.to_string())?;
    let independent = independent_encode(&package, &resource, &resource_bytes)?;
    let decoded = NuifPackage::decode(&encoded).map_err(|error| error.to_string())?;

    let positive = vec![
        trial(
            "cross_writer_exact_bytes",
            independent == encoded,
            json!({"bytes": encoded.len(), "sha256": sha256(&encoded)}),
        ),
        trial(
            "decode_encode_fixpoint",
            decoded.encode().map_err(|error| error.to_string())? == encoded,
            json!({"profile": PROFILE}),
        ),
        trial(
            "resource_digest_exact",
            resource.sha256_hex() == Some(sha256(&resource_bytes).as_str()),
            json!({"resource": resource, "bytes": resource_bytes.len()}),
        ),
        cache_hash_trial(&package, &semantic_hash, &package_hash)?,
        locator_hash_trial(&resource, &resource_bytes)?,
        resolver_trial(&resource, &resource_bytes)?,
        capability_negotiation_trial(),
        shared_session_allocation_trial()?,
    ];
    let negative = negative_trials(&encoded, &package, &resource)?;
    let passed = positive
        .iter()
        .chain(&negative)
        .all(|case| case["passed"] == true);
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:portable-package-resources",
        "status": if passed { "passed" } else { "failed" },
        "source": source_identity(),
        "profile": {
            "name": PROFILE,
            "mime_type": String::from_utf8_lossy(MIME_TYPE),
            "package_bytes": encoded.len(),
            "semantic_hash": semantic_hash,
            "package_hash": package_hash,
            "resource_digest": resource,
            "limits": {
                "package_bytes": MAX_PACKAGE_BYTES,
                "resource_bytes": MAX_RESOURCE_BYTES,
                "total_resource_bytes": MAX_TOTAL_RESOURCE_BYTES,
                "resources": MAX_RESOURCES,
                "required_capabilities": MAX_REQUIRED_CAPABILITIES,
                "capability_bytes": MAX_CAPABILITY_BYTES,
                "session_resource_bytes": MAX_SESSION_RESOURCE_BYTES,
                "session_total_resource_bytes": MAX_SESSION_TOTAL_RESOURCE_BYTES,
                "session_resources": MAX_SESSION_RESOURCES,
            }
        },
        "measurement": {
            "elapsed_microseconds": started.elapsed().as_micros(),
            "kind": "deterministic boundary and one-over profile",
            "allocator": "stats_alloc 0.1.10 instrumented system allocator",
        },
        "positive_trials": positive,
        "negative_trials": negative,
        "summary": {
            "positive": positive.len(),
            "negative": negative.len(),
            "blocking_failures": positive.iter().chain(&negative).filter(|case| case["passed"] != true).count(),
        }
    });
    write_outputs(&output, &package_output, &encoded, report)?;
    println!(
        "package resources: {} positive, {} negative, status {}",
        positive.len(),
        negative.len(),
        if passed { "passed" } else { "failed" }
    );
    if passed {
        Ok(())
    } else {
        Err(format!("report failed; inspect {}", output.display()))
    }
}

fn write_outputs(
    output: &PathBuf,
    package_output: &PathBuf,
    encoded: &[u8],
    report: Value,
) -> Result<(), String> {
    for path in [output, package_output] {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
    }
    fs::write(package_output, encoded).map_err(|error| error.to_string())?;
    let report = if let Value::Object(mut report) = report {
        report.insert("artifacts".to_owned(), json!({"package": package_output}));
        Value::Object(report)
    } else {
        return Err("package report is not an object".to_owned());
    };
    fs::write(
        output,
        serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn fixture() -> Result<(NuifPackage, ResourceDigest, Vec<u8>), String> {
    let mut document = Document::empty(EntityId::new(1));
    let resource_bytes = b"portable exact resource bytes".to_vec();
    let mut package = NuifPackage::new(document.clone(), PackageMode::Portable);
    let digest = package
        .add_embedded(
            resource_bytes.clone(),
            "image/png",
            ResourceRole::Authoring,
            None,
        )
        .map_err(|error| error.to_string())?;
    let id = AssetId::new(0xa0);
    document.assets.insert(
        id,
        Asset {
            schema_version: 1,
            id,
            name: Some("portable image".to_owned()),
            resource: Some(digest.clone()),
            portability: AssetPortability::Portable,
            kind: AssetKind::Image(ImageAsset {
                width: 1,
                height: 1,
                decoder_profile: "nuif-png-rgba8-0".to_owned(),
            }),
        },
    );
    package.document = document;
    Ok((package, digest, resource_bytes))
}

fn independent_encode(
    package: &NuifPackage,
    digest: &ResourceDigest,
    resource: &[u8],
) -> Result<Vec<u8>, String> {
    zip_bytes(
        &canonical_members(package, digest, resource)?,
        CompressionMethod::Stored,
        0o644,
    )
}

fn canonical_members(
    package: &NuifPackage,
    digest: &ResourceDigest,
    resource: &[u8],
) -> Result<Vec<(String, Vec<u8>)>, String> {
    let document = DeterministicCbor
        .encode(&package.document)
        .map_err(|error| error.to_string())?;
    let manifest = encode_canonical_record(&package.manifest().map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())?;
    let blob = format!(
        "blobs/sha256/{}",
        digest
            .sha256_hex()
            .ok_or_else(|| "fixture digest is invalid".to_owned())?
    );
    let mut members = vec![
        ("mimetype".to_owned(), MIME_TYPE.to_vec()),
        (blob, resource.to_vec()),
        ("document.cbor".to_owned(), document),
        ("manifest.cbor".to_owned(), manifest),
    ];
    members[1..].sort_by(|left, right| left.0.cmp(&right.0));
    Ok(members)
}

fn zip_bytes(
    members: &[(String, Vec<u8>)],
    compression: CompressionMethod,
    permissions: u32,
) -> Result<Vec<u8>, String> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(compression)
        .system(ZipSystem::Unix)
        .last_modified_time(
            zip::DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0)
                .map_err(|error| error.to_string())?,
        )
        .unix_permissions(permissions);
    for (name, bytes) in members {
        writer
            .start_file(name, options)
            .map_err(|error| error.to_string())?;
        writer.write_all(bytes).map_err(|error| error.to_string())?;
    }
    writer
        .finish()
        .map(Cursor::into_inner)
        .map_err(|error| error.to_string())
}

fn cache_hash_trial(
    package: &NuifPackage,
    semantic_hash: &str,
    package_hash: &str,
) -> Result<Value, String> {
    let mut with_cache = package.clone();
    with_cache
        .add_embedded(
            b"deletable decoded cache".to_vec(),
            "application/octet-stream",
            ResourceRole::Cache,
            None,
        )
        .map_err(|error| error.to_string())?;
    let cache_semantic = canonical_hash(&with_cache.document).map_err(|error| error.to_string())?;
    let cache_package = with_cache
        .package_hash()
        .map_err(|error| error.to_string())?;
    Ok(trial(
        "semantic_cache_independence",
        cache_semantic == semantic_hash && cache_package != package_hash,
        json!({"semantic_hash": cache_semantic, "cache_package_hash": cache_package}),
    ))
}

fn locator_hash_trial(digest: &ResourceDigest, bytes: &[u8]) -> Result<Value, String> {
    let document = Document::empty(EntityId::new(2));
    let mut embedded = NuifPackage::new(document.clone(), PackageMode::Authoring);
    embedded
        .add_embedded(
            bytes.to_vec(),
            "application/octet-stream",
            ResourceRole::Source,
            None,
        )
        .map_err(|error| error.to_string())?;
    let mut linked = NuifPackage::new(document, PackageMode::Authoring);
    linked
        .add_linked(
            digest.clone(),
            bytes.len() as u64,
            "application/octet-stream",
            ResourceRole::Source,
            "https://example.invalid/resource",
            None,
        )
        .map_err(|error| error.to_string())?;
    let embedded_semantic =
        canonical_hash(&embedded.document).map_err(|error| error.to_string())?;
    let linked_semantic = canonical_hash(&linked.document).map_err(|error| error.to_string())?;
    let embedded_hash = embedded.package_hash().map_err(|error| error.to_string())?;
    let linked_hash = linked.package_hash().map_err(|error| error.to_string())?;
    Ok(trial(
        "locator_changes_package_not_semantics",
        embedded_semantic == linked_semantic && embedded_hash != linked_hash,
        json!({"embedded": embedded_hash, "linked": linked_hash}),
    ))
}

fn resolver_trial(digest: &ResourceDigest, bytes: &[u8]) -> Result<Value, String> {
    struct Resolver(Vec<u8>);
    impl ResourceResolver for Resolver {
        fn resolve(&mut self, _: &ResourceDescriptor) -> Result<Vec<u8>, String> {
            Ok(self.0.clone())
        }
    }
    let mut package = NuifPackage::new(Document::empty(EntityId::new(3)), PackageMode::Authoring);
    package
        .add_linked(
            digest.clone(),
            bytes.len() as u64,
            "application/octet-stream",
            ResourceRole::Source,
            "https://example.invalid/resource",
            None,
        )
        .map_err(|error| error.to_string())?;
    let explicit_required = package.resolve_resource(digest, None).is_err();
    let exact = package
        .resolve_resource(digest, Some(&mut Resolver(bytes.to_vec())))
        .map_err(|error| error.to_string())?
        == bytes;
    let mismatch = package
        .resolve_resource(digest, Some(&mut Resolver(b"wrong".to_vec())))
        .is_err();
    Ok(trial(
        "explicit_verified_resolver",
        explicit_required && exact && mismatch,
        json!({"implicit_rejected": explicit_required, "exact_resolved": exact, "mismatch_rejected": mismatch}),
    ))
}

fn capability_negotiation_trial() -> Value {
    let mut package =
        NuifPackage::new(Document::empty(EntityId::new(0x60)), PackageMode::Authoring);
    package.required_capabilities = std::collections::BTreeSet::from([
        "nuif-behavior-state-machine-0".to_owned(),
        "nuif-layout-profile-0".to_owned(),
    ]);
    let supported = std::collections::BTreeSet::from(["nuif-layout-profile-0".to_owned()]);
    let report = package.capability_report(&supported);
    let missing_exact = matches!(
        package.require_capabilities(&supported),
        Err(PackageError::RequiredCapabilitiesUnavailable { capabilities })
            if capabilities == report.missing_required
    );
    let full = package
        .require_capabilities(&package.required_capabilities)
        .is_ok();
    let mut invalid = package.clone();
    invalid.required_capabilities.insert("Not Valid".to_owned());
    let malformed_rejected = invalid.encode().is_err();
    let mut excessive = package;
    excessive.required_capabilities = (0..=MAX_REQUIRED_CAPABILITIES)
        .map(|index| format!("capability-{index}"))
        .collect();
    let excessive_rejected = excessive.encode().is_err();
    trial(
        "bounded_explicit_capability_negotiation",
        !report.fully_supported
            && report.missing_required
                == std::collections::BTreeSet::from(["nuif-behavior-state-machine-0".to_owned()])
            && missing_exact
            && full
            && malformed_rejected
            && excessive_rejected,
        json!({
            "required": report.required,
            "supported_required": report.supported_required,
            "missing_required": report.missing_required,
            "malformed_rejected": malformed_rejected,
            "limit_plus_one_rejected": excessive_rejected,
        }),
    )
}

fn shared_session_allocation_trial() -> Result<Value, String> {
    let mut package = NuifPackage::new(
        Document::empty(EntityId::new(0x5100)),
        PackageMode::Authoring,
    );
    let digest = package
        .add_embedded(
            vec![0x5a; SHARED_RESOURCE_TRIAL_BYTES],
            "application/octet-stream",
            ResourceRole::Authoring,
            None,
        )
        .map_err(|error| error.to_string())?;
    let package_pointer = package
        .embedded(&digest)
        .ok_or_else(|| "shared resource fixture was not embedded".to_owned())?
        .as_ptr();

    let region = Region::new(GLOBAL);
    let resources = package.embedded_resources();
    let map_pointer = resources
        .get(&digest)
        .ok_or_else(|| "shared resource fixture was not cloned into the map".to_owned())?
        .as_ptr();
    let session = Session::with_resources(package.document.clone(), resources)
        .map_err(|error| error.to_string())?;
    let session_pointer = session
        .resource(&digest)
        .ok_or_else(|| "shared resource fixture was not retained by the session".to_owned())?
        .as_ptr();
    let stats = region.change();
    let retained = retained_bytes(stats);
    let shared = package_pointer == map_pointer && package_pointer == session_pointer;
    let within_budget = stats.bytes_allocated <= MAX_SHARED_HANDOFF_ALLOCATED_BYTES
        && retained <= MAX_SHARED_HANDOFF_RETAINED_BYTES;
    Ok(trial(
        "package_session_handoff_shares_resource_bytes",
        shared && within_budget,
        json!({
            "resource_bytes": SHARED_RESOURCE_TRIAL_BYTES,
            "allocated_bytes": stats.bytes_allocated,
            "retained_bytes": retained,
            "allocations": stats.allocations,
            "reallocations": stats.reallocations,
            "same_allocation": shared,
            "allocated_budget": MAX_SHARED_HANDOFF_ALLOCATED_BYTES,
            "retained_budget": MAX_SHARED_HANDOFF_RETAINED_BYTES,
        }),
    ))
}

fn retained_bytes(stats: Stats) -> usize {
    let retained = i128::try_from(stats.bytes_allocated).unwrap_or(i128::MAX)
        - i128::try_from(stats.bytes_deallocated).unwrap_or(i128::MAX)
        + i128::try_from(stats.bytes_reallocated).unwrap_or(i128::MAX);
    usize::try_from(retained.max(0)).unwrap_or(usize::MAX)
}

#[expect(
    clippy::too_many_lines,
    reason = "the hostile archive matrix stays together so every constructed input is auditable"
)]
fn negative_trials(
    encoded: &[u8],
    fixture: &NuifPackage,
    digest: &ResourceDigest,
) -> Result<Vec<Value>, String> {
    let resource = b"portable exact resource bytes";
    let canonical_members = canonical_members(fixture, digest, resource)?;
    let mut corrupted = encoded.to_vec();
    let offset = corrupted
        .windows(resource.len())
        .position(|window| window == resource)
        .ok_or_else(|| "fixture resource was not found in package bytes".to_owned())?;
    corrupted[offset] ^= 1;

    let traversal = zip_bytes(
        &[
            ("mimetype".to_owned(), MIME_TYPE.to_vec()),
            ("../document.cbor".to_owned(), Vec::new()),
        ],
        CompressionMethod::Stored,
        0o644,
    )?;
    let compressed = zip_bytes(
        &[("mimetype".to_owned(), MIME_TYPE.to_vec())],
        CompressionMethod::Deflated,
        0o644,
    )?;
    let directory = zip_bytes(
        &[
            ("mimetype".to_owned(), MIME_TYPE.to_vec()),
            ("blobs/".to_owned(), Vec::new()),
        ],
        CompressionMethod::Stored,
        0o644,
    )?;
    let symlink = zip_bytes(
        &[("mimetype".to_owned(), MIME_TYPE.to_vec())],
        CompressionMethod::Stored,
        0o120_777,
    )?;
    let mut encrypted = encoded.to_vec();
    set_zip_flag(&mut encrypted, 0x0001)?;
    let mut split = encoded.to_vec();
    let end = split
        .len()
        .checked_sub(22)
        .ok_or_else(|| "encoded fixture lacks an end record".to_owned())?;
    split[end + 4..end + 6].copy_from_slice(&1_u16.to_le_bytes());

    let mut missing_blob_members = canonical_members.clone();
    missing_blob_members.retain(|(name, _)| !name.starts_with("blobs/sha256/"));
    let missing_blob = zip_bytes(&missing_blob_members, CompressionMethod::Stored, 0o644)?;

    let mut mismatch_members = canonical_members.clone();
    mismatch_members
        .iter_mut()
        .find(|(name, _)| name.starts_with("blobs/sha256/"))
        .ok_or_else(|| "fixture has no blob member".to_owned())?
        .1 = b"different exact bytes".to_vec();
    let digest_mismatch = zip_bytes(&mismatch_members, CompressionMethod::Stored, 0o644)?;

    let mut undeclared_members = canonical_members;
    undeclared_members.push((format!("blobs/sha256/{}", "0".repeat(64)), Vec::new()));
    undeclared_members[1..].sort_by(|left, right| left.0.cmp(&right.0));
    let undeclared_blob = zip_bytes(&undeclared_members, CompressionMethod::Stored, 0o644)?;
    let mut appended = encoded.to_vec();
    appended.push(0);
    let oversized = vec![0_u8; MAX_PACKAGE_BYTES + 1];
    let resource_over = fixture
        .clone()
        .add_embedded(
            vec![0_u8; MAX_RESOURCE_BYTES + 1],
            "application/octet-stream",
            ResourceRole::Source,
            None,
        )
        .is_err();
    let mut too_many = NuifPackage::new(Document::empty(EntityId::new(4)), PackageMode::Authoring);
    for index in 0..=MAX_RESOURCES {
        let digest = ResourceDigest::from_sha256_hex(format!("{index:064x}"));
        too_many.resources.insert(
            digest.clone(),
            ResourceDescriptor {
                digest,
                size: 0,
                media_type: "application/octet-stream".to_owned(),
                role: ResourceRole::Source,
                locator: ResourceLocator::Linked {
                    uri: format!("https://example.invalid/{index}"),
                },
                derivation: None,
            },
        );
    }
    let resource_count_over = too_many.encode().is_err();
    let credentials_rejected =
        NuifPackage::new(Document::empty(EntityId::new(5)), PackageMode::Authoring)
            .add_linked(
                digest.clone(),
                1,
                "application/octet-stream",
                ResourceRole::Source,
                "https://user:secret@example.invalid/resource",
                None,
            )
            .is_err();
    Ok(vec![
        trial(
            "corrupted_resource",
            NuifPackage::decode(&corrupted).is_err(),
            json!({}),
        ),
        trial(
            "trailing_archive_data",
            NuifPackage::decode(&appended).is_err(),
            json!({}),
        ),
        trial(
            "traversal_member",
            NuifPackage::decode(&traversal).is_err(),
            json!({}),
        ),
        trial(
            "compressed_member",
            NuifPackage::decode(&compressed).is_err(),
            json!({}),
        ),
        trial(
            "directory_member",
            NuifPackage::decode(&directory).is_err(),
            json!({}),
        ),
        trial(
            "symlink_attributes",
            NuifPackage::decode(&symlink).is_err(),
            json!({}),
        ),
        trial(
            "encrypted_flag",
            NuifPackage::decode(&encrypted).is_err(),
            json!({}),
        ),
        trial(
            "split_archive",
            NuifPackage::decode(&split).is_err(),
            json!({}),
        ),
        trial(
            "missing_declared_blob",
            NuifPackage::decode(&missing_blob).is_err(),
            json!({}),
        ),
        trial(
            "digest_or_size_mismatch",
            NuifPackage::decode(&digest_mismatch).is_err(),
            json!({}),
        ),
        trial(
            "undeclared_blob",
            NuifPackage::decode(&undeclared_blob).is_err(),
            json!({}),
        ),
        trial(
            "package_byte_limit_plus_one",
            NuifPackage::decode(&oversized).is_err(),
            json!({"observed": oversized.len()}),
        ),
        trial(
            "resource_byte_limit_plus_one",
            resource_over,
            json!({"observed": MAX_RESOURCE_BYTES + 1}),
        ),
        trial(
            "resource_count_limit_plus_one",
            resource_count_over,
            json!({"observed": MAX_RESOURCES + 1}),
        ),
        trial("credential_locator", credentials_rejected, json!({})),
    ])
}

fn set_zip_flag(bytes: &mut [u8], flag: u16) -> Result<(), String> {
    let mut local = false;
    let mut central = false;
    for offset in 0..bytes.len().saturating_sub(4) {
        match &bytes[offset..offset + 4] {
            b"PK\x03\x04" => {
                let current = u16::from_le_bytes(bytes[offset + 6..offset + 8].try_into().unwrap());
                bytes[offset + 6..offset + 8].copy_from_slice(&(current | flag).to_le_bytes());
                local = true;
            }
            b"PK\x01\x02" => {
                let current =
                    u16::from_le_bytes(bytes[offset + 8..offset + 10].try_into().unwrap());
                bytes[offset + 8..offset + 10].copy_from_slice(&(current | flag).to_le_bytes());
                central = true;
            }
            _ => {}
        }
    }
    if local && central {
        Ok(())
    } else {
        Err("fixture lacks local or central ZIP headers".to_owned())
    }
}

fn trial(name: &str, passed: bool, details: impl serde::Serialize) -> Value {
    json!({"name": name, "passed": passed, "details": details})
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn source_identity() -> Value {
    json!({
        "revision": command_text("git", &["rev-parse", "HEAD"]),
        "dirty": command_text("git", &["status", "--porcelain"]).map(|value| !value.is_empty()),
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

fn output_paths() -> Result<(PathBuf, PathBuf), String> {
    let mut arguments = env::args().skip(1);
    let mut output = PathBuf::from("target/package-resources-report.json");
    let mut package_output = PathBuf::from("target/package-resources-fixture.nuif");
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--output" => {
                output = arguments
                    .next()
                    .ok_or_else(|| "--output requires a path".to_owned())?
                    .into();
            }
            "--package-output" => {
                package_output = arguments
                    .next()
                    .ok_or_else(|| "--package-output requires a path".to_owned())?
                    .into();
            }
            "--help" | "-h" => {
                return Err(
                    "usage: package-resources [--output <json>] [--package-output <nuif>]"
                        .to_owned(),
                );
            }
            unknown => return Err(format!("unknown argument {unknown:?}")),
        }
    }
    Ok((output, package_output))
}
