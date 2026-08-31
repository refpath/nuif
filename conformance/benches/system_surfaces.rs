use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use nuif_api::{DocumentEncoding, NuifDocument};
use nuif_codec::{CanonicalText, DeterministicCbor, Encoder};
use nuif_collab::creation::{CreationAnchor, CreationChange, CreationOperation};
use nuif_collab::gc::StabilityFrontier;
use nuif_collab::mixed::{MixedChange, MixedOperation, MixedOperationSetEngine};
use nuif_collab::nested_creation::ArbitraryAnchorCreationOperationSetEngine;
use nuif_collab::structural::{
    PositionId, StructuralAnchor, StructuralChange, StructuralOperation,
    StructuralOperationSetEngine,
};
use nuif_collab::{Change, ChangeId, OperationSetEngine, ReplicaLogEngine};
use nuif_core::{Document, Entity, EntityId, EntityKind, PropertyValue, SizeIntent};
use nuif_package::{NuifPackage, PackageMode};
use nuif_protocol::Operation;
use nuif_testing::{performance_fixture, responsive_card_fixture};
use std::collections::{BTreeMap, BTreeSet};
use std::hint::black_box;
use std::time::Duration;

const QUERY_SIZES: &[usize] = &[128, 1_024, 4_096, 8_192];
const COLLABORATOR_SIZES: &[usize] = &[2, 32, 256, 1_024];
const PACKAGE_SIZES: &[usize] = &[128, 1_024, 4_096];

fn benchmark_direct_sdk(criterion: &mut Criterion) {
    let document = performance_fixture(1_024, true);
    let text = CanonicalText.encode(&document).unwrap();
    let cbor = DeterministicCbor.encode(&document).unwrap();
    let package = NuifPackage::new(document, PackageMode::Portable)
        .encode()
        .unwrap();
    let loaded = NuifDocument::load(&cbor, DocumentEncoding::DeterministicCbor).unwrap();
    let mut group = criterion.benchmark_group("sdk/direct_document");

    group.throughput(Throughput::Bytes(text.len() as u64));
    group.bench_function("load_text", |bencher| {
        bencher.iter(|| {
            NuifDocument::load(black_box(&text), black_box(DocumentEncoding::CanonicalText))
                .unwrap()
        });
    });
    group.throughput(Throughput::Bytes(cbor.len() as u64));
    group.bench_function("load_cbor", |bencher| {
        bencher.iter(|| {
            NuifDocument::load(
                black_box(&cbor),
                black_box(DocumentEncoding::DeterministicCbor),
            )
            .unwrap()
        });
    });
    group.throughput(Throughput::Bytes(package.len() as u64));
    group.bench_function("load_package", |bencher| {
        bencher.iter(|| NuifDocument::load_package(black_box(&package)).unwrap());
    });
    group.throughput(Throughput::Bytes(cbor.len() as u64));
    group.bench_function("export_cbor", |bencher| {
        bencher.iter(|| {
            black_box(&loaded)
                .export(black_box(DocumentEncoding::DeterministicCbor))
                .unwrap()
        });
    });
    group.finish();
}

fn benchmark_package_capabilities(criterion: &mut Criterion) {
    let required = BTreeSet::from(["nuif-behavior-state-machine-0".to_owned()]);
    let mut package = NuifPackage::new(performance_fixture(1_024, true), PackageMode::Portable);
    package.required_capabilities.clone_from(&required);
    let bytes = package.encode().unwrap();
    let structural = NuifDocument::load_package(&bytes).unwrap();
    let mut group = criterion.benchmark_group("sdk/package_capabilities");

    group.throughput(Throughput::Bytes(bytes.len() as u64));
    group.bench_function("structural_load", |bencher| {
        bencher.iter(|| NuifDocument::load_package(black_box(&bytes)).unwrap());
    });
    group.bench_function("capability_report", |bencher| {
        bencher
            .iter(|| black_box(&structural).package_capability_report(black_box(&BTreeSet::new())));
    });
    group.bench_function("load_and_authorize", |bencher| {
        bencher.iter(|| {
            NuifDocument::load_package_with_capabilities(black_box(&bytes), black_box(&required))
                .unwrap()
        });
    });
    group.bench_function("authorize_loaded", |bencher| {
        bencher.iter_batched(
            || structural.clone(),
            |mut document| {
                document
                    .require_package_capabilities(black_box(&required))
                    .unwrap();
            },
            BatchSize::LargeInput,
        );
    });
    group.finish();
}

fn benchmark_queries(criterion: &mut Criterion) {
    let mut lookup = criterion.benchmark_group("query/entity_lookup");
    for &entities in QUERY_SIZES {
        let document = performance_fixture(entities, true);
        let target = EntityId::new(entities as u128 + 1);
        lookup.bench_function(BenchmarkId::from_parameter(entities), |bencher| {
            bencher.iter(|| nuif_query::entity(black_box(&document), black_box(target)).unwrap());
        });
    }
    lookup.finish();

    let mut scan = criterion.benchmark_group("query/by_kind_scan");
    for &entities in QUERY_SIZES {
        let document = performance_fixture(entities, true);
        scan.throughput(Throughput::Elements(entities as u64));
        scan.bench_function(BenchmarkId::from_parameter(entities), |bencher| {
            bencher.iter(|| {
                nuif_query::by_kind(black_box(&document), |kind| {
                    matches!(kind, EntityKind::Text)
                })
            });
        });
    }
    scan.finish();
}

fn collaboration_engines(changes: usize) -> (OperationSetEngine, ReplicaLogEngine) {
    let mut operation_set = OperationSetEngine::default();
    let mut replica_logs = ReplicaLogEngine::default();
    for index in 0..changes {
        let change = Change {
            id: ChangeId::new(format!("replica-{index:04}"), 1),
            context: BTreeMap::new(),
            operation: Operation::Rename {
                entity: EntityId::new(0x20),
                name: Some(format!("collaborator {index}")),
            },
        };
        operation_set.ingest(change.clone()).unwrap();
        replica_logs.ingest(change).unwrap();
    }
    (operation_set, replica_logs)
}

fn benchmark_collaboration(criterion: &mut Criterion) {
    let base = responsive_card_fixture();
    let mut group = criterion.benchmark_group("collaboration/conflict_checkpoint");
    for &changes in COLLABORATOR_SIZES {
        let (operation_set, replica_logs) = collaboration_engines(changes);
        group.throughput(Throughput::Elements(changes as u64));
        group.bench_function(BenchmarkId::new("operation_set", changes), |bencher| {
            bencher.iter(|| operation_set.checkpoint(black_box(&base)).unwrap());
        });
        group.bench_function(BenchmarkId::new("replica_logs", changes), |bencher| {
            bencher.iter(|| replica_logs.checkpoint(black_box(&base)).unwrap());
        });
    }
    group.finish();
}

fn advanced_structure_base() -> Document {
    let root_id = EntityId::new(10);
    let left_id = EntityId::new(11);
    let right_id = EntityId::new(12);
    let mut base = Document::empty(EntityId::new(1));
    let mut root = Entity::new(root_id, EntityKind::Container);
    root.children.extend([left_id, right_id]);
    base.roots.push(root_id);
    base.entities.insert(root_id, root);
    base.entities
        .insert(left_id, Entity::new(left_id, EntityKind::Container));
    base.entities
        .insert(right_id, Entity::new(right_id, EntityKind::Container));
    base
}

fn advanced_nested_fixture(base: &Document) -> ArbitraryAnchorCreationOperationSetEngine {
    let creation_changes = vec![
        CreationChange {
            id: ChangeId::new("alice", 1),
            context: BTreeMap::new(),
            operation: CreationOperation::Insert {
                parent: Some(EntityId::new(10)),
                anchor: CreationAnchor::Start,
                entity: Box::new(Entity::new(EntityId::new(20), EntityKind::Container)),
            },
        },
        CreationChange {
            id: ChangeId::new("bob", 1),
            context: BTreeMap::from([("alice".to_owned(), 1)]),
            operation: CreationOperation::Insert {
                parent: Some(EntityId::new(20)),
                anchor: CreationAnchor::Start,
                entity: Box::new(Entity::new(EntityId::new(21), EntityKind::Container)),
            },
        },
    ];
    let mut nested = ArbitraryAnchorCreationOperationSetEngine::new(base.clone()).unwrap();
    for change in creation_changes {
        nested.ingest(change).unwrap();
    }
    nested
}

fn advanced_mixed_fixture(base: &Document) -> MixedOperationSetEngine {
    let mixed_changes = vec![
        MixedChange {
            id: ChangeId::new("alice", 1),
            context: BTreeMap::new(),
            operation: MixedOperation::Structure(StructuralOperation::Move {
                entity: EntityId::new(12),
                new_parent: Some(EntityId::new(11)),
                anchor: StructuralAnchor::Start,
            }),
        },
        MixedChange {
            id: ChangeId::new("bob", 1),
            context: BTreeMap::new(),
            operation: MixedOperation::Property(Operation::Rename {
                entity: EntityId::new(12),
                name: Some("benchmark".to_owned()),
            }),
        },
    ];
    let mut mixed = MixedOperationSetEngine::new(base.clone()).unwrap();
    for change in mixed_changes {
        mixed.ingest(change).unwrap();
    }
    mixed
}

fn causal_prefix_fixture() -> (OperationSetEngine, Document, StabilityFrontier) {
    let prefix_base = responsive_card_fixture();
    let mut prefix = OperationSetEngine::default();
    prefix
        .ingest(Change {
            id: ChangeId::new("alice", 1),
            context: BTreeMap::new(),
            operation: Operation::Rename {
                entity: EntityId::new(0x20),
                name: Some("stable".to_owned()),
            },
        })
        .unwrap();
    prefix
        .ingest(Change {
            id: ChangeId::new("alice", 2),
            context: BTreeMap::from([("alice".to_owned(), 1)]),
            operation: Operation::Rename {
                entity: EntityId::new(0x20),
                name: Some("retained".to_owned()),
            },
        })
        .unwrap();
    let prefix_frontier =
        StabilityFrontier::new(BTreeMap::from([("alice".to_owned(), 1)])).unwrap();
    (prefix, prefix_base, prefix_frontier)
}

fn structural_prefix_fixture(base: &Document) -> (StructuralOperationSetEngine, StabilityFrontier) {
    let mut structural_prefix = StructuralOperationSetEngine::new(base.clone()).unwrap();
    structural_prefix
        .ingest(StructuralChange {
            id: ChangeId::new("prefix", 1),
            context: BTreeMap::new(),
            operation: StructuralOperation::Move {
                entity: EntityId::new(11),
                new_parent: None,
                anchor: StructuralAnchor::Start,
            },
        })
        .unwrap();
    structural_prefix
        .ingest(StructuralChange {
            id: ChangeId::new("prefix", 2),
            context: BTreeMap::from([("prefix".to_owned(), 1)]),
            operation: StructuralOperation::Move {
                entity: EntityId::new(12),
                new_parent: None,
                anchor: StructuralAnchor::After(PositionId::Change(ChangeId::new("prefix", 1))),
            },
        })
        .unwrap();
    let frontier = StabilityFrontier::new(BTreeMap::from([("prefix".to_owned(), 1)])).unwrap();
    (structural_prefix, frontier)
}

fn benchmark_advanced_collaboration(criterion: &mut Criterion) {
    let base = advanced_structure_base();
    let nested = advanced_nested_fixture(&base);
    let mixed = advanced_mixed_fixture(&base);
    let (prefix, prefix_base, prefix_frontier) = causal_prefix_fixture();
    let (structural_prefix, structural_prefix_frontier) = structural_prefix_fixture(&base);
    let mut group = criterion.benchmark_group("collaboration/advanced_profiles");
    group.throughput(Throughput::Elements(2));
    group.bench_function("nested_creation_v1", |bencher| {
        bencher.iter(|| nested.checkpoint().unwrap());
    });
    group.bench_function("mixed_property_structure", |bencher| {
        bencher.iter(|| mixed.checkpoint().unwrap());
    });
    group.bench_function("causal_register_prefix_compaction", |bencher| {
        bencher.iter(|| {
            prefix
                .compact_stable_prefix(black_box(&prefix_base), black_box(&prefix_frontier))
                .unwrap()
        });
    });
    group.bench_function("causal_structural_prefix_compaction", |bencher| {
        bencher.iter(|| {
            structural_prefix
                .compact_stable_prefix(black_box(&structural_prefix_frontier))
                .unwrap()
        });
    });
    group.finish();
}

fn benchmark_package(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("package/nuif_package_0");
    for &entities in PACKAGE_SIZES {
        let package = NuifPackage::new(performance_fixture(entities, true), PackageMode::Portable);
        let bytes = package.encode().unwrap();
        group.throughput(Throughput::Bytes(bytes.len() as u64));
        group.bench_function(BenchmarkId::new("encode", entities), |bencher| {
            bencher.iter(|| black_box(&package).encode().unwrap());
        });
        group.bench_function(BenchmarkId::new("decode", entities), |bencher| {
            bencher.iter(|| NuifPackage::decode(black_box(&bytes)).unwrap());
        });
    }
    group.finish();
}

fn benchmark_resource_profiles(criterion: &mut Criterion) {
    let image_package = nuif_testing::rgba8_image_package_fixture();
    let image_digest = image_package
        .resources
        .keys()
        .next()
        .expect("image benchmark resource");
    let image_bytes = image_package
        .embedded(image_digest)
        .expect("embedded image benchmark resource")
        .to_vec();
    let image_package_bytes = image_package.encode().unwrap();
    let font_package = nuif_testing::static_font_package_fixture();
    let font_digest = font_package
        .resources
        .keys()
        .next()
        .expect("font benchmark resource");
    let font_bytes = font_package
        .embedded(font_digest)
        .expect("embedded font benchmark resource")
        .to_vec();
    let font_package_bytes = font_package.encode().unwrap();

    let mut group = criterion.benchmark_group("resource/profile_baselines");
    group.bench_function("png_inspect", |bencher| {
        bencher.iter(|| nuif_media::inspect_png_rgba8(black_box(&image_bytes)).unwrap());
    });
    group.bench_function("png_decode", |bencher| {
        bencher.iter(|| nuif_media::decode_png_rgba8(black_box(&image_bytes)).unwrap());
    });
    group.bench_function("font_inspect", |bencher| {
        bencher.iter(|| nuif_font::inspect_opentype_static(black_box(&font_bytes), 0).unwrap());
    });
    group.bench_function("image_package_encode", |bencher| {
        bencher.iter(|| black_box(&image_package).encode().unwrap());
    });
    group.bench_function("image_package_decode", |bencher| {
        bencher.iter(|| NuifPackage::decode(black_box(&image_package_bytes)).unwrap());
    });
    group.bench_function("font_package_encode", |bencher| {
        bencher.iter(|| black_box(&font_package).encode().unwrap());
    });
    group.bench_function("font_package_decode", |bencher| {
        bencher.iter(|| NuifPackage::decode(black_box(&font_package_bytes)).unwrap());
    });
    group.finish();
}

fn benchmark_html_adapter(criterion: &mut Criterion) {
    let document = nuif_html::profile_fixture();
    let exported = nuif_html::export_document(&document).unwrap();
    let imported = nuif_html::import_source(&exported.source).unwrap();
    let mut edited = document.clone();
    edited.tokens.get_mut(&EntityId::new(0x100)).unwrap().value = PropertyValue::Real(32.0);
    edited
        .entities
        .get_mut(&EntityId::new(0x11))
        .unwrap()
        .authored
        .text
        .as_mut()
        .unwrap()
        .content = String::from("Edited adapter benchmark");
    let mut group = criterion.benchmark_group("adapter/html_css_0");
    group.throughput(Throughput::Bytes(exported.source.len() as u64));
    group.bench_function("export", |bencher| {
        bencher.iter(|| nuif_html::export_document(black_box(&document)).unwrap());
    });
    group.bench_function("import", |bencher| {
        bencher.iter(|| nuif_html::import_source(black_box(&exported.source)).unwrap());
    });
    group.bench_function("synchronize", |bencher| {
        bencher.iter(|| {
            nuif_html::synchronize(black_box(&imported.retentive), black_box(&edited)).unwrap()
        });
    });
    group.finish();
}

fn benchmark_html_v0_adapter(criterion: &mut Criterion) {
    let document = nuif_html::profile_fixture();
    let exported = nuif_html::export_v0_document(&document).unwrap();
    let imported = nuif_html::import_v0_source(&exported.source).unwrap();
    let mut edited = document.clone();
    edited.tokens.get_mut(&EntityId::new(0x100)).unwrap().value = PropertyValue::Real(32.0);
    edited
        .entities
        .get_mut(&EntityId::new(0x11))
        .unwrap()
        .authored
        .text
        .as_mut()
        .unwrap()
        .content = String::from("Edited v0 adapter benchmark");
    let mut group = criterion.benchmark_group("adapter/html_css_v0");
    group.throughput(Throughput::Bytes(exported.source.len() as u64));
    group.bench_function("export", |bencher| {
        bencher.iter(|| nuif_html::export_v0_document(black_box(&document)).unwrap());
    });
    group.bench_function("import", |bencher| {
        bencher.iter(|| nuif_html::import_v0_source(black_box(&exported.source)).unwrap());
    });
    group.bench_function("synchronize", |bencher| {
        bencher.iter(|| {
            nuif_html::synchronize_v0(black_box(&imported.retentive), black_box(&edited)).unwrap()
        });
    });
    group.finish();
}

fn benchmark_svg_adapter(criterion: &mut Criterion) {
    let document = nuif_svg::profile_fixture();
    let exported = nuif_svg::export_document(&document).unwrap();
    let imported = nuif_svg::import_source(&exported.source).unwrap();
    let mut edited = document.clone();
    edited
        .entities
        .get_mut(&EntityId::new(0x21))
        .unwrap()
        .authored
        .position
        .x = 22.0;
    edited
        .entities
        .get_mut(&EntityId::new(0x22))
        .unwrap()
        .authored
        .width = SizeIntent::Fixed(30.0);
    let mut group = criterion.benchmark_group("adapter/svg_0");
    group.throughput(Throughput::Bytes(exported.source.len() as u64));
    group.bench_function("export", |bencher| {
        bencher.iter(|| nuif_svg::export_document(black_box(&document)).unwrap());
    });
    group.bench_function("import", |bencher| {
        bencher.iter(|| nuif_svg::import_source(black_box(&exported.source)).unwrap());
    });
    group.bench_function("synchronize", |bencher| {
        bencher.iter(|| {
            nuif_svg::synchronize(black_box(&imported.retentive), black_box(&edited)).unwrap()
        });
    });
    group.finish();
}

fn benchmark_dtcg_adapter(criterion: &mut Criterion) {
    let document = nuif_dtcg::profile_fixture();
    let exported = nuif_dtcg::export_document(&document).unwrap();
    let imported = nuif_dtcg::import_source(&exported.source).unwrap();
    let mut edited = document.clone();
    edited.tokens.get_mut(&EntityId::new(0x102)).unwrap().value = PropertyValue::Integer(8);
    let mut group = criterion.benchmark_group("adapter/dtcg_scalar_0");
    group.throughput(Throughput::Bytes(exported.source.len() as u64));
    group.bench_function("export", |bencher| {
        bencher.iter(|| nuif_dtcg::export_document(black_box(&document)).unwrap());
    });
    group.bench_function("import", |bencher| {
        bencher.iter(|| nuif_dtcg::import_source(black_box(&exported.source)).unwrap());
    });
    group.bench_function("synchronize", |bencher| {
        bencher.iter(|| {
            nuif_dtcg::synchronize(black_box(&imported.retentive), black_box(&edited)).unwrap()
        });
    });
    group.finish();
}

fn benchmark_react_adapter(criterion: &mut Criterion) {
    let document = nuif_react::profile_fixture();
    let exported = nuif_react::export_document(&document).unwrap();
    let imported = nuif_react::import_source(&exported.source).unwrap();
    let mut edited = document.clone();
    edited
        .entities
        .get_mut(&EntityId::new(0x10))
        .unwrap()
        .authored
        .layout
        .gap = 20.0;
    let mut group = criterion.benchmark_group("adapter/react_jsx_0");
    group.throughput(Throughput::Bytes(exported.source.len() as u64));
    group.bench_function("export", |bencher| {
        bencher.iter(|| nuif_react::export_document(black_box(&document)).unwrap());
    });
    group.bench_function("import", |bencher| {
        bencher.iter(|| nuif_react::import_source(black_box(&exported.source)).unwrap());
    });
    group.bench_function("synchronize", |bencher| {
        bencher.iter(|| {
            nuif_react::synchronize(black_box(&imported.retentive), black_box(&edited)).unwrap()
        });
    });
    group.finish();
}

fn benchmark_svelte_adapter(criterion: &mut Criterion) {
    let document = nuif_svelte::profile_fixture();
    let exported = nuif_svelte::export_document(&document).unwrap();
    let imported = nuif_svelte::import_source(&exported.source).unwrap();
    let mut edited = document.clone();
    edited
        .entities
        .get_mut(&EntityId::new(0x10))
        .unwrap()
        .authored
        .layout
        .gap = 20.0;
    let mut group = criterion.benchmark_group("adapter/svelte_static_0");
    group.throughput(Throughput::Bytes(exported.source.len() as u64));
    group.bench_function("export", |bencher| {
        bencher.iter(|| nuif_svelte::export_document(black_box(&document)).unwrap());
    });
    group.bench_function("import", |bencher| {
        bencher.iter(|| nuif_svelte::import_source(black_box(&exported.source)).unwrap());
    });
    group.bench_function("synchronize", |bencher| {
        bencher.iter(|| {
            nuif_svelte::synchronize(black_box(&imported.retentive), black_box(&edited)).unwrap()
        });
    });
    group.finish();
}

fn benchmark_penpot_adapter(criterion: &mut Criterion) {
    const FOREIGN_PACKAGE: &[u8] = include_bytes!("../foreign/penpot/fixture.penpot");
    let document = nuif_penpot::profile_fixture();
    let exported = nuif_penpot::export_document(&document).unwrap();
    let imported = nuif_penpot::import_package(&exported.bytes).unwrap();
    let mut edited = document.clone();
    edited.entities.get_mut(&EntityId::new(0x21)).unwrap().name =
        Some("Edited package benchmark".to_owned());
    edited
        .entities
        .get_mut(&EntityId::new(0x21))
        .unwrap()
        .authored
        .width = SizeIntent::Fixed(280.0);
    let mut group = criterion.benchmark_group("adapter/penpot_v3_0");
    group.throughput(Throughput::Bytes(exported.bytes.len() as u64));
    group.bench_function("export", |bencher| {
        bencher.iter(|| nuif_penpot::export_document(black_box(&document)).unwrap());
    });
    group.bench_function("import", |bencher| {
        bencher.iter(|| nuif_penpot::import_package(black_box(&exported.bytes)).unwrap());
    });
    group.bench_function("foreign_import", |bencher| {
        bencher.iter(|| nuif_penpot::import_package(black_box(FOREIGN_PACKAGE)).unwrap());
    });
    group.bench_function("synchronize_noop", |bencher| {
        bencher.iter(|| {
            nuif_penpot::synchronize(black_box(&imported.retentive), black_box(&document)).unwrap()
        });
    });
    group.bench_function("synchronize_edit", |bencher| {
        bencher.iter(|| {
            nuif_penpot::synchronize(black_box(&imported.retentive), black_box(&edited)).unwrap()
        });
    });
    group.finish();
}

fn benchmark_figma_adapter(criterion: &mut Criterion) {
    let document = nuif_figma::profile_fixture();
    let plan = nuif_figma::plan_import(&document, "criterion-fixture").unwrap();
    let snapshot = serde_json::to_vec(&plan.snapshot).unwrap();
    let mut group = criterion.benchmark_group("adapter/figma_plugin_snapshot_0");
    group.throughput(Throughput::Bytes(snapshot.len() as u64));
    group.bench_function("plan_import", |bencher| {
        bencher.iter(|| {
            nuif_figma::plan_import(black_box(&document), black_box("criterion-fixture")).unwrap()
        });
    });
    group.bench_function("import_snapshot", |bencher| {
        bencher.iter(|| nuif_figma::import_snapshot(black_box(&snapshot)).unwrap());
    });
    group.finish();
}

fn benchmark_web_semantic_adapters(criterion: &mut Criterion) {
    let accessibility_document = nuif_html::accessibility::web_accessibility_fixture();
    let (behavior_document, behavior_program, _) = nuif_behavior::behavior_fixture();
    let mut group = criterion.benchmark_group("adapter/web_semantics");

    group.throughput(Throughput::Elements(
        accessibility_document.entities.len() as u64
    ));
    group.bench_function("accessibility_projection", |bencher| {
        bencher.iter(|| {
            nuif_html::accessibility::project_web_accessibility(black_box(&accessibility_document))
                .unwrap()
        });
    });
    group.throughput(Throughput::Elements(behavior_document.entities.len() as u64));
    group.bench_function("behavior_projection", |bencher| {
        bencher.iter(|| {
            nuif_html::behavior::project_web_behavior(
                black_box(&behavior_document),
                black_box(&behavior_program),
            )
            .unwrap()
        });
    });
    group.finish();
}

fn criterion_config() -> Criterion {
    Criterion::default()
        .sample_size(20)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(2))
        .noise_threshold(0.03)
}

criterion_group! {
    name = system_surfaces;
    config = criterion_config();
    targets = benchmark_direct_sdk,
        benchmark_package_capabilities,
        benchmark_queries,
        benchmark_collaboration,
        benchmark_advanced_collaboration,
        benchmark_package,
        benchmark_resource_profiles,
        benchmark_html_adapter,
        benchmark_html_v0_adapter,
        benchmark_svg_adapter,
        benchmark_dtcg_adapter,
        benchmark_react_adapter,
        benchmark_svelte_adapter,
        benchmark_penpot_adapter,
        benchmark_figma_adapter,
        benchmark_web_semantic_adapters
}
criterion_main!(system_surfaces);
