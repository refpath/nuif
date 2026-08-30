use nuif_codec::canonical_hash;
use nuif_core::{
    Asset, AssetId, AssetKind, AssetPortability, CURRENT_SCHEMA_VERSION, CodepointRange, Document,
    EntityId, FontAsset, ResourceRole,
};
use nuif_font::{
    EmbeddingPermission, MAX_FONT_BYTES, MAX_FONT_COVERAGE_RANGES, MAX_FONT_FEATURES,
    MAX_FONT_NAMES, MAX_FONT_TABLES, OPENTYPE_STATIC_PROFILE, classify_fs_type,
    inspect_opentype_static,
};
use nuif_package::{NuifPackage, PackageMode};
use nuif_text::{PINNED_FONT_NAME, PINNED_FONT_SHA256, pinned_font_bytes};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use skrifa::{
    FontRef, MetadataProvider,
    instance::{LocationRef, Size},
};
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
    let independent = inspect_independent(bytes)?;
    let independent_micros = independent_started.elapsed().as_micros();

    let parser_trials = vec![
        trial(
            "exact_resource_identity",
            sha256(bytes) == PINNED_FONT_SHA256 && primary.byte_length == bytes.len(),
        ),
        trial(
            "independent_metrics_agree",
            primary.units_per_em == independent.units_per_em
                && primary.glyph_count == independent.glyph_count,
        ),
        trial(
            "independent_coverage_agrees",
            primary.coverage == independent.coverage,
        ),
        trial(
            "independent_static_axis_state_agrees",
            independent.axis_count == 0 && !primary.table_tags.iter().any(|tag| tag == "fvar"),
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
    let negative_trials = negative_trials(bytes);
    let policy_trials = policy_trials(&primary, bytes);
    let package_trials = package_trials(&primary, bytes)?;
    let passed = parser_trials
        .iter()
        .chain(&negative_trials)
        .chain(&policy_trials)
        .chain(&package_trials)
        .all(passed_trial);
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:font-resource-static-baseline",
        "status": if passed { "passed" } else { "failed" },
        "profile": {
            "name": OPENTYPE_STATIC_PROFILE,
            "primary_parser": "ttf-parser 0.25.1 behind NUIF sfnt range and checksum validation",
            "independent_parser": "Skrifa 0.46.2 / read-fonts",
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
        "independent": independent.to_json(),
        "parser_trials": parser_trials,
        "negative_trials": negative_trials,
        "policy_trials": policy_trials,
        "package_trials": package_trials,
        "source": source_identity(),
        "non_claims": [
            "one Ahem static TrueType fixture is not broad OpenType conformance",
            "no TTC CFF CFF2 variable color bitmap SVG WOFF WOFF2 or subsetting support",
            "no shaping outline raster browser or cross-platform font reproduction is established by this gate",
            "fsType and a recorded review are policy evidence, not an automated license decision or redistribution grant",
            "the parsers are independent libraries but the fixture author and harness remain in this repository",
        ],
        "summary": {
            "parser": parser_trials.len(),
            "negative": negative_trials.len(),
            "policy": policy_trials.len(),
            "package": package_trials.len(),
            "blocking_failures": parser_trials.iter().chain(&negative_trials).chain(&policy_trials).chain(&package_trials).filter(|item| !passed_trial(item)).count(),
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
        "font resources: {} parser, {} negative, {} policy/package, status {}",
        parser_trials.len(),
        negative_trials.len(),
        policy_trials.len() + package_trials.len(),
        if passed { "passed" } else { "failed" }
    );
    if passed {
        Ok(())
    } else {
        Err(format!("report failed; inspect {}", output.display()))
    }
}

#[derive(Debug)]
struct IndependentInspection {
    units_per_em: u16,
    glyph_count: u16,
    axis_count: usize,
    coverage: Vec<CodepointRange>,
    mapping_hash: String,
}

impl IndependentInspection {
    fn to_json(&self) -> Value {
        json!({
            "units_per_em": self.units_per_em,
            "glyph_count": self.glyph_count,
            "axis_count": self.axis_count,
            "coverage": self.coverage,
            "mapping_sha256": self.mapping_hash,
        })
    }
}

fn inspect_independent(bytes: &[u8]) -> Result<IndependentInspection, String> {
    let font = FontRef::new(bytes).map_err(|error| error.to_string())?;
    let metrics = font.metrics(Size::unscaled(), LocationRef::default());
    let mappings = font
        .charmap()
        .mappings()
        .map(|(codepoint, glyph)| (codepoint, glyph.to_u32()))
        .collect::<Vec<_>>();
    let mut mapping_bytes = Vec::with_capacity(mappings.len().saturating_mul(8));
    for (codepoint, glyph) in &mappings {
        mapping_bytes.extend_from_slice(&codepoint.to_be_bytes());
        mapping_bytes.extend_from_slice(&glyph.to_be_bytes());
    }
    Ok(IndependentInspection {
        units_per_em: metrics.units_per_em,
        glyph_count: metrics.glyph_count,
        axis_count: font.axes().len(),
        coverage: coverage_from_mappings(&mappings),
        mapping_hash: sha256(&mapping_bytes),
    })
}

fn coverage_from_mappings(mappings: &[(u32, u32)]) -> Vec<CodepointRange> {
    let mut codepoints = mappings
        .iter()
        .map(|(codepoint, _)| *codepoint)
        .filter(|codepoint| char::from_u32(*codepoint).is_some())
        .collect::<Vec<_>>();
    codepoints.sort_unstable();
    codepoints.dedup();
    let mut ranges = Vec::new();
    let Some(mut start) = codepoints.first().copied() else {
        return ranges;
    };
    let mut previous = start;
    for codepoint in codepoints.into_iter().skip(1) {
        if codepoint == previous + 1 {
            previous = codepoint;
        } else {
            ranges.push(CodepointRange {
                start,
                end: previous,
            });
            start = codepoint;
            previous = codepoint;
        }
    }
    ranges.push(CodepointRange {
        start,
        end: previous,
    });
    ranges
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
    cases
        .into_iter()
        .map(|(name, bytes, face)| trial(name, inspect_opentype_static(&bytes, face).is_err()))
        .collect()
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
