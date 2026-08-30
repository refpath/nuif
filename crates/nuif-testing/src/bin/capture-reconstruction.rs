use nuif_capture::{
    BROWSER_CAPTURE_PROFILE, BrowserCapture, BrowserNode, BrowserResource, OcrSpan,
    PNG_DECODER_PROFILE, SCREENSHOT_CAPTURE_PROFILE, ScreenshotCapture, SourceSpan, Viewport,
    analyze_screenshot, normalize_browser_capture,
};
use nuif_codec::canonical_hash;
use nuif_core::{
    Asset, AssetId, AssetKind, AssetPortability, Document, EntityId, ImageAsset, ResourceDigest,
};
use nuif_package::{NuifPackage, PackageMode};
use nuif_protocol::{Operation, Patch, Transaction};
use nuif_reconstruct::{
    Bounds, CalibrationPoint, CalibrationTable, CandidateEvaluator, CandidateScore, Confidence,
    CorrectionProvider, EvidenceClass, InferenceProvenance, LoopBudget, LoopStatus,
    ObservationValue, Proposal, ProposalPolicy, ProtectedMetric, ReconstructionError,
    ReviewDecision, apply_proposal, run_loop, selective_decision,
};
use png::{BitDepth, ColorType};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

fn main() {
    if let Err(error) = run() {
        eprintln!("capture-reconstruction: {error}");
        std::process::exit(1);
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "the executable intentionally keeps one auditable sequence from fixtures to report"
)]
fn run() -> Result<(), String> {
    let output = output_path()?;
    let total_started = Instant::now();
    let pixels = [0_u8, 102, 255, 255].repeat(16);
    let png = rgba_png(4, 4, &pixels, ColorType::Rgba)?;

    let browser_started = Instant::now();
    let browser_capture = browser_fixture(&png);
    let mut browser_package =
        NuifPackage::new(Document::empty(EntityId::new(1)), PackageMode::Authoring);
    let browser = normalize_browser_capture(&browser_capture, &mut browser_package)
        .map_err(|error| error.to_string())?;
    let mut repeated_browser_package =
        NuifPackage::new(Document::empty(EntityId::new(1)), PackageMode::Authoring);
    let repeated_browser =
        normalize_browser_capture(&browser_capture, &mut repeated_browser_package)
            .map_err(|error| error.to_string())?;
    let browser_elapsed = browser_started.elapsed().as_micros();
    let browser_observations = browser
        .observations
        .encode()
        .map_err(|error| error.to_string())?;
    let browser_resource = browser_package
        .resources
        .keys()
        .next()
        .cloned()
        .ok_or_else(|| "browser fixture retained no resource".to_owned())?;
    let browser_repeatable = browser == repeated_browser
        && browser_package
            .encode()
            .map_err(|error| error.to_string())?
            == repeated_browser_package
                .encode()
                .map_err(|error| error.to_string())?;
    let png_sha256 = sha256(&png);
    let browser_resource_exact = browser_package.embedded(&browser_resource)
        == Some(png.as_slice())
        && browser_resource.sha256_hex() == Some(png_sha256.as_str());
    let browser_secret_absent = !contains(&browser_observations, b"fixture-secret")
        && !contains(
            &serde_json::to_vec(&browser.proposal).map_err(|error| error.to_string())?,
            b"fixture-secret",
        )
        && !contains(
            &browser_package
                .encode()
                .map_err(|error| error.to_string())?,
            b"fixture-secret",
        );
    apply_proposal(
        &mut browser_package.document,
        &browser.observations,
        &browser.proposal,
        &ProposalPolicy::default(),
    )
    .map_err(|error| error.to_string())?;
    let browser_package_valid = browser_package.encode().is_ok()
        && browser_package.document.roots.len() == 1
        && browser_package.document.assets.len() == 1;

    let screenshot_started = Instant::now();
    let screenshot_capture = screenshot_fixture(&png);
    let mut screenshot_package =
        NuifPackage::new(Document::empty(EntityId::new(2)), PackageMode::Authoring);
    let screenshot = analyze_screenshot(&screenshot_capture, &mut screenshot_package)
        .map_err(|error| error.to_string())?;
    let mut repeated_screenshot_package =
        NuifPackage::new(Document::empty(EntityId::new(2)), PackageMode::Authoring);
    let repeated_screenshot =
        analyze_screenshot(&screenshot_capture, &mut repeated_screenshot_package)
            .map_err(|error| error.to_string())?;
    let screenshot_elapsed = screenshot_started.elapsed().as_micros();
    let screenshot_repeatable = screenshot == repeated_screenshot
        && screenshot_package
            .encode()
            .map_err(|error| error.to_string())?
            == repeated_screenshot_package
                .encode()
                .map_err(|error| error.to_string())?;
    let observation_fixpoint = screenshot.observations
        == nuif_reconstruct::ObservationBundle::decode(
            &screenshot
                .observations
                .encode()
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
    let honest_screenshot_evidence = screenshot
        .observations
        .observations
        .iter()
        .any(|observation| observation.evidence == EvidenceClass::ObservedPixels)
        && screenshot
            .observations
            .observations
            .iter()
            .any(|observation| observation.evidence == EvidenceClass::Inferred)
        && [
            "authored-structure",
            "source-resources",
            "responsive-behavior",
            "interaction",
        ]
        .into_iter()
        .all(|category| {
            screenshot
                .observations
                .omissions
                .iter()
                .any(|omission| omission.category == category)
        });
    apply_proposal(
        &mut screenshot_package.document,
        &screenshot.observations,
        &screenshot.proposal,
        &ProposalPolicy::default(),
    )
    .map_err(|error| error.to_string())?;
    let screenshot_package_valid = screenshot_package.encode().is_ok()
        && !screenshot_package.document.roots.is_empty()
        && screenshot_package.document.assets.is_empty();
    let flat_copy_rejected = flat_copy_rejected(&screenshot, EntityId::new(20))?;
    let correction = correction_trial(
        screenshot_package.document.clone(),
        &screenshot.observations,
    )?;
    let calibration = calibration_trial()?;
    let budget_stops = budget_stop_trial(&screenshot_package.document, &screenshot.observations)?;

    let mut cyclic_capture = browser_capture.clone();
    cyclic_capture.nodes[0].parent = Some(2);
    cyclic_capture.nodes[1].parent = Some(1);
    let browser_cycle_rejected = normalize_browser_capture(
        &cyclic_capture,
        &mut NuifPackage::new(Document::empty(EntityId::new(1)), PackageMode::Authoring),
    )
    .is_err();
    let mut wrong_dimensions = screenshot_capture.clone();
    wrong_dimensions.viewport.width = 5.0;
    let screenshot_dimensions_rejected = analyze_screenshot(
        &wrong_dimensions,
        &mut NuifPackage::new(Document::empty(EntityId::new(2)), PackageMode::Authoring),
    )
    .is_err();
    let grayscale = rgba_png(1, 1, &[0], ColorType::Grayscale)?;
    let mut unsupported_png = screenshot_capture;
    unsupported_png.viewport = Viewport {
        width: 1.0,
        height: 1.0,
        device_scale_factor: 1.0,
    };
    unsupported_png.png = grayscale;
    unsupported_png.ocr.clear();
    let ambiguous_png_rejected = analyze_screenshot(
        &unsupported_png,
        &mut NuifPackage::new(Document::empty(EntityId::new(2)), PackageMode::Authoring),
    )
    .is_err();

    let browser_trials = vec![
        trial("normalized_repeat_equivalence", browser_repeatable),
        trial("source_resource_digest_and_bytes", browser_resource_exact),
        trial("credential_query_redaction", browser_secret_absent),
        trial("typed_proposal_package", browser_package_valid),
        trial("cyclic_dom_rejected", browser_cycle_rejected),
    ];
    let screenshot_trials = vec![
        trial("analysis_repeat_equivalence", screenshot_repeatable),
        trial("observation_codec_fixpoint", observation_fixpoint),
        trial("evidence_and_omissions_honest", honest_screenshot_evidence),
        trial("typed_proposal_package", screenshot_package_valid),
        trial("flat_copy_rejected", flat_copy_rejected),
        trial(
            "dimension_mismatch_rejected",
            screenshot_dimensions_rejected,
        ),
        trial("ambiguous_png_rejected", ambiguous_png_rejected),
    ];
    let reconstruction_trials = vec![
        trial(
            "correction_improves_and_stops",
            correction.status == LoopStatus::RepeatedState
                && correction.accepted == 1
                && correction.final_score.objective == 0.0,
        ),
        trial("calibrated_selective_review", calibration),
        trial("finite_budget_stops", budget_stops),
    ];
    let passed = browser_trials
        .iter()
        .chain(&screenshot_trials)
        .chain(&reconstruction_trials)
        .all(|trial| trial["passed"] == true);
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:capture-reconstruction-contract-baselines",
        "status": if passed { "passed" } else { "failed" },
        "source": source_identity(),
        "profiles": {
            "browser": BROWSER_CAPTURE_PROFILE,
            "screenshot": SCREENSHOT_CAPTURE_PROFILE,
            "png": PNG_DECODER_PROFILE,
            "observations": nuif_reconstruct::OBSERVATION_PROFILE,
        },
        "measurements": {
            "browser_normalization_microseconds": browser_elapsed,
            "screenshot_analysis_microseconds": screenshot_elapsed,
            "total_microseconds": total_started.elapsed().as_micros(),
            "browser_observations": browser.observations.observations.len(),
            "screenshot_observations": screenshot.observations.observations.len(),
            "screenshot_regions": screenshot.regions.len(),
            "correction_provider_calls": correction.provider_calls,
            "correction_attempts": correction.attempts.len(),
        },
        "browser_trials": browser_trials,
        "screenshot_trials": screenshot_trials,
        "reconstruction_trials": reconstruction_trials,
        "non_claims": [
            "no live browser protocol capture or held-out responsive prediction",
            "no OCR engine accuracy claim; OCR spans are explicit provider inputs",
            "no broad screenshot reconstruction accuracy or real-task claim",
            "no calibrated risk-coverage corpus or independent evaluator",
            "no model training, LoRA, QLoRA, or distillation artifact",
        ],
        "summary": {
            "browser_trials": browser_trials.len(),
            "screenshot_trials": screenshot_trials.len(),
            "reconstruction_trials": reconstruction_trials.len(),
            "blocking_failures": browser_trials.iter().chain(&screenshot_trials).chain(&reconstruction_trials).filter(|trial| trial["passed"] != true).count(),
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
        "capture reconstruction: {} trials, status {}",
        browser_trials.len() + screenshot_trials.len() + reconstruction_trials.len(),
        if passed { "passed" } else { "failed" }
    );
    if passed {
        Ok(())
    } else {
        Err(format!("report failed; inspect {}", output.display()))
    }
}

fn browser_fixture(png: &[u8]) -> BrowserCapture {
    BrowserCapture {
        schema_version: 1,
        profile: BROWSER_CAPTURE_PROFILE.to_owned(),
        capture_id: "browser-baseline".to_owned(),
        adapter_version: "1".to_owned(),
        source_url: "https://example.invalid/?token=fixture-secret".to_owned(),
        viewport: Viewport {
            width: 100.0,
            height: 100.0,
            device_scale_factor: 1.0,
        },
        nodes: vec![
            BrowserNode {
                backend_node_id: 1,
                parent: None,
                order: 0,
                tag: "main".to_owned(),
                text: None,
                bounds: bounds(0.0, 0.0, 100.0, 100.0),
                background: Some([1.0, 1.0, 1.0, 1.0]),
                accessible_role: Some("main".to_owned()),
                accessible_name: Some("Captured example".to_owned()),
                source_span: Some(SourceSpan {
                    uri: "https://example.invalid/index.html?token=fixture-secret".to_owned(),
                    start: 0,
                    end: 64,
                }),
                resource_url: None,
            },
            BrowserNode {
                backend_node_id: 2,
                parent: Some(1),
                order: 0,
                tag: "img".to_owned(),
                text: None,
                bounds: bounds(10.0, 10.0, 40.0, 40.0),
                background: None,
                accessible_role: Some("image".to_owned()),
                accessible_name: Some("Fixture image".to_owned()),
                source_span: None,
                resource_url: Some(
                    "https://cdn.example.invalid/image.png?token=fixture-secret".to_owned(),
                ),
            },
        ],
        resources: vec![BrowserResource {
            url: "https://example.invalid/image.png?token=fixture-secret".to_owned(),
            final_url: "https://cdn.example.invalid/image.png?token=fixture-secret".to_owned(),
            media_type: "image/png".to_owned(),
            body: png.to_vec(),
            intrinsic_width: Some(4),
            intrinsic_height: Some(4),
        }],
        omitted_runtime: vec!["event-listeners-and-animation".to_owned()],
    }
}

fn screenshot_fixture(png: &[u8]) -> ScreenshotCapture {
    ScreenshotCapture {
        schema_version: 1,
        profile: SCREENSHOT_CAPTURE_PROFILE.to_owned(),
        capture_id: "screenshot-baseline".to_owned(),
        viewport: Viewport {
            width: 4.0,
            height: 4.0,
            device_scale_factor: 1.0,
        },
        png: png.to_vec(),
        ocr: vec![OcrSpan {
            id: "title".to_owned(),
            text: "NUIF".to_owned(),
            bounds: bounds(0.0, 0.0, 4.0, 2.0),
            raw_confidence: 0.9,
            engine: "fixture-ocr".to_owned(),
            engine_version: "1".to_owned(),
        }],
    }
}

fn flat_copy_rejected(
    screenshot: &nuif_capture::ScreenshotAnalysis,
    document_id: EntityId,
) -> Result<bool, String> {
    let evidence = screenshot
        .observations
        .observations
        .iter()
        .find(|observation| {
            matches!(
                observation.value,
                ObservationValue::Resource {
                    digest: Some(_),
                    ..
                }
            )
        })
        .ok_or_else(|| "screenshot has no resource observation".to_owned())?;
    let digest = match &evidence.value {
        ObservationValue::Resource {
            digest: Some(digest),
            ..
        } => digest.clone(),
        _ => unreachable!("resource observation was filtered above"),
    };
    let mut document = Document::empty(document_id);
    let id = AssetId::new(1);
    let proposal = Proposal {
        schema_version: 1,
        provenance: InferenceProvenance {
            method: "flat-copy".to_owned(),
            artifact: None,
            observations: BTreeSet::from([evidence.id.clone()]),
            confidence: Confidence::raw(1.0),
        },
        patch: Patch {
            base_revision: Some(canonical_hash(&document).map_err(|error| error.to_string())?),
            transactions: vec![Transaction {
                id: 1,
                operations: vec![Operation::SetAsset {
                    asset: Asset {
                        schema_version: 1,
                        id,
                        name: Some("flattened screenshot".to_owned()),
                        resource: Some(digest),
                        portability: AssetPortability::PrivateAuthoring,
                        kind: AssetKind::Image(ImageAsset {
                            width: 4,
                            height: 4,
                            decoder_profile: PNG_DECODER_PROFILE.to_owned(),
                        }),
                    },
                }],
            }],
        },
    };
    Ok(matches!(
        apply_proposal(
            &mut document,
            &screenshot.observations,
            &proposal,
            &ProposalPolicy::default()
        ),
        Err(ReconstructionError::ForbiddenOperation(
            "flattened_screenshot_asset"
        ))
    ) && document.assets.is_empty())
}

struct ImprovingProvider {
    entity: EntityId,
}

impl CorrectionProvider for ImprovingProvider {
    fn propose(
        &mut self,
        document: &Document,
        observations: &nuif_reconstruct::ObservationBundle,
        iteration: usize,
    ) -> Result<Option<Proposal>, String> {
        let evidence = observations
            .observations
            .first()
            .map(|observation| observation.id.clone())
            .ok_or_else(|| "correction observations are empty".to_owned())?;
        Ok(Some(Proposal {
            schema_version: 1,
            provenance: InferenceProvenance {
                method: "deterministic-correction".to_owned(),
                artifact: None,
                observations: BTreeSet::from([evidence]),
                confidence: Confidence::raw(0.5),
            },
            patch: Patch {
                base_revision: Some(canonical_hash(document).map_err(|error| error.to_string())?),
                transactions: vec![Transaction {
                    id: u128::try_from(iteration + 1).map_err(|error| error.to_string())?,
                    operations: vec![Operation::Rename {
                        entity: self.entity,
                        name: (iteration == 0).then(|| "improved".to_owned()),
                    }],
                }],
            },
        }))
    }
}

struct ObjectiveEvaluator {
    entity: EntityId,
}

impl CandidateEvaluator for ObjectiveEvaluator {
    fn evaluate(&mut self, document: &Document) -> Result<CandidateScore, String> {
        Ok(CandidateScore {
            objective: if document.entities[&self.entity].name.as_deref() == Some("improved") {
                0.0
            } else {
                1.0
            },
            metrics: BTreeMap::from([("validity".to_owned(), 0.0)]),
        })
    }
}

fn correction_trial(
    mut document: Document,
    observations: &nuif_reconstruct::ObservationBundle,
) -> Result<nuif_reconstruct::ReconstructionReport, String> {
    let entity = document.roots[0];
    run_loop(
        &mut document,
        observations,
        &mut ImprovingProvider { entity },
        &mut ObjectiveEvaluator { entity },
        &LoopBudget {
            protected_metrics: vec![ProtectedMetric {
                name: "validity".to_owned(),
                max_regression: 0.0,
            }],
            ..LoopBudget::default()
        },
    )
    .map_err(|error| error.to_string())
}

fn calibration_trial() -> Result<bool, String> {
    let table = CalibrationTable {
        artifact: ResourceDigest::from_sha256_hex("a".repeat(64)),
        points: vec![
            CalibrationPoint {
                raw: 0.0,
                calibrated: 0.1,
            },
            CalibrationPoint {
                raw: 1.0,
                calibrated: 0.9,
            },
        ],
    };
    let confidence = table.calibrate(0.5).map_err(|error| error.to_string())?;
    Ok((confidence.raw - 0.5).abs() < f64::EPSILON
        && confidence
            .calibrated
            .is_some_and(|value| (value - 0.5).abs() < f64::EPSILON)
        && selective_decision(&confidence, 0.8, 0.4).map_err(|error| error.to_string())?
            == ReviewDecision::Review)
}

struct NoProposal;

impl CorrectionProvider for NoProposal {
    fn propose(
        &mut self,
        _: &Document,
        _: &nuif_reconstruct::ObservationBundle,
        _: usize,
    ) -> Result<Option<Proposal>, String> {
        Ok(None)
    }
}

fn budget_stop_trial(
    document: &Document,
    observations: &nuif_reconstruct::ObservationBundle,
) -> Result<bool, String> {
    let entity = document.roots[0];
    let report = |budget: LoopBudget| {
        run_loop(
            &mut document.clone(),
            observations,
            &mut NoProposal,
            &mut ObjectiveEvaluator { entity },
            &budget,
        )
        .map_err(|error| error.to_string())
    };
    Ok(report(LoopBudget {
        max_provider_calls: 0,
        ..LoopBudget::default()
    })?
    .status
        == LoopStatus::ProviderCallBudget
        && report(LoopBudget {
            max_estimated_bytes: 0,
            ..LoopBudget::default()
        })?
        .status
            == LoopStatus::MemoryBudget
        && report(LoopBudget::default())?.status == LoopStatus::NoProposal)
}

fn rgba_png(width: u32, height: u32, pixels: &[u8], color: ColorType) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(Cursor::new(&mut bytes), width, height);
        encoder.set_color(color);
        encoder.set_depth(BitDepth::Eight);
        encoder.set_filter(png::Filter::NoFilter);
        let mut writer = encoder.write_header().map_err(|error| error.to_string())?;
        writer
            .write_image_data(pixels)
            .map_err(|error| error.to_string())?;
    }
    Ok(bytes)
}

fn bounds(x: f64, y: f64, width: f64, height: f64) -> Bounds {
    Bounds {
        x,
        y,
        width,
        height,
    }
}

fn trial(name: &str, passed: bool) -> Value {
    json!({"name": name, "passed": passed})
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
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
    let mut output = PathBuf::from("target/capture-reconstruction-report.json");
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--output" => {
                output = arguments
                    .next()
                    .ok_or_else(|| "--output requires a path".to_owned())?
                    .into();
            }
            "--help" | "-h" => {
                return Err("usage: capture-reconstruction [--output <json>]".to_owned());
            }
            unknown => return Err(format!("unknown argument {unknown:?}")),
        }
    }
    Ok(output)
}
