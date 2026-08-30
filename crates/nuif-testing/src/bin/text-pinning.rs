use nuif_api::{Session, profile_zero_context};
use nuif_core::{EntityId, Fidelity, SizeIntent};
use nuif_layout::{EvaluationContext, WritingDirection};
use nuif_render::DrawCommand;
use nuif_text::{
    OUTLINE_COORDINATE_DENOMINATOR, OUTLINE_EXTRACTOR_NAME, OUTLINE_EXTRACTOR_VERSION,
    PINNED_FONT_ASCENDER, PINNED_FONT_SHA256, SHAPER_NAME, SHAPER_VERSION, ShapeRequest,
    TextDirection, UNICODE_VERSION, outline_glyph, pinned_font_hash_is_valid, pinned_font_identity,
    shape,
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
    outlines: Vec<GoldenOutline>,
    raster_baseline: RasterBaseline,
    cases: Vec<GoldenCase>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Oracle {
    implementation: String,
    version: String,
    serialization: String,
    capture_command: String,
    outline_serialization: String,
    outline_normalization: String,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GoldenOutline {
    glyph_id: u32,
    expected: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RasterBaseline {
    os: String,
    architecture: String,
    pipeline: String,
    verified_platforms: Vec<PlatformIdentity>,
    cases: Vec<RasterCase>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlatformIdentity {
    os: String,
    architecture: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RasterCase {
    width: u32,
    height: u32,
    direction: WritingDirection,
    scene_sha256: String,
    rgba_sha256: String,
    png_sha256: String,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("text-pinning: {error}");
        std::process::exit(1);
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "the machine report assembly is kept contiguous for auditability"
)]
fn run() -> Result<(), String> {
    let output = output_path()?;
    let golden: GoldenFile =
        serde_json::from_str(GOLDEN_JSON).map_err(|error| error.to_string())?;
    let identity = pinned_font_identity();
    let pin_consistent = golden.schema_version == 2
        && golden.font.family == identity.family
        && golden.font.version == identity.version
        && golden.font.sha256 == identity.sha256
        && golden.font.byte_length == identity.byte_length
        && golden.oracle.implementation == "HarfBuzz"
        && golden.oracle.version == "14.4.0"
        && pinned_font_hash_is_valid();

    let (case_reports, shaping_passed) = shaping_trials(&golden.cases, pin_consistent)?;
    let (outline_reports, outlines_passed) = outline_trials(&golden.outlines, pin_consistent)?;

    let (raster_reports, raster_passed) = raster_trials(&golden.raster_baseline.cases)?;
    let (text_semantic_reports, text_semantics_passed) = text_semantic_trials()?;
    let (negative_reports, negative_passed) = negative_trials();
    let passed = shaping_passed
        && outlines_passed
        && raster_passed
        && text_semantics_passed
        && negative_passed;
    let cross_platform_raster_verified = raster_passed
        && golden
            .raster_baseline
            .verified_platforms
            .iter()
            .any(|platform| {
                platform.os != golden.raster_baseline.os
                    || platform.architecture != golden.raster_baseline.architecture
            });
    let report = json!({
        "schema_version": 2,
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
            "outline_extractor": OUTLINE_EXTRACTOR_NAME,
            "outline_extractor_version": OUTLINE_EXTRACTOR_VERSION,
            "outline_coordinate_denominator": OUTLINE_COORDINATE_DENOMINATOR,
            "rasterizer": "Zeno",
            "rasterizer_version": "0.3.3",
            "hinting": "none",
            "coverage": "8_bit_grayscale_alpha",
            "fill_rule": "nonzero",
            "blend_space": "encoded_srgb_channels",
            "baseline_font_units": PINNED_FONT_ASCENDER,
            "raster_baseline": golden.raster_baseline,
            "independent_oracle": golden.oracle,
            "pin_consistent": pin_consistent
        },
        "classification": {
            "shaping": "exact_cross_implementation_golden",
            "outlines": "exact_cross_implementation_normalized_golden",
            "raster": "exact_raw_rgba_on_recorded_platform_matrix",
            "png_encoding": "repeatable_artifact_non_normative",
            "text_semantics": "exact_hard_break_no_soft_wrap_profile",
            "cross_platform_raster_verified": cross_platform_raster_verified
        },
        "summary": {
            "golden_cases": case_reports.len(),
            "golden_cases_passed": case_reports.iter().filter(|case| case["passed"] == true).count(),
            "outline_goldens": outline_reports.len(),
            "outline_goldens_passed": outline_reports.iter().filter(|case| case["passed"] == true).count(),
            "raster_contexts": raster_reports.len(),
            "text_semantic_cases": text_semantic_reports.len(),
            "negative_cases": negative_reports.len(),
            "blocking_failures": u8::from(!passed)
        },
        "golden_cases": case_reports,
        "outline_goldens": outline_reports,
        "raster_trials": raster_reports,
        "text_semantic_trials": text_semantic_reports,
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
        "text pinning: {} shaping goldens, {} outline goldens, {} raster contexts, status {}",
        golden.cases.len(),
        golden.outlines.len(),
        raster_reports.len(),
        if passed { "passed" } else { "failed" }
    );
    if passed {
        Ok(())
    } else {
        Err(format!("report failed; inspect {}", output.display()))
    }
}

fn shaping_trials(
    cases: &[GoldenCase],
    pin_consistent: bool,
) -> Result<(Vec<Value>, bool), String> {
    let mut reports = Vec::new();
    let mut all_passed = pin_consistent;
    for case in cases {
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
        all_passed &= passed;
        reports.push(json!({
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
    Ok((reports, all_passed))
}

fn outline_trials(
    outlines: &[GoldenOutline],
    pin_consistent: bool,
) -> Result<(Vec<Value>, bool), String> {
    let mut reports = Vec::new();
    let mut all_passed = pin_consistent;
    for expected in outlines {
        let first = outline_glyph(expected.glyph_id).map_err(|error| error.to_string())?;
        let second = outline_glyph(expected.glyph_id).map_err(|error| error.to_string())?;
        let golden_match = first.serialized_path == expected.expected;
        let repeatable = first == second;
        let passed = golden_match && repeatable;
        all_passed &= passed;
        reports.push(json!({
            "glyph_id": expected.glyph_id,
            "expected": expected.expected,
            "observed": first.serialized_path,
            "command_count": first.commands.len(),
            "golden_match": golden_match,
            "repeatable": repeatable,
            "passed": passed
        }));
    }
    Ok((reports, all_passed))
}

fn raster_trials(cases: &[RasterCase]) -> Result<(Vec<Value>, bool), String> {
    let document = nuif_testing::responsive_card_fixture();
    let mut reports = Vec::new();
    let mut all_passed = true;
    for case in cases {
        let mut context = profile_zero_context(f64::from(case.width), f64::from(case.height));
        context.writing_direction = case.direction;
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
                DrawCommand::Rect { .. }
                | DrawCommand::Ellipse { .. }
                | DrawCommand::Image { .. } => None,
            })
            .collect::<Vec<_>>();
        let lossless_text = first.scene.fidelity.iter().any(|item| {
            item.entity == Some(EntityId::new(0x22)) && matches!(item.status, Fidelity::Lossless)
        });
        let scene_sha256 =
            sha256_hex(&serde_json::to_vec(&first.scene).map_err(|error| error.to_string())?);
        let rgba_sha256 = sha256_hex(&first.raster.rgba);
        let png_sha256 = sha256_hex(&first_png);
        let repeatable = first.scene == second.scene && first_png == second_png;
        let baseline_match = scene_sha256 == case.scene_sha256 && rgba_sha256 == case.rgba_sha256;
        let png_reference_match = png_sha256 == case.png_sha256;
        let passed = repeatable && baseline_match && !glyph_runs.is_empty() && lossless_text;
        all_passed &= passed;
        reports.push(json!({
            "context_fingerprint": context.fingerprint(),
            "viewport": [case.width, case.height],
            "writing_direction": case.direction,
            "scene_sha256": scene_sha256,
            "expected_scene_sha256": case.scene_sha256,
            "rgba_sha256": rgba_sha256,
            "expected_rgba_sha256": case.rgba_sha256,
            "png_sha256": png_sha256,
            "expected_png_sha256": case.png_sha256,
            "png_reference_match": png_reference_match,
            "png_bytes": first_png.len(),
            "glyph_runs": glyph_runs,
            "repeatable": repeatable,
            "baseline_match": baseline_match,
            "raster_pipeline": "pinned_unhinted_outline_grayscale",
            "text_semantic_fidelity": "lossless_hard_break_no_soft_wrap_profile",
            "passed": passed
        }));
    }
    Ok((reports, all_passed))
}

fn text_semantic_trials() -> Result<(Vec<Value>, bool), String> {
    let text_id = EntityId::new(0x22);
    let mut hard_break_document = nuif_testing::responsive_card_fixture();
    let hard_break_text = hard_break_document
        .entities
        .get_mut(&text_id)
        .and_then(|entity| entity.authored.text.as_mut())
        .ok_or_else(|| "the text fixture is absent".to_owned())?;
    hard_break_text.content.clear();
    hard_break_text.content.push_str("A\r\nB");
    let hard_break_snapshot = Session::new(hard_break_document)
        .snapshot(&profile_zero_context(360.0, 640.0))
        .map_err(|error| error.to_string())?;
    let hard_break_runs = text_commands(&hard_break_snapshot.scene, text_id);
    let hard_break_passed = hard_break_runs.len() == 2
        && hard_break_runs[0].0 == "A"
        && hard_break_runs[1].0 == "B"
        && (hard_break_runs[1].1 - hard_break_runs[0].1 - 24.0).abs() < f64::EPSILON;

    let mut no_wrap_document = nuif_testing::responsive_card_fixture();
    let no_wrap_entity = no_wrap_document
        .entities
        .get_mut(&text_id)
        .ok_or_else(|| "the text fixture is absent".to_owned())?;
    no_wrap_entity.authored.width = SizeIntent::Fixed(18.0);
    let no_wrap_content = &mut no_wrap_entity
        .authored
        .text
        .as_mut()
        .ok_or_else(|| "the text fixture has no text content".to_owned())?
        .content;
    no_wrap_content.clear();
    no_wrap_content.push_str("A B");
    let no_wrap_snapshot = Session::new(no_wrap_document)
        .snapshot(&profile_zero_context(360.0, 640.0))
        .map_err(|error| error.to_string())?;
    let no_wrap_runs = text_commands(&no_wrap_snapshot.scene, text_id);
    let no_wrap_passed = no_wrap_runs.len() == 1
        && no_wrap_runs[0].0 == "A B"
        && (no_wrap_runs[0].2 - 18.0).abs() < f64::EPSILON;
    let lossless = no_wrap_snapshot
        .scene
        .fidelity
        .iter()
        .any(|item| item.entity == Some(text_id) && matches!(item.status, Fidelity::Lossless));

    Ok((
        vec![
            json!({
                "name": "mandatory-crlf-hard-break",
                "runs": hard_break_runs,
                "passed": hard_break_passed
            }),
            json!({
                "name": "no-automatic-soft-wrap",
                "runs": no_wrap_runs,
                "lossless": lossless,
                "passed": no_wrap_passed && lossless
            }),
        ],
        hard_break_passed && no_wrap_passed && lossless,
    ))
}

fn text_commands(scene: &nuif_render::RenderScene, entity: EntityId) -> Vec<(String, f64, f64)> {
    scene
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text {
                entity: id,
                rect,
                run,
                ..
            } if *id == entity => Some((run.text.clone(), rect.y, rect.width)),
            DrawCommand::Rect { .. }
            | DrawCommand::Ellipse { .. }
            | DrawCommand::Image { .. }
            | DrawCommand::Text { .. } => None,
        })
        .collect()
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
