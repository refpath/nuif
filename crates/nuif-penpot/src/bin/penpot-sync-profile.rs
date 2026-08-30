use nuif_codec::{CanonicalText, Encoder, canonical_hash};
use nuif_core::{Color, ColorSpace, EntityId, Fidelity, SizeIntent};
use nuif_penpot::{
    AdapterError, MAX_MEMBER_BYTES, MAX_PACKAGE_BYTES, export_document, import_package,
    profile_fixture, synchronize,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const FOREIGN_FIXTURE: &[u8] =
    include_bytes!("../../../../conformance/foreign/penpot/fixture.penpot");
const OPAQUE_MEMBER: &str = "objects/org.nuif-probe.bin";
const OPAQUE_PAYLOAD: &[u8] = b"opaque\0Penpot\xffmember";
const VENDOR_FIELD: &str = "\"vendorOpaque\":{\"keep\":[1,2,3]}";

fn main() {
    if let Err(error) = run() {
        eprintln!("penpot-sync-profile: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let outputs = output_paths()?;
    let before = profile_fixture();
    let foreign = import_package(FOREIGN_FIXTURE).map_err(|error| error.to_string())?;
    let exported = export_document(&before).map_err(|error| error.to_string())?;
    let repeated = export_document(&before).map_err(|error| error.to_string())?;
    let retained_bytes = add_opaque_data(&exported.bytes)?;
    let retained = import_package(&retained_bytes).map_err(|error| error.to_string())?;
    let unchanged = synchronize(&retained.retentive, &before).map_err(|error| error.to_string())?;
    let after = edited_document(&before);
    let synchronized =
        synchronize(&retained.retentive, &after).map_err(|error| error.to_string())?;
    let reimported = import_package(&synchronized.bytes).map_err(|error| error.to_string())?;
    let before_members = package_payloads(&retained_bytes)?;
    let after_members = package_payloads(&synchronized.bytes)?;
    let untouched_members_exact = before_members.iter().all(|(name, payload)| {
        synchronized.edits.iter().any(|edit| &edit.member == name)
            || after_members.get(name) == Some(payload)
    });
    let checks = json!({
        "official_library_fixture_exact": foreign.document == before && foreign.retentive.report().is_lossless(),
        "export_import_exact": import_package(&exported.bytes).is_ok_and(|value| value.document == before),
        "export_bytes_deterministic": exported.bytes == repeated.bytes,
        "unmodified_archive_byte_exact": unchanged.bytes == retained_bytes && unchanged.edits.is_empty(),
        "synchronized_import_exact": reimported.document == after,
        "mapped_edit_count_exact": synchronized.edits.len() == 8,
        "untouched_member_payloads_exact": untouched_members_exact,
        "unknown_member_payload_exact": after_members.get(OPAQUE_MEMBER).is_some_and(|value| value == OPAQUE_PAYLOAD),
        "unknown_json_field_byte_exact": after_members.values().any(|value| value.windows(VENDOR_FIELD.len()).any(|window| window == VENDOR_FIELD.as_bytes())),
        "unknown_data_fidelity_typed": retained.retentive.report().fidelity.iter().any(|entry| matches!(entry.status, Fidelity::PreservedUnrenderable { .. } | Fidelity::Unsupported { .. })),
        "unknown_payload_contract_set": synchronized.report.unmapped_member_payloads_preserved,
        "structural_edit_typed": structural_edit_typed(&retained.retentive),
        "package_limit_typed": matches!(import_package(&vec![0; MAX_PACKAGE_BYTES + 1]), Err(AdapterError::PackageTooLarge)),
        "member_limit_typed": member_limit_typed(),
        "path_traversal_typed": path_traversal_typed(),
    });
    let passed = all_checks_pass(&checks);
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:penpot-v3-retentive-package",
        "status": if passed { "passed" } else { "failed" },
        "source": {
            "revision": command_text("git", &["rev-parse", "HEAD"]),
            "dirty": command_text("git", &["status", "--porcelain"]).map(|value| !value.is_empty()),
            "toolchain": command_text("rustc", &["--version"]),
            "os": env::consts::OS,
            "architecture": env::consts::ARCH,
        },
        "profile": {
            "name": nuif_penpot::PROFILE_NAME,
            "foreign_producer": "@penpot/library 1.1.0",
            "zip_reader": "zip 8.6.0",
            "package_limit_bytes": MAX_PACKAGE_BYTES,
            "member_limit_bytes": MAX_MEMBER_BYTES,
            "mapped_semantics": ["identity", "one page and board", "direct containment", "rectangle", "ellipse", "literal text", "pinned font metadata", "opaque quantized sRGB fill"],
        },
        "before": {
            "canonical_hash": canonical_hash(&before).map_err(|error| error.to_string())?,
            "package_sha256": sha256(&retained_bytes),
        },
        "after": {
            "canonical_hash": canonical_hash(&after).map_err(|error| error.to_string())?,
            "package_sha256": sha256(&synchronized.bytes),
        },
        "summary": {
            "mapped_edits": synchronized.edits.len(),
            "correspondences": synchronized.report.correspondences.len(),
            "fidelity_entries": synchronized.report.fidelity.len(),
            "members": after_members.len(),
            "blocking_failures": u8::from(!passed),
        },
        "checks": checks,
        "edits": synchronized.edits,
        "fidelity": synchronized.report.fidelity,
    });
    write_file(
        &outputs.report,
        &serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )?;
    write_file(&outputs.package, &synchronized.bytes)?;
    write_file(
        &outputs.edited,
        &CanonicalText
            .encode(&after)
            .map_err(|error| error.to_string())?,
    )?;
    println!(
        "Penpot v3 retentive package: {} mapped spans, status {}",
        report["summary"]["mapped_edits"], report["status"]
    );
    if passed {
        Ok(())
    } else {
        Err(format!(
            "report failed; inspect {}",
            outputs.report.display()
        ))
    }
}

fn edited_document(before: &nuif_core::Document) -> nuif_core::Document {
    let mut after = before.clone();
    after.entities.get_mut(&EntityId::new(0x21)).unwrap().name = Some("Edited card".to_owned());
    after
        .entities
        .get_mut(&EntityId::new(0x21))
        .unwrap()
        .authored
        .width = SizeIntent::Fixed(280.0);
    after
        .entities
        .get_mut(&EntityId::new(0x22))
        .unwrap()
        .authored
        .fill = Some(Color {
        space: ColorSpace::Srgb,
        red: 1.0,
        green: 128.0 / 255.0,
        blue: 0.0,
        alpha: 1.0,
    });
    "Edited Penpot text".clone_into(
        &mut after
            .entities
            .get_mut(&EntityId::new(0x23))
            .unwrap()
            .authored
            .text
            .as_mut()
            .unwrap()
            .content,
    );
    after
}

fn add_opaque_data(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut members = package_members(bytes)?;
    let rectangle = members
        .iter_mut()
        .find(|(name, _, _)| name.ends_with("/00000000-0000-0000-0000-000000000021.json"))
        .ok_or_else(|| "rectangle member is absent".to_owned())?;
    if rectangle.1.pop() != Some(b'}') {
        return Err("rectangle member is not a compact JSON object".to_owned());
    }
    rectangle.1.extend_from_slice(b",");
    rectangle.1.extend_from_slice(VENDOR_FIELD.as_bytes());
    rectangle.1.push(b'}');
    members.push((
        OPAQUE_MEMBER.to_owned(),
        OPAQUE_PAYLOAD.to_vec(),
        CompressionMethod::Stored,
    ));
    write_package(&members)
}

fn package_members(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>, CompressionMethod)>, String> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|error| error.to_string())?;
    let mut members = Vec::with_capacity(archive.len());
    for index in 0..archive.len() {
        let mut member = archive.by_index(index).map_err(|error| error.to_string())?;
        let mut payload = Vec::new();
        member
            .read_to_end(&mut payload)
            .map_err(|error| error.to_string())?;
        members.push((member.name().to_owned(), payload, member.compression()));
    }
    Ok(members)
}

fn package_payloads(bytes: &[u8]) -> Result<BTreeMap<String, Vec<u8>>, String> {
    package_members(bytes).map(|members| {
        members
            .into_iter()
            .map(|(name, payload, _)| (name, payload))
            .collect()
    })
}

fn write_package(members: &[(String, Vec<u8>, CompressionMethod)]) -> Result<Vec<u8>, String> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, payload, compression) in members {
        let options = SimpleFileOptions::default()
            .compression_method(*compression)
            .last_modified_time(
                zip::DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0)
                    .map_err(|error| error.to_string())?,
            );
        writer
            .start_file(name, options)
            .map_err(|error| error.to_string())?;
        writer
            .write_all(payload)
            .map_err(|error| error.to_string())?;
    }
    writer
        .finish()
        .map(Cursor::into_inner)
        .map_err(|error| error.to_string())
}

fn structural_edit_typed(retained: &nuif_penpot::RetentivePackage) -> bool {
    let mut structural = profile_fixture();
    structural
        .entities
        .get_mut(&EntityId::new(0x10))
        .unwrap()
        .children
        .swap(0, 1);
    matches!(
        synchronize(retained, &structural),
        Err(AdapterError::UnmappedChanges { .. })
    )
}

fn member_limit_typed() -> bool {
    let members = vec![(
        "manifest.json".to_owned(),
        vec![b'x'; MAX_MEMBER_BYTES + 1],
        CompressionMethod::Deflated,
    )];
    write_package(&members).is_ok_and(|bytes| {
        matches!(
            import_package(&bytes),
            Err(AdapterError::MemberTooLarge { .. })
        )
    })
}

fn path_traversal_typed() -> bool {
    let members = vec![(
        "../manifest.json".to_owned(),
        b"{}".to_vec(),
        CompressionMethod::Stored,
    )];
    write_package(&members).is_ok_and(|bytes| {
        matches!(
            import_package(&bytes),
            Err(AdapterError::UnsafeMemberName(_))
        )
    })
}

fn all_checks_pass(checks: &Value) -> bool {
    checks
        .as_object()
        .expect("checks is an object")
        .values()
        .all(|value| value == &Value::Bool(true))
}

struct Outputs {
    report: PathBuf,
    package: PathBuf,
    edited: PathBuf,
}

fn output_paths() -> Result<Outputs, String> {
    let mut report = PathBuf::from("target/penpot-sync-report.json");
    let mut package = PathBuf::from("target/penpot-sync-output.penpot");
    let mut edited = PathBuf::from("target/penpot-sync-edited.nuif.json");
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        let target = match argument.as_str() {
            "--output" => &mut report,
            "--package-output" => &mut package,
            "--edited-output" => &mut edited,
            _ => return Err(format!("unknown argument {argument}")),
        };
        *target = PathBuf::from(
            arguments
                .next()
                .ok_or_else(|| format!("missing value after {argument}"))?,
        );
    }
    Ok(Outputs {
        report,
        package,
        edited,
    })
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(path, bytes).map_err(|error| error.to_string())
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn command_text(program: &str, arguments: &[&str]) -> Option<String> {
    let output = Command::new(program).args(arguments).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
