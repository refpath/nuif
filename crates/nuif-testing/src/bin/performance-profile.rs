use nuif_api::{Session, profile_zero_context};
use nuif_codec::{CanonicalText, Decoder, DeterministicCbor, Encoder};
use nuif_core::{EntityId, validate};
use nuif_layout::evaluate;
use nuif_protocol::{Operation, Patch, Transaction, apply_patch};
use nuif_render::{RenderTarget, build_scene, render_cpu};
use serde::Serialize;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, Stats, StatsAlloc};
use std::alloc::System;
use std::env;
use std::fs;
use std::hint::black_box;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

const SAMPLES: usize = 15;
const WARMUP_INVOCATIONS: usize = 3;
const MAX_ALLOCATED_BYTES_PER_INVOCATION: usize = 256 * 1024 * 1024;

#[derive(Debug, Serialize)]
struct CaseReport {
    name: &'static str,
    scale: usize,
    iterations_per_sample: usize,
    samples: usize,
    median_nanoseconds: u128,
    p95_nanoseconds: u128,
    minimum_nanoseconds: u128,
    maximum_nanoseconds: u128,
    budget_nanoseconds: u128,
    allocations_per_invocation: usize,
    allocated_bytes_per_invocation: usize,
    retained_bytes_per_invocation: i128,
    checksum: u64,
    within_time_budget: bool,
    within_allocation_budget: bool,
    passed: bool,
}

#[expect(
    clippy::too_many_lines,
    reason = "the ordered performance case matrix stays visible in one audit path"
)]
fn main() {
    if env::args().any(|argument| matches!(argument.as_str(), "--help" | "-h")) {
        println!("usage: performance-profile [--output <json>]");
        return;
    }
    let output = parse_output().unwrap_or_else(|error| fail(&error));
    let large = nuif_testing::performance_fixture(1_024, true);
    let text = CanonicalText
        .encode(&large)
        .unwrap_or_else(|error| fail(&error.to_string()));
    let cbor = DeterministicCbor
        .encode(&large)
        .unwrap_or_else(|error| fail(&error.to_string()));
    let context = profile_zero_context(1_440.0, 900.0);
    let layout = evaluate(&large, &context);
    let scene =
        build_scene(&large, &layout, &context).unwrap_or_else(|error| fail(&error.to_string()));
    let card = nuif_testing::responsive_card_fixture();
    let card_context = profile_zero_context(768.0, 640.0);
    let card_layout = evaluate(&card, &card_context);
    let card_scene = build_scene(&card, &card_layout, &card_context)
        .unwrap_or_else(|error| fail(&error.to_string()));
    let raster_target = RenderTarget {
        width: 768,
        height: 640,
        scale_factor: 1.0,
    };
    let patch = Patch {
        base_revision: None,
        transactions: vec![Transaction {
            id: 1,
            operations: vec![Operation::Rename {
                entity: EntityId::new(1_025),
                name: Some("performance edit".to_owned()),
            }],
        }],
    };

    let mut cases = vec![
        measure("validate", 1_024, 3, 500_000_000, || {
            black_box(validate(black_box(&large))).len() as u64
        }),
        measure("canonical_text_encode", 1_024, 1, 1_000_000_000, || {
            black_box(CanonicalText.encode(black_box(&large)).unwrap()).len() as u64
        }),
        measure("canonical_text_decode", 1_024, 1, 1_000_000_000, || {
            black_box(CanonicalText.decode(black_box(&text)).unwrap())
                .entities
                .len() as u64
        }),
        measure("deterministic_cbor_encode", 1_024, 1, 1_000_000_000, || {
            black_box(DeterministicCbor.encode(black_box(&large)).unwrap()).len() as u64
        }),
        measure("deterministic_cbor_decode", 1_024, 1, 1_000_000_000, || {
            black_box(DeterministicCbor.decode(black_box(&cbor)).unwrap())
                .entities
                .len() as u64
        }),
        measure("protocol_apply", 1_024, 1, 1_000_000_000, || {
            let mut candidate = black_box(large.clone());
            apply_patch(&mut candidate, black_box(&patch)).unwrap();
            candidate.entities.len() as u64
        }),
        measure("layout", 1_024, 5, 500_000_000, || {
            black_box(evaluate(black_box(&large), black_box(&context)))
                .boxes
                .len() as u64
        }),
        measure("scene_lowering", 1_024, 2, 2_000_000_000, || {
            let observed = black_box(
                build_scene(black_box(&large), black_box(&layout), black_box(&context)).unwrap(),
            );
            (observed.commands.len() + observed.fidelity.len()) as u64
        }),
        measure("cpu_raster", 768 * 640, 1, 1_000_000_000, || {
            let image = black_box(render_cpu(black_box(&card_scene), raster_target).unwrap());
            image.rgba.len() as u64 ^ u64::from(image.rgba[0])
        }),
        measure("api_snapshot", 768 * 640, 1, 2_000_000_000, || {
            let snapshot = black_box(
                Session::new(black_box(card.clone()))
                    .snapshot(black_box(&card_context))
                    .unwrap(),
            );
            snapshot.raster.rgba.len() as u64 ^ snapshot.canonical_hash.len() as u64
        }),
    ];
    cases.sort_by_key(|case| case.name);
    let passed = cases.iter().all(|case| case.passed);
    let report = serde_json::json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:profile-zero-performance",
        "status": if passed { "passed" } else { "failed" },
        "measurement": {
            "kind": "portable release-mode smoke profile",
            "samples": SAMPLES,
            "warmup_invocations": WARMUP_INVOCATIONS,
            "latency_statistic": "median and nearest-rank p95 of per-invocation wall-clock nanoseconds",
            "allocation_statistic": "one warmed invocation through stats_alloc",
            "criterion_suite": "cargo bench -p nuif-conformance --bench profile_zero",
            "interpretation": "budgets detect catastrophic regressions; Criterion comparisons on controlled hardware detect smaller changes"
        },
        "toolchain": rustc_version(),
        "build_profile": "release",
        "platform": {
            "os": env::consts::OS,
            "architecture": env::consts::ARCH,
            "available_parallelism": std::thread::available_parallelism().map_or(1, usize::from),
            "cpu_model": cpu_model()
        },
        "fixture": {
            "entities": large.entities.len(),
            "canonical_text_bytes": text.len(),
            "deterministic_cbor_bytes": cbor.len(),
            "layout_boxes": layout.boxes.len(),
            "scene_commands": scene.commands.len(),
            "scene_fidelity_records": scene.fidelity.len()
        },
        "limits": {
            "allocated_bytes_per_invocation": MAX_ALLOCATED_BYTES_PER_INVOCATION
        },
        "cases": cases
    });
    let encoded = serde_json::to_vec_pretty(&report).expect("performance report serializes");
    if let Some(path) = output {
        fs::write(&path, &encoded)
            .unwrap_or_else(|error| fail(&format!("cannot write {}: {error}", path.display())));
    }
    println!("{}", String::from_utf8(encoded).expect("JSON is UTF-8"));
    if !passed {
        std::process::exit(1);
    }
}

fn measure(
    name: &'static str,
    scale: usize,
    iterations_per_sample: usize,
    budget_nanoseconds: u128,
    mut operation: impl FnMut() -> u64,
) -> CaseReport {
    for _ in 0..WARMUP_INVOCATIONS {
        black_box(operation());
    }
    let region = Region::new(GLOBAL);
    let allocation_checksum = black_box(operation());
    let allocation = region.change();
    let retained = retained_bytes(allocation);

    let mut samples = Vec::with_capacity(SAMPLES);
    let mut checksum = allocation_checksum;
    for sample in 0..SAMPLES {
        let started = Instant::now();
        for iteration in 0..iterations_per_sample {
            let value = black_box(operation());
            checksum = checksum.rotate_left(7) ^ value ^ (sample as u64) ^ (iteration as u64);
        }
        samples.push(started.elapsed().as_nanos() / iterations_per_sample as u128);
    }
    samples.sort_unstable();
    let median = samples[SAMPLES / 2];
    let p95 = samples[(SAMPLES * 95).div_ceil(100).saturating_sub(1)];
    let within_time = median <= budget_nanoseconds && p95 <= budget_nanoseconds * 2;
    let within_allocations = allocation.bytes_allocated <= MAX_ALLOCATED_BYTES_PER_INVOCATION;
    CaseReport {
        name,
        scale,
        iterations_per_sample,
        samples: SAMPLES,
        median_nanoseconds: median,
        p95_nanoseconds: p95,
        minimum_nanoseconds: samples[0],
        maximum_nanoseconds: samples[SAMPLES - 1],
        budget_nanoseconds,
        allocations_per_invocation: allocation.allocations,
        allocated_bytes_per_invocation: allocation.bytes_allocated,
        retained_bytes_per_invocation: retained,
        checksum,
        within_time_budget: within_time,
        within_allocation_budget: within_allocations,
        passed: within_time && within_allocations,
    }
}

fn retained_bytes(stats: Stats) -> i128 {
    stats.bytes_allocated as i128 - stats.bytes_deallocated as i128
}

fn parse_output() -> Result<Option<PathBuf>, String> {
    let mut args = env::args().skip(1);
    let mut output = None;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--output" => {
                output = Some(
                    args.next()
                        .ok_or_else(|| "--output requires a path".to_owned())?
                        .into(),
                );
            }
            unknown => return Err(format!("unknown argument {unknown:?}")),
        }
    }
    Ok(output)
}

fn rustc_version() -> String {
    Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map_or_else(|| "unknown".to_owned(), |version| version.trim().to_owned())
}

fn cpu_model() -> Option<String> {
    if let Ok(output) = Command::new("sysctl")
        .args(["-n", "machdep.cpu.brand_string"])
        .output()
        && output.status.success()
    {
        return String::from_utf8(output.stdout)
            .ok()
            .map(|value| value.trim().to_owned());
    }
    fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|cpuinfo| {
            cpuinfo.lines().find_map(|line| {
                line.strip_prefix("model name\t:")
                    .map(|value| value.trim().to_owned())
            })
        })
}

fn fail(message: &str) -> ! {
    eprintln!("performance-profile: {message}");
    std::process::exit(1)
}
