use nuif_core::{
    Color as ModelColor, ColorSpace, Document, Entity, EntityId, EntityKind, Fidelity,
    OpaqueEncoding, OpaquePayload, ShapeKind, UnknownKind, validate,
};
use nuif_layout::{EvaluationContext, LayoutSnapshot, Rect};
use nuif_render::{
    Color, DrawCommand, RenderError, RenderScene, RenderTarget, build_scene, render_cpu,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const GOLDEN_JSON: &str = include_str!("../../../../conformance/render/profile-zero-v1.json");

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GoldenFile {
    schema_version: u32,
    rasterizer: String,
    coverage: String,
    compositing: String,
    verified_platforms: Vec<PlatformIdentity>,
    cases: Vec<GoldenCase>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlatformIdentity {
    os: String,
    architecture: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GoldenCase {
    name: String,
    kind: PaintKind,
    rect: Rect,
    fill: Color,
    target: RenderTarget,
    scene_sha256: String,
    rgba_sha256: String,
    png_sha256: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum PaintKind {
    Rectangle,
    Ellipse,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("render-profile: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let golden: GoldenFile =
        serde_json::from_str(GOLDEN_JSON).map_err(|error| error.to_string())?;
    let (raster_trials, raster_passed) = raster_trials(&golden.cases)?;
    let (fidelity_trials, fidelity_passed) = fidelity_trials()?;
    let (negative_trials, negative_passed) = negative_trials();
    let passed = golden.schema_version == 2 && raster_passed && fidelity_passed && negative_passed;
    let cross_platform_verified = raster_passed
        && golden.verified_platforms.iter().any(|platform| {
            platform.os != env::consts::OS || platform.architecture != env::consts::ARCH
        });
    let report = json!({
        "schema_version": 2,
        "experiment": "nuif:experiment:render-profile-zero",
        "status": if passed { "passed" } else { "failed" },
        "source": {
            "revision": command_text("git", &["rev-parse", "HEAD"]),
            "dirty": command_text("git", &["status", "--porcelain"]).map(|value| !value.is_empty()),
            "toolchain": command_text("rustc", &["--version"]),
            "os": env::consts::OS,
            "architecture": env::consts::ARCH,
        },
        "profile": {
            "rasterizer": golden.rasterizer,
            "coverage": golden.coverage,
            "compositing": golden.compositing,
            "color_space": "srgb",
            "verified_platforms": golden.verified_platforms,
            "cross_platform_verified": cross_platform_verified,
        },
        "summary": {
            "raster_cases": raster_trials.len(),
            "fidelity_cases": fidelity_trials.len(),
            "negative_cases": negative_trials.len(),
            "blocking_failures": u8::from(!passed),
        },
        "raster_trials": raster_trials,
        "fidelity_trials": fidelity_trials,
        "negative_trials": negative_trials,
    });
    let output = output_path()?;
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
        "render profile: {} raster cases, {} fidelity cases, status {}",
        golden.cases.len(),
        fidelity_trials.len(),
        if passed { "passed" } else { "failed" }
    );
    if passed {
        Ok(())
    } else {
        Err(format!("report failed; inspect {}", output.display()))
    }
}

fn raster_trials(cases: &[GoldenCase]) -> Result<(Vec<Value>, bool), String> {
    let mut reports = Vec::new();
    let mut all_passed = true;
    for case in cases {
        let command = match case.kind {
            PaintKind::Rectangle => DrawCommand::Rect {
                entity: EntityId::new(1),
                rect: case.rect,
                fill: case.fill,
            },
            PaintKind::Ellipse => DrawCommand::Ellipse {
                entity: EntityId::new(1),
                rect: case.rect,
                fill: case.fill,
            },
        };
        let scene = RenderScene {
            commands: vec![command],
            fidelity: Vec::new(),
        };
        let first = render_cpu(&scene, case.target).map_err(|error| error.to_string())?;
        let second = render_cpu(&scene, case.target).map_err(|error| error.to_string())?;
        let first_png = first.to_png().map_err(|error| error.to_string())?;
        let second_png = second.to_png().map_err(|error| error.to_string())?;
        let scene_sha256 =
            sha256_hex(&serde_json::to_vec(&scene).map_err(|error| error.to_string())?);
        let rgba_sha256 = sha256_hex(&first.rgba);
        let png_sha256 = sha256_hex(&first_png);
        let repeatable = first == second && first_png == second_png;
        let baseline_match = scene_sha256 == case.scene_sha256 && rgba_sha256 == case.rgba_sha256;
        let png_reference_match = png_sha256 == case.png_sha256;
        let passed = repeatable && baseline_match;
        all_passed &= passed;
        reports.push(json!({
            "name": case.name,
            "kind": case.kind,
            "rect": case.rect,
            "fill": case.fill,
            "target": case.target,
            "scene_sha256": scene_sha256,
            "expected_scene_sha256": case.scene_sha256,
            "rgba_sha256": rgba_sha256,
            "expected_rgba_sha256": case.rgba_sha256,
            "png_sha256": png_sha256,
            "expected_png_sha256": case.png_sha256,
            "png_reference_match": png_reference_match,
            "repeatable": repeatable,
            "baseline_match": baseline_match,
            "passed": passed,
        }));
    }
    Ok((reports, all_passed))
}

fn fidelity_trials() -> Result<(Vec<Value>, bool), String> {
    let mut document = Document::empty(EntityId::new(1));
    document.extensions.0.insert(
        "vendor.global".to_owned(),
        OpaquePayload {
            encoding: OpaqueEncoding::Octets,
            bytes: vec![1],
        },
    );
    let mut path = Entity::new(EntityId::new(2), EntityKind::Shape(ShapeKind::Path));
    path.extensions.0.insert(
        "vendor.effect".to_owned(),
        OpaquePayload {
            encoding: OpaqueEncoding::Octets,
            bytes: vec![2],
        },
    );
    let entities = [
        path,
        Entity::new(EntityId::new(3), EntityKind::Image),
        Entity::new(
            EntityId::new(4),
            EntityKind::Instance {
                component: EntityId::new(9),
            },
        ),
        Entity::new(
            EntityId::new(5),
            EntityKind::Unknown(UnknownKind {
                namespace: "vendor.probe".to_owned(),
                kind: "future_widget".to_owned(),
                schema_version: 1,
                payload: OpaquePayload {
                    encoding: OpaqueEncoding::Octets,
                    bytes: vec![0, 255],
                },
            }),
        ),
    ];
    let mut layout = LayoutSnapshot {
        context_fingerprint: "render-profile-zero".to_owned(),
        ..LayoutSnapshot::default()
    };
    for entity in entities {
        layout.boxes.insert(
            entity.id,
            Rect {
                x: 0.0,
                y: 0.0,
                width: 8.0,
                height: 8.0,
            },
        );
        document.entities.insert(entity.id, entity);
    }
    let scene = build_scene(&document, &layout, &EvaluationContext::viewport(8.0, 8.0))
        .map_err(|error| error.to_string())?;
    let reports = scene
        .fidelity
        .iter()
        .map(|entry| {
            let passed = match entry.pointer.as_str() {
                "/extensions/vendor.global" => {
                    entry.entity.is_none()
                        && matches!(entry.status, Fidelity::PreservedUnrenderable { .. })
                }
                "/entities/00000000000000000000000000000002/extensions/vendor.effect" => {
                    entry.entity == Some(EntityId::new(2))
                        && matches!(entry.status, Fidelity::PreservedUnrenderable { .. })
                }
                "/entities/00000000000000000000000000000002/kind"
                | "/entities/00000000000000000000000000000004/kind" => {
                    matches!(entry.status, Fidelity::Unsupported { .. })
                }
                "/entities/00000000000000000000000000000003/authored/image" => {
                    entry.entity == Some(EntityId::new(3))
                        && matches!(entry.status, Fidelity::Unsupported { .. })
                }
                "/entities/00000000000000000000000000000005/kind" => {
                    entry.entity == Some(EntityId::new(5))
                        && matches!(entry.status, Fidelity::PreservedUnrenderable { .. })
                }
                _ => false,
            };
            json!({
                "entity": entry.entity,
                "pointer": entry.pointer,
                "status": entry.status,
                "passed": passed,
            })
        })
        .collect::<Vec<_>>();
    let passed = reports.len() == 6 && reports.iter().all(|report| report["passed"] == true);
    Ok((reports, passed))
}

fn negative_trials() -> (Vec<Value>, bool) {
    let invalid_color = Color {
        space: ColorSpace::Srgb,
        red: 1.1,
        green: 0.0,
        blue: 0.0,
        alpha: 1.0,
    };
    let scene = RenderScene {
        commands: vec![DrawCommand::Rect {
            entity: EntityId::new(1),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            },
            fill: invalid_color,
        }],
        fidelity: Vec::new(),
    };
    let raster_rejected = matches!(
        render_cpu(
            &scene,
            RenderTarget {
                width: 1,
                height: 1,
                scale_factor: 1.0,
            }
        ),
        Err(RenderError::InvalidScene { .. })
    );

    let mut document = Document::empty(EntityId::new(1));
    let mut entity = Entity::new(EntityId::new(2), EntityKind::Shape(ShapeKind::Rectangle));
    entity.authored.fill = Some(ModelColor {
        space: invalid_color.space,
        red: invalid_color.red,
        green: invalid_color.green,
        blue: invalid_color.blue,
        alpha: invalid_color.alpha,
    });
    document.roots.push(entity.id);
    document.entities.insert(entity.id, entity);
    let validation_rejected = validate(&document)
        .iter()
        .any(|diagnostic| diagnostic.code == "COLOR_CHANNEL_OUT_OF_RANGE");
    let reports = vec![
        json!({"name": "raster-rejects-out-of-range-color", "passed": raster_rejected}),
        json!({"name": "model-rejects-out-of-range-color", "passed": validation_rejected}),
    ];
    (reports, raster_rejected && validation_rejected)
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn output_path() -> Result<PathBuf, String> {
    let mut args = env::args().skip(1);
    let mut output = PathBuf::from("target/render-profile-report.json");
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--output" => {
                output = PathBuf::from(
                    args.next()
                        .ok_or_else(|| "--output requires a path".to_owned())?,
                );
            }
            _ => return Err(format!("unknown argument: {argument}")),
        }
    }
    Ok(output)
}

fn command_text(program: &str, arguments: &[&str]) -> Option<String> {
    let output = Command::new(program).args(arguments).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
