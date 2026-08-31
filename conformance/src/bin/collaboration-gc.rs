use nuif_codec::{CanonicalText, Encoder};
use nuif_collab::gc::StabilityFrontier;
use nuif_collab::structural::{
    StructuralAnchor, StructuralChange, StructuralOperation, StructuralOperationSetEngine,
};
use nuif_collab::{Change, ChangeId, CollaborationError, OperationSetEngine, ReplicaLogEngine};
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
        eprintln!("collaboration-gc: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let output = output_path()?;
    let register_base = nuif_testing::responsive_card_fixture();
    let register_changes = vec![
        register_change("alice", 1, "Alice"),
        register_change("bob", 1, "Bob"),
    ];
    let register_frontier = frontier(&[("alice", 1), ("bob", 1)])?;
    let register_before = register_checkpoint(&register_base, &register_changes)?;
    let register_compacted =
        register_compaction(&register_base, &register_changes, &register_frontier)
            .map_err(|error| error.to_string())?;
    let register_replica_compacted =
        replica_compaction(&register_base, &register_changes, &register_frontier)?;

    let structural_base = structural_fixture();
    let structural_change = StructuralChange {
        id: ChangeId::new("alice", 1),
        context: BTreeMap::new(),
        operation: StructuralOperation::Move {
            entity: RIGHT,
            new_parent: Some(LEFT),
            anchor: StructuralAnchor::Start,
        },
    };
    let mut structural_engine =
        StructuralOperationSetEngine::new(structural_base).map_err(|e| e.to_string())?;
    structural_engine
        .ingest(structural_change)
        .map_err(|e| e.to_string())?;
    let structural_before = structural_engine.checkpoint().map_err(|e| e.to_string())?;
    let structural_frontier = frontier(&[("alice", 1)])?;
    let structural_compacted = structural_engine
        .compact_stable(&structural_frontier)
        .map_err(|e| e.to_string())?;

    let empty = OperationSetEngine::default()
        .compact_stable(
            &register_base,
            &StabilityFrontier::new(BTreeMap::new()).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    let partial = StabilityFrontier::new(BTreeMap::from([("alice".to_owned(), 1)]))
        .map_err(|e| e.to_string())?;
    let ahead = StabilityFrontier::new(BTreeMap::from([
        ("alice".to_owned(), 2),
        ("bob".to_owned(), 1),
    ]))
    .map_err(|e| e.to_string())?;
    let partial_is_typed = matches!(
        register_compaction(&register_base, &register_changes, &partial),
        Err(CollaborationError::UnsafeCompaction { .. })
    );
    let ahead_is_typed = matches!(
        register_compaction(&register_base, &register_changes, &ahead),
        Err(CollaborationError::UnsafeCompaction { .. })
    );
    let checks = json!({
        "register_materializer_exact": register_compacted.checkpoint == register_before,
        "replica_log_materializer_exact": register_replica_compacted.checkpoint == register_before,
        "materializer_receipts_exact": register_compacted.receipt == register_replica_compacted.receipt,
        "register_history_fully_dropped": register_compacted.receipt.dropped.len() == register_changes.len()
            && register_compacted.receipt.retained.is_empty(),
        "structural_checkpoint_exact": structural_compacted.checkpoint == structural_before,
        "structural_history_fully_dropped": structural_compacted.receipt.dropped.len() == 1
            && structural_compacted.receipt.retained.is_empty(),
        "empty_history_compacts": empty.receipt.dropped.is_empty() && empty.receipt.retained.is_empty(),
        "partial_frontier_refused": partial_is_typed,
        "ahead_frontier_refused": ahead_is_typed,
        "metadata_absent_from_register_document": metadata_absent(&register_compacted.checkpoint.document)?,
        "metadata_absent_from_structural_document": metadata_absent(&structural_compacted.checkpoint.document)?,
        "profile_receipt_versioned": register_compacted.receipt.profile == nuif_collab::gc::PROFILE_NAME
            && structural_compacted.receipt.profile == nuif_collab::gc::PROFILE_NAME,
    });
    let passed = checks
        .as_object()
        .expect("checks is an object")
        .values()
        .all(|value| value == &Value::Bool(true));
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:crdt-causal-compaction",
        "status": if passed { "passed" } else { "failed" },
        "profile": {
            "name": nuif_collab::gc::PROFILE_NAME,
            "mode": "complete-history checkpoint compaction",
            "supported": [
                "caller-attested causal stability frontiers",
                "exact local-clock coverage",
                "register and structural checkpoint receipts",
                "metadata-free canonical checkpoints",
            ],
            "refused": [
                "partial history pruning",
                "frontiers behind local history",
                "frontiers ahead of local history",
                "implicit context or position-anchor rebasing",
            ],
        },
        "summary": {
            "register_changes": register_changes.len(),
            "structural_changes": 1,
            "register_checkpoint_hash": register_compacted.checkpoint.canonical_hash,
            "structural_checkpoint_hash": structural_compacted.checkpoint.canonical_hash,
            "dropped_register_changes": register_compacted.receipt.dropped,
            "dropped_structural_changes": structural_compacted.receipt.dropped,
        },
        "checks": checks,
    });
    write_json(&output, &report)?;
    println!(
        "causal compaction: {} register + {} structural changes, status {}",
        register_changes.len(),
        1,
        report["status"]
    );
    if passed {
        Ok(())
    } else {
        Err(format!("report failed; inspect {}", output.display()))
    }
}

fn register_change(replica: &str, counter: u64, name: &str) -> Change {
    Change {
        id: ChangeId::new(replica, counter),
        context: BTreeMap::new(),
        operation: Operation::Rename {
            entity: EntityId::new(0x20),
            name: Some(name.to_owned()),
        },
    }
}

fn register_checkpoint(
    base: &Document,
    changes: &[Change],
) -> Result<nuif_collab::Checkpoint, String> {
    let mut engine = OperationSetEngine::default();
    for change in changes {
        engine.ingest(change.clone()).map_err(|e| e.to_string())?;
    }
    engine.checkpoint(base).map_err(|e| e.to_string())
}

fn register_compaction(
    base: &Document,
    changes: &[Change],
    frontier: &StabilityFrontier,
) -> Result<nuif_collab::CompactedCheckpoint, CollaborationError> {
    let mut engine = OperationSetEngine::default();
    for change in changes {
        engine.ingest(change.clone())?;
    }
    engine.compact_stable(base, frontier)
}

fn replica_compaction(
    base: &Document,
    changes: &[Change],
    frontier: &StabilityFrontier,
) -> Result<nuif_collab::CompactedCheckpoint, String> {
    let mut engine = ReplicaLogEngine::default();
    for change in changes {
        engine.ingest(change.clone()).map_err(|e| e.to_string())?;
    }
    engine
        .compact_stable(base, frontier)
        .map_err(|e| e.to_string())
}

fn frontier(values: &[(&str, u64)]) -> Result<StabilityFrontier, String> {
    StabilityFrontier::new(
        values
            .iter()
            .map(|(replica, counter)| ((*replica).to_owned(), *counter))
            .collect(),
    )
    .map_err(|error| error.to_string())
}

fn structural_fixture() -> Document {
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

fn metadata_absent(document: &Document) -> Result<bool, String> {
    let encoded = CanonicalText
        .encode(document)
        .map_err(|error| error.to_string())?;
    let text = String::from_utf8(encoded).map_err(|error| error.to_string())?;
    Ok(![
        "alice",
        "bob",
        "counter",
        "replica",
        "active_positions",
        "trash",
    ]
    .iter()
    .any(|marker| text.contains(marker)))
}

fn output_path() -> Result<PathBuf, String> {
    let mut args = env::args_os().skip(1);
    let Some(argument) = args.next() else {
        return Ok(PathBuf::from("target/collaboration-gc-report.json"));
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
