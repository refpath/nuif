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
}
