use nuif_core::{Document, Fidelity};
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    AdapterError, AdapterReport, CorrespondenceRecord, CorrespondenceTarget, FidelityEntry,
    RetentiveSource, SourceEdit, SynchronizedSource, entity_pointer,
};

type Key = (CorrespondenceTarget, String);

/// Applies mapped semantic changes as atomic byte-local JSX replacements.
///
/// # Errors
///
/// Rejects structural changes, stale spans, profile expansion and any result
/// that does not self-import to the edited document.
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

    let current = correspondence_map(&retentive.report.correspondences)?;
    let before = correspondence_values(
        &generated_before.source,
        &generated_before.report.correspondences,
    )?;
    let after = correspondence_values(
        &generated_after.source,
        &generated_after.report.correspondences,
    )?;
    let current_keys = current.keys().cloned().collect::<BTreeSet<_>>();
    let before_keys = before.keys().cloned().collect::<BTreeSet<_>>();
    let after_keys = after.keys().cloned().collect::<BTreeSet<_>>();
    if current_keys != before_keys || current_keys != after_keys {
        let report = unmapped_key_report(edited, &current_keys, &after_keys);
        return Err(AdapterError::UnmappedChanges {
            issues: report.fidelity.len(),
            report: Box::new(report),
        });
    }

    let mut edits = Vec::new();
    for (key, record) in current {
        let expected = &before[&key];
        let observed = retentive
            .source
            .get(record.span.start..record.span.end)
            .ok_or_else(|| AdapterError::StaleSpan {
                pointer: record.pointer.clone(),
            })?;
        if observed != expected {
            return Err(AdapterError::StaleSpan {
                pointer: record.pointer,
            });
        }
        let replacement = &after[&key];
        if observed != replacement {
            edits.push(SourceEdit {
                target: record.target,
                pointer: record.pointer,
                span: record.span,
                replacement: replacement.clone(),
            });
        }
    }
    edits.sort_by_key(|edit| std::cmp::Reverse(edit.span.start));
    for pair in edits.windows(2) {
        if pair[1].span.end > pair[0].span.start {
            return Err(AdapterError::StaleSpan {
                pointer: pair[1].pointer.clone(),
            });
        }
    }
    let mut source = retentive.source.clone();
    for edit in &edits {
        source.replace_range(edit.span.start..edit.span.end, &edit.replacement);
    }
    edits.sort_by_key(|edit| edit.span.start);
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

fn correspondence_map(
    records: &[CorrespondenceRecord],
) -> Result<BTreeMap<Key, CorrespondenceRecord>, AdapterError> {
    let mut map = BTreeMap::new();
    for record in records {
        let key = (record.target.clone(), record.pointer.clone());
        if map.insert(key, record.clone()).is_some() {
            return Err(AdapterError::ProfileMarker(format!(
                "correspondence {} is duplicated",
                record.pointer
            )));
        }
    }
    Ok(map)
}

fn correspondence_values(
    source: &str,
    records: &[CorrespondenceRecord],
) -> Result<BTreeMap<Key, String>, AdapterError> {
    let mut values = BTreeMap::new();
    for record in records {
        let value = source
            .get(record.span.start..record.span.end)
            .ok_or_else(|| AdapterError::StaleSpan {
                pointer: record.pointer.clone(),
            })?
            .to_owned();
        let key = (record.target.clone(), record.pointer.clone());
        if values.insert(key, value).is_some() {
            return Err(AdapterError::ProfileMarker(format!(
                "correspondence {} is duplicated",
                record.pointer
            )));
        }
    }
    Ok(values)
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

fn unmapped_key_report(
    document: &Document,
    before: &BTreeSet<Key>,
    after: &BTreeSet<Key>,
) -> AdapterReport {
    let mut fidelity = before
        .symmetric_difference(after)
        .map(|(target, pointer)| {
            unsupported(
                target.clone(),
                pointer.clone(),
                "edit adds or removes a mapped source property",
            )
        })
        .collect::<Vec<_>>();
    if fidelity.is_empty() {
        fidelity.push(unsupported(
            CorrespondenceTarget::Document { id: document.id },
            "/".to_owned(),
            "source correspondence sets differ",
        ));
    }
    AdapterReport {
        schema_version: 1,
        source_format: crate::PROFILE_NAME.to_owned(),
        canonical_hash: None,
        fidelity,
        correspondences: Vec::new(),
        unmapped_source_preserved: false,
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
