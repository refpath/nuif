//! Conformance evidence for register-only causal prefix collection.
//!
//! The profile keeps a canonical checkpoint for a causally closed stable
//! prefix and a retained suffix that explicitly dominates that prefix. It is
//! intentionally not a structural anchor rebasing protocol.

use nuif_codec::{CanonicalText, Encoder};
use nuif_collab::gc::{ResumedOperationSetEngine, StabilityFrontier};
use nuif_collab::{Change, ChangeId, CollaborationError, OperationSetEngine};
use nuif_core::{Document, EntityId};
use nuif_protocol::Operation;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const CARD: EntityId = EntityId::new(0x20);

fn main() {
    if let Err(error) = run() {
        eprintln!("collaboration-gc-prefix: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let output = output_path()?;
    let base = nuif_testing::responsive_card_fixture();
    let changes = fixture_changes();
    let frontier = StabilityFrontier::new(BTreeMap::from([("alice".to_owned(), 1)]))
        .map_err(|error| error.to_string())?;

    let mut engine = OperationSetEngine::default();
    for change in &changes {
        engine
            .ingest(change.clone())
            .map_err(|error| error.to_string())?;
    }
    let full = engine
        .checkpoint(&base)
        .map_err(|error| error.to_string())?;
    let compacted = engine
        .compact_stable_prefix(&base, &frontier)
        .map_err(|error| error.to_string())?;
    let resumed = replay_resumed(&compacted.base, &compacted.retained)?;
    let checks = json!({
        "semantic_checkpoint_preserved": semantic_equal(&full, &compacted.checkpoint),
        "stable_prefix_dropped": compacted.receipt.dropped == vec![changes[0].id.clone()],
        "retained_suffix_recorded": compacted.receipt.retained == vec![changes[1].id.clone()],
        "resumed_suffix_matches_full": semantic_equal(&full, &resumed),
        "base_is_stable_prefix": compacted.base.checkpoint.document.entities[&CARD].name.as_deref() == Some("stable"),
        "canonical_document_has_no_history_metadata": metadata_absent(&compacted.checkpoint.document)?,
        "non_closed_prefix_typed": non_closed_prefix_typed(),
        "concurrent_retained_change_typed": concurrent_retained_typed(),
    });
    let passed = checks
        .as_object()
        .expect("checks is an object")
        .values()
        .all(|value| value == &Value::Bool(true));
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:crdt-gc-prefix",
        "status": if passed { "passed" } else { "failed" },
        "profile": {
            "name": nuif_collab::gc::PARTIAL_PROFILE_NAME,
            "base_profile": nuif_collab::gc::BASE_PROFILE_NAME,
            "supported": [
                "register-only stable prefix collection",
                "causally closed dropped prefix",
                "retained suffix that dominates the frontier",
                "resumable metadata-bearing causal base",
            ],
            "unsupported": [
                "structural anchor rebasing",
                "concurrent stable-versus-retained register conflicts",
                "frontier inference",
            ],
            "limits": {
                "changes": nuif_collab::MAX_CHANGES,
                "replicas": nuif_collab::MAX_REPLICAS,
            },
        },
        "summary": {
            "changes": changes.len(),
            "dropped": compacted.receipt.dropped,
            "retained": compacted.receipt.retained,
            "stable_checkpoint_hash": compacted.base.checkpoint.canonical_hash,
            "canonical_hash": compacted.checkpoint.canonical_hash,
        },
        "checks": checks,
    });
    write_json(&output, &report)?;
    println!(
        "causal prefix compaction: {} dropped, {} retained, status {}",
        compacted.receipt.dropped.len(),
        compacted.receipt.retained.len(),
        report["status"]
    );
    if passed {
        Ok(())
    } else {
        Err(format!("report failed; inspect {}", output.display()))
    }
}

fn fixture_changes() -> Vec<Change> {
    vec![
        rename("alice", 1, &[], "stable"),
        rename("alice", 2, &[("alice", 1)], "retained"),
    ]
}

fn rename(replica: &str, counter: u64, context: &[(&str, u64)], name: &str) -> Change {
    Change {
        id: ChangeId::new(replica, counter),
        context: context
            .iter()
            .map(|(replica, counter)| ((*replica).to_owned(), *counter))
            .collect(),
        operation: Operation::Rename {
            entity: CARD,
            name: Some(name.to_owned()),
        },
    }
}

fn replay_resumed(
    base: &nuif_collab::gc::CausalCheckpointBase,
    changes: &[Change],
) -> Result<nuif_collab::Checkpoint, String> {
    let mut resumed = ResumedOperationSetEngine::new(base.clone()).map_err(|e| e.to_string())?;
    for change in changes {
        resumed.ingest(change.clone()).map_err(|e| e.to_string())?;
    }
    resumed.checkpoint().map_err(|e| e.to_string())
}

fn semantic_equal(left: &nuif_collab::Checkpoint, right: &nuif_collab::Checkpoint) -> bool {
    left.canonical_hash == right.canonical_hash
        && left.document == right.document
        && left.conflicts == right.conflicts
}

fn metadata_absent(document: &Document) -> Result<bool, String> {
    let bytes = CanonicalText
        .encode(document)
        .map_err(|error| error.to_string())?;
    let text = String::from_utf8(bytes).map_err(|error| error.to_string())?;
    Ok(!text.contains("alice") && !text.contains("nuif-collab"))
}

fn non_closed_prefix_typed() -> bool {
    let changes = vec![
        rename("alice", 1, &[("bob", 1)], "stable"),
        rename("bob", 1, &[], "dependency"),
    ];
    let frontier = StabilityFrontier::new(BTreeMap::from([("alice".to_owned(), 1)])).unwrap();
    matches!(
        frontier.validate_prefix(&changes),
        Err(CollaborationError::StablePrefixNotClosed { .. })
    )
}

fn concurrent_retained_typed() -> bool {
    let changes = vec![
        rename("alice", 1, &[], "stable"),
        rename("bob", 1, &[], "concurrent"),
    ];
    let frontier = StabilityFrontier::new(BTreeMap::from([("alice".to_owned(), 1)])).unwrap();
    matches!(
        frontier.validate_prefix(&changes),
        Err(CollaborationError::RetainedChangeNotAfterFrontier { .. })
    )
}

fn output_path() -> Result<PathBuf, String> {
    let mut args = env::args_os().skip(1);
    let Some(argument) = args.next() else {
        return Ok(PathBuf::from("target/collaboration-gc-prefix-report.json"));
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
    fs::write(
        path,
        serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}
