use nuif_codec::canonical_hash;
use nuif_core::{
    Asset, AssetId, AssetKind, AssetPortability, CURRENT_SCHEMA_VERSION, Document, EntityId,
    FontAsset, ResourceRole,
};
use nuif_font::{
    EmbeddingPermission, MAX_FONT_BYTES, MAX_FONT_COVERAGE_RANGES, MAX_FONT_FEATURES,
    MAX_FONT_NAMES, MAX_FONT_TABLES, OPENTYPE_STATIC_PROFILE, classify_fs_type,
    inspect_opentype_static,
};
use nuif_package::{NuifPackage, PackageMode, ResourceResolver};
use nuif_text::{PINNED_FONT_NAME, PINNED_FONT_SHA256, pinned_font_bytes};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

fn main() {
    if let Err(error) = run() {
        eprintln!("font-resources: {error}");
        std::process::exit(1);
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "the evidence executable keeps parser, package and policy trials auditable in one flow"
)]
fn run() -> Result<(), String> {
    let output = output_path()?;
    let started = Instant::now();
    let bytes = pinned_font_bytes();
    let primary_started = Instant::now();
    let primary = inspect_opentype_static(bytes, 0).map_err(|error| error.to_string())?;
    let primary_micros = primary_started.elapsed().as_micros();
    let independent_started = Instant::now();
    let independent = harfbuzz_golden()?;
    let independent_micros = independent_started.elapsed().as_micros();

    let parser_trials = vec![
        trial(
            "independent_oracle_identity",
            independent.schema_version == 1
                && independent.tool == "hb-info"
                && independent.version == "14.4.0",
        ),
        trial(
            "exact_resource_identity",
            sha256(bytes) == PINNED_FONT_SHA256
                && primary.byte_length == bytes.len()
                && independent.font_sha256 == PINNED_FONT_SHA256,
        ),
        trial(
            "independent_metrics_agree",
            primary.units_per_em == independent.units_per_em
                && primary.glyph_count == independent.glyph_count,
        ),
        trial(
            "independent_unicode_coverage_agrees",
            unicode_scalar_count(&primary.coverage) == independent.unicode_count
                && unicode_scalar_hash(&primary.coverage) == independent.unicode_scalar_sha256,
        ),
        trial(
            "independent_names_and_tables_agree",
            !independent.family_names.is_empty()
                && independent
                    .family_names
                    .iter()
                    .all(|name| primary.names.contains(name))
                && primary.table_tags == independent.table_tags,
        ),
        trial(
            "static_axis_state_is_explicit",
            !primary.table_tags.iter().any(|tag| tag == "fvar"),
        ),
        trial(
            "profile_metadata_is_exact",
            primary.decoder_profile == OPENTYPE_STATIC_PROFILE
                && primary.face_index == 0
                && primary.names.iter().any(|name| name == PINNED_FONT_NAME)
                && primary.permission == EmbeddingPermission::Installable
                && primary.fs_type == 0,
        ),
    ];
    let accepted_trials = accepted_static_trials();
    let negative_trials = negative_trials(bytes);
    let policy_trials = policy_trials(&primary, bytes);
    let portability_trials = portability_trials(&primary, bytes)?;
    let package_trials = package_trials(&primary, bytes)?;
    let passed = parser_trials
        .iter()
        .chain(&accepted_trials)
        .chain(&negative_trials)
        .chain(&policy_trials)
        .chain(&portability_trials)
        .chain(&package_trials)
        .all(passed_trial);
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:font-resource-static-baseline",
        "status": if passed { "passed" } else { "failed" },
        "profile": {
            "name": OPENTYPE_STATIC_PROFILE,
            "primary_parser": "Skrifa 0.46.2 / read-fonts behind NUIF sfnt range, checksum, and OS/2 validation",
            "independent_parser": "pinned hb-info 14.4.0 metadata capture",
            "fixture_corpus": "font-test-data 0.9.1",
            "container": "single-face TrueType-outline sfnt at face index zero",
            "policy": "exact bytes plus matching metadata, fsType evidence, license expression and explicit embedding review",
        },
        "limits": {
            "encoded_bytes": MAX_FONT_BYTES,
            "tables": MAX_FONT_TABLES,
            "family_names": MAX_FONT_NAMES,
            "coverage_ranges": MAX_FONT_COVERAGE_RANGES,
            "feature_settings": MAX_FONT_FEATURES,
        },
        "measurements": {
            "font_bytes": bytes.len(),
            "table_count": primary.table_tags.len(),
            "glyph_count": primary.glyph_count,
            "coverage_ranges": primary.coverage.len(),
            "primary_microseconds": primary_micros,
            "independent_microseconds": independent_micros,
            "total_microseconds": started.elapsed().as_micros(),
        },
        "inspection": primary,
        "independent": independent,
        "parser_trials": parser_trials,
        "accepted_trials": accepted_trials,
        "negative_trials": negative_trials,
        "policy_trials": policy_trials,
        "portability_trials": portability_trials,
        "package_trials": package_trials,
        "source": source_identity(),
        "non_claims": [
            "four accepted static TrueType fixtures with only an Ahem HarfBuzz metadata golden are not broad OpenType conformance",
            "no TTC CFF CFF2 variable color bitmap SVG WOFF WOFF2 or subsetting support",
            "no shaping outline raster browser or cross-platform font reproduction is established by this gate",
            "fsType and a recorded review are policy evidence, not an automated license decision or redistribution grant",
            "the HarfBuzz oracle is a committed capture rather than a live executable dependency in every run",
        ],
        "summary": {
            "parser": parser_trials.len(),
            "accepted": accepted_trials.len(),
            "negative": negative_trials.len(),
            "policy": policy_trials.len(),
            "portability": portability_trials.len(),
            "package": package_trials.len(),
            "blocking_failures": parser_trials.iter().chain(&accepted_trials).chain(&negative_trials).chain(&policy_trials).chain(&portability_trials).chain(&package_trials).filter(|item| !passed_trial(item)).count(),
        }
    });
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
        "font resources: {} parser/accepted, {} negative, {} policy/portability/package, status {}",
        parser_trials.len() + accepted_trials.len(),
        negative_trials.len(),
        policy_trials.len() + portability_trials.len() + package_trials.len(),
        if passed { "passed" } else { "failed" }
    );
    if passed {
        Ok(())
    } else {
        Err(format!("report failed; inspect {}", output.display()))
    }
}

fn accepted_static_trials() -> Vec<Value> {
    [
        ("ahem_static_truetype", font_test_data::AHEM),
        ("tinos_static_truetype", font_test_data::TINOS_SUBSET),
        (
            "cousine_hint_static_truetype",
            font_test_data::COUSINE_HINT_SUBSET,
        ),
        ("tthint_static_truetype", font_test_data::TTHINT_SUBSET),
    ]
    .into_iter()
    .map(|(name, bytes)| trial(name, inspect_opentype_static(bytes, 0).is_ok()))
    .collect()
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct HarfBuzzGolden {
    schema_version: u32,
    tool: String,
    version: String,
    capture_command: String,
    font_sha256: String,
    family_names: Vec<String>,
    units_per_em: u16,
    glyph_count: u16,
    unicode_count: usize,
    unicode_scalar_sha256: String,
    table_tags: Vec<String>,
}

fn harfbuzz_golden() -> Result<HarfBuzzGolden, String> {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../conformance/font/harfbuzz-14.4.0-ahem.json"
    )))
    .map_err(|error| format!("invalid HarfBuzz font metadata golden: {error}"))
}

fn unicode_scalar_count(ranges: &[nuif_core::CodepointRange]) -> usize {
    ranges
        .iter()
        .map(|range| {
            (range.start..=range.end)
                .filter(|codepoint| char::from_u32(*codepoint).is_some())
                .count()
        })
        .sum()
}

fn unicode_scalar_hash(ranges: &[nuif_core::CodepointRange]) -> String {
    let mut digest = Sha256::new();
    for range in ranges {
        for codepoint in range.start..=range.end {
            if char::from_u32(codepoint).is_some() {
                digest.update(format!("U+{codepoint:04X}\n").as_bytes());
            }
        }
    }
    format!("{:x}", digest.finalize())
}

fn negative_trials(canonical: &[u8]) -> Vec<Value> {
    let mut cases = Vec::new();
    cases.push(("truncated", canonical[..64].to_vec(), 0));
    let mut signature = canonical.to_vec();
    signature[..4].copy_from_slice(b"OTTO");
    cases.push(("cff_signature", signature, 0));
    let mut corrupt = canonical.to_vec();
    corrupt[256] ^= 1;
    cases.push(("checksum_corruption", corrupt, 0));
    let mut table_count = canonical.to_vec();
    table_count[4..6].copy_from_slice(&257_u16.to_be_bytes());
    cases.push(("table_count_one_over", table_count, 0));
    let mut duplicate = canonical.to_vec();
    let first_tag: [u8; 4] = duplicate[12..16].try_into().expect("fixture tag");
    duplicate[28..32].copy_from_slice(&first_tag);
    cases.push(("duplicate_table_tag", duplicate, 0));
    let mut bad_offset = canonical.to_vec();
    bad_offset[20..24].copy_from_slice(&u32::MAX.to_be_bytes());
    cases.push(("table_offset_out_of_range", bad_offset, 0));
    let mut wrong_search_range = canonical.to_vec();
    wrong_search_range[6..8].copy_from_slice(&0_u16.to_be_bytes());
    cases.push(("invalid_table_search_parameters", wrong_search_range, 0));
    if let Some(padding) = table_padding_offset(canonical) {
        let mut nonzero_padding = canonical.to_vec();
        nonzero_padding[padding] = 1;
        cases.push(("nonzero_table_padding", nonzero_padding, 0));
    }
    let mut trailing = canonical.to_vec();
    trailing.push(0);
    cases.push(("trailing_byte", trailing, 0));
    let mut collection = canonical.to_vec();
    collection[..4].copy_from_slice(b"ttcf");
    cases.push(("collection_container", collection, 0));
    let mut variable = canonical.to_vec();
    if let Some(offset) = directory_tag_offset(&variable, *b"gasp") {
        variable[offset..offset + 4].copy_from_slice(b"fvar");
    }
    cases.push(("variable_table", variable, 0));
    cases.push(("nonzero_face_index", canonical.to_vec(), 1));
    let mut oversized = vec![0; MAX_FONT_BYTES + 1];
    oversized[..4].copy_from_slice(&canonical[..4]);
    cases.push(("encoded_byte_one_over", oversized, 0));
    cases.extend([
        ("real_ttc_collection", font_test_data::ttc::TTC.to_vec(), 0),
        ("real_cff_otf", font_test_data::NOTO_SANS_JP_CFF.to_vec(), 0),
        (
            "real_variable_truetype",
            font_test_data::VAZIRMATN_VAR.to_vec(),
            0,
        ),
        ("real_colrv1", font_test_data::COLRV0V1.to_vec(), 0),
        (
            "real_embedded_bitmap",
            font_test_data::EMBEDDED_BITMAPS.to_vec(),
            0,
        ),
        ("real_cbdt_bitmap", font_test_data::CBDT.to_vec(), 0),
        (
            "real_sbix_bitmap",
            font_test_data::NOTO_HANDWRITING_SBIX.to_vec(),
            0,
        ),
    ]);
    cases
        .into_iter()
        .map(|(name, bytes, face)| trial(name, inspect_opentype_static(&bytes, face).is_err()))
        .collect()
}

fn portability_trials(
    inspection: &nuif_font::FontInspection,
    bytes: &[u8],
) -> Result<Vec<Value>, String> {
    let private_authoring = font_package(
        inspection,
        bytes,
        PackageMode::Authoring,
        AssetPortability::PrivateAuthoring,
        false,
    )?;
    let private_portable = font_package(
        inspection,
        bytes,
        PackageMode::Portable,
        AssetPortability::PrivateAuthoring,
        false,
    )?;
    let substituted = font_package(
        inspection,
        bytes,
        PackageMode::Portable,
        AssetPortability::Substituted,
        false,
    )?;
    let unavailable = font_package(
        inspection,
        bytes,
        PackageMode::Portable,
        AssetPortability::Unavailable,
        false,
    )?;
    let linked = font_package(
        inspection,
        bytes,
        PackageMode::Authoring,
        AssetPortability::Linked,
        true,
    )?;
    let linked_portable = font_package(
        inspection,
        bytes,
        PackageMode::Portable,
        AssetPortability::Linked,
        true,
    )?;
    let digest = linked
        .document
        .assets
        .get(&AssetId::new(0xf0))
        .and_then(|asset| asset.resource.clone())
        .ok_or_else(|| "linked fixture lacks resource digest".to_owned())?;
    let mut resolver = FixedResolver(bytes.to_vec());
    Ok(vec![
        trial(
            "private_authoring_embedded_in_authoring_package",
            private_authoring.encode().is_ok(),
        ),
        trial(
            "private_authoring_rejected_from_portable_package",
            private_portable.encode().is_err(),
        ),
        trial(
            "substituted_exact_bytes_allowed_in_portable_package",
            substituted.encode().is_ok(),
        ),
        trial(
            "unavailable_asset_has_no_false_resource_claim",
            unavailable.encode().is_ok()
                && unavailable
                    .document
                    .assets
                    .get(&AssetId::new(0xf0))
                    .is_some_and(|asset| asset.resource.is_none()),
        ),
        trial(
            "linked_authoring_requires_explicit_resolution",
            linked.manifest().is_ok()
                && linked.resolve_resource(&digest, None).is_err()
                && linked
                    .resolve_resource(&digest, Some(&mut resolver))
                    .is_ok(),
        ),
        trial(
            "linked_resource_rejected_from_portable_package",
            linked_portable.encode().is_err(),
        ),
    ])
}

struct FixedResolver(Vec<u8>);

impl ResourceResolver for FixedResolver {
    fn resolve(&mut self, _: &nuif_core::ResourceDescriptor) -> Result<Vec<u8>, String> {
        Ok(self.0.clone())
    }
}

fn font_package(
    inspection: &nuif_font::FontInspection,
    bytes: &[u8],
    mode: PackageMode,
    portability: AssetPortability,
    linked: bool,
) -> Result<NuifPackage, String> {
    let mut package = NuifPackage::new(Document::empty(EntityId::new(1)), mode);
    let resource = if portability == AssetPortability::Unavailable {
        None
    } else if linked {
        let digest = nuif_core::ResourceDigest::from_sha256_hex(sha256(bytes));
        package
            .add_linked(
                digest.clone(),
                bytes.len() as u64,
                "font/ttf",
                ResourceRole::Authoring,
                "https://example.invalid/font.ttf",
                None,
            )
            .map_err(|error| error.to_string())?;
        Some(digest)
    } else {
        Some(
            package
                .add_embedded(bytes.to_vec(), "font/ttf", ResourceRole::Authoring, None)
                .map_err(|error| error.to_string())?,
        )
    };
    let mut asset = font_asset(inspection, resource);
    asset.portability = portability;
    package.document.assets.insert(asset.id, asset);
    Ok(package)
}

fn policy_trials(inspection: &nuif_font::FontInspection, bytes: &[u8]) -> Vec<Value> {
    let mut trials = Vec::new();
    for (name, mutate) in [
        ("missing_embedding_review", "review"),
        ("missing_license_expression", "license"),
        ("wrong_fs_type_evidence", "fs_type"),
        ("wrong_decoder_profile", "decoder"),
        ("stale_coverage", "coverage"),
        ("nonzero_static_axis", "axis"),
        ("invalid_feature_tag", "feature"),
    ] {
        let mut asset = font_asset(inspection, None);
        let AssetKind::Font(font) = &mut asset.kind else {
            unreachable!();
        };
        match mutate {
            "review" => {
                font.policy_evidence.remove("license.embedding_review");
            }
            "license" => {
                font.policy_evidence
                    .insert("license.expression".to_owned(), " ".to_owned());
            }
            "fs_type" => {
                font.policy_evidence
                    .insert("opentype.fs_type".to_owned(), "0x0004".to_owned());
            }
            "decoder" => {
                font.policy_evidence
                    .insert("font.decoder_profile".to_owned(), "other".to_owned());
            }
            "coverage" => {
                font.coverage.pop();
            }
            "axis" => {
                font.axes.insert("wght".to_owned(), 400.0);
            }
            "feature" => {
                font.features.insert("bad-tag".to_owned(), 1);
            }
            _ => unreachable!(),
        }
        trials.push(trial(
            name,
            nuif_font::validate_packaged_font(&asset, bytes).is_err(),
        ));
    }
    trials.push(trial(
        "contradictory_fs_type_bits",
        classify_fs_type(3, 0x0006).is_err(),
    ));
    trials.push(trial(
        "reserved_fs_type_bits",
        classify_fs_type(3, 0x0400).is_err(),
    ));
    trials.push(trial(
        "restricted_embedding_signal",
        classify_fs_type(3, 0x0002) == Ok(EmbeddingPermission::Restricted),
    ));
    trials
}

fn package_trials(
    inspection: &nuif_font::FontInspection,
    bytes: &[u8],
) -> Result<Vec<Value>, String> {
    let mut package = NuifPackage::new(Document::empty(EntityId::new(1)), PackageMode::Portable);
    let digest = package
        .add_embedded(bytes.to_vec(), "font/ttf", ResourceRole::Authoring, None)
        .map_err(|error| error.to_string())?;
    let asset = font_asset(inspection, Some(digest.clone()));
    package.document.assets.insert(asset.id, asset);
    let document_hash = canonical_hash(&package.document).map_err(|error| error.to_string())?;
    let encoded = package.encode().map_err(|error| error.to_string())?;
    let decoded = NuifPackage::decode(&encoded).map_err(|error| error.to_string())?;
    let fixed = decoded.encode().map_err(|error| error.to_string())?;
    let mut edited = decoded;
    edited
        .document
        .assets
        .get_mut(&AssetId::new(0xf0))
        .expect("fixture asset")
        .name = Some("unrelated edit".to_owned());
    let edited = NuifPackage::decode(&edited.encode().map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())?;
    let edited_hash = canonical_hash(&edited.document).map_err(|error| error.to_string())?;
    Ok(vec![
        trial("package_byte_fixpoint", encoded == fixed),
        trial(
            "exact_font_bytes_survive",
            edited.embedded(&digest) == Some(bytes),
        ),
        trial(
            "unrelated_edit_changes_document_not_resource",
            document_hash != edited_hash && digest.sha256_hex() == Some(PINNED_FONT_SHA256),
        ),
    ])
}

fn font_asset(
    inspection: &nuif_font::FontInspection,
    resource: Option<nuif_core::ResourceDigest>,
) -> Asset {
    Asset {
        schema_version: CURRENT_SCHEMA_VERSION,
        id: AssetId::new(0xf0),
        name: Some(PINNED_FONT_NAME.to_owned()),
        resource,
        portability: AssetPortability::Portable,
        kind: AssetKind::Font(FontAsset {
            face_index: 0,
            names: inspection.names.clone(),
            axes: BTreeMap::new(),
            features: BTreeMap::new(),
            coverage: inspection.coverage.clone(),
            policy_evidence: BTreeMap::from([
                (
                    "font.decoder_profile".to_owned(),
                    OPENTYPE_STATIC_PROFILE.to_owned(),
                ),
                (
                    "opentype.fs_type".to_owned(),
                    format!("0x{:04x}", inspection.fs_type),
                ),
                ("license.expression".to_owned(), "CC0-1.0".to_owned()),
                ("license.embedding_review".to_owned(), "approved".to_owned()),
            ]),
        }),
    }
}

fn directory_tag_offset(bytes: &[u8], target: [u8; 4]) -> Option<usize> {
    let count = usize::from(u16::from_be_bytes(bytes.get(4..6)?.try_into().ok()?));
    (0..count)
        .map(|index| 12 + index * 16)
        .find(|offset| bytes.get(*offset..*offset + 4) == Some(target.as_slice()))
}

fn table_padding_offset(bytes: &[u8]) -> Option<usize> {
    let count = usize::from(u16::from_be_bytes(bytes.get(4..6)?.try_into().ok()?));
    (0..count).find_map(|index| {
        let record = 12 + index * 16;
        let offset = usize::try_from(u32::from_be_bytes(
            bytes.get(record + 8..record + 12)?.try_into().ok()?,
        ))
        .ok()?;
        let length = usize::try_from(u32::from_be_bytes(
            bytes.get(record + 12..record + 16)?.try_into().ok()?,
        ))
        .ok()?;
        let end = offset.checked_add(length)?;
        (end % 4 != 0).then_some(end)
    })
}

fn trial(name: &str, passed: bool) -> Value {
    json!({"name": name, "passed": passed})
}

fn passed_trial(value: &Value) -> bool {
    value["passed"] == true
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

fn output_path() -> Result<PathBuf, String> {
    let mut arguments = env::args().skip(1);
    let mut output = PathBuf::from("target/font-resources-report.json");
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--output" => {
                output = arguments
                    .next()
                    .ok_or_else(|| "--output requires a path".to_owned())?
                    .into();
            }
            "--help" | "-h" => return Err("usage: font-resources [--output <json>]".to_owned()),
            unknown => return Err(format!("unknown argument {unknown:?}")),
        }
    }
    Ok(output)
}
