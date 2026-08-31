use nuif_api::NuifDocument;
use nuif_core::Fidelity;
use nuif_font::OPENTYPE_VARIABLE_TRUETYPE_PROFILE;
use nuif_layout::EvaluationContext;
use nuif_package::NuifPackage;
use nuif_render::DrawCommand;
use nuif_testing::{
    VARIABLE_FONT_FIXTURE_SHA256, VARIABLE_FONT_FIXTURE_TEXT, VariableFontFixtureLocation,
    variable_font_package_fixture,
};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::PathBuf;

const FONT: &[u8] = include_bytes!(
    "../../../../conformance/font/fixtures/noto-sans-variable-subset/NotoSans-variable-subset.ttf"
);
const GOLDEN: &str =
    include_str!("../../../../conformance/font/harfbuzz-14.4.0-noto-sans-variable.json");

#[derive(Deserialize)]
struct Golden {
    font_sha256: String,
    cases: Vec<GoldenCase>,
}

#[derive(Deserialize)]
struct GoldenCase {
    label: String,
    user: BTreeMap<String, f64>,
    normalized_2_14: Vec<i16>,
    text: String,
    serialized_glyphs: String,
    glyph_advances_font_units: Vec<i32>,
    outline_glyph_id: u32,
    outline_serialized_path: String,
}

struct CaseEvidence {
    report: Value,
    canonical_hash: String,
    layout_width: f64,
    serialized_glyphs: String,
    outline_path: String,
    raster_sha256: String,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("variable-font-runtime: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let output = output_path()?;
    let golden: Golden = serde_json::from_str(GOLDEN).map_err(|error| error.to_string())?;
    if golden.font_sha256 != VARIABLE_FONT_FIXTURE_SHA256
        || format!("{:x}", Sha256::digest(FONT)) != VARIABLE_FONT_FIXTURE_SHA256
    {
        return Err("runtime fixture identity disagrees with its committed oracle".to_owned());
    }
    let default = evaluate_case(golden_case(&golden, "default")?)?;
    let interior = evaluate_case(golden_case(&golden, "interior")?)?;
    let cross_case_trials = vec![
        trial(
            "coordinates_change_canonical_document_identity",
            default.canonical_hash != interior.canonical_hash,
            &json!({
                "default": default.canonical_hash,
                "interior": interior.canonical_hash,
            }),
        ),
        trial(
            "coordinates_change_shaping_and_intrinsic_layout",
            default.serialized_glyphs != interior.serialized_glyphs
                && (default.layout_width - interior.layout_width).abs() > 0.001,
            &json!({
                "default_width": default.layout_width,
                "interior_width": interior.layout_width,
            }),
        ),
        trial(
            "coordinates_change_gvar_outlines_and_cpu_raster",
            default.outline_path != interior.outline_path
                && default.raster_sha256 != interior.raster_sha256,
            &json!({
                "default_raster": default.raster_sha256,
                "interior_raster": interior.raster_sha256,
            }),
        ),
    ];
    let passed = [&default.report, &interior.report]
        .into_iter()
        .flat_map(|report| report["trials"].as_array().into_iter().flatten())
        .chain(cross_case_trials.iter())
        .all(passed_trial);
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:variable-font-runtime",
        "status": if passed { "passed" } else { "failed" },
        "profile": OPENTYPE_VARIABLE_TRUETYPE_PROFILE,
        "fixture": {
            "family": "Noto Sans",
            "sha256": VARIABLE_FONT_FIXTURE_SHA256,
            "bytes": FONT.len(),
            "oracle": "HarfBuzz 14.4.0 public C API and hb-shape",
            "license": "OFL-1.1",
        },
        "cases": [default.report, interior.report],
        "cross_case_trials": cross_case_trials,
        "summary": {
            "cases": 2,
            "case_trials": 18,
            "cross_case_trials": 3,
            "blocking_failures": i32::from(!passed),
        },
        "non_claims": [
            "this gate proves the in-process Rust package-to-raster surface, not yet WASM, FFI, CLI, or MCP parity",
            "one runtime family does not replace the broader two-family parser and oracle corpus",
            "unhinted CPU raster identity is local to the pinned reference renderer and is not a cross-platform system-raster claim",
            "VVAR and vertical text remain outside the horizontal variable TrueType profile",
        ],
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
        "variable font runtime: 2 package-to-raster cases, status {}",
        if passed { "passed" } else { "failed" }
    );
    if passed {
        Ok(())
    } else {
        Err(format!("report failed; inspect {}", output.display()))
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "the case evaluator keeps the package-to-raster evidence chain visible in one audit flow"
)]
fn evaluate_case(oracle: &GoldenCase) -> Result<CaseEvidence, String> {
    if oracle.text != "AHfixÅé" {
        return Err(format!("{} oracle text drifted", oracle.label));
    }
    let location = match oracle.label.as_str() {
        "default" => VariableFontFixtureLocation::Default,
        "interior" => VariableFontFixtureLocation::Interior,
        _ => {
            return Err(format!(
                "{} is not a runtime fixture location",
                oracle.label
            ));
        }
    };
    let package = variable_font_package_fixture(location);
    let encoded = package.encode().map_err(|error| error.to_string())?;
    let decoded = NuifPackage::decode(&encoded).map_err(|error| error.to_string())?;
    let byte_fixpoint = decoded.encode().map_err(|error| error.to_string())? == encoded;
    let context = EvaluationContext::viewport(640.0, 96.0);
    let unauthorized_rejected = NuifDocument::load_package(&encoded)
        .map_err(|error| error.to_string())?
        .snapshot(&context)
        .is_err();
    let capabilities = BTreeSet::from([OPENTYPE_VARIABLE_TRUETYPE_PROFILE.to_owned()]);
    let document = NuifDocument::load_package_with_capabilities(&encoded, &capabilities)
        .map_err(|error| error.to_string())?;
    let first = document
        .snapshot(&context)
        .map_err(|error| error.to_string())?;
    let second = document
        .snapshot(&context)
        .map_err(|error| error.to_string())?;
    let (run, outlines) = first
        .scene
        .commands
        .iter()
        .find_map(|command| match command {
            DrawCommand::Text { run, outlines, .. } => Some((run.as_ref(), outlines.as_ref())),
            DrawCommand::Rect { .. } | DrawCommand::Ellipse { .. } | DrawCommand::Image { .. } => {
                None
            }
        })
        .ok_or_else(|| format!("{} case did not lower text", oracle.label))?;
    let outline = outlines
        .get(&oracle.outline_glyph_id)
        .ok_or_else(|| format!("{} case omitted the oracle glyph outline", oracle.label))?;
    let layout_width = first
        .layout
        .boxes
        .get(&VARIABLE_FONT_FIXTURE_TEXT)
        .map_or(0.0, |rect| rect.width);
    let expected_width = run
        .glyphs
        .iter()
        .map(|glyph| f64::from(glyph.x_advance))
        .sum::<f64>()
        .abs()
        * run.font_size
        / f64::from(run.units_per_em);
    let observed_advances = run
        .glyphs
        .iter()
        .map(|glyph| glyph.x_advance)
        .collect::<Vec<_>>();
    let observed_normalized = run
        .variation_coordinates
        .iter()
        .map(|coordinate| coordinate.normalized_2_14)
        .collect::<Vec<_>>();
    let raster_sha256 = format!("{:x}", Sha256::digest(&first.raster.rgba));
    let trials = vec![
        trial(
            "package_byte_fixpoint",
            byte_fixpoint,
            &json!({"package_bytes": encoded.len()}),
        ),
        trial(
            "capability_required_before_snapshot",
            unauthorized_rejected,
            &json!({"capability": OPENTYPE_VARIABLE_TRUETYPE_PROFILE}),
        ),
        trial(
            "exact_font_identity_reaches_scene",
            run.font.sha256 == VARIABLE_FONT_FIXTURE_SHA256 && run.font.family == "Noto Sans",
            &json!({"font_sha256": run.font.sha256, "family": run.font.family}),
        ),
        trial(
            "normalized_coordinates_reach_scene",
            observed_normalized == oracle.normalized_2_14
                && run.variation_coordinates.len() == oracle.user.len(),
            &json!({"normalized_2_14": observed_normalized}),
        ),
        trial(
            "harfbuzz_shaping_oracle_matches",
            run.serialized_glyphs == oracle.serialized_glyphs
                && observed_advances == oracle.glyph_advances_font_units,
            &json!({"serialized_glyphs": run.serialized_glyphs, "advances": observed_advances}),
        ),
        trial(
            "hvar_advances_drive_intrinsic_layout",
            (layout_width - expected_width).abs() < 0.001,
            &json!({"layout_width": layout_width, "expected_width": expected_width}),
        ),
        trial(
            "gvar_outline_oracle_matches",
            outline.serialized_path == oracle.outline_serialized_path,
            &json!({"glyph_id": oracle.outline_glyph_id}),
        ),
        trial(
            "exact_variable_render_is_lossless",
            first.scene.fidelity.iter().any(|entry| {
                entry.entity == Some(VARIABLE_FONT_FIXTURE_TEXT)
                    && entry.status == Fidelity::Lossless
            }),
            &json!({}),
        ),
        trial(
            "snapshot_is_deterministic",
            first == second,
            &json!({"raster_sha256": raster_sha256}),
        ),
    ];
    Ok(CaseEvidence {
        report: json!({
            "label": oracle.label,
            "user_coordinates": oracle.user,
            "trials": trials,
        }),
        canonical_hash: first.canonical_hash,
        layout_width,
        serialized_glyphs: run.serialized_glyphs.clone(),
        outline_path: outline.serialized_path.clone(),
        raster_sha256,
    })
}

fn golden_case<'a>(golden: &'a Golden, label: &str) -> Result<&'a GoldenCase, String> {
    golden
        .cases
        .iter()
        .find(|case| case.label == label)
        .ok_or_else(|| format!("oracle omitted {label} case"))
}

fn trial(name: &str, passed: bool, evidence: &Value) -> Value {
    json!({
        "name": name,
        "status": if passed { "passed" } else { "failed" },
        "evidence": evidence,
    })
}

fn passed_trial(value: &Value) -> bool {
    value.get("status").and_then(Value::as_str) == Some("passed")
}

fn output_path() -> Result<PathBuf, String> {
    let mut args = env::args().skip(1);
    match (args.next().as_deref(), args.next(), args.next()) {
        (None, None, None) => Ok(PathBuf::from("target/variable-font-runtime-report.json")),
        (Some("--output"), Some(path), None) => Ok(PathBuf::from(path)),
        _ => Err("usage: variable-font-runtime [--output PATH]".to_owned()),
    }
}
