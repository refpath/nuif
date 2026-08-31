use nuif_font::{OPENTYPE_VARIABLE_TRUETYPE_PROFILE, inspect_opentype_variable_metadata};
use nuif_text::{ShapeRequest, TextDirection, VariableGlobalMetrics, VariableResourceTrial};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;

const FONT_BYTES: &[u8] = include_bytes!(
    "../../../../conformance/font/fixtures/roboto-flex-mvar-subset/RobotoFlex-MVAR-subset.ttf"
);
const GOLDEN_JSON: &str =
    include_str!("../../../../conformance/font/harfbuzz-14.4.0-roboto-flex-mvar.json");
const FIXTURE_SHA256: &str = "4fe568be6e73133adf9eb03e87d094ddd7c73f4250c61d3356b55e2ea7886ea9";
const FIXTURE_FAMILY: &str = "Roboto Flex";
const FIXTURE_LICENSE_EVIDENCE: &str =
    "OFL-1.1; exact license and derived-fixture provenance retained";

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
    global_metrics_font_units: GoldenMetrics,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct GoldenMetrics {
    hasc: i32,
    hdsc: i32,
    hlgp: i32,
    xhgt: i32,
    cpht: i32,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("variable-font-global-metrics: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let output = output_path()?;
    let golden: Golden = serde_json::from_str(GOLDEN_JSON).map_err(|error| error.to_string())?;
    let inspection =
        inspect_opentype_variable_metadata(FONT_BYTES, 0).map_err(|error| error.to_string())?;
    let identity_trials = vec![
        trial(
            "pinned_harfbuzz_mvar_oracle",
            golden.schema_version == 1
                && golden.tool == "HarfBuzz public C API and hb-shape"
                && golden.version == "14.4.0"
                && !golden.capture_command.is_empty()
                && golden.fixture.contains("Roboto Flex"),
        ),
        trial(
            "exact_derived_resource_identity",
            sha256(FONT_BYTES) == FIXTURE_SHA256 && golden.font_sha256 == FIXTURE_SHA256,
        ),
        trial(
            "bounded_mvar_variable_truetype",
            inspection.font.decoder_profile == OPENTYPE_VARIABLE_TRUETYPE_PROFILE
                && inspection.axes.len() == 13
                && inspection.avar_version.as_deref() == Some("1.0")
                && inspection
                    .font
                    .names
                    .iter()
                    .any(|name| name == FIXTURE_FAMILY)
                && inspection.font.table_tags.iter().any(|tag| tag == "MVAR")
                && inspection.font.table_tags.iter().any(|tag| tag == "HVAR"),
        ),
    ];
    let mut case_trials = Vec::new();
    let mut observations = BTreeMap::new();
    for expected in &golden.cases {
        let face = VariableResourceTrial::new_with_features(
            FONT_BYTES,
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
        let metrics = face
            .global_metrics_font_units()
            .map_err(|error| error.to_string())?;
        let repeated_metrics = face
            .global_metrics_font_units()
            .map_err(|error| error.to_string())?;
        let normalized = first
            .coordinates
            .iter()
            .map(|coordinate| coordinate.normalized_2_14)
            .collect::<Vec<_>>();
        let expected_metrics = VariableGlobalMetrics {
            ascent: expected.global_metrics_font_units.hasc,
            descent: expected.global_metrics_font_units.hdsc,
            line_gap: expected.global_metrics_font_units.hlgp,
            x_height: Some(expected.global_metrics_font_units.xhgt),
            cap_height: Some(expected.global_metrics_font_units.cpht),
        };
        case_trials.push(trial(
            &format!("harfbuzz_{}_mvar_metrics_agree", expected.label),
            first == second
                && first.run.serialized_glyphs == expected.serialized_glyphs
                && normalized == expected.normalized_2_14
                && metrics == repeated_metrics
                && metrics == expected_metrics,
        ));
        observations.insert(
            expected.label.clone(),
            json!({
                "coordinates": first.coordinates,
                "serialized_glyphs": first.run.serialized_glyphs,
                "global_metrics_font_units": metrics,
            }),
        );
    }
    let metamorphic_trials = metamorphic_trials(&observations);
    finish_report(
        &output,
        &inspection,
        &identity_trials,
        &case_trials,
        &metamorphic_trials,
        &observations,
    )
}

fn metamorphic_trials(observations: &BTreeMap<String, Value>) -> Vec<Value> {
    let metric = |label: &str, name: &str| {
        observations
            .get(label)?
            .get("global_metrics_font_units")?
            .get(name)?
            .as_i64()
    };
    let shape = |label: &str| observations.get(label)?.get("serialized_glyphs")?.as_str();
    vec![
        trial(
            "x_height_axis_changes_only_x_height_metric",
            metric("x_height_minimum", "x_height") < metric("default", "x_height")
                && metric("default", "x_height") < metric("x_height_interior", "x_height")
                && metric("x_height_interior", "x_height") < metric("x_height_maximum", "x_height")
                && metric("x_height_minimum", "cap_height")
                    == metric("x_height_maximum", "cap_height")
                && shape("x_height_minimum") == shape("x_height_maximum"),
        ),
        trial(
            "cap_height_axis_changes_only_cap_height_metric",
            metric("cap_height_maximum", "cap_height") > metric("default", "cap_height")
                && metric("cap_height_minimum", "x_height")
                    == metric("cap_height_maximum", "x_height")
                && shape("cap_height_minimum") == shape("cap_height_maximum"),
        ),
        trial(
            "optical_size_changes_mvar_and_glyph_advances",
            metric("optical_minimum", "x_height") > metric("default", "x_height")
                && metric("default", "x_height") > metric("optical_maximum", "x_height")
                && shape("optical_minimum") != shape("optical_maximum"),
        ),
        trial(
            "line_metrics_stay_constant_in_bounded_cases",
            observations.keys().all(|label| {
                metric(label, "ascent") == Some(1900)
                    && metric(label, "descent") == Some(-500)
                    && metric(label, "line_gap") == Some(0)
            }),
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
        "experiment": "nuif:experiment:variable-font-mvar-baseline",
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
            "one two-glyph OFL-derived fixture is not broad MVAR conformance",
            "VVAR side-bearing metrics gvar phantom-point fallback malformed variation graphs and resource ceilings remain outside this trial",
            "the variable resource remains unavailable to package validation layout sessions and render fidelity claims",
            "retained OFL provenance is fixture evidence rather than a general automated font-rights determination",
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
        "variable font global metrics: {} trials, status {}",
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
    let mut output = PathBuf::from("target/variable-font-global-metrics-report.json");
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
                return Err("usage: variable-font-global-metrics [--output <json>]".to_owned());
            }
            _ => return Err(format!("unknown argument: {argument}")),
        }
    }
    Ok(output)
}
