use nuif_font::{OPENTYPE_VARIABLE_TRUETYPE_PROFILE, inspect_opentype_variable_metadata};
use nuif_text::{ShapeRequest, TextDirection, VariableResourceTrial};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;

const GOLDEN_JSON: &str =
    include_str!("../../../../conformance/font/harfbuzz-14.4.0-hvar-truncated-map.json");
const FIXTURE_SHA256: &str = "6549bbde7cdb55869d167a2587fad20dd33503a3b5c63dcd74b5be20d1978d1a";
const FIXTURE_FAMILY: &str = "HVAR SingleModel Indirect";
const FIXTURE_LICENSE_EVIDENCE: &str =
    "font-test-data package: MIT OR Apache-2.0; publisher review not asserted";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Golden {
    schema_version: u32,
    tool: String,
    version: String,
    capture_command: String,
    fixture: String,
    font_sha256: String,
    cases: Vec<GoldenCase>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GoldenCase {
    label: String,
    text: String,
    user: BTreeMap<String, f64>,
    normalized_2_14: Vec<i16>,
    serialized_glyphs: String,
    glyph_advances_font_units: Vec<i32>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("variable-font-metrics: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let output = output_path()?;
    let bytes = font_test_data::HVAR_WITH_TRUNCATED_ADVANCE_INDEX_MAP;
    let golden: Golden = serde_json::from_str(GOLDEN_JSON).map_err(|error| error.to_string())?;
    let inspection =
        inspect_opentype_variable_metadata(bytes, 0).map_err(|error| error.to_string())?;
    let identity_trials = vec![
        trial(
            "pinned_harfbuzz_hvar_oracle",
            golden.schema_version == 1
                && golden.tool == "HarfBuzz public C API and hb-shape"
                && golden.version == "14.4.0"
                && !golden.capture_command.is_empty()
                && golden
                    .fixture
                    .contains("hvar_with_truncated_adv_index_map.ttf"),
        ),
        trial(
            "exact_resource_identity",
            sha256(bytes) == FIXTURE_SHA256 && golden.font_sha256 == FIXTURE_SHA256,
        ),
        trial(
            "bounded_single_hvar_axis",
            inspection.font.decoder_profile == OPENTYPE_VARIABLE_TRUETYPE_PROFILE
                && inspection.axes.len() == 1
                && inspection.axes[0].tag == "wght"
                && inspection.avar_version.is_none()
                && inspection.font.table_tags.iter().any(|tag| tag == "HVAR"),
        ),
    ];
    let mut case_trials = Vec::new();
    let mut observations = BTreeMap::new();
    for expected in &golden.cases {
        let face = VariableResourceTrial::new_with_features(
            bytes,
            FIXTURE_SHA256,
            FIXTURE_FAMILY,
            FIXTURE_LICENSE_EVIDENCE,
            &expected.user,
            &BTreeMap::new(),
        )
        .map_err(|error| error.to_string())?;
        let request = ShapeRequest {
            text: &expected.text,
            font_sha256: FIXTURE_SHA256,
            font_size: 24.0,
            direction: TextDirection::LeftToRight,
            language: "en",
        };
        let first = face.shape(&request).map_err(|error| error.to_string())?;
        let second = face.shape(&request).map_err(|error| error.to_string())?;
        let normalized = first
            .coordinates
            .iter()
            .map(|coordinate| coordinate.normalized_2_14)
            .collect::<Vec<_>>();
        let skrifa_advances = first
            .run
            .glyphs
            .iter()
            .map(|glyph| face.glyph_advance_font_units(glyph.glyph_id))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        let run_advances = first
            .run
            .glyphs
            .iter()
            .map(|glyph| glyph.x_advance)
            .collect::<Vec<_>>();
        let passed = first == second
            && first.run.serialized_glyphs == expected.serialized_glyphs
            && normalized == expected.normalized_2_14
            && run_advances == expected.glyph_advances_font_units
            && skrifa_advances == expected.glyph_advances_font_units;
        case_trials.push(trial(
            &format!("harfbuzz_{}_hvar_metrics_agree", expected.label),
            passed,
        ));
        observations.insert(
            expected.label.clone(),
            json!({
                "coordinates": first.coordinates,
                "serialized_glyphs": first.run.serialized_glyphs,
                "glyph_advances_font_units": skrifa_advances,
            }),
        );
    }
    let metamorphic_trials = metamorphic_trials(bytes, &observations);
    finish_report(
        &output,
        &inspection,
        &identity_trials,
        &case_trials,
        &metamorphic_trials,
        &observations,
    )
}

fn metamorphic_trials(bytes: &[u8], observations: &BTreeMap<String, Value>) -> Vec<Value> {
    let advances = |label: &str| {
        observations
            .get(label)
            .and_then(|value| value.get("glyph_advances_font_units"))
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(Value::as_i64).collect::<Vec<_>>())
    };
    let default = advances("default");
    let medium = advances("named_medium");
    let interior = advances("interior");
    let maximum = advances("maximum");
    vec![
        trial(
            "all_three_hvar_advances_change",
            default.as_ref().is_some_and(|start| {
                maximum
                    .as_ref()
                    .is_some_and(|end| start.iter().zip(end).all(|(a, b)| a != b))
            }),
        ),
        trial(
            "hvar_interpolation_is_monotonic",
            [default, medium, interior, maximum]
                .windows(2)
                .all(|window| match (&window[0], &window[1]) {
                    (Some(left), Some(right)) => left.iter().zip(right).all(|(a, b)| a < b),
                    _ => false,
                }),
        ),
        trial(
            "below_minimum_is_rejected",
            VariableResourceTrial::new_with_features(
                bytes,
                FIXTURE_SHA256,
                FIXTURE_FAMILY,
                FIXTURE_LICENSE_EVIDENCE,
                &BTreeMap::from([("wght".to_owned(), 399.0)]),
                &BTreeMap::new(),
            )
            .is_err(),
        ),
    ]
}

fn finish_report(
    output: &PathBuf,
    inspection: &nuif_font::VariableFontInspection,
    identity_trials: &[Value],
    case_trials: &[Value],
    metamorphic_trials: &[Value],
    observations: &BTreeMap<String, Value>,
) -> Result<(), String> {
    let passed = identity_trials
        .iter()
        .chain(case_trials)
        .chain(metamorphic_trials)
        .all(passed_trial);
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:variable-font-hvar-baseline",
        "status": if passed { "passed" } else { "failed" },
        "candidate_profile": OPENTYPE_VARIABLE_TRUETYPE_PROFILE,
        "inspection": inspection,
        "identity_trials": identity_trials,
        "case_trials": case_trials,
        "metamorphic_trials": metamorphic_trials,
        "observations": observations,
        "summary": {
            "blocking_failures": identity_trials.iter().chain(case_trials).chain(metamorphic_trials).filter(|item| !passed_trial(item)).count(),
        },
        "non_claims": [
            "one deliberately truncated valid HVAR advance-index-map fixture is not broad HVAR conformance",
            "MVAR VVAR vertical text side-bearing variation and gvar phantom-point fallback remain outside this trial",
            "the variable resource remains unavailable to package validation layout sessions and render fidelity claims",
            "test-package distribution metadata does not replace a publisher embedding and redistribution review",
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
        "variable font metrics: {} trials, status {}",
        identity_trials.len() + case_trials.len() + metamorphic_trials.len(),
        if passed { "passed" } else { "failed" }
    );
    if passed {
        Ok(())
    } else {
        Err(format!("report failed; inspect {}", output.display()))
    }
}

fn trial(name: &str, passed: bool) -> Value {
    json!({ "name": name, "status": if passed { "passed" } else { "failed" } })
}

fn passed_trial(value: &Value) -> bool {
    value.get("status").and_then(Value::as_str) == Some("passed")
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn output_path() -> Result<PathBuf, String> {
    let mut arguments = env::args().skip(1);
    let mut output = PathBuf::from("target/variable-font-metrics-report.json");
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
                return Err("usage: variable-font-metrics [--output <json>]".to_owned());
            }
            _ => return Err(format!("unknown argument: {argument}")),
        }
    }
    Ok(output)
}
