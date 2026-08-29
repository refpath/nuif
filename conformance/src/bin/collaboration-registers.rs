use nuif_codec::{CanonicalText, Encoder};
use nuif_collab::{Change, ChangeId, CollaborationError, OperationSetEngine, ReplicaLogEngine};
use nuif_core::{EntityId, PropertyValue, SizeIntent};
use nuif_protocol::{Anchor, Axis, Operation};
use serde_json::{Value, json};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const CARD: EntityId = EntityId::new(0x20);
const COPY: EntityId = EntityId::new(0x22);
const UNKNOWN: EntityId = EntityId::new(0x25);

fn main() {
    if let Err(error) = run() {
        eprintln!("collaboration-registers: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let report_path = output_path()?;
    let base = nuif_testing::responsive_card_fixture();
    let changes = fixture_changes();
    let expected = checkpoints_for_order(&base, &changes)?;
    let (permutations, all_orders_converge) = exhaustive_deliveries(&base, &changes, &expected)?;
    let checks = json!({
        "two_materializers_exact": expected.0 == expected.1,
        "all_delivery_orders_converge": all_orders_converge,
        "delivery_permutations_exact": permutations == 5040,
        "merge_commutative_associative": merge_orders_converge(&base, &changes, &expected.0)?,
        "merge_idempotent": duplicate_delivery_is_idempotent(&base, &changes, &expected.0)?,
        "two_conflicts_explicit": expected.0.conflicts.len() == 2,
        "conflict_pointers_exact": conflict_pointers_exact(&expected.0),
        "causal_overwrite_selected": expected.0.applied.contains(&ChangeId::new("alice", 3))
            && !expected.0.applied.contains(&ChangeId::new("alice", 2)),
        "opaque_payload_preserved": expected.0.document.entities[&UNKNOWN] == base.entities[&UNKNOWN],
        "collaboration_metadata_absent": collaboration_metadata_absent(&expected.0.document)?,
        "missing_history_typed": missing_history_typed(&base),
        "duplicate_identifier_typed": duplicate_identifier_typed(),
        "structural_operation_typed": structural_operation_typed(),
        "invalid_local_context_typed": invalid_local_context_typed(),
        "semantic_apply_failure_typed": semantic_apply_failure_typed(&base),
    });
    let passed = checks
        .as_object()
        .expect("checks is an object")
        .values()
        .all(|value| value == &Value::Bool(true));
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:crdt-checkpoint",
        "status": if passed { "passed" } else { "failed" },
        "source": {
            "revision": command_text("git", &["rev-parse", "HEAD"]),
            "dirty": command_text("git", &["status", "--porcelain"]).map(|value| !value.is_empty()),
            "toolchain": command_text("rustc", &["--version"]),
            "os": env::consts::OS,
            "architecture": env::consts::ARCH,
        },
        "profile": {
            "name": nuif_collab::PROFILE_NAME,
            "engines": ["operation-set maximality", "replica-log incremental frontier"],
            "oracle": "algorithmically-independent-in-repository-materializers",
            "supported": ["rename", "size", "layout", "token registers", "property values", "extension registers", "unknown payload"],
            "unsupported": ["insert", "remove", "move", "restore_subtree"],
            "limits": {
                "changes": nuif_collab::MAX_CHANGES,
                "replicas": nuif_collab::MAX_REPLICAS,
                "replica_bytes": nuif_collab::MAX_REPLICA_BYTES,
            },
        },
        "summary": {
            "changes": changes.len(),
            "delivery_permutations": permutations,
            "selected_changes": expected.0.applied.len(),
            "semantic_conflicts": expected.0.conflicts.len(),
            "canonical_hash": expected.0.canonical_hash,
            "blocking_failures": u8::from(!passed),
        },
        "checks": checks,
        "conflicts": expected.0.conflicts,
        "applied": expected.0.applied,
    });
    write_file(
        &report_path,
        &serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )?;
    println!(
        "collaboration registers: {permutations} delivery orders, {} conflicts, status {}",
        report["summary"]["semantic_conflicts"], report["status"]
    );
    if passed {
        Ok(())
    } else {
        Err(format!("report failed; inspect {}", report_path.display()))
    }
}

fn fixture_changes() -> Vec<Change> {
    vec![
        change(
            "alice",
            1,
            &[],
            Operation::Rename {
                entity: CARD,
                name: Some("Alice card".to_owned()),
            },
        ),
        change(
            "bob",
            1,
            &[],
            Operation::Rename {
                entity: CARD,
                name: Some("Bob card".to_owned()),
            },
        ),
        change(
            "alice",
            2,
            &[("alice", 1)],
            Operation::SetSize {
                entity: CARD,
                axis: Axis::Horizontal,
                value: SizeIntent::Fixed(600.0),
            },
        ),
        change(
            "bob",
            2,
            &[("bob", 1)],
            Operation::SetSize {
                entity: CARD,
                axis: Axis::Vertical,
                value: SizeIntent::Fixed(320.0),
            },
        ),
        change(
            "alice",
            3,
            &[("alice", 2)],
            Operation::SetSize {
                entity: CARD,
                axis: Axis::Horizontal,
                value: SizeIntent::Fixed(620.0),
            },
        ),
        change(
            "bob",
            3,
            &[("bob", 2)],
            Operation::SetValue {
                entity: CARD,
                key: "variant".to_owned(),
                value: PropertyValue::String("compact".to_owned()),
            },
        ),
        change(
            "carol",
            1,
            &[],
            Operation::SetValue {
                entity: CARD,
                key: "variant".to_owned(),
                value: PropertyValue::String("expanded".to_owned()),
            },
        ),
    ]
}

fn change(replica: &str, counter: u64, context: &[(&str, u64)], operation: Operation) -> Change {
    Change {
        id: ChangeId::new(replica, counter),
        context: context
            .iter()
            .map(|(replica, counter)| ((*replica).to_owned(), *counter))
            .collect(),
        operation,
    }
}

fn checkpoints_for_order(
    base: &nuif_core::Document,
    changes: &[Change],
) -> Result<(nuif_collab::Checkpoint, nuif_collab::Checkpoint), String> {
    let mut set = OperationSetEngine::default();
    let mut logs = ReplicaLogEngine::default();
    for item in changes {
        set.ingest(item.clone())
            .map_err(|error| error.to_string())?;
        logs.ingest(item.clone())
            .map_err(|error| error.to_string())?;
    }
    Ok((
        set.checkpoint(base).map_err(|error| error.to_string())?,
        logs.checkpoint(base).map_err(|error| error.to_string())?,
    ))
}

fn exhaustive_deliveries(
    base: &nuif_core::Document,
    changes: &[Change],
    expected: &(nuif_collab::Checkpoint, nuif_collab::Checkpoint),
) -> Result<(usize, bool), String> {
    fn visit(
        base: &nuif_core::Document,
        changes: &mut [Change],
        index: usize,
        expected: &(nuif_collab::Checkpoint, nuif_collab::Checkpoint),
        count: &mut usize,
        converged: &mut bool,
    ) -> Result<(), String> {
        if index == changes.len() {
            let observed = checkpoints_for_order(base, changes)?;
            *count += 1;
            *converged &= observed == *expected;
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
    base: &nuif_core::Document,
    changes: &[Change],
    expected: &nuif_collab::Checkpoint,
) -> Result<bool, String> {
    let mut set_parts = [
        OperationSetEngine::default(),
        OperationSetEngine::default(),
        OperationSetEngine::default(),
    ];
    let mut log_parts = [
        ReplicaLogEngine::default(),
        ReplicaLogEngine::default(),
        ReplicaLogEngine::default(),
    ];
    for (index, change) in changes.iter().enumerate() {
        let partition = index % 3;
        set_parts[partition]
            .ingest(change.clone())
            .map_err(|error| error.to_string())?;
        log_parts[partition]
            .ingest(change.clone())
            .map_err(|error| error.to_string())?;
    }
    let mut set_left = set_parts[0].clone();
    set_left
        .merge(&set_parts[1])
        .map_err(|error| error.to_string())?;
    set_left
        .merge(&set_parts[2])
        .map_err(|error| error.to_string())?;
    let mut set_right = set_parts[2].clone();
    set_right
        .merge(&set_parts[0])
        .map_err(|error| error.to_string())?;
    set_right
        .merge(&set_parts[1])
        .map_err(|error| error.to_string())?;
    let mut log_left = log_parts[0].clone();
    log_left
        .merge(&log_parts[1])
        .map_err(|error| error.to_string())?;
    log_left
        .merge(&log_parts[2])
        .map_err(|error| error.to_string())?;
    let mut log_right = log_parts[2].clone();
    log_right
        .merge(&log_parts[0])
        .map_err(|error| error.to_string())?;
    log_right
        .merge(&log_parts[1])
        .map_err(|error| error.to_string())?;
    Ok([
        set_left.checkpoint(base),
        set_right.checkpoint(base),
        log_left.checkpoint(base),
        log_right.checkpoint(base),
    ]
    .into_iter()
    .all(|checkpoint| checkpoint.as_ref() == Ok(expected)))
}

fn duplicate_delivery_is_idempotent(
    base: &nuif_core::Document,
    changes: &[Change],
    expected: &nuif_collab::Checkpoint,
) -> Result<bool, String> {
    let mut set = OperationSetEngine::default();
    let mut logs = ReplicaLogEngine::default();
    for item in changes.iter().chain(changes.iter().rev()) {
        set.ingest(item.clone())
            .map_err(|error| error.to_string())?;
        logs.ingest(item.clone())
            .map_err(|error| error.to_string())?;
    }
    Ok(set.checkpoint(base).as_ref() == Ok(expected)
        && logs.checkpoint(base).as_ref() == Ok(expected))
}

fn conflict_pointers_exact(checkpoint: &nuif_collab::Checkpoint) -> bool {
    checkpoint
        .conflicts
        .iter()
        .map(|conflict| conflict.key.pointer.as_str())
        .collect::<Vec<_>>()
        == [
            "/entities/00000000000000000000000000000020/authored/values/variant",
            "/entities/00000000000000000000000000000020/name",
        ]
}

fn collaboration_metadata_absent(document: &nuif_core::Document) -> Result<bool, String> {
    let encoded = CanonicalText
        .encode(document)
        .map_err(|error| error.to_string())?;
    let text = String::from_utf8(encoded).map_err(|error| error.to_string())?;
    Ok(!["alice", "bob", "carol", "context", "counter", "replica"]
        .iter()
        .any(|marker| text.contains(marker)))
}

fn missing_history_typed(base: &nuif_core::Document) -> bool {
    let mut engine = OperationSetEngine::default();
    engine
        .ingest(change(
            "alice",
            2,
            &[("alice", 1)],
            Operation::Rename {
                entity: COPY,
                name: Some("missing base".to_owned()),
            },
        ))
        .is_ok()
        && matches!(
            engine.checkpoint(base),
            Err(CollaborationError::MissingReplicaChange { .. })
        )
}

fn duplicate_identifier_typed() -> bool {
    let first = change(
        "alice",
        1,
        &[],
        Operation::Rename {
            entity: COPY,
            name: Some("first".to_owned()),
        },
    );
    let mut second = first.clone();
    second.operation = Operation::Rename {
        entity: COPY,
        name: Some("second".to_owned()),
    };
    let mut engine = OperationSetEngine::default();
    engine.ingest(first).is_ok()
        && matches!(
            engine.ingest(second),
            Err(CollaborationError::DuplicateChange { .. })
        )
}

fn structural_operation_typed() -> bool {
    matches!(
        OperationSetEngine::default().ingest(change(
            "alice",
            1,
            &[],
            Operation::Move {
                entity: COPY,
                new_parent: None,
                anchor: Anchor::Start,
            },
        )),
        Err(CollaborationError::UnsupportedOperation { operation: "move" })
    )
}

fn invalid_local_context_typed() -> bool {
    matches!(
        OperationSetEngine::default().ingest(change(
            "alice",
            2,
            &[],
            Operation::Rename {
                entity: COPY,
                name: Some("invalid clock".to_owned()),
            },
        )),
        Err(CollaborationError::InvalidLocalContext { .. })
    )
}

fn semantic_apply_failure_typed(base: &nuif_core::Document) -> bool {
    let mut engine = OperationSetEngine::default();
    engine
        .ingest(change(
            "alice",
            1,
            &[],
            Operation::Rename {
                entity: EntityId::new(0xffff),
                name: Some("missing".to_owned()),
            },
        ))
        .is_ok()
        && matches!(
            engine.checkpoint(base),
            Err(CollaborationError::Apply { .. })
        )
}

fn output_path() -> Result<PathBuf, String> {
    let mut args = env::args().skip(1);
    let mut output = PathBuf::from("target/collaboration-report.json");
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--output" => output = PathBuf::from(args.next().ok_or("--output requires a path")?),
            _ => return Err(format!("unknown argument {argument}")),
        }
    }
    Ok(output)
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(path, bytes).map_err(|error| error.to_string())
}

fn command_text(program: &str, arguments: &[&str]) -> Option<String> {
    let output = Command::new(program).args(arguments).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
