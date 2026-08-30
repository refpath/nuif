use nuif_core::{Document, Fidelity};
use std::collections::BTreeSet;

use crate::{
    AdapterError, AdapterReport, CorrespondenceTarget, FidelityEntry, RetentiveSource,
    SynchronizedSource, entity_pointer,
};
use nuif_adapter::{ScalarSyncError, apply_scalar_edits, plan_scalar_edits};

/// Applies mapped semantic changes as byte-local replacements in retained source.
///
/// # Errors
///
/// Returns a typed stale-span, unsupported-change, parser, fidelity or
/// self-verification error without returning partially edited source.
pub fn synchronize(
    retentive: &RetentiveSource,
    edited: &Document,
) -> Result<SynchronizedSource, AdapterError> {
    let generated_before = crate::export_document(&retentive.document)?;
    let generated_after = match crate::export_document(edited) {
        Ok(exported) => exported,
        Err(AdapterError::UnsupportedProfile { report, .. }) => {
            return Err(AdapterError::UnmappedChanges {
                issues: report.fidelity.len(),
                report,
            });
        }
        Err(error) => return Err(error),
    };
    let structural = structural_issues(&retentive.document, edited);
    if !structural.is_empty() {
        return Err(AdapterError::UnmappedChanges {
            issues: structural.len(),
            report: Box::new(AdapterReport {
                schema_version: 1,
                source_format: crate::PROFILE_NAME.to_owned(),
                canonical_hash: generated_after.report.canonical_hash,
                fidelity: structural,
                correspondences: retentive.report.correspondences.clone(),
                unmapped_source_preserved: false,
            }),
        });
    }

    let edits = match plan_scalar_edits(
        &retentive.source,
        &retentive.report.correspondences,
        &generated_before.source,
        &generated_before.report.correspondences,
        &generated_after.source,
        &generated_after.report.correspondences,
    ) {
        Ok(edits) => edits,
        Err(ScalarSyncError::CorrespondenceSetMismatch) => {
            let report = unmapped_key_report(edited);
            return Err(AdapterError::UnmappedChanges {
                issues: report.fidelity.len(),
                report: Box::new(report),
            });
        }
        Err(error) => return Err(sync_error(error)),
    };
    let source = apply_scalar_edits(&retentive.source, &edits).map_err(sync_error)?;
    let imported = crate::import_source(&source)?;
    if imported.document != *edited {
        return Err(AdapterError::SynchronizationMismatch);
    }
    let mut report = imported.retentive.report;
    report.unmapped_source_preserved = true;
    Ok(SynchronizedSource {
        source,
        edits,
        report,
    })
}

fn structural_issues(before: &Document, after: &Document) -> Vec<FidelityEntry> {
    let mut issues = Vec::new();
    if before.roots != after.roots {
        issues.push(unsupported(
            CorrespondenceTarget::Document { id: after.id },
            "/roots".to_owned(),
            "source synchronization cannot insert, remove or reorder roots",
        ));
    }
    for id in before
        .entities
        .keys()
        .chain(after.entities.keys())
        .copied()
        .collect::<BTreeSet<_>>()
    {
        match (before.entities.get(&id), after.entities.get(&id)) {
            (Some(left), Some(right)) if left.children != right.children => {
                issues.push(unsupported(
                    CorrespondenceTarget::Entity { id },
                    entity_pointer(id, "/children"),
                    "source synchronization cannot insert, remove or reorder children",
                ));
            }
            (None, Some(_)) | (Some(_), None) => issues.push(unsupported(
                CorrespondenceTarget::Entity { id },
                entity_pointer(id, ""),
                "source synchronization cannot insert or remove entities",
            )),
            _ => {}
        }
    }
    issues
}

fn unmapped_key_report(document: &Document) -> AdapterReport {
    AdapterReport {
        schema_version: 1,
        source_format: crate::PROFILE_NAME.to_owned(),
        canonical_hash: None,
        fidelity: vec![unsupported(
            CorrespondenceTarget::Document { id: document.id },
            "/".to_owned(),
            "source correspondence sets differ",
        )],
        correspondences: Vec::new(),
        unmapped_source_preserved: false,
    }
}

fn sync_error(error: ScalarSyncError) -> AdapterError {
    match error {
        ScalarSyncError::DuplicateCorrespondence { pointer } => {
            AdapterError::ProfileMarker(format!("correspondence {pointer} is duplicated"))
        }
        ScalarSyncError::SpanOutOfBounds { pointer }
        | ScalarSyncError::StaleSpan { pointer }
        | ScalarSyncError::OverlappingSpans { pointer } => AdapterError::StaleSpan { pointer },
        ScalarSyncError::CorrespondenceSetMismatch => {
            AdapterError::ProfileMarker("source correspondence sets differ".to_owned())
        }
    }
}

fn unsupported(target: CorrespondenceTarget, pointer: String, reason: &str) -> FidelityEntry {
    FidelityEntry {
        target,
        pointer,
        status: Fidelity::Unsupported {
            reason: reason.to_owned(),
        },
    }
}
