use nuif_text::{ShapeRequest, TextDirection, VariableResourceTrial};
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
const FIXTURE_FAMILY: &str = "Material Symbols Outlined";
const FIXTURE_LICENSE_EVIDENCE: &str =
    "font-test-data package: MIT OR Apache-2.0; publisher review not asserted";

#[derive(Deserialize)]
struct Golden {
    schema_version: u32,
    tool: String,
    version: String,
    font_sha256: String,
    shaping: Vec<GoldenShape>,
}

#[derive(Deserialize)]
struct GoldenShape {
    label: String,
    text: String,
    user: BTreeMap<String, f64>,
    serialized_glyphs: String,
    glyph_advance_font_units: i32,
    outline_glyph_id: u32,
    outline_serialized_path: String,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("variable-font-shaping: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let output = output_path()?;
    let bytes = font_test_data::MATERIAL_SYMBOLS_SUBSET;
    let golden: Golden = serde_json::from_str(GOLDEN_JSON).map_err(|error| error.to_string())?;
    let identity_trials = vec![
        trial(
            "pinned_harfbuzz_shaping_oracle",
            golden.schema_version == 1
                && golden.tool == "HarfBuzz public C API and hb-shape"
                && golden.version == "14.4.0",
        ),
        trial(
            "exact_resource_identity",
            sha256(bytes) == FIXTURE_SHA256 && golden.font_sha256 == FIXTURE_SHA256,
        ),
        trial(
            "required_location_corpus",
            [
                "default",
                "minimum",
                "maximum",
                "positive_interior",
                "negative_interior",
                "feature_variation_below",
                "feature_variation_at",
            ]
            .iter()
            .all(|label| golden.shaping.iter().any(|item| &item.label == label)),
        ),
    ];
    let mut case_trials = Vec::with_capacity(golden.shaping.len());
    let mut observations = BTreeMap::new();
    for expected in &golden.shaping {
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
        let glyph = first
            .run
            .glyphs
            .first()
            .ok_or_else(|| format!("{} produced no glyph", expected.label))?;
        let advance = face
            .glyph_advance_font_units(glyph.glyph_id)
            .map_err(|error| error.to_string())?;
        let outline = face
            .outline_glyph(glyph.glyph_id)
            .map_err(|error| error.to_string())?;
        let repeated_outline = face
            .outline_glyph(glyph.glyph_id)
            .map_err(|error| error.to_string())?;
        let passed = first == second
            && first.run.serialized_glyphs == expected.serialized_glyphs
            && first.run.glyphs.len() == 1
            && glyph.glyph_id == expected.outline_glyph_id
            && advance == glyph.x_advance
            && advance == expected.glyph_advance_font_units
            && !outline.commands.is_empty()
            && outline.serialized_path == expected.outline_serialized_path
            && outline == repeated_outline;
        case_trials.push(trial(
            &format!("harfbuzz_{}_shaping_agrees", expected.label),
            passed,
        ));
        observations.insert(
            expected.label.clone(),
            json!({
                "coordinates": first.coordinates,
                "serialized_glyphs": first.run.serialized_glyphs,
                "ascender_font_units": first.run.ascender_font_units,
                "glyph_advance_font_units": advance,
                "outline_sha256": sha256(outline.serialized_path.as_bytes()),
                "harfbuzz_outline_sha256": sha256(expected.outline_serialized_path.as_bytes()),
                "outline_serialized_path": outline.serialized_path,
                "outline_commands": outline.commands.len(),
            }),
        );
    }
    let coherence_trials = coherence_trials(&observations);
    finish_report(
        &output,
        &identity_trials,
        &case_trials,
        &coherence_trials,
        &observations,
    )
}

fn coherence_trials(observations: &BTreeMap<String, Value>) -> Vec<Value> {
    let serialized = |label: &str| {
        observations
            .get(label)
            .and_then(|value| value.get("serialized_glyphs"))
            .and_then(Value::as_str)
    };
    let outline = |label: &str| {
        observations
            .get(label)
            .and_then(|value| value.get("outline_sha256"))
            .and_then(Value::as_str)
    };
    vec![
        trial(
            "feature_variation_threshold_changes_glyph",
            serialized("feature_variation_below") == Some("[1=0+960]")
                && serialized("feature_variation_at") == Some("[2=0+960]"),
        ),
        trial(
            "gvar_location_changes_outline",
            outline("default").is_some() && outline("default") != outline("positive_interior"),
        ),
        trial(
            "default_and_filled_instances_are_distinct",
            serialized("default") != serialized("maximum")
                && outline("default") != outline("maximum"),
        ),
    ]
}

fn finish_report(
    output: &PathBuf,
    identity_trials: &[Value],
    case_trials: &[Value],
    coherence_trials: &[Value],
    observations: &BTreeMap<String, Value>,
) -> Result<(), String> {
    let passed = identity_trials
        .iter()
        .chain(case_trials)
        .chain(coherence_trials)
        .all(passed_trial);
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:variable-font-shaping-baseline",
        "status": if passed { "passed" } else { "failed" },
        "candidate_profile": nuif_font::OPENTYPE_VARIABLE_TRUETYPE_PROFILE,
        "fixture_sha256": FIXTURE_SHA256,
        "identity_trials": identity_trials,
        "case_trials": case_trials,
        "coherence_trials": coherence_trials,
        "observations": observations,
        "summary": {
            "blocking_failures": identity_trials.iter().chain(case_trials).chain(coherence_trials).filter(|item| !passed_trial(item)).count(),
        },
        "non_claims": [
            "the variable resource remains unavailable to package validation layout sessions and render fidelity claims",
            "one fixture does not establish broad FeatureVariations shaping HVAR MVAR or gvar conformance",
            "the HarfBuzz advance and draw-callback capture is independent of the Rust metric and outline implementation but covers only one fixture",
            "the committed HarfBuzz oracle is not executed live in offline CI",
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
        "variable font shaping: {} trials, status {}",
        identity_trials.len() + case_trials.len() + coherence_trials.len(),
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
    let mut output = PathBuf::from("target/variable-font-shaping-report.json");
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
                return Err("usage: variable-font-shaping [--output <json>]".to_owned());
            }
            _ => return Err(format!("unknown argument: {argument}")),
        }
    }
    Ok(output)
}
