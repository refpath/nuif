use nuif_codec::{CanonicalText, Encoder, canonical_hash};
use nuif_collab::structural::{
    PositionId, ResumedStructuralOperationSetEngine, StructuralAnchor, StructuralChange,
    StructuralCheckpoint, StructuralConflict, StructuralError, StructuralOperation,
    StructuralOperationSetEngine, StructuralUndoRedoEngine,
};
use nuif_collab::{ChangeId, CollaborationError};
use nuif_core::{Document, Entity, EntityId, EntityKind, Severity, validate};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

const ROOT: EntityId = EntityId::new(10);
const LEFT: EntityId = EntityId::new(11);
const RIGHT: EntityId = EntityId::new(12);
const LEAF: EntityId = EntityId::new(13);
const SCALE_CHANGES: u64 = 4_096;

fn main() {
    if let Err(error) = run() {
        eprintln!("collaboration-structure: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let (report_path, oracle_path) = output_paths()?;
    let base = fixture();
    let changes = fixture_changes();
    let expected = checkpoints_for_order(&base, &changes)?;
    let (permutations, all_orders_converge) = exhaustive_deliveries(&base, &changes, &expected)?;
    let scale = scaling_trial()?;
    let prefix = structural_prefix_trial()?;
    let checks = json!({
        "two_materializers_exact": expected.0 == expected.1,
        "all_delivery_orders_converge": all_orders_converge,
        "delivery_permutations_exact": permutations == 5040,
        "merge_commutative_associative": merge_orders_converge(&base, &changes, &expected.0)?,
        "duplicate_delivery_idempotent": duplicate_delivery_is_idempotent(&base, &changes, &expected.0)?,
        "one_parent_and_acyclic": valid_structure(&expected.0.document),
        "cycle_conflict_explicit": has_conflict(&expected.0, "cycle"),
        "concurrent_move_explicit": has_conflict(&expected.0, "move"),
        "delete_move_explicit": has_conflict(&expected.0, "delete_move"),
        "deleted_parent_explicit": has_conflict(&expected.0, "deleted_parent"),
        "delete_descendant_move_explicit": has_conflict(&expected.0, "delete_descendant"),
        "metadata_absent_from_canonical_document": collaboration_metadata_absent(&expected.0.document)?,
        "missing_history_typed": missing_history_typed(&base),
        "duplicate_identifier_typed": duplicate_identifier_typed(),
        "unknown_parent_typed": unknown_parent_typed(&base),
        "missing_anchor_change_typed": missing_anchor_change_typed(&base),
        "noncausal_anchor_typed": noncausal_anchor_typed(&base),
        "wrong_parent_anchor_explicit": wrong_parent_anchor_explicit(&base),
        "causal_change_anchor_applied": expected.0.active_positions.get(&LEFT)
            == Some(&PositionId::Change(ChangeId::new("gina", 1))),
        "different_base_merge_typed": different_base_merge_typed(&base),
        "scale_materializers_exact": scale["exact"] == Value::Bool(true),
        "scale_catastrophic_budget": scale["elapsed_millis"].as_u64().is_some_and(|millis| millis < 10_000),
        "structural_prefix_compaction": prefix["status"] == Value::String("passed".to_owned()),
    });
    let passed = checks
        .as_object()
        .expect("checks is an object")
        .values()
        .all(|value| value == &Value::Bool(true));
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:crdt-structural-checkpoint",
        "status": if passed { "passed" } else { "failed" },
        "source": source_metadata(),
        "profile": {
            "name": nuif_collab::structural::PROFILE_NAME,
            "engines": ["sorted operation set", "incremental rollback and replay"],
            "tree_algorithm": "unique Lamport order with cycle-rejected move replay",
            "sibling_algorithm": "RGA-style stable origins, descending same-origin identifiers, retained tombstones",
            "supported": ["move existing entity", "reorder existing entity", "delete as move to profile trash", "resurrection by later move"],
            "partial_profile": "nuif-collab-tree-prefix-0",
            "unsupported": ["concurrent entity creation", "structural collection with inactive anchors", "combined property and structural transactions"],
            "limits": {
                "changes": nuif_collab::MAX_CHANGES,
                "replicas": nuif_collab::MAX_REPLICAS,
                "replica_bytes": nuif_collab::MAX_REPLICA_BYTES,
            },
        },
        "summary": {
            "changes": changes.len(),
            "delivery_permutations": permutations,
            "conflicts": expected.0.conflicts.len(),
            "retained_entities": expected.0.document.entities.len(),
            "canonical_hash": expected.0.canonical_hash,
            "blocking_failures": u8::from(!passed),
        },
        "checks": checks,
        "scaling": scale,
        "structural_prefix": prefix,
        "conflicts": expected.0.conflicts,
        "applied": expected.0.applied,
        "foreign_oracle": {
            "engine": "@automerge/automerge",
            "role": "convergent operation-set transport; not the tree materializer",
            "input": oracle_path,
            "claim_boundary": "Exact operation records after foreign merges imply the same deterministic NUIF checkpoint; Automerge is not credited with NUIF cycle or deletion semantics."
        }
    });
    write_json(&report_path, &report)?;
    write_oracle_input(&oracle_path, &base, &changes, &expected.0)?;
    println!(
        "structural collaboration: {permutations} delivery orders, {} conflicts, {} scale changes, status {}",
        report["summary"]["conflicts"], SCALE_CHANGES, report["status"]
    );
    if passed {
        Ok(())
    } else {
        Err(format!("report failed; inspect {}", report_path.display()))
    }
}

fn fixture() -> Document {
    let mut document = Document::empty(EntityId::new(1));
    let mut root = Entity::new(ROOT, EntityKind::Container);
    root.children.extend([LEFT, RIGHT]);
    let mut left = Entity::new(LEFT, EntityKind::Container);
    left.children.push(LEAF);
    document.roots.push(ROOT);
    document.entities.insert(ROOT, root);
    document.entities.insert(LEFT, left);
    document
        .entities
        .insert(RIGHT, Entity::new(RIGHT, EntityKind::Container));
    document
        .entities
        .insert(LEAF, Entity::new(LEAF, EntityKind::Container));
    document
}

fn fixture_changes() -> Vec<StructuralChange> {
    vec![
        movement("alice", LEAF, Some(RIGHT), StructuralAnchor::Start),
        movement("bob", LEAF, None, StructuralAnchor::Start),
        deletion("carol", LEFT),
        movement("dave", LEFT, Some(RIGHT), StructuralAnchor::Start),
        movement("erin", RIGHT, Some(LEFT), StructuralAnchor::Start),
        deletion("frank", RIGHT),
        change(
            "gina",
            1,
            BTreeMap::from([("bob".to_owned(), 1)]),
            StructuralOperation::Move {
                entity: LEFT,
                new_parent: None,
                anchor: StructuralAnchor::After(PositionId::Change(ChangeId::new("bob", 1))),
            },
        ),
    ]
}

fn movement(
    replica: &str,
    entity: EntityId,
    new_parent: Option<EntityId>,
    anchor: StructuralAnchor,
) -> StructuralChange {
    change(
        replica,
        1,
        BTreeMap::new(),
        StructuralOperation::Move {
            entity,
            new_parent,
            anchor,
        },
    )
}

fn deletion(replica: &str, entity: EntityId) -> StructuralChange {
    change(
        replica,
        1,
        BTreeMap::new(),
        StructuralOperation::Delete { entity },
    )
}

fn change(
    replica: &str,
    counter: u64,
    context: BTreeMap<String, u64>,
    operation: StructuralOperation,
) -> StructuralChange {
    StructuralChange {
        id: ChangeId::new(replica, counter),
        context,
        operation,
    }
}

fn checkpoints_for_order(
    base: &Document,
    changes: &[StructuralChange],
) -> Result<(StructuralCheckpoint, StructuralCheckpoint), String> {
    let mut set =
        StructuralOperationSetEngine::new(base.clone()).map_err(|error| error.to_string())?;
    let mut incremental =
        StructuralUndoRedoEngine::new(base.clone()).map_err(|error| error.to_string())?;
    for item in changes {
        set.ingest(item.clone())
            .map_err(|error| error.to_string())?;
        incremental
            .ingest(item.clone())
            .map_err(|error| error.to_string())?;
    }
    Ok((
        set.checkpoint().map_err(|error| error.to_string())?,
        incremental
            .checkpoint()
            .map_err(|error| error.to_string())?,
    ))
}

fn exhaustive_deliveries(
    base: &Document,
    changes: &[StructuralChange],
    expected: &(StructuralCheckpoint, StructuralCheckpoint),
) -> Result<(usize, bool), String> {
    fn visit(
        base: &Document,
        changes: &mut [StructuralChange],
        index: usize,
        expected: &(StructuralCheckpoint, StructuralCheckpoint),
        count: &mut usize,
        converged: &mut bool,
    ) -> Result<(), String> {
        if index == changes.len() {
            *count += 1;
            *converged &= checkpoints_for_order(base, changes)? == *expected;
            return Ok(());
        }
        for cursor in index..changes.len() {
            changes.swap(index, cursor);
            visit(base, changes, index + 1, expected, count, converged)?;
            changes.swap(index, cursor);
        }
        Ok(())
    }
    let mut ordered = changes.to_vec();
    let mut count = 0;
    let mut converged = true;
    visit(base, &mut ordered, 0, expected, &mut count, &mut converged)?;
    Ok((count, converged))
}

fn merge_orders_converge(
    base: &Document,
    changes: &[StructuralChange],
    expected: &StructuralCheckpoint,
) -> Result<bool, String> {
    let mut sets = [
        StructuralOperationSetEngine::new(base.clone()).map_err(|error| error.to_string())?,
        StructuralOperationSetEngine::new(base.clone()).map_err(|error| error.to_string())?,
        StructuralOperationSetEngine::new(base.clone()).map_err(|error| error.to_string())?,
    ];
    let mut incrementals = [
        StructuralUndoRedoEngine::new(base.clone()).map_err(|error| error.to_string())?,
        StructuralUndoRedoEngine::new(base.clone()).map_err(|error| error.to_string())?,
        StructuralUndoRedoEngine::new(base.clone()).map_err(|error| error.to_string())?,
    ];
    for (index, item) in changes.iter().enumerate() {
        sets[index % 3]
            .ingest(item.clone())
            .map_err(|error| error.to_string())?;
        incrementals[index % 3]
            .ingest(item.clone())
            .map_err(|error| error.to_string())?;
    }
    let mut set_left = sets[0].clone();
    set_left
        .merge(&sets[1])
        .map_err(|error| error.to_string())?;
    set_left
        .merge(&sets[2])
        .map_err(|error| error.to_string())?;
    let mut set_right = sets[2].clone();
    set_right
        .merge(&sets[0])
        .map_err(|error| error.to_string())?;
    set_right
        .merge(&sets[1])
        .map_err(|error| error.to_string())?;
    let mut incremental_left = incrementals[0].clone();
    incremental_left
        .merge(&incrementals[1])
        .map_err(|error| error.to_string())?;
    incremental_left
        .merge(&incrementals[2])
        .map_err(|error| error.to_string())?;
    let mut incremental_right = incrementals[2].clone();
    incremental_right
        .merge(&incrementals[0])
        .map_err(|error| error.to_string())?;
    incremental_right
        .merge(&incrementals[1])
        .map_err(|error| error.to_string())?;
    Ok(set_left.checkpoint().as_ref() == Ok(expected)
        && set_right.checkpoint().as_ref() == Ok(expected)
        && incremental_left.checkpoint().as_ref() == Ok(expected)
        && incremental_right.checkpoint().as_ref() == Ok(expected))
}

fn duplicate_delivery_is_idempotent(
    base: &Document,
    changes: &[StructuralChange],
    expected: &StructuralCheckpoint,
) -> Result<bool, String> {
    let mut set =
        StructuralOperationSetEngine::new(base.clone()).map_err(|error| error.to_string())?;
    let mut incremental =
        StructuralUndoRedoEngine::new(base.clone()).map_err(|error| error.to_string())?;
    for item in changes.iter().chain(changes.iter().rev()) {
        set.ingest(item.clone())
            .map_err(|error| error.to_string())?;
        incremental
            .ingest(item.clone())
            .map_err(|error| error.to_string())?;
    }
    Ok(set.checkpoint().as_ref() == Ok(expected)
        && incremental.checkpoint().as_ref() == Ok(expected))
}

fn structural_prefix_trial() -> Result<Value, String> {
    let base = fixture();
    let stable = change(
        "prefix",
        1,
        BTreeMap::new(),
        StructuralOperation::Move {
            entity: LEFT,
            new_parent: None,
            anchor: StructuralAnchor::Start,
        },
    );
    let retained = change(
        "prefix",
        2,
        BTreeMap::from([("prefix".to_owned(), 1)]),
        StructuralOperation::Move {
            entity: LEAF,
            new_parent: None,
            anchor: StructuralAnchor::After(PositionId::Change(ChangeId::new("prefix", 1))),
        },
    );
    let mut engine =
        StructuralOperationSetEngine::new(base.clone()).map_err(|error| error.to_string())?;
    engine
        .ingest(stable.clone())
        .map_err(|error| error.to_string())?;
    engine
        .ingest(retained.clone())
        .map_err(|error| error.to_string())?;
    let full = engine.checkpoint().map_err(|error| error.to_string())?;
    let frontier =
        nuif_collab::gc::StabilityFrontier::new(BTreeMap::from([("prefix".to_owned(), 1)]))
            .map_err(|error| error.to_string())?;
    let compacted = engine
        .compact_stable_prefix(&frontier)
        .map_err(|error| error.to_string())?;
    let mut resumed = ResumedStructuralOperationSetEngine::new(compacted.base.clone())
        .map_err(|error| error.to_string())?;
    resumed
        .ingest(retained.clone())
        .map_err(|error| error.to_string())?;
    let resumed_checkpoint = resumed.checkpoint().map_err(|error| error.to_string())?;

    let inactive_anchor_typed = inactive_anchor_trial()?;
    let checks = json!({
        "canonical_document_exact": compacted.checkpoint.document == full.document,
        "canonical_hash_exact": compacted.checkpoint.canonical_hash == full.canonical_hash,
        "conflicts_exact": compacted.checkpoint.conflicts == full.conflicts
            && resumed_checkpoint.conflicts == full.conflicts,
        "receipt_profile_exact": compacted.receipt.profile == nuif_collab::structural::PARTIAL_PROFILE_NAME,
        "dropped_retained_exact": compacted.receipt.dropped == vec![ChangeId::new("prefix", 1)]
            && compacted.receipt.retained == vec![ChangeId::new("prefix", 2)],
        "active_anchor_rebound": compacted.base.checkpoint.active_positions.get(&LEFT)
            == Some(&PositionId::Change(ChangeId::new("prefix", 1))),
        "resumed_exact": resumed_checkpoint.document == full.document
            && resumed_checkpoint.canonical_hash == full.canonical_hash,
        "inactive_anchor_typed": inactive_anchor_typed,
    });
    let passed = checks
        .as_object()
        .expect("checks object")
        .values()
        .all(|value| value == &Value::Bool(true));
    Ok(json!({
        "schema_version": 1,
        "profile": nuif_collab::structural::PARTIAL_PROFILE_NAME,
        "status": if passed { "passed" } else { "failed" },
        "checks": checks,
        "summary": { "dropped": compacted.receipt.dropped.len(), "retained": compacted.receipt.retained.len() },
    }))
}

fn inactive_anchor_trial() -> Result<bool, String> {
    let base = fixture();
    let mut engine = StructuralOperationSetEngine::new(base).map_err(|error| error.to_string())?;
    let changes = vec![
        change(
            "prefix",
            1,
            BTreeMap::new(),
            StructuralOperation::Move {
                entity: LEFT,
                new_parent: None,
                anchor: StructuralAnchor::Start,
            },
        ),
        change(
            "prefix",
            2,
            BTreeMap::from([("prefix".to_owned(), 1)]),
            StructuralOperation::Move {
                entity: LEFT,
                new_parent: Some(RIGHT),
                anchor: StructuralAnchor::Start,
            },
        ),
        change(
            "prefix",
            3,
            BTreeMap::from([("prefix".to_owned(), 2)]),
            StructuralOperation::Move {
                entity: LEAF,
                new_parent: None,
                anchor: StructuralAnchor::After(PositionId::Change(ChangeId::new("prefix", 1))),
            },
        ),
    ];
    for operation in changes {
        engine
            .ingest(operation)
            .map_err(|error| error.to_string())?;
    }
    let frontier =
        nuif_collab::gc::StabilityFrontier::new(BTreeMap::from([("prefix".to_owned(), 2)]))
            .map_err(|error| error.to_string())?;
    Ok(matches!(
        engine.compact_stable_prefix(&frontier),
        Err(StructuralError::StableAnchorNotRepresentable { .. })
    ))
}

fn valid_structure(document: &Document) -> bool {
    validate(document)
        .iter()
        .all(|diagnostic| diagnostic.severity != Severity::Error)
}

fn has_conflict(checkpoint: &StructuralCheckpoint, kind: &str) -> bool {
    checkpoint.conflicts.iter().any(|conflict| {
        matches!(
            (kind, conflict),
            ("cycle", StructuralConflict::CycleRejected { .. })
                | ("move", StructuralConflict::ConcurrentMove { .. })
                | ("delete_move", StructuralConflict::DeleteMove { .. })
                | ("deleted_parent", StructuralConflict::DeletedParent { .. })
                | (
                    "delete_descendant",
                    StructuralConflict::DeleteDescendantMove { .. }
                )
        )
    })
}

fn collaboration_metadata_absent(document: &Document) -> Result<bool, String> {
    let encoded = CanonicalText
        .encode(document)
        .map_err(|error| error.to_string())?;
    let text = String::from_utf8(encoded).map_err(|error| error.to_string())?;
    Ok(![
        "alice",
        "bob",
        "carol",
        "\"counter\"",
        "\"replica\"",
        "active_positions",
        "trash",
        "tombstone",
    ]
    .iter()
    .any(|marker| text.contains(marker)))
}

fn missing_history_typed(base: &Document) -> bool {
    let Ok(mut engine) = StructuralOperationSetEngine::new(base.clone()) else {
        return false;
    };
    engine
        .ingest(change(
            "alice",
            2,
            BTreeMap::from([("alice".to_owned(), 1)]),
            StructuralOperation::Delete { entity: LEAF },
        ))
        .is_ok()
        && matches!(
            engine.checkpoint(),
            Err(StructuralError::Causal(
                CollaborationError::MissingReplicaChange { .. }
            ))
        )
}

fn duplicate_identifier_typed() -> bool {
    let first = movement("alice", LEAF, Some(RIGHT), StructuralAnchor::Start);
    let mut second = first.clone();
    second.operation = StructuralOperation::Delete { entity: LEAF };
    let Ok(mut engine) = StructuralOperationSetEngine::new(fixture()) else {
        return false;
    };
    engine.ingest(first).is_ok()
        && matches!(
            engine.ingest(second),
            Err(StructuralError::DuplicateChange { .. })
        )
}

fn unknown_parent_typed(base: &Document) -> bool {
    let Ok(mut engine) = StructuralOperationSetEngine::new(base.clone()) else {
        return false;
    };
    matches!(
        engine.ingest(movement(
            "alice",
            LEAF,
            Some(EntityId::new(u128::MAX)),
            StructuralAnchor::Start,
        )),
        Err(StructuralError::ParentMissing { .. })
    )
}

fn missing_anchor_change_typed(base: &Document) -> bool {
    let Ok(mut engine) = StructuralOperationSetEngine::new(base.clone()) else {
        return false;
    };
    let missing = PositionId::Change(ChangeId::new("missing", 1));
    matches!(
        engine.ingest(movement(
            "alice",
            LEAF,
            Some(RIGHT),
            StructuralAnchor::After(missing),
        )),
        Ok(true)
    ) && matches!(
        engine.checkpoint(),
        Err(StructuralError::AnchorChangeMissing { .. })
    )
}

fn noncausal_anchor_typed(base: &Document) -> bool {
    let Ok(mut engine) = StructuralOperationSetEngine::new(base.clone()) else {
        return false;
    };
    let origin = movement("alice", LEFT, Some(RIGHT), StructuralAnchor::Start);
    let dependent = movement(
        "bob",
        LEAF,
        Some(RIGHT),
        StructuralAnchor::After(PositionId::Change(origin.id.clone())),
    );
    engine.ingest(origin).is_ok()
        && engine.ingest(dependent).is_ok()
        && matches!(
            engine.checkpoint(),
            Err(StructuralError::AnchorNotCausal { .. })
        )
}

fn wrong_parent_anchor_explicit(base: &Document) -> bool {
    let Ok(mut engine) = StructuralOperationSetEngine::new(base.clone()) else {
        return false;
    };
    if engine
        .ingest(movement(
            "alice",
            LEAF,
            Some(RIGHT),
            StructuralAnchor::After(PositionId::Base(LEFT)),
        ))
        .is_err()
    {
        return false;
    }
    engine.checkpoint().is_ok_and(|checkpoint| {
        checkpoint
            .conflicts
            .iter()
            .any(|conflict| matches!(conflict, StructuralConflict::AnchorUnavailable { .. }))
    })
}

fn different_base_merge_typed(base: &Document) -> bool {
    let Ok(mut left) = StructuralUndoRedoEngine::new(base.clone()) else {
        return false;
    };
    let mut other = base.clone();
    other.id = EntityId::new(2);
    let Ok(right) = StructuralUndoRedoEngine::new(other.clone()) else {
        return false;
    };
    let incremental = matches!(
        left.merge(&right),
        Err(StructuralError::BaseMismatch { .. })
    );
    let Ok(mut left) = StructuralOperationSetEngine::new(base.clone()) else {
        return false;
    };
    let Ok(right) = StructuralOperationSetEngine::new(other) else {
        return false;
    };
    incremental
        && matches!(
            left.merge(&right),
            Err(StructuralError::BaseMismatch { .. })
        )
}

fn scaling_trial() -> Result<Value, String> {
    let mut base = Document::empty(EntityId::new(0x1000));
    let root = EntityId::new(0x1001);
    let mut root_entity = Entity::new(root, EntityKind::Container);
    for index in 0..SCALE_CHANGES {
        let child = EntityId::new(0x2000 + u128::from(index));
        root_entity.children.push(child);
        base.entities
            .insert(child, Entity::new(child, EntityKind::Container));
    }
    base.roots.push(root);
    base.entities.insert(root, root_entity);
    let started = Instant::now();
    let mut set =
        StructuralOperationSetEngine::new(base.clone()).map_err(|error| error.to_string())?;
    let mut incremental =
        StructuralUndoRedoEngine::new(base.clone()).map_err(|error| error.to_string())?;
    for counter in 1..=SCALE_CHANGES {
        let child = EntityId::new(0x2000 + u128::from(counter - 1));
        let context = if counter > 1 {
            BTreeMap::from([("scale".to_owned(), counter - 1)])
        } else {
            BTreeMap::new()
        };
        let item = change(
            "scale",
            counter,
            context,
            StructuralOperation::Move {
                entity: child,
                new_parent: Some(root),
                anchor: StructuralAnchor::Start,
            },
        );
        set.ingest(item.clone())
            .map_err(|error| error.to_string())?;
        incremental
            .ingest(item)
            .map_err(|error| error.to_string())?;
    }
    let left = set.checkpoint().map_err(|error| error.to_string())?;
    let right = incremental
        .checkpoint()
        .map_err(|error| error.to_string())?;
    Ok(json!({
        "changes": SCALE_CHANGES,
        "entities": base.entities.len(),
        "exact": left == right,
        "conflicts": left.conflicts.len(),
        "elapsed_millis": u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        "canonical_hash": left.canonical_hash,
    }))
}

fn write_oracle_input(
    path: &Path,
    base: &Document,
    changes: &[StructuralChange],
    expected: &StructuralCheckpoint,
) -> Result<(), String> {
    let mut replicas = BTreeMap::<String, Vec<&StructuralChange>>::new();
    for item in changes {
        replicas
            .entry(item.id.replica.clone())
            .or_default()
            .push(item);
    }
    write_json(
        path,
        &json!({
            "schema_version": 1,
            "profile": nuif_collab::structural::PROFILE_NAME,
            "base_canonical_hash": canonical_hash(base).map_err(|error| error.to_string())?,
            "expected_canonical_hash": expected.canonical_hash,
            "base_document": base,
            "expected_tree": {
                "roots": expected.document.roots,
                "children": expected.document.entities.iter().map(|(entity, value)| {
                    (entity.to_string(), value.children.clone())
                }).collect::<BTreeMap<_, _>>(),
                "active_positions": expected.active_positions,
                "replay_conflicts": expected.conflicts.iter().filter(|conflict| matches!(
                    conflict,
                    StructuralConflict::CycleRejected { .. }
                        | StructuralConflict::AnchorUnavailable { .. }
                        | StructuralConflict::SelfAnchor { .. }
                )).collect::<Vec<_>>(),
            },
            "replicas": replicas,
            "expected_changes": changes,
        }),
    )
}

fn output_paths() -> Result<(PathBuf, PathBuf), String> {
    let mut args = env::args().skip(1);
    let mut report = PathBuf::from("target/collaboration-structure-report.json");
    let mut oracle = PathBuf::from("target/collaboration-automerge-input.json");
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--output" => report = PathBuf::from(args.next().ok_or("--output requires a path")?),
            "--oracle-input" => {
                oracle = PathBuf::from(args.next().ok_or("--oracle-input requires a path")?);
            }
            _ => return Err(format!("unknown argument {argument}")),
        }
    }
    Ok((report, oracle))
}

fn source_metadata() -> Value {
    json!({
        "revision": command_text("git", &["rev-parse", "HEAD"]),
        "dirty": command_text("git", &["status", "--porcelain"]).map(|value| !value.is_empty()),
        "toolchain": command_text("rustc", &["--version"]),
        "os": env::consts::OS,
        "architecture": env::consts::ARCH,
    })
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    fs::write(path, bytes).map_err(|error| error.to_string())
}

fn command_text(program: &str, arguments: &[&str]) -> Option<String> {
    let output = Command::new(program).args(arguments).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
