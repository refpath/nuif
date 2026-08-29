use crate::parse::encoded_scalar;
use crate::{
    AdapterError, CorrespondenceRecord, CorrespondenceTarget, FidelityEntry, PROFILE_NAME,
    RetentiveSource, SourceEdit, SynchronizedSource, export_document, import_source,
};
use nuif_adapter::AdapterReport;
use nuif_core::{Document, EntityId, EntityKind, Fidelity};
use std::collections::BTreeMap;

type Key = (CorrespondenceTarget, String);
type EntityStructure = (EntityId, EntityKind, Vec<EntityId>);
type StructureSignature = (Vec<EntityId>, Vec<EntityStructure>);

/// Applies mapped semantic changes as byte-local edits to retained SVG source.
///
/// # Errors
///
/// Returns typed stale-span, unsupported, structural, parse or
/// synchronization errors without returning partial source.
pub fn synchronize(
    retained: &RetentiveSource,
    edited: &Document,
) -> Result<SynchronizedSource, AdapterError> {
    let observed = import_source(&retained.source)?;
    if observed.document != retained.document
        || observed.retentive.report.correspondences != retained.report.correspondences
    {
        return Err(AdapterError::StaleSpan {
            pointer: String::new(),
        });
    }
    if structure_signature(&retained.document) != structure_signature(edited) {
        return Err(unmapped(edited, "containment or entity kinds changed"));
    }

    let canonical_before = export_document(&retained.document)?;
    let canonical_after = match export_document(edited) {
        Ok(exported) => exported,
        Err(AdapterError::UnsupportedProfile { report, .. }) => {
            return Err(AdapterError::UnmappedChanges {
                issues: report.fidelity.len(),
                report,
            });
        }
        Err(error) => return Err(error),
    };
    let retained_values = values(&retained.source, &retained.report.correspondences)?;
    let before_values = values(
        &canonical_before.source,
        &canonical_before.report.correspondences,
    )?;
    let after_values = values(
        &canonical_after.source,
        &canonical_after.report.correspondences,
    )?;
    if retained_values.keys().collect::<Vec<_>>() != before_values.keys().collect::<Vec<_>>()
        || retained_values.keys().collect::<Vec<_>>() != after_values.keys().collect::<Vec<_>>()
    {
        return Err(unmapped(edited, "correspondence inventory changed"));
    }

    let retained_records = records_by_key(&retained.report.correspondences);
    let mut edits = Vec::new();
    for (key, current) in &retained_values {
        let expected_before = &before_values[key];
        let expected_after = &after_values[key];
        if current.len() != expected_before.len() || current.len() != expected_after.len() {
            return Err(unmapped(edited, "correspondence multiplicity changed"));
        }
        for index in 0..current.len() {
            if current[index] != expected_before[index] {
                return Err(AdapterError::StaleSpan {
                    pointer: key.1.clone(),
                });
            }
            if current[index] != expected_after[index] {
                let record = retained_records[key][index];
                edits.push(SourceEdit {
                    target: key.0.clone(),
                    pointer: key.1.clone(),
                    span: record.span,
                    replacement: expected_after[index].clone(),
                });
            }
        }
    }
    edits.sort_by_key(|edit| (edit.span.start, edit.span.end));
    let mut source = retained.source.clone();
    for edit in edits.iter().rev() {
        source.replace_range(edit.span.start..edit.span.end, &edit.replacement);
    }
    let imported = import_source(&source)?;
    if imported.document != *edited {
        return Err(AdapterError::SynchronizationMismatch);
    }
    Ok(SynchronizedSource {
        source,
        edits,
        report: imported.retentive.report,
    })
}

fn values(
    source: &str,
    records: &[CorrespondenceRecord],
) -> Result<BTreeMap<Key, Vec<String>>, AdapterError> {
    let mut values = BTreeMap::<Key, Vec<String>>::new();
    for record in records {
        values
            .entry((record.target.clone(), record.pointer.clone()))
            .or_default()
            .push(encoded_scalar(source, record)?);
    }
    Ok(values)
}

fn records_by_key(records: &[CorrespondenceRecord]) -> BTreeMap<Key, Vec<&CorrespondenceRecord>> {
    let mut grouped = BTreeMap::<Key, Vec<&CorrespondenceRecord>>::new();
    for record in records {
        grouped
            .entry((record.target.clone(), record.pointer.clone()))
            .or_default()
            .push(record);
    }
    grouped
}

fn structure_signature(document: &Document) -> StructureSignature {
    (
        document.roots.clone(),
        document
            .entities
            .iter()
            .map(|(id, entity)| (*id, entity.kind.clone(), entity.children.clone()))
            .collect(),
    )
}

fn unmapped(document: &Document, reason: &str) -> AdapterError {
    AdapterError::UnmappedChanges {
        issues: 1,
        report: Box::new(AdapterReport {
            schema_version: 1,
            source_format: PROFILE_NAME.to_owned(),
            canonical_hash: None,
            fidelity: vec![FidelityEntry {
                target: CorrespondenceTarget::Document { id: document.id },
                pointer: String::new(),
                status: Fidelity::Unsupported {
                    reason: reason.to_owned(),
                },
            }],
            correspondences: Vec::new(),
            unmapped_source_preserved: true,
        }),
    }
}
