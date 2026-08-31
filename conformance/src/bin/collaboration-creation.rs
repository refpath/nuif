use nuif_codec::{CanonicalText, Encoder};
use nuif_collab::creation::{
    CreationAnchor, CreationChange, CreationConflict, CreationError, CreationOperation,
    CreationOperationSetEngine,
};
use nuif_collab::{ChangeId, CollaborationError};
use nuif_core::{Document, Entity, EntityId, EntityKind, ShapeKind};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const ROOT: EntityId = EntityId::new(10);
const BASE_CHILD: EntityId = EntityId::new(11);

fn main() {
    if let Err(error) = run() {
        eprintln!("collaboration-creation: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let output = output_path()?;
    let base = fixture();
    let changes = fixture_changes();
    let expected = checkpoint(&base, &changes)?;
    let all_orders_converge = permutations(&base, &changes, &expected)?;
    let merge_converges = merge_converges(&base, &changes, &expected)?;
    let expected_children = vec![
        EntityId::new(20),
        EntityId::new(22),
        BASE_CHILD,
        EntityId::new(21),
    ];
    let checks = json!({
        "all_delivery_orders_converge": all_orders_converge,
        "delivery_permutations_exact": 24,
        "same_anchor_order_is_descending_dot": expected.document.entities[&ROOT].children == expected_children,
        "id_collision_explicit": expected.conflicts.iter().any(|conflict| matches!(
            conflict,
            CreationConflict::EntityIdCollision { entity, selected, .. }
                if *entity == EntityId::new(20) && *selected == ChangeId::new("dave", 1)
        )),
        "merge_converges": merge_converges,
        "metadata_absent_from_canonical_document": metadata_absent(&expected.document)?,
        "nested_creation_typed": nested_creation_typed(&base),
        "unknown_parent_typed": unknown_parent_typed(&base),
        "unknown_anchor_typed": unknown_anchor_typed(&base),
        "missing_history_typed": missing_history_typed(&base),
    });
    let passed = checks
        .as_object()
        .expect("checks is an object")
        .values()
        .all(|value| value == &Value::Bool(true) || value == &Value::Number(24.into()));
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:crdt-concurrent-creation",
        "status": if passed { "passed" } else { "failed" },
        "profile": {
            "name": nuif_collab::creation::PROFILE_NAME,
            "supported": ["concurrent leaf creation under a base parent", "Start and base-sibling anchors", "deterministic same-anchor ordering", "explicit entity-ID collision conflicts"],
            "unsupported": ["nested creation", "creation under a concurrently created parent", "deletion or resurrection", "mixed property and structural transactions", "causally stable garbage collection"],
            "limits": {
                "changes": nuif_collab::MAX_CHANGES,
                "replicas": nuif_collab::MAX_REPLICAS,
                "replica_bytes": nuif_collab::MAX_REPLICA_BYTES,
            },
        },
        "summary": {
            "changes": changes.len(),
            "delivery_permutations": 24,
            "conflicts": expected.conflicts.len(),
            "retained_entities": expected.document.entities.len(),
            "canonical_hash": expected.canonical_hash,
        },
        "checks": checks,
        "conflicts": expected.conflicts,
        "applied": expected.applied,
    });
    write_json(&output, &report)?;
    println!(
        "creation collaboration: {} delivery orders, {} conflicts, status {}",
        24, report["summary"]["conflicts"], report["status"]
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
    root.children.push(BASE_CHILD);
    document.roots.push(ROOT);
    document.entities.insert(ROOT, root);
    document.entities.insert(
        BASE_CHILD,
        Entity::new(BASE_CHILD, EntityKind::Shape(ShapeKind::Rectangle)),
    );
    document
}

fn fixture_changes() -> Vec<CreationChange> {
    vec![
        create("alice", 20, CreationAnchor::After(BASE_CHILD)),
        create("bob", 21, CreationAnchor::After(BASE_CHILD)),
        create("carol", 22, CreationAnchor::Start),
        create("dave", 20, CreationAnchor::Start),
    ]
}

fn create(replica: &str, entity: u128, anchor: CreationAnchor) -> CreationChange {
    CreationChange {
        id: ChangeId::new(replica, 1),
        context: BTreeMap::new(),
        operation: CreationOperation::Insert {
            parent: Some(ROOT),
            anchor,
            entity: Box::new(Entity::new(EntityId::new(entity), EntityKind::Container)),
        },
    }
}

fn checkpoint(
    base: &Document,
    changes: &[CreationChange],
) -> Result<nuif_collab::creation::CreationCheckpoint, String> {
    let mut engine =
        CreationOperationSetEngine::new(base.clone()).map_err(|error| error.to_string())?;
    for change in changes {
        engine
            .ingest(change.clone())
            .map_err(|error| error.to_string())?;
    }
    engine.checkpoint().map_err(|error| error.to_string())
}

fn permutations(
    base: &Document,
    changes: &[CreationChange],
    expected: &nuif_collab::creation::CreationCheckpoint,
) -> Result<bool, String> {
    let mut orders = Vec::new();
    permute(changes.to_vec(), 0, &mut orders);
    if orders.len() != 24 {
        return Ok(false);
    }
    for order in orders {
        let checkpoint = checkpoint(base, &order)?;
        if checkpoint != *expected {
            return Ok(false);
        }
    }
    Ok(true)
}

fn permute(values: Vec<CreationChange>, index: usize, output: &mut Vec<Vec<CreationChange>>) {
    if index == values.len() {
        output.push(values);
        return;
    }
    for cursor in index..values.len() {
        let mut next = values.clone();
        next.swap(index, cursor);
        permute(next, index + 1, output);
    }
}

fn merge_converges(
    base: &Document,
    changes: &[CreationChange],
    expected: &nuif_collab::creation::CreationCheckpoint,
) -> Result<bool, String> {
    let mut left =
        CreationOperationSetEngine::new(base.clone()).map_err(|error| error.to_string())?;
    let mut right =
        CreationOperationSetEngine::new(base.clone()).map_err(|error| error.to_string())?;
    for change in &changes[..2] {
        left.ingest(change.clone())
            .map_err(|error| error.to_string())?;
    }
    for change in &changes[2..] {
        right
            .ingest(change.clone())
            .map_err(|error| error.to_string())?;
    }
    left.merge(&right).map_err(|error| error.to_string())?;
    Ok(left.checkpoint().map_err(|error| error.to_string())? == *expected)
}

fn metadata_absent(document: &Document) -> Result<bool, String> {
    let bytes = CanonicalText
        .encode(document)
        .map_err(|error| error.to_string())?;
    let text = String::from_utf8(bytes).map_err(|error| error.to_string())?;
    Ok(!text.contains("alice") && !text.contains("bob") && !text.contains("carol"))
}

fn nested_creation_typed(base: &Document) -> bool {
    let mut change = create("alice", 30, CreationAnchor::Start);
    let CreationOperation::Insert { entity, .. } = &mut change.operation;
    entity.children.push(BASE_CHILD);
    matches!(
        CreationOperationSetEngine::new(base.clone()).and_then(|mut engine| engine.ingest(change)),
        Err(CreationError::NestedEntity { .. })
    )
}

fn unknown_parent_typed(base: &Document) -> bool {
    let mut change = create("alice", 30, CreationAnchor::Start);
    let CreationOperation::Insert { parent, .. } = &mut change.operation;
    *parent = Some(EntityId::new(99));
    matches!(
        CreationOperationSetEngine::new(base.clone()).and_then(|mut engine| engine.ingest(change)),
        Err(CreationError::ParentMissing { .. })
    )
}

fn unknown_anchor_typed(base: &Document) -> bool {
    let change = create("alice", 30, CreationAnchor::After(EntityId::new(99)));
    matches!(
        CreationOperationSetEngine::new(base.clone()).and_then(|mut engine| engine.ingest(change)),
        Err(CreationError::AnchorMissing { .. })
    )
}

fn missing_history_typed(base: &Document) -> bool {
    let mut change = create("alice", 30, CreationAnchor::Start);
    change.id.counter = 2;
    change.context.insert("alice".to_owned(), 1);
    let result = CreationOperationSetEngine::new(base.clone())
        .and_then(|mut engine| engine.ingest(change).map(|_| engine))
        .and_then(|engine| engine.checkpoint());
    matches!(
        result,
        Err(CreationError::Causal(
            CollaborationError::MissingReplicaChange { .. }
        ))
    )
}

fn output_path() -> Result<PathBuf, String> {
    let mut args = env::args_os().skip(1);
    let Some(argument) = args.next() else {
        return Ok(PathBuf::from("target/collaboration-creation-report.json"));
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
