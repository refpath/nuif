#![doc = "Format-neutral correspondence, fidelity and retained-source contracts."]

use nuif_core::{Document, EntityId, Fidelity};
use serde::{Deserialize, Serialize};

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
}
