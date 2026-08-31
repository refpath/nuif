use nuif_capture::live::{
    LIVE_CAPTURE_ADAPTER_VERSION, LIVE_CONTEXT_LOCALE, LIVE_CONTEXT_PROFILE, LIVE_CONTEXT_TIMEZONE,
    LiveBrowserCanaries, LiveBrowserEvidence, LiveBrowserOptions, capture_chromium,
};
use nuif_capture::{BrowserCapture, Viewport, normalize_browser_capture};
use nuif_core::{Document, EntityId};
use nuif_package::{NuifPackage, PackageMode};
use nuif_reconstruct::layout_inference::{
    LayoutInferenceReport, LayoutItemObservation, LayoutSnapshot, infer_layout,
};
use nuif_reconstruct::{EvidenceClass, ObservationId, ObservationValue};
use png::{BitDepth, ColorType};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::{Cursor, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const QUERY_CANARY: &str = "nuifQueryCanary8301";
const COOKIE_CANARY: &str = "nuifCookieCanary8302";
const STORAGE_CANARY: &str = "nuifStorageCanary8303";
const AUTH_CANARY: &str = "nuifAuthorizationCanary8304";
const HEADER_CANARY: &str = "nuifHeaderCanary8305";
const MAX_CAPTURE_ATTEMPTS: usize = 3;
const INDEX_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <title>NUIF live capture fixture</title>
  <link rel="icon" href="data:,">
  <link rel="stylesheet" href="/style.css">
</head>
<body>
  <main aria-label="Capture fixture">
    <section class="panel" aria-label="Responsive panel">
      <p class="eyebrow">SOURCE EVIDENCE</p>
      <h1>One core, measurable output</h1>
      <p class="copy">A pinned browser resolves layout, accessibility, resources, and actual font use.</p>
      <img src="/image.png" alt="Four-color verification tile" width="48" height="48">
      <button type="button" aria-label="Inspect evidence">Inspect</button>
    </section>
  </main>
</body>
</html>
"#;
const STYLE_CSS: &str = r#"@font-face {
  font-family: "Ahem";
  src: url("/font.ttf") format("truetype");
  font-display: block;
}
* { box-sizing: border-box; }
html, body { width: 100%; height: 100%; margin: 0; }
body { background: rgb(28, 29, 31); color: rgb(250, 251, 255); font-family: "Ahem"; }
main { position: relative; width: 100vw; height: 100vh; background: rgb(28, 29, 31); }
.panel {
  position: absolute;
  left: 24px;
  top: 32px;
  width: calc(100vw - 48px);
  min-height: 260px;
  padding: 24px;
  background: rgb(41, 107, 232);
}
.eyebrow { margin: 0 0 20px; font-size: 10px; line-height: 14px; }
h1 { width: 75%; margin: 0 0 24px; font-size: 24px; line-height: 30px; }
.copy { width: 70%; margin: 0; font-size: 12px; line-height: 18px; }
img { position: absolute; right: 24px; bottom: 24px; image-rendering: pixelated; }
button { position: absolute; left: 24px; bottom: 24px; height: 32px; border: 0; background: rgb(250, 171, 28); font: 10px/14px "Ahem"; }
@media (min-width: 700px) {
  .panel { left: 64px; top: 48px; width: calc(100vw - 128px); min-height: 320px; padding: 32px; }
  h1 { font-size: 32px; line-height: 40px; }
  img { right: 32px; bottom: 32px; width: 64px; height: 64px; }
  button { left: 32px; bottom: 32px; }
}
@media (min-width: 850px) {
  .panel { left: 100px; width: calc(100vw - 200px); }
}
"#;

fn main() {
    if let Err(error) = run() {
        eprintln!("live-browser-capture: {error}");
        std::process::exit(1);
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "the executable keeps the complete live evidence chain and its assertions together"
)]
fn run() -> Result<(), String> {
    let arguments = arguments()?;
    let image_png = fixture_png()?;
    let mut expected_hashes = vec![
        sha256(INDEX_HTML.as_bytes()),
        sha256(STYLE_CSS.as_bytes()),
        sha256(&image_png),
        sha256(nuif_text::pinned_font_bytes()),
        sha256(b"ok"),
    ];
    expected_hashes.sort();
    let server = FixtureServer::start(image_png)?;
    let started = Instant::now();
    let source_url = format!(
        "http://127.0.0.1:{}/index.html?access_token={QUERY_CANARY}",
        server.port
    );
    let mut captures = Vec::new();
    let mut capture_attempts = Vec::new();
    let mut capture_attempt_log = Vec::new();
    let mut capture_error = None;
    for (capture_id, width) in [
        ("live-narrow", 360.0),
        ("live-wide", 768.0),
        ("live-heldout", 900.0),
        ("live-narrow", 360.0),
    ] {
        let mut accepted = None;
        let mut last_error = "capture did not run".to_owned();
        for attempt in 1..=MAX_CAPTURE_ATTEMPTS {
            match capture_chromium(&LiveBrowserOptions {
                chrome: &arguments.chrome,
                source_url: &source_url,
                capture_id,
                viewport: Viewport {
                    width,
                    height: 560.0,
                    device_scale_factor: 1.0,
                },
                canaries: Some(LiveBrowserCanaries {
                    cookie: COOKIE_CANARY,
                    storage: STORAGE_CANARY,
                    authorization: AUTH_CANARY,
                    header: HEADER_CANARY,
                    probe_path: "/probe",
                }),
            }) {
                Ok(evidence) if capture_has_exact_fixture(&evidence, &expected_hashes) => {
                    capture_attempt_log.push(json!({
                        "capture_id": capture_id,
                        "viewport_width": width,
                        "attempt": attempt,
                        "outcome": "accepted",
                    }));
                    accepted = Some((evidence, attempt));
                    break;
                }
                Ok(evidence) => {
                    capture_attempt_log.push(json!({
                        "capture_id": capture_id,
                        "viewport_width": width,
                        "attempt": attempt,
                        "outcome": "incomplete-evidence",
                        "resource_count": evidence.capture.resources.len(),
                        "font_status": evidence.font_status.as_str(),
                    }));
                    last_error = format!(
                        "incomplete evidence: {} resources, font status {}",
                        evidence.capture.resources.len(),
                        evidence.font_status
                    );
                }
                Err(error) => {
                    capture_attempt_log.push(json!({
                        "capture_id": capture_id,
                        "viewport_width": width,
                        "attempt": attempt,
                        "outcome": "adapter-error",
                    }));
                    last_error = error.to_string();
                }
            }
        }
        if let Some((evidence, attempts)) = accepted {
            captures.push(evidence);
            capture_attempts.push(attempts);
        } else {
            capture_error = Some(format!(
                "{capture_id} at {width}px failed after {MAX_CAPTURE_ATTEMPTS} attempts: {last_error}"
            ));
            break;
        }
    }
    let fixture_port = server.port;
    let probe = server.stop()?;
    if let Some(error) = capture_error {
        return Err(error);
    }

    let normalized = captures
        .iter()
        .zip([1_u128, 2, 3, 1])
        .map(|(evidence, root)| normalize(evidence, root))
        .collect::<Result<Vec<_>, _>>()?;
    let narrow = &captures[0];
    let wide = &captures[1];
    let heldout = &captures[2];
    let repeated = &captures[3];
    let responsive = responsive_errors(&narrow.capture, &wide.capture, &heldout.capture)?;
    let layout_inference = infer_captured_layout(&narrow.capture, &wide.capture, &heldout.capture)?;
    let resource_bodies_exact = captures.iter().all(|evidence| {
        let mut observed = evidence
            .capture
            .resources
            .iter()
            .map(|resource| sha256(&resource.body))
            .collect::<Vec<_>>();
        observed.sort();
        observed == expected_hashes
    });
    let font_use_observed = captures.iter().all(|evidence| {
        evidence.font_status == "loaded"
            && evidence.capture.nodes.iter().any(|node| {
                node.font_uses
                    .iter()
                    .any(|font| font.custom && font.family == nuif_text::PINNED_FONT_NAME)
            })
    }) && normalized.iter().all(|item| item.font_observation);
    let accessibility_observed = narrow.capture.nodes.iter().any(|node| {
        node.accessible_role.as_deref() == Some("main")
            && node.accessible_name.as_deref() == Some("Capture fixture")
    }) && narrow.capture.nodes.iter().any(|node| {
        node.accessible_role.as_deref() == Some("button")
            && node.accessible_name.as_deref() == Some("Inspect evidence")
    });
    let screenshots_valid = captures.iter().all(|evidence| {
        nuif_media::inspect_png_basic_rgba8(&evidence.screenshot_png).is_ok_and(|header| {
            (f64::from(header.width) - evidence.capture.viewport.width).abs() < f64::EPSILON
                && (f64::from(header.height) - evidence.capture.viewport.height).abs()
                    < f64::EPSILON
        })
    });
    let repeat_exact = narrow == repeated && normalized[0].bytes == normalized[3].bytes;
    let secrets = [
        QUERY_CANARY,
        COOKIE_CANARY,
        STORAGE_CANARY,
        AUTH_CANARY,
        HEADER_CANARY,
    ];
    let secret_absent = captures
        .iter()
        .map(|evidence| serde_json::to_vec(&evidence.capture))
        .chain(normalized.iter().map(|item| Ok(item.bytes.clone())))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?
        .iter()
        .all(|bytes| {
            secrets
                .iter()
                .all(|secret| !contains(bytes, secret.as_bytes()))
        });
    let browser_pinned = captures.iter().all(|evidence| {
        evidence.browser_product == format!("Chrome/{}", arguments.browser_version)
            && evidence
                .capture
                .adapter_version
                .starts_with(LIVE_CAPTURE_ADAPTER_VERSION)
    });
    let context_pinned = captures.iter().all(|evidence| {
        evidence.capture.context.as_ref().is_some_and(|context| {
            context.profile == LIVE_CONTEXT_PROFILE
                && context.properties.get("browser-product") == Some(&evidence.browser_product)
                && context.properties.get("protocol-version") == Some(&evidence.protocol_version)
                && context.properties.get("locale").map(String::as_str) == Some(LIVE_CONTEXT_LOCALE)
                && context.properties.get("timezone").map(String::as_str)
                    == Some(LIVE_CONTEXT_TIMEZONE)
                && context
                    .properties
                    .get("animation-policy")
                    .map(String::as_str)
                    == Some("playback-rate-zero-plus-freeze-stylesheet")
                && context
                    .properties
                    .get("settling-policy")
                    .map(String::as_str)
                    == Some("load-freeze-assets-ready-event-quiet-in-page-probe-stable-screenshot")
        })
    });
    let explicit_omissions = narrow
        .capture
        .omitted_runtime
        .iter()
        .any(|reason| reason.contains("source-spans"))
        && narrow
            .capture
            .omitted_runtime
            .iter()
            .any(|reason| reason.contains("local-font"))
        && narrow
            .capture
            .omitted_runtime
            .iter()
            .any(|reason| reason.contains("event-listeners"));
    let limits_respected = captures.iter().all(|evidence| {
        evidence.capture.nodes.len() <= nuif_capture::MAX_CAPTURE_NODES
            && evidence.capture.resources.len() <= nuif_capture::MAX_CAPTURE_RESOURCES
            && evidence
                .capture
                .nodes
                .iter()
                .map(|node| node.font_uses.len())
                .sum::<usize>()
                <= nuif_capture::live::MAX_TOTAL_FONT_USES
            && evidence
                .capture
                .resources
                .iter()
                .map(|resource| resource.body.len())
                .sum::<usize>()
                <= nuif_capture::live::MAX_LIVE_TOTAL_RESOURCE_BYTES
    });
    let elapsed = started.elapsed();
    let total_attempts = capture_attempts.iter().sum::<usize>();
    let trials = vec![
        trial("pinned_browser_and_protocol_adapter", browser_pinned),
        trial("runtime_context_is_explicit_and_pinned", context_pinned),
        trial("repeat_capture_and_normalization_exact", repeat_exact),
        trial("response_resource_bytes_exact", resource_bodies_exact),
        trial("downloaded_font_use_observed", font_use_observed),
        trial("computed_accessibility_observed", accessibility_observed),
        trial("viewport_screenshots_bounded", screenshots_valid),
        trial(
            "credential_canaries_exercised_but_not_retained",
            probe.valid
                && (captures.len()..=total_attempts).contains(&probe.completed)
                && (captures.len()..=total_attempts).contains(&probe.query_requests)
                && captures
                    .iter()
                    .all(|evidence| evidence.canary_probe_completed)
                && secret_absent,
        ),
        trial("unavailable_runtime_evidence_explicit", explicit_omissions),
        trial("capture_resource_limits_respected", limits_respected),
        trial(
            "fresh_profile_retries_are_bounded",
            capture_attempts.len() == captures.len()
                && capture_attempts
                    .iter()
                    .all(|attempts| (1..=MAX_CAPTURE_ATTEMPTS).contains(attempts)),
        ),
        trial(
            "four_capture_gate_completes_within_120_seconds",
            elapsed <= Duration::from_secs(120),
        ),
        trial(
            "multi_viewport_beats_single_viewport_on_heldout",
            responsive.compared_nodes > 0 && responsive.multi_error < responsive.single_error,
        ),
        trial(
            "layout_candidates_ranked_before_heldout_evaluation",
            layout_inference.selection_basis == "training_score_only"
                && layout_inference.evidence == EvidenceClass::Inferred
                && layout_inference.provenance.confidence.calibrated.is_none()
                && layout_inference.candidates.len() == 5
                && layout_inference.beats_freeform_on_heldout(),
        ),
    ];
    let passed = trials.iter().all(|trial| trial["passed"] == true);
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:live-browser-source-capture",
        "status": if passed { "passed" } else { "failed" },
        "source": source_identity(),
        "browser": {
            "product": narrow.browser_product,
            "protocol_version": narrow.protocol_version,
            "adapter": LIVE_CAPTURE_ADAPTER_VERSION,
            "context": narrow.capture.context,
        },
        "fixture": {
            "origin": format!("http://127.0.0.1:{fixture_port}"),
            "response_body_sha256": expected_hashes,
            "probe_requests": probe.completed,
            "query_canary_requests": probe.query_requests,
            "connection_handler_errors": probe.server_errors,
        },
        "measurements": {
            "capture_count": captures.len(),
            "capture_attempts": capture_attempts,
            "capture_attempt_log": capture_attempt_log,
            "total_capture_attempts": total_attempts,
            "elapsed_milliseconds": elapsed.as_millis(),
            "nodes": captures.iter().map(|item| item.capture.nodes.len()).collect::<Vec<_>>(),
            "resources": captures.iter().map(|item| item.capture.resources.len()).collect::<Vec<_>>(),
            "resource_urls": captures.iter().map(|item| item.capture.resources.iter().map(|resource| resource.url.as_str()).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "resource_body_sha256": captures.iter().map(|item| item.capture.resources.iter().map(|resource| sha256(&resource.body)).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "observation_counts": normalized.iter().map(|item| item.observations).collect::<Vec<_>>(),
            "font_use_counts": captures.iter().map(|item| item.capture.nodes.iter().map(|node| node.font_uses.len()).sum::<usize>()).collect::<Vec<_>>(),
            "font_status": captures.iter().map(|item| item.font_status.as_str()).collect::<Vec<_>>(),
            "omissions": captures.iter().map(|item| &item.capture.omitted_runtime).collect::<Vec<_>>(),
            "screenshot_sha256": captures.iter().map(|item| sha256(&item.screenshot_png)).collect::<Vec<_>>(),
            "single_viewport_heldout_absolute_error": responsive.single_error,
            "multi_viewport_heldout_absolute_error": responsive.multi_error,
            "responsive_compared_nodes": responsive.compared_nodes,
        },
        "layout_inference": layout_inference,
        "trials": trials,
        "non_claims": [
            "no cross-browser, cross-operating-system, cross-origin iframe, or authenticated-site coverage",
            "no arbitrary animation, application-state, canvas, video, WebGL, or worklet determinism",
            "no source-map correlation or recovery of local host font bytes",
            "no broad screenshot reconstruction accuracy or model-training claim"
        ],
        "summary": {
            "trials": trials.len(),
            "blocking_failures": trials.iter().filter(|trial| trial["passed"] != true).count(),
        }
    });
    if let Some(parent) = arguments.output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    if let Some(parent) = arguments.layout_output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut bytes = serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    fs::write(&arguments.output, bytes).map_err(|error| error.to_string())?;
    let mut layout_bytes =
        serde_json::to_vec_pretty(&layout_inference).map_err(|error| error.to_string())?;
    layout_bytes.push(b'\n');
    fs::write(&arguments.layout_output, layout_bytes).map_err(|error| error.to_string())?;
    println!(
        "live browser capture: {} trials, status {}",
        trials.len(),
        if passed { "passed" } else { "failed" }
    );
    passed
        .then_some(())
        .ok_or_else(|| format!("report failed; inspect {}", arguments.output.display()))
}

fn capture_has_exact_fixture(evidence: &LiveBrowserEvidence, expected_hashes: &[String]) -> bool {
    let mut observed = evidence
        .capture
        .resources
        .iter()
        .map(|resource| sha256(&resource.body))
        .collect::<Vec<_>>();
    observed.sort();
    evidence.canary_probe_completed
        && evidence.font_status == "loaded"
        && evidence.capture.nodes.iter().any(|node| {
            node.font_uses
                .iter()
                .any(|font| font.custom && font.family == nuif_text::PINNED_FONT_NAME)
        })
        && observed == expected_hashes
}

struct NormalizedCapture {
    bytes: Vec<u8>,
    observations: usize,
    font_observation: bool,
}

fn normalize(evidence: &LiveBrowserEvidence, root: u128) -> Result<NormalizedCapture, String> {
    let mut package =
        NuifPackage::new(Document::empty(EntityId::new(root)), PackageMode::Authoring);
    let result = normalize_browser_capture(&evidence.capture, &mut package)
        .map_err(|error| error.to_string())?;
    if result.observations.context != evidence.capture.context {
        return Err("normalization lost the pinned capture context".to_owned());
    }
    let mut bytes = result
        .observations
        .encode()
        .map_err(|error| error.to_string())?;
    bytes.extend(serde_json::to_vec(&result.proposal).map_err(|error| error.to_string())?);
    bytes.extend(package.encode().map_err(|error| error.to_string())?);
    let font_observation = result.observations.observations.iter().any(|observation| {
        matches!(
            observation.value,
            ObservationValue::FontUse { custom: true, .. }
        )
    });
    Ok(NormalizedCapture {
        bytes,
        observations: result.observations.observations.len(),
        font_observation,
    })
}

struct ResponsiveErrors {
    single_error: f64,
    multi_error: f64,
    compared_nodes: usize,
}

fn infer_captured_layout(
    narrow: &BrowserCapture,
    wide: &BrowserCapture,
    heldout: &BrowserCapture,
) -> Result<LayoutInferenceReport, String> {
    let (narrow, mut evidence) = panel_snapshot(narrow)?;
    let (wide, wide_evidence) = panel_snapshot(wide)?;
    let (heldout, _) = panel_snapshot(heldout)?;
    evidence.extend(wide_evidence);
    infer_layout(&[narrow, wide], &heldout, evidence).map_err(|error| error.to_string())
}

fn panel_snapshot(
    capture: &BrowserCapture,
) -> Result<(LayoutSnapshot, std::collections::BTreeSet<ObservationId>), String> {
    let panels = capture
        .nodes
        .iter()
        .filter(|node| node.tag == "section")
        .collect::<Vec<_>>();
    let [panel] = panels.as_slice() else {
        return Err("live fixture must expose one section layout parent".to_owned());
    };
    let mut children = capture
        .nodes
        .iter()
        .filter(|node| node.parent == Some(panel.backend_node_id))
        .collect::<Vec<_>>();
    children.sort_by_key(|node| (node.order, node.backend_node_id));
    if children.len() < 2 {
        return Err("live fixture layout parent has fewer than two children".to_owned());
    }
    let mut evidence = std::collections::BTreeSet::from([ObservationId(format!(
        "{}-node-{}-geometry",
        capture.capture_id, panel.backend_node_id
    ))]);
    let items = children
        .iter()
        .enumerate()
        .map(|(index, node)| {
            evidence.insert(ObservationId(format!(
                "{}-node-{}-geometry",
                capture.capture_id, node.backend_node_id
            )));
            LayoutItemObservation {
                id: format!("child-{index}-{}", node.tag),
                bounds: node.bounds,
            }
        })
        .collect();
    Ok((
        LayoutSnapshot {
            viewport_width: capture.viewport.width,
            parent: panel.bounds,
            items,
        },
        evidence,
    ))
}

fn responsive_errors(
    narrow: &BrowserCapture,
    wide: &BrowserCapture,
    heldout: &BrowserCapture,
) -> Result<ResponsiveErrors, String> {
    if narrow.nodes.len() != wide.nodes.len() || narrow.nodes.len() != heldout.nodes.len() {
        return Err("responsive captures changed normalized node count".to_owned());
    }
    let narrow_width = narrow.viewport.width;
    let wide_width = wide.viewport.width;
    let heldout_width = heldout.viewport.width;
    let ratio = (heldout_width - narrow_width) / (wide_width - narrow_width);
    let mut single_error = 0.0;
    let mut multi_error = 0.0;
    for ((narrow_node, wide_node), heldout_node) in
        narrow.nodes.iter().zip(&wide.nodes).zip(&heldout.nodes)
    {
        if narrow_node.tag != wide_node.tag || narrow_node.tag != heldout_node.tag {
            return Err("responsive captures changed normalized preorder".to_owned());
        }
        let narrow_values = bounds_values(narrow_node.bounds);
        let wide_values = bounds_values(wide_node.bounds);
        let heldout_values = bounds_values(heldout_node.bounds);
        for index in 0..4 {
            let predicted =
                narrow_values[index] + ratio * (wide_values[index] - narrow_values[index]);
            single_error += (narrow_values[index] - heldout_values[index]).abs();
            multi_error += (predicted - heldout_values[index]).abs();
        }
    }
    Ok(ResponsiveErrors {
        single_error,
        multi_error,
        compared_nodes: narrow.nodes.len(),
    })
}

fn bounds_values(bounds: nuif_reconstruct::Bounds) -> [f64; 4] {
    [bounds.x, bounds.y, bounds.width, bounds.height]
}

struct Arguments {
    chrome: PathBuf,
    browser_version: String,
    output: PathBuf,
    layout_output: PathBuf,
}

fn arguments() -> Result<Arguments, String> {
    let mut values = env::args().skip(1);
    let mut chrome = None;
    let mut browser_version = None;
    let mut output = PathBuf::from("target/live-browser-capture-report.json");
    let mut layout_output = PathBuf::from("target/layout-inference-report.json");
    while let Some(argument) = values.next() {
        match argument.as_str() {
            "--chrome" => chrome = values.next().map(PathBuf::from),
            "--browser-version" => browser_version = values.next(),
            "--output" => {
                output = values
                    .next()
                    .ok_or_else(|| "--output requires a path".to_owned())?
                    .into();
            }
            "--layout-output" => {
                layout_output = values
                    .next()
                    .ok_or_else(|| "--layout-output requires a path".to_owned())?
                    .into();
            }
            "--help" | "-h" => {
                return Err(
                    "usage: live-browser-capture --chrome <binary> --browser-version <version> [--output <json>] [--layout-output <json>]".to_owned(),
                );
            }
            unknown => return Err(format!("unknown argument {unknown:?}")),
        }
    }
    let chrome = chrome.ok_or_else(|| "--chrome requires a path".to_owned())?;
    if !chrome.is_file() {
        return Err(format!("browser binary is absent: {}", chrome.display()));
    }
    let browser_version = browser_version
        .filter(|version| {
            !version.is_empty()
                && version.len() <= 64
                && version
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || byte == b'.')
        })
        .ok_or_else(|| "--browser-version requires a dotted numeric version".to_owned())?;
    Ok(Arguments {
        chrome,
        browser_version,
        output,
        layout_output,
    })
}

#[derive(Clone, Copy)]
struct ProbeResult {
    completed: usize,
    query_requests: usize,
    server_errors: usize,
    valid: bool,
}

struct FixtureServer {
    port: u16,
    stop: Arc<AtomicBool>,
    active: Arc<AtomicUsize>,
    probe: Arc<Mutex<ProbeResult>>,
    handle: Option<JoinHandle<()>>,
}

impl FixtureServer {
    fn start(image_png: Vec<u8>) -> Result<Self, String> {
        let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
        listener
            .set_nonblocking(true)
            .map_err(|error| error.to_string())?;
        let port = listener
            .local_addr()
            .map_err(|error| error.to_string())?
            .port();
        let stop = Arc::new(AtomicBool::new(false));
        let probe = Arc::new(Mutex::new(ProbeResult {
            completed: 0,
            query_requests: 0,
            server_errors: 0,
            valid: true,
        }));
        let active = Arc::new(AtomicUsize::new(0));
        let thread_stop = Arc::clone(&stop);
        let thread_probe = Arc::clone(&probe);
        let thread_active = Arc::clone(&active);
        let image_png = Arc::new(image_png);
        let handle = thread::spawn(move || {
            while !thread_stop.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let image_png = Arc::clone(&image_png);
                        let probe = Arc::clone(&thread_probe);
                        let active = Arc::clone(&thread_active);
                        active.fetch_add(1, Ordering::AcqRel);
                        thread::spawn(move || {
                            if serve_request(&mut stream, &image_png, &probe).is_err()
                                && let Ok(mut state) = probe.lock()
                            {
                                state.server_errors = state.server_errors.saturating_add(1);
                            }
                            active.fetch_sub(1, Ordering::AcqRel);
                        });
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2));
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            port,
            stop,
            active,
            probe,
            handle: Some(handle),
        })
    }

    fn stop(mut self) -> Result<ProbeResult, String> {
        self.stop.store(true, Ordering::Release);
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        if let Some(handle) = self.handle.take() {
            handle
                .join()
                .map_err(|_| "fixture server thread panicked".to_owned())?;
        }
        let deadline = Instant::now() + Duration::from_secs(3);
        while self.active.load(Ordering::Acquire) != 0 {
            if Instant::now() >= deadline {
                return Err("fixture server workers did not stop before deadline".to_owned());
            }
            thread::sleep(Duration::from_millis(2));
        }
        let result = *self
            .probe
            .lock()
            .map_err(|_| "fixture probe lock was poisoned".to_owned())?;
        Ok(result)
    }
}

fn serve_request(
    stream: &mut TcpStream,
    image_png: &[u8],
    probe: &Mutex<ProbeResult>,
) -> Result<(), String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| error.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| error.to_string())?;
    stream
        .set_nodelay(true)
        .map_err(|error| error.to_string())?;
    let mut request = Vec::new();
    let mut buffer = [0_u8; 2_048];
    loop {
        while request.len() <= 16 * 1_024 && !request.windows(4).any(|window| window == b"\r\n\r\n")
        {
            let read = match stream.read(&mut buffer) {
                Ok(0) => return Ok(()),
                Ok(read) => read,
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock
                            | std::io::ErrorKind::TimedOut
                            | std::io::ErrorKind::ConnectionReset
                    ) =>
                {
                    return Ok(());
                }
                Err(error) => return Err(error.to_string()),
            };
            request.extend_from_slice(&buffer[..read]);
        }
        if request.len() > 16 * 1_024 {
            return Err("fixture request exceeded header limit".to_owned());
        }
        let end = request
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|index| index + 4)
            .ok_or_else(|| "fixture request terminator is absent".to_owned())?;
        let request_bytes = request.drain(..end).collect::<Vec<_>>();
        let request_text = String::from_utf8(request_bytes).map_err(|error| error.to_string())?;
        let target = request_text
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap_or("/");
        let path = target.split('?').next().unwrap_or(target);
        if path == "/index.html" && target.contains(&format!("access_token={QUERY_CANARY}")) {
            let mut state = probe
                .lock()
                .map_err(|_| "fixture probe lock was poisoned".to_owned())?;
            state.query_requests = state.query_requests.saturating_add(1);
        }
        let (status, media_type, body): (&str, &str, &[u8]) = match path {
            "/" | "/index.html" => ("200 OK", "text/html; charset=utf-8", INDEX_HTML.as_bytes()),
            "/style.css" => ("200 OK", "text/css; charset=utf-8", STYLE_CSS.as_bytes()),
            "/image.png" => ("200 OK", "image/png", image_png),
            "/font.ttf" => ("200 OK", "font/ttf", nuif_text::pinned_font_bytes()),
            "/probe" => {
                let lower = request_text.to_ascii_lowercase();
                let valid = lower.contains(&format!(
                    "authorization: {}",
                    AUTH_CANARY.to_ascii_lowercase()
                )) && lower.contains(&format!(
                    "x-nuif-canary: {}",
                    HEADER_CANARY.to_ascii_lowercase()
                )) && lower.contains(&format!(
                    "nuif_capture_canary={}",
                    COOKIE_CANARY.to_ascii_lowercase()
                ));
                let mut state = probe
                    .lock()
                    .map_err(|_| "fixture probe lock was poisoned".to_owned())?;
                state.completed = state.completed.saturating_add(1);
                state.valid &= valid;
                ("200 OK", "text/plain; charset=utf-8", b"ok")
            }
            _ => ("404 Not Found", "text/plain; charset=utf-8", b"not found"),
        };
        let head = format!(
            "HTTP/1.1 {status}\r\nContent-Type: {media_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: keep-alive\r\nKeep-Alive: timeout=2, max=100\r\n\r\n",
            body.len()
        );
        match stream
            .write_all(head.as_bytes())
            .and_then(|()| stream.write_all(body))
            .and_then(|()| stream.flush())
        {
            Ok(()) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::ConnectionReset
                ) =>
            {
                return Ok(());
            }
            Err(error) => return Err(error.to_string()),
        }
    }
}

fn fixture_png() -> Result<Vec<u8>, String> {
    let pixels = [
        245_u8, 88, 85, 255, 250, 171, 28, 255, 41, 107, 232, 255, 52, 199, 89, 255,
    ];
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(Cursor::new(&mut bytes), 2, 2);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(BitDepth::Eight);
        encoder.set_filter(png::Filter::NoFilter);
        let mut writer = encoder.write_header().map_err(|error| error.to_string())?;
        writer
            .write_image_data(&pixels)
            .map_err(|error| error.to_string())?;
    }
    Ok(bytes)
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
