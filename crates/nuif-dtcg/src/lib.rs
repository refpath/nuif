#![doc = "Retentive DTCG 2025.10 flat scalar-token profile adapter."]

mod export;
mod parse;
mod profile;
mod sync;

use nuif_adapter::AdapterReport;
use thiserror::Error;

pub use export::export_document;
pub use nuif_adapter::{
    CorrespondenceRecord, CorrespondenceTarget, ExportedSource, FidelityEntry, ImportedSource,
    RetentiveSource, SourceEdit, SourceSpan, SynchronizedSource,
};
pub use parse::import_source;
pub use profile::profile_fixture;
pub use sync::synchronize;

pub const PROFILE_NAME: &str = "nuif-dtcg-scalar-0";
pub const NUIF_EXTENSION: &str = "org.nuif";
pub const MAX_SOURCE_BYTES: usize = 1024 * 1024;
pub const MAX_TOKENS: usize = 4_096;
pub const MAX_JSON_DEPTH: usize = 128;

#[derive(Clone, Debug, Error)]
pub enum AdapterError {
    #[error("source exceeds the {MAX_SOURCE_BYTES}-byte DTCG profile limit")]
    SourceTooLarge,
    #[error("DTCG JSON is invalid for the bounded profile: {0}")]
    JsonSyntax(String),
    #[error("DTCG profile marker is missing or inconsistent: {0}")]
    ProfileMarker(String),
    #[error("DTCG profile value is invalid at {pointer}: {reason}")]
    InvalidValue { pointer: String, reason: String },
    #[error("document is outside the declared DTCG profile ({issues} fidelity issues)")]
    UnsupportedProfile {
        issues: usize,
        report: Box<AdapterReport>,
    },
    #[error("edited document changes {issues} properties without source correspondence")]
    UnmappedChanges {
        issues: usize,
        report: Box<AdapterReport>,
    },
    #[error("retentive DTCG span for {pointer} is stale")]
    StaleSpan { pointer: String },
    #[error("synchronized DTCG did not import to the edited document")]
    SynchronizationMismatch,
    #[error("canonical hashing failed: {0}")]
    Canonical(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use nuif_core::{EntityId, PropertyValue, Token};

    #[test]
    fn declared_profile_exports_and_imports_exactly() {
        let document = profile_fixture();
        let exported = export_document(&document).unwrap();
        let imported = import_source(&exported.source).unwrap();
        assert_eq!(imported.document, document);
        assert!(exported.report.is_lossless());
        assert_eq!(exported.report.correspondences.len(), 21);
    }

    #[test]
    fn mapped_edits_preserve_unknown_extensions_and_whitespace() {
        let document = profile_fixture();
        let source = export_document(&document)
            .unwrap()
            .source
            .replacen(
                "\"org.nuif\": {",
                "\"com.example.retained\": {\"opaque\": [1, 2, 3]}, \"org.nuif\": {",
                1,
            )
            .replacen(
                "\"org.nuif\": {\"id\"",
                "\"com.example.token\": {\"opaque\": true}, \"org.nuif\": {\"id\"",
                1,
            )
            .replace("  \"enabled\"", "\n  \"enabled\"");
        let imported = import_source(&source).unwrap();
        let mut edited = document;
        edited.tokens.get_mut(&EntityId::new(0x101)).unwrap().name = "title".to_owned();
        edited.tokens.get_mut(&EntityId::new(0x102)).unwrap().value = PropertyValue::Integer(8);
        edited.tokens.get_mut(&EntityId::new(0x103)).unwrap().value = PropertyValue::Real(1.0);
        let first = synchronize(&imported.retentive, &edited).unwrap();
        let second = synchronize(&imported.retentive, &edited).unwrap();
        assert_eq!(first, second);
        assert_eq!(import_source(&first.source).unwrap().document, edited);
        assert!(
            first
                .source
                .contains("\"com.example.retained\": {\"opaque\": [1, 2, 3]}")
        );
        assert!(
            first
                .source
                .contains("\"com.example.token\": {\"opaque\": true}")
        );
        assert_unchanged_outside_edits(&source, &first.source, &first.edits);
    }

    #[test]
    fn structural_unsupported_and_stale_changes_are_typed() {
        let document = profile_fixture();
        let exported = export_document(&document).unwrap();
        let imported = import_source(&exported.source).unwrap();
        let mut structural = document.clone();
        let id = EntityId::new(0x104);
        structural.tokens.insert(
            id,
            Token {
                id,
                name: "added".to_owned(),
                value: PropertyValue::Boolean(true),
            },
        );
        assert!(matches!(
            synchronize(&imported.retentive, &structural),
            Err(AdapterError::UnmappedChanges { .. })
        ));

        let mut unsupported = document;
        unsupported
            .tokens
            .get_mut(&EntityId::new(0x100))
            .unwrap()
            .value = PropertyValue::Array(vec![]);
        assert!(matches!(
            synchronize(&imported.retentive, &unsupported),
            Err(AdapterError::UnmappedChanges { .. })
        ));

        let stale_source = exported
            .source
            .replacen("\"$value\": true", "\"$value\":false", 1);
        let mut stale = imported.retentive;
        stale.source = stale_source;
        assert!(matches!(
            synchronize(&stale, &stale.document),
            Err(AdapterError::StaleSpan { .. })
        ));
    }

    #[test]
    fn parser_rejects_duplicate_members_aliases_depth_and_size() {
        let source = export_document(&profile_fixture()).unwrap().source;
        let duplicate = source.replacen(
            "\"$type\": \"boolean\"",
            "\"$type\": \"boolean\", \"$type\": \"boolean\"",
            1,
        );
        assert!(matches!(
            import_source(&duplicate),
            Err(AdapterError::JsonSyntax(_))
        ));
        let alias = source.replacen("\"$value\": true", "\"$value\": \"{other}\"", 1);
        assert!(matches!(
            import_source(&alias),
            Err(AdapterError::InvalidValue { .. })
        ));
        let nested_value = "[".repeat(200) + &"]".repeat(200);
        let nested = source.replacen(
            "\"org.nuif\": {",
            &format!("\"com.example.deep\": {nested_value}, \"org.nuif\": {{"),
            1,
        );
        assert!(matches!(
            import_source(&nested),
            Err(AdapterError::JsonSyntax(_))
        ));
        assert!(matches!(
            import_source(&" ".repeat(MAX_SOURCE_BYTES + 1)),
            Err(AdapterError::SourceTooLarge)
        ));
    }

    fn assert_unchanged_outside_edits(before: &str, after: &str, edits: &[SourceEdit]) {
        let mut before_cursor = 0;
        let mut after_cursor = 0;
        for edit in edits {
            let unchanged = &before[before_cursor..edit.span.start];
            assert_eq!(
                unchanged,
                &after[after_cursor..after_cursor + unchanged.len()]
            );
            before_cursor = edit.span.end;
            after_cursor += unchanged.len() + edit.replacement.len();
        }
        assert_eq!(&before[before_cursor..], &after[after_cursor..]);
    }
}
