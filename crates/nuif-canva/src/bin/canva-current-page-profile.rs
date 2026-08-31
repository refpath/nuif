use nuif_canva::{
    AdapterError, MAX_SNAPSHOT_BYTES, export_page, import_current_page, profile_fixture,
};
use nuif_codec::canonical_hash;
use nuif_core::Fidelity;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    if let Err(error) = run() {
        eprintln!("canva-current-page-profile: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let output = env::args()
        .nth(1)
        .unwrap_or_else(|| "target/canva-current-page-report.json".to_owned());
    let document = profile_fixture();
    let exported = export_page(&document, "2026.1").map_err(|error| error.to_string())?;
    let snapshot = serde_json::to_vec_pretty(&exported.page).map_err(|error| error.to_string())?;
    let repeated = serde_json::to_vec_pretty(
        &export_page(&document, "2026.1")
            .map_err(|error| error.to_string())?
            .page,
    )
    .map_err(|error| error.to_string())?;
    let imported = import_current_page(&snapshot).map_err(|error| error.to_string())?;
    let exact = imported.document == document;
    if !exact || snapshot != repeated || !imported.report.is_lossless() {
        return Err("exact Canva current-page mapping did not converge".to_owned());
    }

    let mut lossy = exported.page.clone();
    lossy.elements[0].opacity = 0.5;
    lossy.elements[0]
        .unsupported_properties
        .push("effects".to_owned());
    let lossy_bytes = serde_json::to_vec(&lossy).map_err(|error| error.to_string())?;
    let lossy_import = import_current_page(&lossy_bytes);
    let unsupported = match lossy_import {
        Err(AdapterError::UnsupportedProfile { report, .. }) => report
            .fidelity
            .iter()
            .filter(|entry| matches!(entry.status, Fidelity::Unsupported { .. }))
            .count(),
        _ => 0,
    };

    let mut duplicate = exported.page.clone();
    let duplicate_id = duplicate.elements[0].id.clone();
    duplicate.elements[1].id = duplicate_id;
    let duplicate_rejected = matches!(
        import_current_page(&serde_json::to_vec(&duplicate).map_err(|error| error.to_string())?),
        Err(AdapterError::DuplicateHostId(_))
    );
    let over_limit_rejected = matches!(
        import_current_page(&vec![b' '; MAX_SNAPSHOT_BYTES + 1]),
        Err(AdapterError::SnapshotTooLarge)
    );
    if unsupported != 2 || !duplicate_rejected || !over_limit_rejected {
        return Err("bounded Canva negative cases were not rejected exactly".to_owned());
    }

    let report = json!({
        "schema_version": 1,
        "status": "passed",
        "profile": nuif_canva::PROFILE_NAME,
        "scope": "pure normalized Canva current-page mapping; no live Canva execution",
        "canonical_hash": canonical_hash(&document).map_err(|error| error.to_string())?,
        "snapshot": {
            "bytes": snapshot.len(),
            "sha256": format!("{:x}", Sha256::digest(&snapshot)),
            "elements": imported.document.entities.len(),
            "correspondences": imported.report.correspondences.len(),
            "fidelity_entries": imported.report.fidelity.len(),
            "repeated_bytes_equal": snapshot == repeated,
            "document_exact": exact
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
