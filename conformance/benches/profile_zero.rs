use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use nuif_api::{Session, profile_zero_context};
use nuif_codec::{CanonicalText, Decoder, DeterministicCbor, Encoder};
use nuif_core::{EntityId, validate};
use nuif_layout::evaluate;
use nuif_protocol::{Operation, Patch, Transaction, apply_patch};
use nuif_render::{RenderTarget, build_scene, render_cpu};
use nuif_testing::{performance_fixture, responsive_card_fixture};
use std::hint::black_box;
use std::time::Duration;

const MODEL_SIZES: &[usize] = &[8, 128, 1_024, 4_096];
const CODEC_SIZES: &[usize] = &[8, 128, 1_024];

fn benchmark_validation(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("model/validate");
    for &entities in MODEL_SIZES {
        let document = performance_fixture(entities, true);
        group.throughput(Throughput::Elements(entities as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(entities),
            &document,
            |bencher, document| {
                bencher.iter(|| validate(black_box(document)));
            },
        );
    }
    group.finish();
}

fn benchmark_codecs(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("codec");
    for &entities in CODEC_SIZES {
        let document = performance_fixture(entities, true);
        let text = CanonicalText.encode(&document).unwrap();
        let cbor = DeterministicCbor.encode(&document).unwrap();
        group.throughput(Throughput::Elements(entities as u64));
        group.bench_function(BenchmarkId::new("text_encode", entities), |bencher| {
            bencher.iter(|| CanonicalText.encode(black_box(&document)).unwrap());
        });
        group.bench_function(BenchmarkId::new("text_decode", entities), |bencher| {
            bencher.iter(|| CanonicalText.decode(black_box(&text)).unwrap());
        });
        group.bench_function(BenchmarkId::new("cbor_encode", entities), |bencher| {
            bencher.iter(|| DeterministicCbor.encode(black_box(&document)).unwrap());
        });
        group.bench_function(BenchmarkId::new("cbor_decode", entities), |bencher| {
            bencher.iter(|| DeterministicCbor.decode(black_box(&cbor)).unwrap());
        });
    }
    group.finish();
}

fn benchmark_protocol(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("protocol/apply_single_rename");
    for &entities in MODEL_SIZES {
        let document = performance_fixture(entities, false);
        let target = EntityId::new(entities as u128 + 1);
        let patch = Patch {
            base_revision: None,
            transactions: vec![Transaction {
                id: 1,
                operations: vec![Operation::Rename {
                    entity: target,
                    name: Some("benchmark edit".to_owned()),
                }],
            }],
        };
        group.throughput(Throughput::Elements(entities as u64));
        group.bench_function(BenchmarkId::from_parameter(entities), |bencher| {
            bencher.iter_batched(
                || document.clone(),
                |mut candidate| apply_patch(black_box(&mut candidate), black_box(&patch)).unwrap(),
                BatchSize::LargeInput,
            );
        });
    }
    group.finish();
}

fn benchmark_layout_and_scene(criterion: &mut Criterion) {
    let context = profile_zero_context(1_440.0, 900.0);
    let mut layout_group = criterion.benchmark_group("layout/evaluate");
    for &entities in MODEL_SIZES {
        let document = performance_fixture(entities, true);
        layout_group.throughput(Throughput::Elements(entities as u64));
        layout_group.bench_function(BenchmarkId::from_parameter(entities), |bencher| {
            bencher.iter(|| evaluate(black_box(&document), black_box(&context)));
        });
    }
    layout_group.finish();

    let mut scene_group = criterion.benchmark_group("render/build_scene");
    for &entities in CODEC_SIZES {
        let document = performance_fixture(entities, true);
        let layout = evaluate(&document, &context);
        scene_group.throughput(Throughput::Elements(entities as u64));
        scene_group.bench_function(BenchmarkId::from_parameter(entities), |bencher| {
            bencher.iter(|| {
                build_scene(
                    black_box(&document),
                    black_box(&layout),
                    black_box(&context),
                )
                .unwrap()
            });
        });
    }
    scene_group.finish();
}

fn benchmark_raster_and_snapshot(criterion: &mut Criterion) {
    let document = responsive_card_fixture();
    let mut raster_group = criterion.benchmark_group("render/cpu_raster");
    for &(width, height) in &[(360_u32, 640_u32), (768, 640), (1_440, 900)] {
        let context = profile_zero_context(f64::from(width), f64::from(height));
        let layout = evaluate(&document, &context);
        let scene = build_scene(&document, &layout, &context).unwrap();
        let target = RenderTarget {
            width,
            height,
            scale_factor: 1.0,
        };
        raster_group.throughput(Throughput::Bytes(u64::from(width) * u64::from(height) * 4));
        raster_group.bench_function(BenchmarkId::new("rgba", format!("{width}x{height}")), |b| {
            b.iter(|| render_cpu(black_box(&scene), black_box(target)).unwrap());
        });
    }
    raster_group.finish();

    let session = Session::new(document);
    let mut snapshot_group = criterion.benchmark_group("api/snapshot");
    for &(width, height) in &[(360_u32, 640_u32), (768, 640), (1_440, 900)] {
        let context = profile_zero_context(f64::from(width), f64::from(height));
        snapshot_group.throughput(Throughput::Bytes(u64::from(width) * u64::from(height) * 4));
        snapshot_group.bench_function(
            BenchmarkId::from_parameter(format!("{width}x{height}")),
            |b| {
                b.iter(|| session.snapshot(black_box(&context)).unwrap());
            },
        );
    }
    snapshot_group.finish();
}

fn criterion_config() -> Criterion {
    Criterion::default()
        .sample_size(20)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(2))
        .noise_threshold(0.03)
}

criterion_group! {
    name = profile_zero;
    config = criterion_config();
    targets = benchmark_validation,
        benchmark_codecs,
        benchmark_protocol,
        benchmark_layout_and_scene,
        benchmark_raster_and_snapshot
}
criterion_main!(profile_zero);
