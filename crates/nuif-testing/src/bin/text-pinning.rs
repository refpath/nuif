use nuif_api::{Session, profile_zero_context};
use nuif_core::{EntityId, Fidelity};
use nuif_layout::{EvaluationContext, WritingDirection};
use nuif_render::DrawCommand;
use nuif_text::{
    PINNED_FONT_SHA256, SHAPER_NAME, SHAPER_VERSION, ShapeRequest, TextDirection, UNICODE_VERSION,
    pinned_font_hash_is_valid, pinned_font_identity, shape,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const GOLDEN_JSON: &str = include_str!("../../../../conformance/text/harfbuzz-14.4.0-ahem.json");

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GoldenFile {
    schema_version: u32,
    oracle: Oracle,
    font: GoldenFont,
    cases: Vec<GoldenCase>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Oracle {
    implementation: String,
    version: String,
    serialization: String,
    capture_command: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GoldenFont {
    family: String,
    version: String,
    sha256: String,
    byte_length: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GoldenCase {
    name: String,
    text: String,
    direction: TextDirection,
    language: String,
    expected: String,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("text-pinning: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let output = output_path()?;
    let golden: GoldenFile =
        serde_json::from_str(GOLDEN_JSON).map_err(|error| error.to_string())?;
    let identity = pinned_font_identity();
    let pin_consistent = golden.schema_version == 1
        && golden.font.family == identity.family
        && golden.font.version == identity.version
        && golden.font.sha256 == identity.sha256
        && golden.font.byte_length == identity.byte_length
        && golden.oracle.implementation == "HarfBuzz"
        && golden.oracle.version == "14.4.0"
        && pinned_font_hash_is_valid();

    let mut case_reports = Vec::new();
    let mut shaping_passed = pin_consistent;
    for case in &golden.cases {
        let request = ShapeRequest {
            text: &case.text,
            font_sha256: PINNED_FONT_SHA256,
            font_size: 18.0,
            direction: case.direction,
            language: &case.language,
        };
        let first = shape(&request).map_err(|error| error.to_string())?;
        let second = shape(&request).map_err(|error| error.to_string())?;
        let golden_match = first.serialized_glyphs == case.expected;
        let repeatable = first == second;
        let passed = golden_match && repeatable;
        shaping_passed &= passed;
        case_reports.push(json!({
            "name": case.name,
            "text": case.text,
            "direction": case.direction,
            "language": case.language,
            "expected": case.expected,
            "observed": first.serialized_glyphs,
            "glyph_count": first.glyphs.len(),
            "x_advance_font_units": first.x_advance_font_units,
            "golden_match": golden_match,
            "repeatable": repeatable,
            "passed": passed
        }));
    }

    let (raster_reports, raster_passed) = raster_trials()?;
    let (negative_reports, negative_passed) = negative_trials();
    let passed = shaping_passed && raster_passed && negative_passed;
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:text-pinning",
        "status": if passed { "passed" } else { "failed" },
        "source": {
            "revision": command_text("git", &["rev-parse", "HEAD"]),
            "dirty": command_text("git", &["status", "--porcelain"]).map(|value| !value.is_empty()),
            "toolchain": command_text("rustc", &["--version"]),
            "os": env::consts::OS,
            "architecture": env::consts::ARCH
        },
        "pins": {
            "font": identity,
            "font_hash_valid": pinned_font_hash_is_valid(),
            "shaper": SHAPER_NAME,
            "shaper_version": SHAPER_VERSION,
            "unicode_version": UNICODE_VERSION,
            "independent_oracle": golden.oracle,
            "pin_consistent": pin_consistent
        },
        "classification": {
            "shaping": "exact_cross_implementation_golden",
            "raster": "approximated_deterministic_glyph_id_proxy",
            "normative_outline_rasterization": false
        },
        "summary": {
            "golden_cases": case_reports.len(),
            "golden_cases_passed": case_reports.iter().filter(|case| case["passed"] == true).count(),
            "raster_contexts": raster_reports.len(),
            "negative_cases": negative_reports.len(),
            "blocking_failures": u8::from(!passed)
        },
        "golden_cases": case_reports,
        "raster_trials": raster_reports,
        "negative_trials": negative_reports
    });
    let encoded = serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?;
    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(&output, &encoded).map_err(|error| error.to_string())?;
    println!(
        "text pinning: {} golden cases, {} raster contexts, status {}",
        golden.cases.len(),
        raster_reports.len(),
        if passed { "passed" } else { "failed" }
    );
    if passed {
        Ok(())
    } else {
        Err(format!("report failed; inspect {}", output.display()))
    }
}

fn raster_trials() -> Result<(Vec<Value>, bool), String> {
    let document = nuif_testing::responsive_card_fixture();
    let configurations = [
        (360_u32, 640_u32, WritingDirection::LeftToRight),
        (768, 768, WritingDirection::LeftToRight),
        (1440, 900, WritingDirection::RightToLeft),
    ];
    let mut reports = Vec::new();
    let mut all_passed = true;
    for (width, height, direction) in configurations {
        let mut context = profile_zero_context(f64::from(width), f64::from(height));
        context.writing_direction = direction;
        let session = Session::new(document.clone());
        let first = session
            .snapshot(&context)
            .map_err(|error| error.to_string())?;
        let second = session
            .snapshot(&context)
            .map_err(|error| error.to_string())?;
        let first_png = first.raster.to_png().map_err(|error| error.to_string())?;
        let second_png = second.raster.to_png().map_err(|error| error.to_string())?;
        let glyph_runs = first
            .scene
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { run, .. } => Some(run.serialized_glyphs.clone()),
                DrawCommand::Rect { .. } => None,
            })
            .collect::<Vec<_>>();
        let approximated_text = first.scene.fidelity.iter().any(|item| {
            item.entity == EntityId::new(0x22)
                && matches!(item.status, Fidelity::Approximated { .. })
        });
        let repeatable = first.scene == second.scene && first_png == second_png;
        let passed = repeatable && !glyph_runs.is_empty() && approximated_text;
        all_passed &= passed;
        reports.push(json!({
            "context_fingerprint": context.fingerprint(),
            "viewport": [width, height],
            "writing_direction": direction,
            "scene_sha256": sha256_hex(&serde_json::to_vec(&first.scene).map_err(|error| error.to_string())?),
            "png_sha256": sha256_hex(&first_png),
            "png_bytes": first_png.len(),
            "glyph_runs": glyph_runs,
            "repeatable": repeatable,
            "text_raster_fidelity": "approximated",
            "passed": passed
        }));
    }
    Ok((reports, all_passed))
}

fn negative_trials() -> (Vec<Value>, bool) {
    let document = nuif_testing::responsive_card_fixture();
    let missing_context = Session::new(document.clone())
        .snapshot(&EvaluationContext::viewport(360.0, 640.0))
        .map_or_else(
            |error| error.to_string(),
            |_| "unexpected success".to_owned(),
        );
    let missing_passed = missing_context.contains("absent from the evaluation context");

    let mut invalid = document;
    let invalid_hash = &mut invalid
        .entities
        .get_mut(&EntityId::new(0x22))
        .and_then(|entity| entity.authored.text.as_mut())
        .expect("fixture copy is text")
        .font_sha256;
    invalid_hash.clear();
    invalid_hash.push_str("INVALID");
    let invalid_result = Session::new(invalid)
        .snapshot(&profile_zero_context(360.0, 640.0))
        .map_or_else(
            |error| error.to_string(),
            |_| "unexpected success".to_owned(),
        );
    let invalid_passed = invalid_result.contains("document validation failed");

    (
        vec![
            json!({
                "name": "font-absent-from-context",
                "observed": missing_context,
                "passed": missing_passed
            }),
            json!({
                "name": "malformed-font-hash",
                "observed": invalid_result,
                "passed": invalid_passed
            }),
        ],
        missing_passed && invalid_passed,
    )
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn output_path() -> Result<PathBuf, String> {
    let mut args = env::args().skip(1);
    let mut output = PathBuf::from("target/text-pinning-report.json");
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--output" => {
                output = PathBuf::from(
                    args.next()
                        .ok_or_else(|| "--output requires a path".to_owned())?,
                );
            }
            unknown => return Err(format!("unknown argument: {unknown}")),
        }
    }
    Ok(output)
}

fn command_text(program: &str, arguments: &[&str]) -> Option<String> {
    let output = Command::new(program).args(arguments).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
