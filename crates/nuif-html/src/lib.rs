#![doc = "Retentive HTML/CSS profile adapter with byte-span correspondence."]

mod export;
mod parse;
mod profile;
mod sync;
mod syntax;
mod v0;

use nuif_core::EntityId;
use thiserror::Error;

pub use export::export_document;
pub use nuif_adapter::{
    AdapterReport, CorrespondenceRecord, CorrespondenceTarget, ExportedSource, FidelityEntry,
    ImportedSource, RetentiveSource, SourceEdit, SourceSpan, SynchronizedSource,
};
pub use parse::import_source;
pub use profile::{PROFILE_NAME, profile_fixture};
pub use sync::synchronize;
pub use v0::{V0_PROFILE_NAME, export_v0_document, import_v0_source, synchronize_v0};

pub const MAX_SOURCE_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, Error)]
pub enum AdapterError {
    #[error("source exceeds the {MAX_SOURCE_BYTES}-byte HTML/CSS profile limit")]
    SourceTooLarge,
    #[error("HTML syntax is not valid for the bounded profile: {0}")]
    HtmlSyntax(String),
    #[error("CSS syntax is not valid for the bounded profile: {0}")]
    CssSyntax(String),
    #[error("HTML/CSS profile marker is missing or duplicated: {0}")]
    ProfileMarker(String),
    #[error("HTML/CSS profile value is invalid at {pointer}: {reason}")]
    InvalidValue { pointer: String, reason: String },
    #[error("document is outside the declared HTML/CSS profile ({issues} fidelity issues)")]
    UnsupportedProfile {
        issues: usize,
        report: Box<AdapterReport>,
    },
    #[error("edited document changes {issues} properties without source correspondence")]
    UnmappedChanges {
        issues: usize,
        report: Box<AdapterReport>,
    },
    #[error("retentive source span for {pointer} is stale")]
    StaleSpan { pointer: String },
    #[error("synchronized HTML/CSS did not import to the edited document")]
    SynchronizationMismatch,
    #[error("canonical hashing failed: {0}")]
    Canonical(String),
}

pub(crate) fn entity_pointer(id: EntityId, suffix: &str) -> String {
    format!("/entities/{id}{suffix}")
}

pub(crate) fn token_pointer(id: EntityId, suffix: &str) -> String {
    format!("/tokens/{id}{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use nuif_core::{Color, ColorSpace, Fidelity, PropertyValue};
    use std::collections::BTreeSet;

    #[test]
    fn declared_profile_exports_and_imports_exactly() {
        let document = profile_fixture();
        let exported = export_document(&document).unwrap();
        let imported = import_source(&exported.source).unwrap();
        assert_eq!(imported.document, document);
        assert!(exported.report.is_lossless());
        assert!(!exported.report.correspondences.is_empty());
        for record in &exported.report.correspondences {
            assert!(record.span.start <= record.span.end);
            assert!(record.span.end <= exported.source.len());
            assert!(exported.source.is_char_boundary(record.span.start));
            assert!(exported.source.is_char_boundary(record.span.end));
        }
    }

    #[test]
    fn mapped_edits_preserve_every_unmapped_byte() {
        let document = profile_fixture();
        let exported = export_document(&document).unwrap();
        let source = exported
            .source
            .replace(
                "    :root {",
                "    /* user CSS comment stays byte-exact */\n    :root {",
            )
            .replace(
                "</body>",
                "  <!-- user HTML comment stays byte-exact -->\n  <aside data-user-region>untouched</aside>\n</body>",
            );
        let imported = import_source(&source).unwrap();
        assert_eq!(imported.document, document);

        let mut edited = document;
        edited.tokens.get_mut(&EntityId::new(0x100)).unwrap().value = PropertyValue::Real(32.0);
        let card = &mut edited
            .entities
            .get_mut(&EntityId::new(0x10))
            .unwrap()
            .authored
            .layout;
        card.padding.top = 30.0;
        card.padding.right = 31.0;
        card.padding.bottom = 32.0;
        card.padding.left = 33.0;
        edited
            .entities
            .get_mut(&EntityId::new(0x11))
            .unwrap()
            .authored
            .text
            .as_mut()
            .unwrap()
            .content = "Edited & verified".to_owned();

        let synchronized = synchronize(&imported.retentive, &edited).unwrap();
        assert_eq!(
            import_source(&synchronized.source).unwrap().document,
            edited
        );
        assert!(synchronized.report.unmapped_source_preserved);
        assert!(
            synchronized
                .source
                .contains("/* user CSS comment stays byte-exact */")
        );
        assert!(
            synchronized
                .source
                .contains("<!-- user HTML comment stays byte-exact -->")
        );
        assert!(
            synchronized
                .source
                .contains("<aside data-user-region>untouched</aside>")
        );
        assert_eq!(synchronized.edits.len(), 6);
        let pointers = synchronized
            .edits
            .iter()
            .map(|edit| edit.pointer.as_str())
            .collect::<BTreeSet<_>>();
        assert!(pointers.contains("/tokens/00000000000000000000000000000100/value"));
        assert!(
            pointers.contains("/entities/00000000000000000000000000000011/authored/text/content")
        );
        assert_unchanged_outside_edits(&source, &synchronized.source, &synchronized.edits);
    }

    #[test]
    fn unsupported_property_edit_has_entity_and_pointer_fidelity() {
        let document = profile_fixture();
        let imported = import_source(&export_document(&document).unwrap().source).unwrap();
        let mut edited = document;
        edited
            .entities
            .get_mut(&EntityId::new(0x10))
            .unwrap()
            .authored
            .fill = Some(Color {
            space: ColorSpace::Srgb,
            red: 0.1,
            green: 0.2,
            blue: 0.3,
            alpha: 1.0,
        });
        let AdapterError::UnmappedChanges { report, .. } =
            synchronize(&imported.retentive, &edited).unwrap_err()
        else {
            panic!("unsupported edit returned the wrong error");
        };
        assert!(report.fidelity.iter().any(|entry| {
            entry.target
                == CorrespondenceTarget::Entity {
                    id: EntityId::new(0x10),
                }
                && entry.pointer == "/entities/00000000000000000000000000000010/authored"
                && matches!(entry.status, Fidelity::Unsupported { .. })
        }));
    }

    #[test]
    fn stale_mapped_span_is_rejected_before_editing() {
        let document = profile_fixture();
        let mut imported = import_source(&export_document(&document).unwrap().source).unwrap();
        let record = imported
            .retentive
            .report
            .correspondences
            .iter()
            .find(|record| record.pointer.ends_with("/authored/text/content"))
            .unwrap()
            .clone();
        imported.retentive.source.replace_range(
            record.span.start..record.span.end,
            "stale stale stale stale",
        );
        let error = synchronize(&imported.retentive, &document).unwrap_err();
        assert!(matches!(error, AdapterError::StaleSpan { .. }));
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
