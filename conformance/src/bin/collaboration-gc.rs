use nuif_codec::{CanonicalText, Encoder};
use nuif_collab::creation::{
    CreationAnchor, CreationChange, CreationOperation, CreationOperationSetEngine,
};
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

struct RegisterEvidence {
    changes: Vec<Change>,
    before: nuif_collab::Checkpoint,
    compacted: nuif_collab::CompactedCheckpoint,
    replica_compacted: nuif_collab::CompactedCheckpoint,
}

struct StructuralEvidence {
    before: nuif_collab::structural::StructuralCheckpoint,
    compacted: nuif_collab::structural::StructuralCompaction,
}

struct CreationEvidence {
    before: nuif_collab::creation::CreationCheckpoint,
    compacted: nuif_collab::creation::CreationCompaction,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("collaboration-gc: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let output = output_path()?;
    let register = register_evidence()?;
    let structural = structural_evidence()?;
    let creation = creation_evidence()?;
    let empty = empty_compaction()?;
    let register_base = nuif_testing::responsive_card_fixture();
    let partial = StabilityFrontier::new(BTreeMap::from([("alice".to_owned(), 1)]))
        .map_err(|e| e.to_string())?;
    let ahead = StabilityFrontier::new(BTreeMap::from([
        ("alice".to_owned(), 2),
        ("bob".to_owned(), 1),
    ]))
    .map_err(|e| e.to_string())?;
    let partial_is_typed = matches!(
        register_compaction(&register_base, &register.changes, &partial),
        Err(CollaborationError::UnsafeCompaction { .. })
    );
    let ahead_is_typed = matches!(
        register_compaction(&register_base, &register.changes, &ahead),
        Err(CollaborationError::UnsafeCompaction { .. })
    );
    let checks = json!({
        "register_materializer_exact": register.compacted.checkpoint == register.before,
        "replica_log_materializer_exact": register.replica_compacted.checkpoint == register.before,
        "materializer_receipts_exact": register.compacted.receipt == register.replica_compacted.receipt,
        "register_history_fully_dropped": register.compacted.receipt.dropped.len() == register.changes.len()
            && register.compacted.receipt.retained.is_empty(),
        "structural_checkpoint_exact": structural.compacted.checkpoint == structural.before,
        "structural_history_fully_dropped": structural.compacted.receipt.dropped.len() == 1
            && structural.compacted.receipt.retained.is_empty(),
        "creation_checkpoint_exact": creation.compacted.checkpoint == creation.before,
        "creation_history_fully_dropped": creation.compacted.receipt.dropped.len() == 1
            && creation.compacted.receipt.retained.is_empty(),
        "empty_history_compacts": empty.receipt.dropped.is_empty() && empty.receipt.retained.is_empty(),
        "partial_frontier_refused": partial_is_typed,
        "ahead_frontier_refused": ahead_is_typed,
        "metadata_absent_from_register_document": metadata_absent(&register.compacted.checkpoint.document)?,
        "metadata_absent_from_structural_document": metadata_absent(&structural.compacted.checkpoint.document)?,
        "metadata_absent_from_creation_document": metadata_absent(&creation.compacted.checkpoint.document)?,
        "profile_receipt_versioned": register.compacted.receipt.profile == nuif_collab::gc::PROFILE_NAME
            && structural.compacted.receipt.profile == nuif_collab::gc::PROFILE_NAME
            && creation.compacted.receipt.profile == nuif_collab::gc::PROFILE_NAME,
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
            "register_changes": register.changes.len(),
            "structural_changes": 1,
            "creation_changes": 1,
            "register_checkpoint_hash": register.compacted.checkpoint.canonical_hash,
            "structural_checkpoint_hash": structural.compacted.checkpoint.canonical_hash,
            "creation_checkpoint_hash": creation.compacted.checkpoint.canonical_hash,
            "dropped_register_changes": register.compacted.receipt.dropped,
            "dropped_structural_changes": structural.compacted.receipt.dropped,
        },
        "checks": checks,
    });
    write_json(&output, &report)?;
    println!(
        "causal compaction: {} register + {} structural + {} creation changes, status {}",
        register.changes.len(),
        1,
        1,
        report["status"]
    );
    if passed {
        Ok(())
    } else {
        Err(format!("report failed; inspect {}", output.display()))
    }
}

fn register_evidence() -> Result<RegisterEvidence, String> {
    let base = nuif_testing::responsive_card_fixture();
    let changes = vec![
        register_change("alice", 1, "Alice"),
        register_change("bob", 1, "Bob"),
    ];
    let frontier = frontier(&[("alice", 1), ("bob", 1)])?;
    let before = register_checkpoint(&base, &changes)?;
    let compacted =
        register_compaction(&base, &changes, &frontier).map_err(|error| error.to_string())?;
    let replica_compacted = replica_compaction(&base, &changes, &frontier)?;
    Ok(RegisterEvidence {
        changes,
        before,
        compacted,
        replica_compacted,
    })
}

fn structural_evidence() -> Result<StructuralEvidence, String> {
    let mut engine = StructuralOperationSetEngine::new(structural_fixture())
        .map_err(|error| error.to_string())?;
    engine
        .ingest(StructuralChange {
            id: ChangeId::new("alice", 1),
            context: BTreeMap::new(),
            operation: StructuralOperation::Move {
                entity: RIGHT,
                new_parent: Some(LEFT),
                anchor: StructuralAnchor::Start,
            },
        })
        .map_err(|error| error.to_string())?;
    let before = engine.checkpoint().map_err(|error| error.to_string())?;
    let frontier = frontier(&[("alice", 1)])?;
    let compacted = engine
        .compact_stable(&frontier)
        .map_err(|error| error.to_string())?;
    Ok(StructuralEvidence { before, compacted })
}

fn creation_evidence() -> Result<CreationEvidence, String> {
    let mut engine =
        CreationOperationSetEngine::new(structural_fixture()).map_err(|error| error.to_string())?;
    engine
        .ingest(CreationChange {
            id: ChangeId::new("alice", 1),
            context: BTreeMap::new(),
            operation: CreationOperation::Insert {
                parent: Some(ROOT),
                anchor: CreationAnchor::Start,
                entity: Box::new(Entity::new(EntityId::new(20), EntityKind::Container)),
            },
        })
        .map_err(|error| error.to_string())?;
    let before = engine.checkpoint().map_err(|error| error.to_string())?;
    let frontier = frontier(&[("alice", 1)])?;
    let compacted = engine
        .compact_stable(&frontier)
        .map_err(|error| error.to_string())?;
    Ok(CreationEvidence { before, compacted })
}

fn empty_compaction() -> Result<nuif_collab::CompactedCheckpoint, String> {
    OperationSetEngine::default()
        .compact_stable(
            &nuif_testing::responsive_card_fixture(),
            &StabilityFrontier::new(BTreeMap::new()).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())
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
