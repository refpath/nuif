#![doc = "Format-neutral correspondence, fidelity and retained-source contracts."]

use nuif_core::{Document, EntityId, Fidelity};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum CorrespondenceTarget {
    Document { id: EntityId },
    Entity { id: EntityId },
    Token { id: EntityId },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrespondenceRecord {
    pub target: CorrespondenceTarget,
    pub pointer: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FidelityEntry {
    pub target: CorrespondenceTarget,
    pub pointer: String,
    pub status: Fidelity,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdapterReport {
    pub schema_version: u32,
    pub source_format: String,
    pub canonical_hash: Option<String>,
    pub fidelity: Vec<FidelityEntry>,
    pub correspondences: Vec<CorrespondenceRecord>,
    pub unmapped_source_preserved: bool,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostDirection {
    Import,
    Export,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostCorrespondenceRecord {
    pub target: CorrespondenceTarget,
    pub host_object_id: String,
    pub host_property: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostAdapterReport {
    pub schema_version: u32,
    pub profile: String,
    pub direction: HostDirection,
    pub host_application: String,
    pub host_api_version: String,
    pub host_document_revision: Option<String>,
    pub canonical_hash: Option<String>,
    pub fidelity: Vec<FidelityEntry>,
    pub correspondences: Vec<HostCorrespondenceRecord>,
    pub unmapped_host_data_preserved: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostReportError {
    UnsupportedSchemaVersion(u32),
    EmptyProfile,
    EmptyHostApplication,
    EmptyHostApiVersion,
    EmptyHostObjectId(usize),
}

impl HostAdapterReport {
    #[must_use]
    pub fn is_lossless(&self) -> bool {
        self.fidelity
            .iter()
            .all(|entry| entry.status == Fidelity::Lossless)
    }

    #[must_use]
    pub fn validation_errors(&self) -> Vec<HostReportError> {
        let mut errors = Vec::new();
        if self.schema_version != 1 {
            errors.push(HostReportError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        if self.profile.trim().is_empty() {
            errors.push(HostReportError::EmptyProfile);
        }
        if self.host_application.trim().is_empty() {
            errors.push(HostReportError::EmptyHostApplication);
        }
        if self.host_api_version.trim().is_empty() {
            errors.push(HostReportError::EmptyHostApiVersion);
        }
        for (index, correspondence) in self.correspondences.iter().enumerate() {
            if correspondence.host_object_id.trim().is_empty() {
                errors.push(HostReportError::EmptyHostObjectId(index));
            }
        }
        errors
    }
}

impl AdapterReport {
    #[must_use]
    pub fn is_lossless(&self) -> bool {
        self.fidelity
            .iter()
            .all(|entry| entry.status == Fidelity::Lossless)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportedSource {
    pub source: String,
    pub report: AdapterReport,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RetentiveSource {
    pub source: String,
    pub document: Document,
    pub report: AdapterReport,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImportedSource {
    pub document: Document,
    pub retentive: RetentiveSource,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceEdit {
    pub target: CorrespondenceTarget,
    pub pointer: String,
    pub span: SourceSpan,
    pub replacement: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SynchronizedSource {
    pub source: String,
    pub edits: Vec<SourceEdit>,
    pub report: AdapterReport,
}

/// Failure while comparing retained scalar spans with canonical before/after
/// source. Format adapters translate this small mechanical error into their
/// public typed error vocabulary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScalarSyncError {
    DuplicateCorrespondence { pointer: String },
    SpanOutOfBounds { pointer: String },
    StaleSpan { pointer: String },
    CorrespondenceSetMismatch,
    OverlappingSpans { pointer: String },
}

type CorrespondenceKey = (CorrespondenceTarget, String);

/// Plans non-overlapping scalar replacements shared by retentive source adapters.
///
/// The retained, canonical-before and canonical-after inventories must contain
/// exactly one span for every `(target, pointer)` key. Returned edits are sorted
/// by ascending source offset and do not mutate the input source.
///
/// # Errors
///
/// Rejects duplicate or mismatched correspondence inventories, invalid spans,
/// stale retained values and overlapping changed spans.
pub fn plan_scalar_edits(
    retained_source: &str,
    retained_records: &[CorrespondenceRecord],
    canonical_before_source: &str,
    canonical_before_records: &[CorrespondenceRecord],
    canonical_after_source: &str,
    canonical_after_records: &[CorrespondenceRecord],
) -> Result<Vec<SourceEdit>, ScalarSyncError> {
    let retained = correspondence_map(retained_records)?;
    let before = correspondence_values(canonical_before_source, canonical_before_records)?;
    let after = correspondence_values(canonical_after_source, canonical_after_records)?;
    let retained_keys = retained.keys().cloned().collect::<BTreeSet<_>>();
    let before_keys = before.keys().cloned().collect::<BTreeSet<_>>();
    let after_keys = after.keys().cloned().collect::<BTreeSet<_>>();
    if retained_keys != before_keys || retained_keys != after_keys {
        return Err(ScalarSyncError::CorrespondenceSetMismatch);
    }

    let mut edits = Vec::new();
    for (key, record) in retained {
        let observed = retained_source
            .get(record.span.start..record.span.end)
            .ok_or_else(|| ScalarSyncError::SpanOutOfBounds {
                pointer: record.pointer.clone(),
            })?;
        if observed != before[&key] {
            return Err(ScalarSyncError::StaleSpan {
                pointer: record.pointer,
            });
        }
        if observed != after[&key] {
            edits.push(SourceEdit {
                target: record.target,
                pointer: record.pointer,
                span: record.span,
                replacement: after[&key].clone(),
            });
        }
    }
    edits.sort_by_key(|edit| edit.span.start);
    for pair in edits.windows(2) {
        if pair[0].span.end > pair[1].span.start {
            return Err(ScalarSyncError::OverlappingSpans {
                pointer: pair[1].pointer.clone(),
            });
        }
    }
    Ok(edits)
}

/// Applies an ascending, non-overlapping edit plan from the end of the source.
///
/// # Errors
///
/// Rejects an out-of-bounds or overlapping edit plan without returning partial
/// output.
pub fn apply_scalar_edits(source: &str, edits: &[SourceEdit]) -> Result<String, ScalarSyncError> {
    for (index, edit) in edits.iter().enumerate() {
        if edit.span.start > edit.span.end || source.get(edit.span.start..edit.span.end).is_none() {
            return Err(ScalarSyncError::SpanOutOfBounds {
                pointer: edit.pointer.clone(),
            });
        }
        if index > 0 && edits[index - 1].span.end > edit.span.start {
            return Err(ScalarSyncError::OverlappingSpans {
                pointer: edit.pointer.clone(),
            });
        }
    }
    let mut output = source.to_owned();
    for edit in edits.iter().rev() {
        output.replace_range(edit.span.start..edit.span.end, &edit.replacement);
    }
    Ok(output)
}

fn correspondence_map(
    records: &[CorrespondenceRecord],
) -> Result<BTreeMap<CorrespondenceKey, CorrespondenceRecord>, ScalarSyncError> {
    let mut map = BTreeMap::new();
    for record in records {
        let key = (record.target.clone(), record.pointer.clone());
        if map.insert(key, record.clone()).is_some() {
            return Err(ScalarSyncError::DuplicateCorrespondence {
                pointer: record.pointer.clone(),
            });
        }
    }
    Ok(map)
}

fn correspondence_values(
    source: &str,
    records: &[CorrespondenceRecord],
) -> Result<BTreeMap<CorrespondenceKey, String>, ScalarSyncError> {
    let mut values = BTreeMap::new();
    for record in records {
        let value = source
            .get(record.span.start..record.span.end)
            .ok_or_else(|| ScalarSyncError::SpanOutOfBounds {
                pointer: record.pointer.clone(),
            })?
            .to_owned();
        let key = (record.target.clone(), record.pointer.clone());
        if values.insert(key, value).is_some() {
            return Err(ScalarSyncError::DuplicateCorrespondence {
                pointer: record.pointer.clone(),
            });
        }
    }
    Ok(values)
}

/// A byte span inside one uncompressed package member.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageCorrespondenceRecord {
    pub target: CorrespondenceTarget,
    pub member: String,
    pub pointer: String,
    pub span: SourceSpan,
}

/// Fidelity evidence for archive-based adapters whose members are the
/// smallest retained source units.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageReport {
    pub schema_version: u32,
    pub source_format: String,
    pub canonical_hash: Option<String>,
    pub fidelity: Vec<FidelityEntry>,
    pub correspondences: Vec<PackageCorrespondenceRecord>,
    pub unmapped_member_payloads_preserved: bool,
}

impl PackageReport {
    #[must_use]
    pub fn is_lossless(&self) -> bool {
        self.fidelity
            .iter()
            .all(|entry| entry.status == Fidelity::Lossless)
    }
}

/// One semantic edit applied to an uncompressed package member.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageEdit {
    pub target: CorrespondenceTarget,
    pub member: String,
    pub pointer: String,
    pub span: SourceSpan,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_is_lossless_only_when_every_entry_is_lossless() {
        let target = CorrespondenceTarget::Document {
            id: EntityId::new(1),
        };
        let mut report = AdapterReport {
            schema_version: 1,
            source_format: "fixture".to_owned(),
            canonical_hash: None,
            fidelity: vec![FidelityEntry {
                target,
                pointer: String::new(),
                status: Fidelity::Lossless,
            }],
            correspondences: Vec::new(),
            unmapped_source_preserved: true,
        };
        assert!(report.is_lossless());
        report.fidelity[0].status = Fidelity::Unsupported {
            reason: "fixture".to_owned(),
        };
        assert!(!report.is_lossless());
    }

    #[test]
    fn host_report_round_trips_and_validates_required_identity() {
        let target = CorrespondenceTarget::Entity {
            id: EntityId::new(2),
        };
        let report = HostAdapterReport {
            schema_version: 1,
            profile: "nuif-figma-plugin-0".to_owned(),
            direction: HostDirection::Import,
            host_application: "figma".to_owned(),
            host_api_version: "1.0.0".to_owned(),
            host_document_revision: Some("42".to_owned()),
            canonical_hash: Some("nuif-cbor-0:sha256:fixture".to_owned()),
            fidelity: vec![FidelityEntry {
                target: target.clone(),
                pointer: "/entities/2".to_owned(),
                status: Fidelity::Lossless,
            }],
            correspondences: vec![HostCorrespondenceRecord {
                target,
                host_object_id: "7:11".to_owned(),
                host_property: None,
            }],
            unmapped_host_data_preserved: true,
        };
        assert!(report.validation_errors().is_empty());
        assert!(report.is_lossless());
        let encoded = serde_json::to_vec(&report).unwrap();
        assert_eq!(
            serde_json::from_slice::<HostAdapterReport>(&encoded).unwrap(),
            report
        );
    }

    #[test]
    fn host_report_rejects_empty_contract_fields() {
        let report = HostAdapterReport {
            schema_version: 2,
            profile: String::new(),
            direction: HostDirection::Export,
            host_application: String::new(),
            host_api_version: String::new(),
            host_document_revision: None,
            canonical_hash: None,
            fidelity: Vec::new(),
            correspondences: vec![HostCorrespondenceRecord {
                target: CorrespondenceTarget::Document {
                    id: EntityId::new(1),
                },
                host_object_id: " ".to_owned(),
                host_property: None,
            }],
            unmapped_host_data_preserved: false,
        };
        assert_eq!(
            report.validation_errors(),
            vec![
                HostReportError::UnsupportedSchemaVersion(2),
                HostReportError::EmptyProfile,
                HostReportError::EmptyHostApplication,
                HostReportError::EmptyHostApiVersion,
                HostReportError::EmptyHostObjectId(0),
            ]
        );
    }

    #[test]
    fn scalar_edit_planning_requires_fresh_equal_inventories() {
        let target = CorrespondenceTarget::Document {
            id: EntityId::new(1),
        };
        let records = vec![CorrespondenceRecord {
            target,
            pointer: "/id".to_owned(),
            span: SourceSpan { start: 1, end: 2 },
        }];
        let edits = plan_scalar_edits("[a]", &records, "[a]", &records, "[b]", &records)
            .expect("fresh correspondence should produce one edit");
        assert_eq!(edits.len(), 1);
        assert_eq!(apply_scalar_edits("[a]", &edits).unwrap(), "[b]");
        assert!(matches!(
            plan_scalar_edits("[x]", &records, "[a]", &records, "[b]", &records),
            Err(ScalarSyncError::StaleSpan { .. })
        ));
        assert!(matches!(
            plan_scalar_edits("[a]", &[], "[a]", &records, "[b]", &records),
            Err(ScalarSyncError::CorrespondenceSetMismatch)
        ));
    }
}
