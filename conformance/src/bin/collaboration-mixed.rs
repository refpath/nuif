use nuif_codec::{CanonicalText, Encoder};
use nuif_collab::mixed::{
    MixedChange, MixedCheckpoint, MixedError, MixedOperation, MixedOperationSetEngine,
};
use nuif_collab::structural::{StructuralAnchor, StructuralOperation};
use nuif_collab::{ChangeId, CollaborationError};
use nuif_core::{Document, Entity, EntityId, EntityKind};
use nuif_protocol::Operation;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const ROOT: EntityId = EntityId::new(10);
const LEFT: EntityId = EntityId::new(11);
const RIGHT: EntityId = EntityId::new(12);

fn main() {
    if let Err(error) = run() {
        eprintln!("collaboration-mixed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let output = output_path()?;
    let base = fixture();
    let changes = fixture_changes();
    let expected = checkpoint(&base, &changes)?;
    let orders = permutations(&changes);
    let all_orders_converge = orders
        .iter()
        .map(|order| checkpoint(&base, order))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .all(|checkpoint| checkpoint == expected);
    let checks = json!({
        "all_delivery_orders_converge": all_orders_converge,
        "delivery_permutations_exact": orders.len() == 24,
        "structure_materialized_before_properties": expected.document.entities[&ROOT].children == vec![RIGHT, LEFT] && expected.document.entities[&LEFT].children.is_empty(),
        "property_winner_materialized": expected.document.entities[&RIGHT].name.as_deref() == Some("second value"),
        "property_conflict_explicit": expected.property_conflicts.len() == 1,
        "structural_conflict_explicit": expected.structural_conflicts.len() == 1,
        "merge_converges": merge_converges(&base, &changes, &expected)?,
        "metadata_absent_from_canonical_document": metadata_absent(&expected.document)?,
        "deleted_property_target_typed": deleted_property_target_typed(&base),
        "cross_kind_dependency_typed": cross_kind_dependency_typed(&base),
    });
    let passed = checks
        .as_object()
        .expect("checks is an object")
        .values()
        .all(|value| value == &Value::Bool(true));
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:crdt-mixed-property-structure",
        "status": if passed { "passed" } else { "failed" },
        "profile": {
            "name": nuif_collab::mixed::PROFILE_NAME,
            "ordering": "materialize existing-tree structure, then apply property registers",
            "supported": [
                "one causal operation set carrying property and structure changes",
                "explicit property and structural conflict reports",
                "metadata-free canonical checkpoints",
            ],
            "unsupported": [
                "creation changes",
                "property edits targeting structurally removed entities",
                "mixed transaction operations under one change dot",
            ],
            "limits": {
                "changes": nuif_collab::MAX_CHANGES,
                "replicas": nuif_collab::MAX_REPLICAS,
            },
        },
        "summary": {
            "changes": changes.len(),
            "delivery_permutations": orders.len(),
            "retained_entities": expected.document.entities.len(),
            "canonical_hash": expected.canonical_hash,
            "property_conflicts": expected.property_conflicts,
            "structural_conflicts": expected.structural_conflicts,
        },
        "checks": checks,
    });
    write_json(&output, &report)?;
    println!(
        "mixed property/structure: {} delivery orders, status {}",
        orders.len(),
        report["status"]
    );
    if passed {
        Ok(())
    } else {
        Err(format!("report failed; inspect {}", output.display()))
    }
}

fn fixture() -> Document {
    let mut document = Document::empty(EntityId::new(1));
    let mut root = Entity::new(ROOT, EntityKind::Container);
    root.children.extend([LEFT, RIGHT]);
    document.roots.push(ROOT);
    document.entities.insert(ROOT, root);
    document
        .entities
        .insert(LEFT, Entity::new(LEFT, EntityKind::Container));
    document
        .entities
        .insert(RIGHT, Entity::new(RIGHT, EntityKind::Container));
    document
}

fn fixture_changes() -> Vec<MixedChange> {
    vec![
        structure_move("alice", 1, &[], Some(LEFT), StructuralAnchor::Start),
        property_rename("bob", 1, &[], "first value"),
        property_rename("carol", 1, &[], "second value"),
        structure_move("dave", 1, &[], Some(ROOT), StructuralAnchor::Start),
    ]
}

fn structure_move(
    replica: &str,
    counter: u64,
    context: &[(&str, u64)],
    new_parent: Option<EntityId>,
    anchor: StructuralAnchor,
) -> MixedChange {
    MixedChange {
        id: ChangeId::new(replica, counter),
        context: context_map(context),
        operation: MixedOperation::Structure(StructuralOperation::Move {
            entity: RIGHT,
            new_parent,
            anchor,
        }),
    }
}

fn property_rename(
    replica: &str,
    counter: u64,
    context: &[(&str, u64)],
    name: &str,
) -> MixedChange {
    MixedChange {
        id: ChangeId::new(replica, counter),
        context: context_map(context),
        operation: MixedOperation::Property(Operation::Rename {
            entity: RIGHT,
            name: Some(name.to_owned()),
        }),
    }
}

fn context_map(values: &[(&str, u64)]) -> BTreeMap<String, u64> {
    values
        .iter()
        .map(|(replica, counter)| ((*replica).to_owned(), *counter))
        .collect()
}

fn checkpoint(base: &Document, changes: &[MixedChange]) -> Result<MixedCheckpoint, String> {
    let mut engine = MixedOperationSetEngine::new(base.clone()).map_err(|e| e.to_string())?;
    for change in changes {
        engine.ingest(change.clone()).map_err(|e| e.to_string())?;
    }
    engine.checkpoint().map_err(|e| e.to_string())
}

fn permutations(changes: &[MixedChange]) -> Vec<Vec<MixedChange>> {
    fn visit(values: &mut [MixedChange], index: usize, output: &mut Vec<Vec<MixedChange>>) {
        if index == values.len() {
            output.push(values.to_vec());
            return;
        }
        for cursor in index..values.len() {
            values.swap(index, cursor);
            visit(values, index + 1, output);
            values.swap(index, cursor);
        }
    }
    let mut values = changes.to_vec();
    let mut output = Vec::new();
    visit(&mut values, 0, &mut output);
    output
}

fn merge_converges(
    base: &Document,
    changes: &[MixedChange],
    expected: &MixedCheckpoint,
) -> Result<bool, String> {
    let mut left = MixedOperationSetEngine::new(base.clone()).map_err(|e| e.to_string())?;
    let mut right = MixedOperationSetEngine::new(base.clone()).map_err(|e| e.to_string())?;
    for change in &changes[..2] {
        left.ingest(change.clone()).map_err(|e| e.to_string())?;
    }
    for change in &changes[2..] {
        right.ingest(change.clone()).map_err(|e| e.to_string())?;
    }
    left.merge(&right).map_err(|e| e.to_string())?;
    Ok(left.checkpoint().map_err(|e| e.to_string())? == *expected)
}

fn deleted_property_target_typed(base: &Document) -> bool {
    let mut engine = MixedOperationSetEngine::new(base.clone()).unwrap();
    engine
        .ingest(MixedChange {
            id: ChangeId::new("alice", 1),
            context: BTreeMap::new(),
            operation: MixedOperation::Structure(StructuralOperation::Delete { entity: RIGHT }),
        })
        .unwrap();
    engine
        .ingest(property_rename("bob", 1, &[], "unavailable"))
        .unwrap();
    matches!(
        engine.checkpoint(),
        Err(MixedError::PropertyTargetUnavailable { entity, .. }) if entity == RIGHT
    )
}

fn cross_kind_dependency_typed(base: &Document) -> bool {
    let mut engine = MixedOperationSetEngine::new(base.clone()).unwrap();
    engine
        .ingest(MixedChange {
            id: ChangeId::new("bob", 1),
            context: BTreeMap::from([("alice".to_owned(), 1)]),
            operation: MixedOperation::Property(Operation::Rename {
                entity: RIGHT,
                name: Some("missing causal structure".to_owned()),
            }),
        })
        .unwrap();
    matches!(
        engine.checkpoint(),
        Err(MixedError::Causal(
            CollaborationError::MissingDependency { .. }
        ))
    )
}

fn metadata_absent(document: &Document) -> Result<bool, String> {
    let encoded = CanonicalText
        .encode(document)
        .map_err(|error| error.to_string())?;
    let text = String::from_utf8(encoded).map_err(|error| error.to_string())?;
    Ok(![
        "alice",
        "bob",
        "carol",
        "dave",
        "counter",
        "replica",
        "property_conflicts",
    ]
    .iter()
    .any(|marker| text.contains(marker)))
}

fn output_path() -> Result<PathBuf, String> {
    let mut args = env::args_os().skip(1);
    let Some(argument) = args.next() else {
        return Ok(PathBuf::from("target/collaboration-mixed-report.json"));
    };
    if argument == "--output" {
        return args
            .next()
            .map(PathBuf::from)
            .ok_or_else(|| "--output requires a path".to_owned());
    }
    Err(format!("unknown argument {}", argument.to_string_lossy()))
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    fs::write(path, bytes).map_err(|error| error.to_string())
}
