use nuif_codec::{CanonicalText, Encoder};
use nuif_collab::creation::{CreationAnchor, CreationChange, CreationOperation};
use nuif_collab::nested_creation::{
    ArbitraryAnchorCreationOperationSetEngine, NestedCreationCheckpoint, NestedCreationError,
};
use nuif_collab::{ChangeId, CollaborationError};
use nuif_core::{Document, Entity, EntityId, EntityKind};
use serde_json::{Value, json};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const ROOT: EntityId = EntityId::new(10);
const BASE_CHILD: EntityId = EntityId::new(11);

fn main() {
    if let Err(error) = run() {
        eprintln!("collaboration-nested-creation-v1: {error}");
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
        "created_parent_sibling_anchor_materialized": expected.document.entities[&EntityId::new(20)].children == vec![EntityId::new(22), EntityId::new(23)],
        "created_base_parent_anchor_materialized": expected.document.entities[&ROOT].children == vec![EntityId::new(20), EntityId::new(21), BASE_CHILD],
        "merge_converges": merge_converges(&base, &changes, &expected)?,
        "metadata_absent_from_canonical_document": metadata_absent(&expected.document)?,
        "noncausal_anchor_typed": noncausal_anchor_typed(&base),
        "unknown_anchor_typed": unknown_anchor_typed(&base),
        "wrong_parent_anchor_typed": wrong_parent_anchor_typed(&base),
        "incomplete_history_typed": incomplete_history_typed(&base),
    });
    let passed = checks
        .as_object()
        .expect("checks is an object")
        .values()
        .all(|value| value == &Value::Bool(true));
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:crdt-nested-creation-arbitrary-anchor",
        "status": if passed { "passed" } else { "failed" },
        "profile": {
            "name": nuif_collab::nested_creation::ARBITRARY_ANCHOR_PROFILE_NAME,
            "supported": [
                "causal child creation under a selected created parent",
                "Start and After anchors under base or selected created parents",
                "causal witness requirements for created sibling anchors",
                "deterministic entity-ID collision reporting",
            ],
            "unsupported": [
                "deletion or resurrection",
                "mixed property and structural transactions",
                "anchors to non-selected collision candidates",
            ],
            "limits": {
                "changes": nuif_collab::MAX_CHANGES,
                "replicas": nuif_collab::MAX_REPLICAS,
                "parent_depth": nuif_collab::nested_creation::MAX_PARENT_DEPTH,
            },
        },
        "summary": {
            "changes": changes.len(),
            "delivery_permutations": orders.len(),
            "retained_entities": expected.document.entities.len(),
            "canonical_hash": expected.canonical_hash,
            "conflicts": expected.conflicts,
        },
        "checks": checks,
    });
    write_json(&output, &report)?;
    println!(
        "nested creation v1: {} delivery orders, status {}",
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
    root.children.push(BASE_CHILD);
    document.roots.push(ROOT);
    document.entities.insert(ROOT, root);
    document
        .entities
        .insert(BASE_CHILD, Entity::new(BASE_CHILD, EntityKind::Container));
    document
}

fn fixture_changes() -> Vec<CreationChange> {
    vec![
        create("alice", 1, &[], 20, Some(ROOT.0), CreationAnchor::Start),
        create(
            "bob",
            1,
            &[("alice", 1)],
            21,
            Some(ROOT.0),
            CreationAnchor::After(EntityId::new(20)),
        ),
        create(
            "carol",
            1,
            &[("alice", 1)],
            22,
            Some(20),
            CreationAnchor::Start,
        ),
        create(
            "dave",
            1,
            &[("alice", 1), ("carol", 1)],
            23,
            Some(20),
            CreationAnchor::After(EntityId::new(22)),
        ),
    ]
}

fn create(
    replica: &str,
    counter: u64,
    context: &[(&str, u64)],
    entity: u128,
    parent: Option<u128>,
    anchor: CreationAnchor,
) -> CreationChange {
    CreationChange {
        id: ChangeId::new(replica, counter),
        context: context
            .iter()
            .map(|(replica, counter)| ((*replica).to_owned(), *counter))
            .collect(),
        operation: CreationOperation::Insert {
            parent: parent.map(EntityId::new),
            anchor,
            entity: Box::new(Entity::new(EntityId::new(entity), EntityKind::Container)),
        },
    }
}

fn checkpoint(
    base: &Document,
    changes: &[CreationChange],
) -> Result<NestedCreationCheckpoint, String> {
    let mut engine = ArbitraryAnchorCreationOperationSetEngine::new(base.clone())
        .map_err(|error| error.to_string())?;
    for change in changes {
        engine
            .ingest(change.clone())
            .map_err(|error| error.to_string())?;
    }
    engine.checkpoint().map_err(|error| error.to_string())
}

fn permutations(changes: &[CreationChange]) -> Vec<Vec<CreationChange>> {
    fn visit(values: &mut [CreationChange], index: usize, output: &mut Vec<Vec<CreationChange>>) {
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
    changes: &[CreationChange],
    expected: &NestedCreationCheckpoint,
) -> Result<bool, String> {
    let mut left = ArbitraryAnchorCreationOperationSetEngine::new(base.clone())
        .map_err(|error| error.to_string())?;
    let mut right = ArbitraryAnchorCreationOperationSetEngine::new(base.clone())
        .map_err(|error| error.to_string())?;
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

fn noncausal_anchor_typed(base: &Document) -> bool {
    let mut engine = ArbitraryAnchorCreationOperationSetEngine::new(base.clone()).unwrap();
    engine
        .ingest(create(
            "alice",
            1,
            &[],
            20,
            Some(ROOT.0),
            CreationAnchor::Start,
        ))
        .unwrap();
    engine
        .ingest(create(
            "carol",
            1,
            &[("alice", 1)],
            22,
            Some(20),
            CreationAnchor::Start,
        ))
        .unwrap();
    engine
        .ingest(create(
            "dave",
            1,
            &[("alice", 1)],
            23,
            Some(20),
            CreationAnchor::After(EntityId::new(22)),
        ))
        .unwrap();
    matches!(
        engine.checkpoint(),
        Err(NestedCreationError::AnchorNotCausal { anchor, .. })
            if anchor == EntityId::new(22)
    )
}

fn unknown_anchor_typed(base: &Document) -> bool {
    let mut engine = ArbitraryAnchorCreationOperationSetEngine::new(base.clone()).unwrap();
    engine
        .ingest(create(
            "alice",
            1,
            &[],
            20,
            Some(ROOT.0),
            CreationAnchor::After(EntityId::new(99)),
        ))
        .unwrap();
    matches!(
        engine.checkpoint(),
        Err(NestedCreationError::AnchorUnavailable { anchor, .. })
            if anchor == EntityId::new(99)
    )
}

fn wrong_parent_anchor_typed(base: &Document) -> bool {
    let mut engine = ArbitraryAnchorCreationOperationSetEngine::new(base.clone()).unwrap();
    engine
        .ingest(create(
            "alice",
            1,
            &[],
            20,
            Some(ROOT.0),
            CreationAnchor::Start,
        ))
        .unwrap();
    engine
        .ingest(create(
            "bob",
            1,
            &[("alice", 1)],
            21,
            Some(20),
            CreationAnchor::After(BASE_CHILD),
        ))
        .unwrap();
    matches!(
        engine.checkpoint(),
        Err(NestedCreationError::AnchorUnavailable { anchor, .. })
            if anchor == BASE_CHILD
    )
}

fn incomplete_history_typed(base: &Document) -> bool {
    let mut engine = ArbitraryAnchorCreationOperationSetEngine::new(base.clone()).unwrap();
    let change = create(
        "alice",
        2,
        &[("alice", 1)],
        20,
        Some(ROOT.0),
        CreationAnchor::Start,
    );
    engine.ingest(change).unwrap();
    matches!(
        engine.checkpoint(),
        Err(NestedCreationError::Causal(
            CollaborationError::MissingReplicaChange { .. }
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
        "active_positions",
    ]
    .iter()
    .any(|marker| text.contains(marker)))
}

fn output_path() -> Result<PathBuf, String> {
    let mut args = env::args_os().skip(1);
    let Some(argument) = args.next() else {
        return Ok(PathBuf::from(
            "target/collaboration-nested-creation-v1-report.json",
        ));
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
