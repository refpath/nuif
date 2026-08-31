use nuif_core::ResourceDigest;
use nuif_reconstruct::Bounds;
use nuif_reconstruct::evaluation::aggregation::EvaluationAggregate;
use nuif_reconstruct::evaluation::{
    CalibrationMetrics, CalibrationSample, CostMetrics, DetectionMetrics, EVALUATION_PROFILE,
    ErrorMetrics, EvaluationReport, EvaluationSuite, GeometryMetrics, PerceptualDiagnostic,
    PixelMetrics, PropertyMetrics, RateMetric, TextMetrics, TreeMetrics, character_error,
    word_error,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

fn main() {
    if let Err(error) = run() {
        eprintln!("reconstruction-evaluation: {error}");
        std::process::exit(1);
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "the gate keeps one auditable path from metric inputs through hostile checks to its report"
)]
fn run() -> Result<(), String> {
    let output = output_path()?;
    let started = Instant::now();

    let reference_bounds = [bounds(0.0, 0.0, 40.0, 20.0), bounds(44.0, 0.0, 20.0, 20.0)];
    let candidate_bounds = [bounds(0.0, 0.0, 40.0, 20.0), bounds(46.0, 0.0, 20.0, 20.0)];
    let held_out_reference = [bounds(0.0, 0.0, 96.0, 24.0)];
    let held_out_candidate = [bounds(0.0, 0.0, 92.0, 24.0)];

    let reference_pixels = [0_u8, 102, 255, 255].repeat(16);
    let mut candidate_pixels = reference_pixels.clone();
    candidate_pixels[0] = 8;
    candidate_pixels[1] = 94;
    candidate_pixels[2] = 247;
    let pixels = PixelMetrics::compare(4, 4, &reference_pixels, &candidate_pixels)
        .map_err(|error| error.to_string())?;
    let elements = DetectionMetrics::new(3, 0, 1);
    let confidence = CalibrationMetrics::evaluate(
        &[
            CalibrationSample {
                confidence: 0.95,
                correct: true,
            },
            CalibrationSample {
                confidence: 0.80,
                correct: true,
            },
            CalibrationSample {
                confidence: 0.70,
                correct: false,
            },
            CalibrationSample {
                confidence: 0.40,
                correct: true,
            },
        ],
        5,
        &[0.0, 0.5, 0.75, 0.9],
    )
    .map_err(|error| error.to_string())?;

    let mut report = EvaluationReport {
        schema_version: 1,
        profile: EVALUATION_PROFILE.to_owned(),
        example_id: "synthetic-evaluation-fixture-1".to_owned(),
        suite: EvaluationSuite::SyntheticExact,
        validity: RateMetric::new(3, 4),
        text: TextMetrics {
            regions: DetectionMetrics::new(2, 1, 1),
            characters: character_error("NUIF editor", "NUIF edtor")
                .map_err(|error| error.to_string())?,
            words: word_error("portable design document", "portable document")
                .map_err(|error| error.to_string())?,
            baseline_geometry: ErrorMetrics::new(3, 1.25),
        },
        elements,
        tree: TreeMetrics {
            parents: RateMetric::new(3, 4),
            sibling_pairs: RateMetric::new(2, 3),
        },
        properties: PropertyMetrics {
            exact: RateMetric::new(7, 9),
            numeric: ErrorMetrics::new(4, 1.5),
        },
        geometry: GeometryMetrics::compare(&reference_bounds, &candidate_bounds)
            .map_err(|error| error.to_string())?,
        held_out_layout: Some(
            GeometryMetrics::compare(&held_out_reference, &held_out_candidate)
                .map_err(|error| error.to_string())?,
        ),
        resources: RateMetric::new(2, 3),
        provenance_honesty: RateMetric::new(4, 4),
        accessibility: Some(RateMetric::new(2, 3)),
        pixels,
        perceptual_diagnostics: vec![PerceptualDiagnostic {
            method: "rec709-luma-similarity-v1".to_owned(),
            value: rec709_luma_similarity(&reference_pixels, &candidate_pixels),
            lower_is_better: false,
            artifact: Some(ResourceDigest::from_sha256_hex(sha256(
                b"nuif fixture rec709 luma similarity v1: 1 - mean(abs(y1-y2))/255",
            ))),
        }],
        confidence,
        cost: CostMetrics {
            latency_microseconds: None,
            peak_ram_bytes: None,
            peak_vram_bytes: None,
            iterations: 2,
            external_cost_microunits: 0,
            currency: None,
        },
    };
    report.cost.latency_microseconds =
        Some(u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX));
    report.validate().map_err(|error| error.to_string())?;

    let mut second_report = report.clone();
    "synthetic-evaluation-fixture-2".clone_into(&mut second_report.example_id);
    second_report.validity = RateMetric::new(4, 4);
    second_report.accessibility = None;
    second_report.cost.latency_microseconds = None;
    let mut third_report = report.clone();
    "synthetic-evaluation-fixture-3".clone_into(&mut third_report.example_id);
    third_report.validity = RateMetric::new(2, 4);
    third_report.accessibility = Some(RateMetric::new(3, 3));
    third_report.cost.latency_microseconds = Some(40);
    let aggregate =
        EvaluationAggregate::build(&[third_report.clone(), report.clone(), second_report.clone()])
            .map_err(|error| error.to_string())?;
    let aggregate_bytes = serde_json::to_vec(&aggregate).map_err(|error| error.to_string())?;
    let decoded_aggregate: EvaluationAggregate =
        serde_json::from_slice(&aggregate_bytes).map_err(|error| error.to_string())?;
    let aggregate_round_trip = decoded_aggregate.validate().is_ok();

    let encoded = serde_json::to_vec(&report).map_err(|error| error.to_string())?;
    let decoded: EvaluationReport =
        serde_json::from_slice(&encoded).map_err(|error| error.to_string())?;
    let typed_report_round_trip = decoded.validate().is_ok()
        && decoded.profile == report.profile
        && decoded.example_id == report.example_id;

    let mut screenshot_resource_claim = report.clone();
    screenshot_resource_claim.suite = EvaluationSuite::RealScreenshot;
    let screenshot_resource_claim_rejected = screenshot_resource_claim.validate().is_err();

    let empty_denominators_unscored =
        RateMetric::new(0, 0).value.is_none() && DetectionMetrics::new(0, 0, 0).f1.is_none();
    let oversized_edit_rejected = character_error(&"x".repeat(4_097), &"x".repeat(4_097)).is_err();
    let local_error_visible = report.pixels.differing_pixels == 1
        && report.pixels.exact_pixel_rate.value == Some(15.0 / 16.0)
        && report.elements.false_negative == 1
        && report.elements.recall.value == Some(0.75);
    let mut derived_rate_drift = report.clone();
    derived_rate_drift.validity.value = Some(1.0);
    let derived_rate_drift_rejected = derived_rate_drift.validate().is_err();
    let aggregate_order_and_missingness = aggregate.example_ids
        == [
            "synthetic-evaluation-fixture-1",
            "synthetic-evaluation-fixture-2",
            "synthetic-evaluation-fixture-3",
        ]
        && aggregate.metrics.validity.micro.numerator == 9
        && aggregate.metrics.validity.micro.denominator == 12
        && aggregate
            .metrics
            .accessibility_accuracy
            .per_example
            .scored_examples
            == 2
        && aggregate
            .metrics
            .accessibility_accuracy
            .per_example
            .unscored_examples
            == 1;

    let checks = vec![
        check("typed_report_round_trip", typed_report_round_trip),
        check(
            "screenshot_source_resource_claim_rejected",
            screenshot_resource_claim_rejected,
        ),
        check(
            "zero_denominators_remain_unscored",
            empty_denominators_unscored,
        ),
        check(
            "edit_work_budget_precedes_allocation",
            oversized_edit_rejected,
        ),
        check(
            "local_and_element_errors_remain_visible",
            local_error_visible,
        ),
        check("derived_rate_drift_rejected", derived_rate_drift_rejected),
        check("typed_aggregate_round_trip", aggregate_round_trip),
        check(
            "aggregate_order_and_missingness_are_explicit",
            aggregate_order_and_missingness,
        ),
    ];
    let passed = checks.iter().all(|check| check["passed"] == true);
    let artifact = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:reconstruction-evaluation-contract",
        "status": if passed { "passed" } else { "failed" },
        "source": source_identity(),
        "metrics": report,
        "synthetic_aggregate": aggregate,
        "checks": checks,
        "non_claims": [
            "one deterministic synthetic contract fixture is not an accuracy distribution",
            "the fixture does not evaluate an OCR, detector, vision-language model, or correction provider",
            "the REC.709 luma diagnostic is not SSIM, FLIP, LPIPS, or a human perceptual oracle",
            "latency is shared-runner diagnostic evidence; peak RAM and VRAM are unavailable",
            "no independent evaluator or licensed real-screenshot corpus is exercised"
        ]
    });
    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut bytes = serde_json::to_vec_pretty(&artifact).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    fs::write(&output, bytes).map_err(|error| error.to_string())?;
    println!(
        "reconstruction evaluation: {} checks, status {}",
        checks.len(),
        if passed { "passed" } else { "failed" }
    );
    if passed {
        Ok(())
    } else {
        Err(format!("report failed; inspect {}", output.display()))
    }
}

fn bounds(x: f64, y: f64, width: f64, height: f64) -> Bounds {
    Bounds {
        x,
        y,
        width,
        height,
    }
}

fn rec709_luma_similarity(reference: &[u8], candidate: &[u8]) -> f64 {
    let (reference, reference_remainder) = reference.as_chunks::<4>();
    let (candidate, candidate_remainder) = candidate.as_chunks::<4>();
    debug_assert!(reference_remainder.is_empty() && candidate_remainder.is_empty());
    let absolute_error = reference
        .iter()
        .zip(candidate)
        .map(|(reference, candidate)| {
            let reference = 0.2126 * f64::from(reference[0])
                + 0.7152 * f64::from(reference[1])
                + 0.0722 * f64::from(reference[2]);
            let candidate = 0.2126 * f64::from(candidate[0])
                + 0.7152 * f64::from(candidate[1])
                + 0.0722 * f64::from(candidate[2]);
            (reference - candidate).abs()
        })
        .sum::<f64>();
    let pixels = f64::from(u32::try_from(reference.len()).unwrap_or(u32::MAX));
    1.0 - absolute_error / (pixels * 255.0)
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn check(name: &str, passed: bool) -> Value {
    json!({ "name": name, "passed": passed })
}

fn source_identity() -> Value {
    json!({
        "revision": command_text("git", &["rev-parse", "HEAD"]),
        "dirty": Command::new("git")
            .args(["diff", "--quiet", "--ignore-submodules", "HEAD", "--"])
            .status()
            .is_ok_and(|status| !status.success()),
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
    let mut output = PathBuf::from("target/reconstruction-evaluation-report.json");
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--output" => {
                output = arguments
                    .next()
                    .ok_or_else(|| "--output requires a path".to_owned())?
                    .into();
            }
            "--help" | "-h" => {
                return Err("usage: reconstruction-evaluation [--output <json>]".to_owned());
            }
            unknown => return Err(format!("unknown argument {unknown:?}")),
        }
    }
    Ok(output)
}
