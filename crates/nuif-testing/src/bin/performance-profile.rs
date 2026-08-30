use nuif_api::{Session, profile_zero_context};
use nuif_codec::{CanonicalText, Decoder, DeterministicCbor, Encoder};
use nuif_core::{EntityId, PropertyValue, SizeIntent, validate};
use nuif_font::inspect_opentype_static;
use nuif_layout::evaluate;
use nuif_media::{decode_png_rgba8, inspect_png_rgba8};
use nuif_package::{NuifPackage, PackageMode};
use nuif_protocol::{Operation, Patch, Transaction, apply_patch};
use nuif_render::{RenderTarget, build_scene, build_scene_with_resources, render_cpu};
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
const FOREIGN_PENPOT: &[u8] =
    include_bytes!("../../../../conformance/foreign/penpot/fixture.penpot");

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
    let package = NuifPackage::new(large.clone(), PackageMode::Portable);
    let package_bytes = package
        .encode()
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
    let mut warmed_session = Session::new(large.clone());
    warmed_session
        .apply_transaction(
            0,
            vec![Operation::Rename {
                entity: EntityId::new(1_025),
                name: Some("warm session".to_owned()),
            }],
        )
        .unwrap_or_else(|error| fail(&error.to_string()));

    let html_document = nuif_html::profile_fixture();
    let html_exported = nuif_html::export_document(&html_document).unwrap();
    let html_imported = nuif_html::import_source(&html_exported.source).unwrap();
    let mut html_edited = html_document.clone();
    html_edited
        .tokens
        .get_mut(&EntityId::new(0x100))
        .unwrap()
        .value = PropertyValue::Real(32.0);

    let svg_document = nuif_svg::profile_fixture();
    let svg_exported = nuif_svg::export_document(&svg_document).unwrap();
    let svg_imported = nuif_svg::import_source(&svg_exported.source).unwrap();
    let mut svg_edited = svg_document.clone();
    svg_edited
        .entities
        .get_mut(&EntityId::new(0x21))
        .unwrap()
        .authored
        .position
        .x = 22.0;

    let dtcg_document = nuif_dtcg::profile_fixture();
    let dtcg_exported = nuif_dtcg::export_document(&dtcg_document).unwrap();
    let dtcg_imported = nuif_dtcg::import_source(&dtcg_exported.source).unwrap();
    let mut dtcg_edited = dtcg_document.clone();
    dtcg_edited
        .tokens
        .get_mut(&EntityId::new(0x102))
        .unwrap()
        .value = PropertyValue::Integer(8);

    let react_document = nuif_react::profile_fixture();
    let react_exported = nuif_react::export_document(&react_document).unwrap();
    let react_imported = nuif_react::import_source(&react_exported.source).unwrap();
    let mut react_edited = react_document.clone();
    react_edited
        .entities
        .get_mut(&EntityId::new(0x10))
        .unwrap()
        .authored
        .layout
        .gap = 20.0;

    let svelte_document = nuif_svelte::profile_fixture();
    let svelte_exported = nuif_svelte::export_document(&svelte_document).unwrap();
    let svelte_imported = nuif_svelte::import_source(&svelte_exported.source).unwrap();
    let mut svelte_edited = svelte_document.clone();
    svelte_edited
        .entities
        .get_mut(&EntityId::new(0x10))
        .unwrap()
        .authored
        .layout
        .gap = 20.0;

    let penpot_document = nuif_penpot::profile_fixture();
    let penpot_exported = nuif_penpot::export_document(&penpot_document).unwrap();
    let penpot_imported = nuif_penpot::import_package(&penpot_exported.bytes).unwrap();
    let mut penpot_edited = penpot_document.clone();
    penpot_edited
        .entities
        .get_mut(&EntityId::new(0x21))
        .unwrap()
        .authored
        .width = SizeIntent::Fixed(280.0);

    let mut image_package = nuif_testing::rgba8_image_package_fixture();
    let image_entity = image_package
        .document
        .entities
        .get_mut(&EntityId::new(2))
        .expect("image performance fixture entity");
    image_entity.authored.width = SizeIntent::Fixed(256.0);
    image_entity.authored.height = SizeIntent::Fixed(256.0);
    let image_digest = image_package
        .resources
        .keys()
        .next()
        .expect("image performance fixture resource")
        .clone();
    let image_resource = image_package
        .embedded(&image_digest)
        .expect("embedded image performance fixture")
        .to_vec();
    let image_package_bytes = image_package.encode().unwrap();
    let image_context = profile_zero_context(256.0, 256.0);
    let image_layout = evaluate(&image_package.document, &image_context);
    let image_scene = build_scene_with_resources(
        &image_package.document,
        &image_layout,
        &image_context,
        |digest| image_package.embedded(digest),
    )
    .unwrap();
    let image_target = RenderTarget {
        width: 256,
        height: 256,
        scale_factor: 1.0,
    };
    let font_package = nuif_testing::static_font_package_fixture();
    let font_digest = font_package
        .resources
        .keys()
        .next()
        .expect("font performance fixture resource")
        .clone();
    let font_resource = font_package
        .embedded(&font_digest)
        .expect("embedded font performance fixture");
    let font_package_bytes = font_package.encode().unwrap();

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
        measure("package_encode", 1_024, 1, 1_000_000_000, || {
            black_box(package.encode().unwrap()).len() as u64
        }),
        measure("package_decode", 1_024, 1, 1_000_000_000, || {
            black_box(NuifPackage::decode(black_box(&package_bytes)).unwrap())
                .document
                .entities
                .len() as u64
        }),
        measure("protocol_apply", 1_024, 1, 1_000_000_000, || {
            let mut candidate = black_box(large.clone());
            apply_patch(&mut candidate, black_box(&patch)).unwrap();
            candidate.entities.len() as u64
        }),
        measure("session_transaction_cold", 1_024, 1, 1_000_000_000, || {
            let mut session = Session::new(black_box(large.clone()));
            let applied = session
                .apply_transaction(
                    1,
                    vec![Operation::Rename {
                        entity: EntityId::new(1_025),
                        name: Some("performance edit".to_owned()),
                    }],
                )
                .unwrap();
            session.document().entities.len() as u64 ^ applied.transactions.len() as u64
        }),
        measure("session_transaction_warm", 1_024, 1, 1_000_000_000, || {
            let mut session = black_box(warmed_session.clone());
            let applied = session
                .apply_transaction(
                    2,
                    vec![Operation::Rename {
                        entity: EntityId::new(1_025),
                        name: Some("performance edit".to_owned()),
                    }],
                )
                .unwrap();
            session.document().entities.len() as u64 ^ applied.transactions.len() as u64
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
        measure(
            "media_png_inspect",
            image_resource.len(),
            50,
            500_000_000,
            || {
                let header = inspect_png_rgba8(black_box(&image_resource)).unwrap();
                (u64::from(header.width) * u64::from(header.height)) ^ header.chunks as u64
            },
        ),
        measure(
            "media_png_decode",
            image_resource.len(),
            20,
            500_000_000,
            || {
                let image = decode_png_rgba8(black_box(&image_resource)).unwrap();
                image.rgba.len() as u64 ^ u64::from(image.rgba[0])
            },
        ),
        measure(
            "font_static_inspect",
            font_resource.len(),
            20,
            500_000_000,
            || {
                let font = inspect_opentype_static(black_box(font_resource), 0).unwrap();
                u64::from(font.glyph_count)
                    ^ font.table_tags.len() as u64
                    ^ font.coverage.len() as u64
            },
        ),
        measure(
            "image_package_encode",
            image_package.document.assets.len(),
            5,
            1_000_000_000,
            || black_box(image_package.encode().unwrap()).len() as u64,
        ),
        measure(
            "image_package_decode",
            image_package.document.assets.len(),
            5,
            1_000_000_000,
            || {
                let decoded = NuifPackage::decode(black_box(&image_package_bytes)).unwrap();
                decoded.resources.len() as u64 ^ decoded.document.entities.len() as u64
            },
        ),
        measure(
            "font_package_encode",
            font_package.document.assets.len(),
            5,
            1_000_000_000,
            || black_box(font_package.encode().unwrap()).len() as u64,
        ),
        measure(
            "font_package_decode",
            font_package.document.assets.len(),
            5,
            1_000_000_000,
            || {
                let decoded = NuifPackage::decode(black_box(&font_package_bytes)).unwrap();
                decoded.resources.len() as u64 ^ decoded.document.assets.len() as u64
            },
        ),
        measure(
            "resource_scene_lowering",
            256 * 256,
            10,
            1_000_000_000,
            || {
                let scene = build_scene_with_resources(
                    black_box(&image_package.document),
                    black_box(&image_layout),
                    black_box(&image_context),
                    |digest| image_package.embedded(digest),
                )
                .unwrap();
                scene.commands.len() as u64 ^ scene.fidelity.len() as u64
            },
        ),
        measure("resource_cpu_raster", 256 * 256, 2, 1_000_000_000, || {
            let image = render_cpu(black_box(&image_scene), image_target).unwrap();
            image.rgba.len() as u64 ^ u64::from(image.rgba[0])
        }),
        measure("adapter_html_export", 2, 20, 500_000_000, || {
            nuif_html::export_document(black_box(&html_document))
                .unwrap()
                .source
                .len() as u64
        }),
        measure("adapter_html_import", 2, 20, 500_000_000, || {
            nuif_html::import_source(black_box(&html_exported.source))
                .unwrap()
                .document
                .entities
                .len() as u64
        }),
        measure("adapter_html_sync", 2, 20, 500_000_000, || {
            nuif_html::synchronize(black_box(&html_imported.retentive), black_box(&html_edited))
                .unwrap()
                .source
                .len() as u64
        }),
        measure("adapter_svg_export", 4, 20, 500_000_000, || {
            nuif_svg::export_document(black_box(&svg_document))
                .unwrap()
                .source
                .len() as u64
        }),
        measure("adapter_svg_import", 4, 20, 500_000_000, || {
            nuif_svg::import_source(black_box(&svg_exported.source))
                .unwrap()
                .document
                .entities
                .len() as u64
        }),
        measure("adapter_svg_sync", 4, 20, 500_000_000, || {
            nuif_svg::synchronize(black_box(&svg_imported.retentive), black_box(&svg_edited))
                .unwrap()
                .source
                .len() as u64
        }),
        measure("adapter_dtcg_export", 3, 20, 500_000_000, || {
            nuif_dtcg::export_document(black_box(&dtcg_document))
                .unwrap()
                .source
                .len() as u64
        }),
        measure("adapter_dtcg_import", 3, 20, 500_000_000, || {
            nuif_dtcg::import_source(black_box(&dtcg_exported.source))
                .unwrap()
                .document
                .tokens
                .len() as u64
        }),
        measure("adapter_dtcg_sync", 3, 20, 500_000_000, || {
            nuif_dtcg::synchronize(black_box(&dtcg_imported.retentive), black_box(&dtcg_edited))
                .unwrap()
                .source
                .len() as u64
        }),
        measure("adapter_react_export", 2, 20, 500_000_000, || {
            nuif_react::export_document(black_box(&react_document))
                .unwrap()
                .source
                .len() as u64
        }),
        measure("adapter_react_import", 2, 20, 500_000_000, || {
            nuif_react::import_source(black_box(&react_exported.source))
                .unwrap()
                .document
                .entities
                .len() as u64
        }),
        measure("adapter_react_sync", 2, 20, 500_000_000, || {
            nuif_react::synchronize(
                black_box(&react_imported.retentive),
                black_box(&react_edited),
            )
            .unwrap()
            .source
            .len() as u64
        }),
        measure("adapter_svelte_export", 2, 20, 500_000_000, || {
            nuif_svelte::export_document(black_box(&svelte_document))
                .unwrap()
                .source
                .len() as u64
        }),
        measure("adapter_svelte_import", 2, 20, 500_000_000, || {
            nuif_svelte::import_source(black_box(&svelte_exported.source))
                .unwrap()
                .document
                .entities
                .len() as u64
        }),
        measure("adapter_svelte_sync", 2, 20, 500_000_000, || {
            nuif_svelte::synchronize(
                black_box(&svelte_imported.retentive),
                black_box(&svelte_edited),
            )
            .unwrap()
            .source
            .len() as u64
        }),
        measure("adapter_penpot_export", 4, 20, 500_000_000, || {
            nuif_penpot::export_document(black_box(&penpot_document))
                .unwrap()
                .bytes
                .len() as u64
        }),
        measure("adapter_penpot_import", 4, 20, 500_000_000, || {
            nuif_penpot::import_package(black_box(&penpot_exported.bytes))
                .unwrap()
                .document
                .entities
                .len() as u64
        }),
        measure("adapter_penpot_foreign_import", 4, 20, 500_000_000, || {
            nuif_penpot::import_package(black_box(FOREIGN_PENPOT))
                .unwrap()
                .document
                .entities
                .len() as u64
        }),
        measure("adapter_penpot_sync_noop", 4, 50, 500_000_000, || {
            nuif_penpot::synchronize(
                black_box(&penpot_imported.retentive),
                black_box(&penpot_document),
            )
            .unwrap()
            .bytes
            .len() as u64
        }),
        measure("adapter_penpot_sync_edit", 4, 20, 500_000_000, || {
            nuif_penpot::synchronize(
                black_box(&penpot_imported.retentive),
                black_box(&penpot_edited),
            )
            .unwrap()
            .bytes
            .len() as u64
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
            "criterion_suites": ["cargo bench -p nuif-conformance --bench profile_zero", "cargo bench -p nuif-conformance --bench system_surfaces"],
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
            "package_bytes": package_bytes.len(),
            "layout_boxes": layout.boxes.len(),
            "scene_commands": scene.commands.len(),
            "scene_fidelity_records": scene.fidelity.len()
        },
        "adapter_fixtures": {
            "html_css_bytes": html_exported.source.len(),
            "svg_bytes": svg_exported.source.len(),
            "dtcg_bytes": dtcg_exported.source.len(),
            "react_jsx_bytes": react_exported.source.len(),
            "svelte_bytes": svelte_exported.source.len(),
            "penpot_bytes": penpot_exported.bytes.len(),
            "foreign_penpot_bytes": FOREIGN_PENPOT.len()
        },
        "resource_fixtures": {
            "image_resource_bytes": image_resource.len(),
            "image_package_bytes": image_package_bytes.len(),
            "font_resource_bytes": font_resource.len(),
            "font_package_bytes": font_package_bytes.len(),
            "resource_scene_commands": image_scene.commands.len()
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
