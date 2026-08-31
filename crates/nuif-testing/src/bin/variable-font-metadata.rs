use nuif_font::{
    MAX_AVAR_SEGMENTS_PER_AXIS, MAX_FONT_BYTES, MAX_FONT_TABLES, MAX_VARIABLE_AXES,
    MAX_VARIABLE_INSTANCES, OPENTYPE_VARIABLE_TRUETYPE_PROFILE, inspect_opentype_variable_metadata,
    normalize_variable_coordinates,
};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;

const GOLDEN_JSON: &str =
    include_str!("../../../../conformance/font/harfbuzz-14.4.0-material-symbols-variable.json");
const FIXTURE_SHA256: &str = "fdd9bade0cde742725168298e39291309c95a826acb979cef1142063f17f44ab";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Golden {
    schema_version: u32,
    tool: String,
    version: String,
    capture_command: String,
    font_sha256: String,
    axes: Vec<GoldenAxis>,
    named_instance_count: usize,
    coordinates: Vec<GoldenCoordinates>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GoldenAxis {
    tag: String,
    minimum: f64,
    default: f64,
    maximum: f64,
    hidden: bool,
    name_id: u16,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GoldenCoordinates {
    label: String,
    user: BTreeMap<String, f64>,
    normalized_2_14: Vec<i16>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("variable-font-metadata: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let output = output_path()?;
    let bytes = font_test_data::MATERIAL_SYMBOLS_SUBSET;
    let inspection = inspect_opentype_variable_metadata(bytes, 0)
        .map_err(|error| format!("NUIF metadata inspection failed: {error}"))?;
    let golden: Golden = serde_json::from_str(GOLDEN_JSON).map_err(|error| error.to_string())?;

    let identity_trials = vec![
        trial(
            "pinned_oracle_identity",
            golden.schema_version == 1
                && golden.tool == "HarfBuzz public C API"
                && golden.version == "14.4.0"
                && !golden.capture_command.is_empty(),
        ),
        trial(
            "exact_resource_identity",
            sha256(bytes) == FIXTURE_SHA256 && golden.font_sha256 == FIXTURE_SHA256,
        ),
        trial(
            "candidate_profile_is_explicit",
            inspection.font.decoder_profile == OPENTYPE_VARIABLE_TRUETYPE_PROFILE
                && inspection.font.face_index == 0
                && inspection.avar_version.as_deref() == Some("1.0"),
        ),
        trial(
            "variable_tables_are_bounded",
            ["fvar", "gvar", "STAT"]
                .iter()
                .all(|tag| inspection.font.table_tags.iter().any(|item| item == tag))
                && !inspection.font.table_tags.iter().any(|tag| {
                    ["CFF ", "CFF2", "COLR", "CPAL", "SVG ", "VARC"].contains(&tag.as_str())
                }),
        ),
    ];

    let axis_trials = inspection
        .axes
        .iter()
        .zip(&golden.axes)
        .enumerate()
        .map(|(index, (observed, expected))| {
            trial(
                &format!("harfbuzz_axis_{index}_agrees"),
                observed.tag == expected.tag
                    && observed.minimum_16_16 == to_fixed(expected.minimum)
                    && observed.default_16_16 == to_fixed(expected.default)
                    && observed.maximum_16_16 == to_fixed(expected.maximum)
                    && observed.hidden == expected.hidden
                    && observed.name_id == expected.name_id,
            )
        })
        .chain(std::iter::once(trial(
            "harfbuzz_axis_and_instance_counts_agree",
            inspection.axes.len() == golden.axes.len()
                && inspection.named_instance_count == golden.named_instance_count,
        )))
        .collect::<Vec<_>>();

    let coordinate_trials = golden
        .coordinates
        .iter()
        .map(|expected| {
            let first = normalize_variable_coordinates(bytes, &expected.user);
            let second = normalize_variable_coordinates(bytes, &expected.user);
            let agrees = first.as_ref().is_ok_and(|coordinates| {
                coordinates
                    .iter()
                    .map(|coordinate| coordinate.normalized_2_14)
                    .eq(expected.normalized_2_14.iter().copied())
            });
            trial(
                &format!("harfbuzz_{}_normalization_agrees", expected.label),
                agrees && first == second,
            )
        })
        .collect::<Vec<_>>();

    let negative_trials = negative_trials(bytes, &inspection.axes);
    finish_report(
        &output,
        &inspection,
        &identity_trials,
        &axis_trials,
        &coordinate_trials,
        &negative_trials,
    )
}

fn finish_report(
    output: &PathBuf,
    inspection: &nuif_font::VariableFontInspection,
    identity_trials: &[Value],
    axis_trials: &[Value],
    coordinate_trials: &[Value],
    negative_trials: &[Value],
) -> Result<(), String> {
    let passed = identity_trials
        .iter()
        .chain(axis_trials)
        .chain(coordinate_trials)
        .chain(negative_trials)
        .all(passed_trial);
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:variable-font-metadata-baseline",
        "status": if passed { "passed" } else { "failed" },
        "candidate_profile": OPENTYPE_VARIABLE_TRUETYPE_PROFILE,
        "fixture": {
            "name": "font-test-data 0.9.1 material_symbols_subset.ttf",
            "sha256": FIXTURE_SHA256,
            "package_license": "MIT OR Apache-2.0",
            "upstream": "https://github.com/googlefonts/fontations",
        },
        "limits": {
            "encoded_bytes": MAX_FONT_BYTES,
            "tables": MAX_FONT_TABLES,
            "axes": MAX_VARIABLE_AXES,
            "named_instances": MAX_VARIABLE_INSTANCES,
            "avar_segments_per_axis": MAX_AVAR_SEGMENTS_PER_AXIS,
        },
        "inspection": inspection,
        "oracle": serde_json::from_str::<Value>(GOLDEN_JSON).map_err(|error| error.to_string())?,
        "identity_trials": identity_trials,
        "axis_trials": axis_trials,
        "coordinate_trials": coordinate_trials,
        "negative_trials": negative_trials,
        "summary": {
            "blocking_failures": identity_trials.iter().chain(axis_trials).chain(coordinate_trials).chain(negative_trials).filter(|item| !passed_trial(item)).count(),
        },
        "non_claims": [
            "metadata and coordinate agreement does not admit variable fonts into NUIF packages",
            "one four-axis fixture is not broad OpenType variable-font conformance",
            "shaping FeatureVariations HVAR MVAR gvar outlines rendering and cross-surface parity remain unproven",
            "the fixture package license and fsType metadata do not replace a publisher redistribution review",
            "the HarfBuzz oracle is a committed capture and the capture helper is not executed in offline CI",
        ],
    });
    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(
        output,
        serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    println!(
        "variable font metadata: {} trials, status {}",
        identity_trials.len() + axis_trials.len() + coordinate_trials.len() + negative_trials.len(),
        if passed { "passed" } else { "failed" }
    );
    if passed {
        Ok(())
    } else {
        Err(format!("report failed; inspect {}", output.display()))
    }
}

fn negative_trials(bytes: &[u8], axes: &[nuif_font::VariableAxisInspection]) -> Vec<Value> {
    let defaults = axes
        .iter()
        .map(|axis| (axis.tag.clone(), f64::from(axis.default_16_16) / 65_536.0))
        .collect::<BTreeMap<_, _>>();
    let mut missing = defaults.clone();
    missing.remove(&axes[0].tag);
    let mut unknown = defaults.clone();
    unknown.insert("ZZZZ".to_owned(), 0.0);
    let mut outside = defaults.clone();
    outside.insert(
        axes[0].tag.clone(),
        f64::from(axes[0].maximum_16_16) / 65_536.0 + 1.0,
    );
    let mut non_finite = defaults;
    non_finite.insert(axes[0].tag.clone(), f64::NAN);
    vec![
        trial(
            "static_font_rejected",
            inspect_opentype_variable_metadata(font_test_data::AHEM, 0).is_err(),
        ),
        trial(
            "cff2_variable_rejected",
            inspect_opentype_variable_metadata(font_test_data::CANTARELL_VF_TRIMMED, 0).is_err(),
        ),
        trial(
            "color_variable_rejected",
            inspect_opentype_variable_metadata(font_test_data::COLRV0V1_VARIABLE, 0).is_err(),
        ),
        trial(
            "nonzero_face_rejected",
            inspect_opentype_variable_metadata(bytes, 1).is_err(),
        ),
        trial(
            "missing_axis_rejected",
            normalize_variable_coordinates(bytes, &missing).is_err(),
        ),
        trial(
            "unknown_axis_rejected",
            normalize_variable_coordinates(bytes, &unknown).is_err(),
        ),
        trial(
            "out_of_range_axis_rejected",
            normalize_variable_coordinates(bytes, &outside).is_err(),
        ),
        trial(
            "non_finite_axis_rejected",
            normalize_variable_coordinates(bytes, &non_finite).is_err(),
        ),
    ]
}

fn trial(name: &str, passed: bool) -> Value {
    json!({ "name": name, "status": if passed { "passed" } else { "failed" } })
}

fn passed_trial(value: &Value) -> bool {
    value.get("status").and_then(Value::as_str) == Some("passed")
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "oracle axis values are small exact OpenType values"
)]
fn to_fixed(value: f64) -> i32 {
    (value * 65_536.0).round() as i32
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn output_path() -> Result<PathBuf, String> {
    let mut arguments = env::args().skip(1);
    let mut output = PathBuf::from("target/variable-font-metadata-report.json");
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
                return Err("usage: variable-font-metadata [--output <json>]".to_owned());
            }
            _ => return Err(format!("unknown argument: {argument}")),
        }
    }
    Ok(output)
}
