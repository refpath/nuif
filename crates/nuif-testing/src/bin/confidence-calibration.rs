use nuif_core::ResourceDigest;
use nuif_reconstruct::evaluation::confidence::{
    CONFIDENCE_EVALUATION_PROFILE, ConfidenceCase, ConfidenceEvaluationConfig,
    ConfidenceEvaluationReport, ConfidencePartition, DecisionKind, DistributionCondition,
    ShiftAxis,
};
use serde_json::{Value, json};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    if let Err(error) = run() {
        eprintln!("confidence-calibration: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let output = output_path()?;
    let cases = cases();
    let report = ConfidenceEvaluationReport::evaluate(config(), &cases)
        .map_err(|error| format!("evaluation failed: {error}"))?;
    let bytes = serde_json::to_vec(&report).map_err(|error| error.to_string())?;
    let decoded: ConfidenceEvaluationReport =
        serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    let reencoded = serde_json::to_vec(&decoded).map_err(|error| error.to_string())?;
    let decoded_valid = decoded.validate().is_ok();
    let original_value: Value =
        serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    let reencoded_value: Value =
        serde_json::from_slice(&reencoded).map_err(|error| error.to_string())?;
    let round_trip = decoded_valid && original_value == reencoded_value;
    let decisions = report.decisions.len();
    let shifted_conditions = report
        .decisions
        .iter()
        .flat_map(|decision| decision.conditions.iter())
        .filter(|condition| !matches!(condition.condition, DistributionCondition::InDistribution))
        .count();
    let selective_evidence = report
        .decisions
        .iter()
        .flat_map(|decision| decision.conditions.iter())
        .all(|condition| {
            condition.policy.samples == condition.raw.samples
                && condition.raw.brier_score.is_finite()
                && condition.calibrated.expected_calibration_error.is_finite()
        });
    let checks = vec![
        check("typed_report_round_trip", round_trip),
        check("multiple_decision_kinds", decisions >= 2),
        check("normal_and_shifted_holdouts", shifted_conditions >= 2),
        check(
            "selective_metrics_are_derived_and_bounded",
            selective_evidence,
        ),
    ];
    let passed = checks.iter().all(|check| check["passed"] == true);
    let artifact = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:inference-confidence-calibration",
        "profile": CONFIDENCE_EVALUATION_PROFILE,
        "status": if passed { "passed" } else { "failed" },
        "source": source_identity(),
        "cases": cases,
        "report": report,
        "checks": checks,
        "non_claims": [
            "the synthetic cases exercise evaluator arithmetic and split policy only",
            "no model was trained and no calibration threshold is suitable for production",
            "shifted cases are contract fixtures, not a distributional accuracy estimate",
            "automatic application still requires a frozen rights-cleared corpus and held-out calibration"
        ]
    });
    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut output_bytes =
        serde_json::to_vec_pretty(&artifact).map_err(|error| error.to_string())?;
    output_bytes.push(b'\n');
    fs::write(&output, output_bytes).map_err(|error| error.to_string())?;
    println!(
        "confidence calibration: {} decisions, {} cases, status {}",
        decisions,
        cases.len(),
        if passed { "passed" } else { "failed" }
    );
    if passed {
        Ok(())
    } else {
        Err(format!("report failed; inspect {}", output.display()))
    }
}

fn config() -> ConfidenceEvaluationConfig {
    ConfidenceEvaluationConfig {
        schema_version: 1,
        profile: CONFIDENCE_EVALUATION_PROFILE.to_owned(),
        corpus_manifest: digest('a'),
        samples_artifact: digest('b'),
        calibrator_artifact: digest('c'),
        evaluator_artifact: digest('d'),
        bins: 5,
        automatic_risk_limit: 0.25,
        review_risk_limit: 0.5,
    }
}

fn cases() -> Vec<ConfidenceCase> {
    [
        (DecisionKind::Text, "text"),
        (DecisionKind::Geometry, "geometry"),
    ]
    .into_iter()
    .flat_map(|(decision, prefix)| cases_for(decision, prefix))
    .collect()
}

fn cases_for(decision: DecisionKind, prefix: &str) -> Vec<ConfidenceCase> {
    let mut cases = calibration_cases(decision, prefix);
    cases.extend(in_distribution_test_cases(decision, prefix));
    cases.extend(shifted_test_cases(decision, prefix));
    cases
}

fn calibration_cases(decision: DecisionKind, prefix: &str) -> Vec<ConfidenceCase> {
    vec![
        case(
            prefix,
            "cal-1",
            ConfidencePartition::Calibration,
            0.2,
            0.25,
            false,
            decision,
            DistributionCondition::InDistribution,
        ),
        case(
            prefix,
            "cal-2",
            ConfidencePartition::Calibration,
            0.4,
            0.45,
            true,
            decision,
            DistributionCondition::InDistribution,
        ),
        case(
            prefix,
            "cal-3",
            ConfidencePartition::Calibration,
            0.7,
            0.75,
            true,
            decision,
            DistributionCondition::InDistribution,
        ),
        case(
            prefix,
            "cal-4",
            ConfidencePartition::Calibration,
            0.9,
            0.9,
            true,
            decision,
            DistributionCondition::InDistribution,
        ),
    ]
}

fn in_distribution_test_cases(decision: DecisionKind, prefix: &str) -> Vec<ConfidenceCase> {
    vec![
        case(
            prefix,
            "test-1",
            ConfidencePartition::Test,
            0.15,
            0.2,
            false,
            decision,
            DistributionCondition::InDistribution,
        ),
        case(
            prefix,
            "test-2",
            ConfidencePartition::Test,
            0.5,
            0.55,
            true,
            decision,
            DistributionCondition::InDistribution,
        ),
        case(
            prefix,
            "test-3",
            ConfidencePartition::Test,
            0.85,
            0.88,
            true,
            decision,
            DistributionCondition::InDistribution,
        ),
    ]
}

fn shifted_test_cases(decision: DecisionKind, prefix: &str) -> Vec<ConfidenceCase> {
    vec![
        case(
            prefix,
            "shift-1",
            ConfidencePartition::Test,
            0.3,
            0.35,
            false,
            decision,
            shifted_font_condition(),
        ),
        case(
            prefix,
            "shift-2",
            ConfidencePartition::Test,
            0.6,
            0.65,
            true,
            decision,
            shifted_font_condition(),
        ),
    ]
}

fn shifted_font_condition() -> DistributionCondition {
    DistributionCondition::Shifted {
        axis: ShiftAxis::Font,
        label: "unseen-font".to_owned(),
    }
}

#[allow(clippy::too_many_arguments)]
fn case(
    prefix: &str,
    suffix: &str,
    partition: ConfidencePartition,
    raw_confidence: f64,
    calibrated_confidence: f64,
    correct: bool,
    decision: DecisionKind,
    condition: DistributionCondition,
) -> ConfidenceCase {
    ConfidenceCase {
        id: format!("{prefix}-{suffix}"),
        group_id: format!("{prefix}-{suffix}"),
        partition,
        condition,
        decision,
        raw_confidence,
        calibrated_confidence,
        correct,
    }
}

fn digest(value: char) -> ResourceDigest {
    ResourceDigest::from_sha256_hex(value.to_string().repeat(64))
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
    let mut output = PathBuf::from("target/confidence-calibration-report.json");
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--output" => {
                output = arguments
                    .next()
                    .ok_or_else(|| "--output requires a path".to_owned())?
                    .into();
            }
            "--help" | "-h" => {
                return Err("usage: confidence-calibration [--output <json>]".to_owned());
            }
            unknown => return Err(format!("unknown argument {unknown:?}")),
        }
    }
    Ok(output)
}
