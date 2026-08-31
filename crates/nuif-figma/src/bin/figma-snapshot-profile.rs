use nuif_codec::canonical_hash;
use nuif_core::Fidelity;
use nuif_figma::{AdapterError, MAX_SNAPSHOT_BYTES, import_snapshot, plan_import, profile_fixture};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let output = env::args()
        .nth(1)
        .unwrap_or_else(|| "target/figma-snapshot-report.json".to_owned());
    let document = profile_fixture();
    let plan =
        plan_import(&document, "credential-free-fixture").map_err(|error| error.to_string())?;
    let snapshot = serde_json::to_vec_pretty(&plan.snapshot).map_err(|error| error.to_string())?;
    let repeated = serde_json::to_vec_pretty(
        &plan_import(&document, "credential-free-fixture")
            .map_err(|error| error.to_string())?
            .snapshot,
    )
    .map_err(|error| error.to_string())?;
    let imported = import_snapshot(&snapshot).map_err(|error| error.to_string())?;
    if imported.document != document || snapshot != repeated || !imported.report.is_lossless() {
        return Err("exact Figma snapshot mapping did not converge".to_owned());
    }

    let mut lossy = plan.snapshot.clone();
    lossy.root.children[0].visible = false;
    lossy.root.children[0].opacity = 0.5;
    lossy.root.children[0]
        .unsupported_properties
        .extend(["effects".to_owned(), "boundVariables".to_owned()]);
    let lossy_import =
        import_snapshot(&serde_json::to_vec(&lossy).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    let unsupported = lossy_import
        .report
        .fidelity
        .iter()
        .filter(|entry| matches!(entry.status, Fidelity::Unsupported { .. }))
        .count();
    if unsupported != 3 || lossy_import.report.unmapped_host_data_preserved {
        return Err("unsupported Figma properties were not reported exactly".to_owned());
    }

    let mut duplicate = plan.snapshot.clone();
    let duplicate_id = duplicate.root.id.clone();
    duplicate.root.children[0].id.clone_from(&duplicate_id);
    let duplicate_rejected = matches!(
        import_snapshot(&serde_json::to_vec(&duplicate).map_err(|error| error.to_string())?),
        Err(AdapterError::DuplicateHostId(_))
    );
    let over_limit_rejected = matches!(
        import_snapshot(&vec![b' '; MAX_SNAPSHOT_BYTES + 1]),
        Err(AdapterError::SnapshotTooLarge)
    );
    if !duplicate_rejected || !over_limit_rejected {
        return Err("hostile Figma snapshot cases were not rejected".to_owned());
    }

    let report = json!({
        "schema_version": 1,
        "status": "passed",
        "profile": nuif_figma::PROFILE_NAME,
        "scope": "pure normalized Plugin API snapshot and mutation-plan mapping; no live Figma execution",
        "canonical_hash": canonical_hash(&document).map_err(|error| error.to_string())?,
        "snapshot": {
            "bytes": snapshot.len(),
            "sha256": format!("{:x}", Sha256::digest(&snapshot)),
            "nodes": imported.document.entities.len(),
            "correspondences": imported.report.correspondences.len(),
            "fidelity_entries": imported.report.fidelity.len(),
            "repeated_bytes_equal": snapshot == repeated
        },
        "negative_cases": {
            "unsupported_properties_reported": unsupported,
            "duplicate_host_id_rejected": duplicate_rejected,
            "input_limit_plus_one_rejected": over_limit_rejected
        },
        "live_host": {
            "status": "not_run",
            "required_before_vendor_integration_claim": true
        }
    });
    if let Some(parent) = Path::new(&output).parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(
        output,
        serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}
