#![doc = "Retentive SVG 2 basic-shape profile adapter."]

mod export;
mod parse;
mod profile;
mod sync;

use nuif_adapter::AdapterReport;
use nuif_core::EntityId;
use thiserror::Error;

pub use export::export_document;
pub use nuif_adapter::{
    CorrespondenceRecord, CorrespondenceTarget, ExportedSource, FidelityEntry, ImportedSource,
    RetentiveSource, SourceEdit, SourceSpan, SynchronizedSource,
};
pub use parse::import_source;
pub use profile::profile_fixture;
pub use sync::synchronize;

pub const PROFILE_NAME: &str = "nuif-svg-0";
pub const MAX_SOURCE_BYTES: usize = 1024 * 1024;
pub const MAX_XML_NODES: u32 = 16_384;
pub const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

#[derive(Clone, Debug, Error)]
pub enum AdapterError {
    #[error("source exceeds the {MAX_SOURCE_BYTES}-byte SVG profile limit")]
    SourceTooLarge,
    #[error("SVG XML is invalid for the bounded profile: {0}")]
    XmlSyntax(String),
    #[error("SVG profile marker is missing or inconsistent: {0}")]
    ProfileMarker(String),
    #[error("SVG profile value is invalid at {pointer}: {reason}")]
    InvalidValue { pointer: String, reason: String },
    #[error("document is outside the declared SVG profile ({issues} fidelity issues)")]
    UnsupportedProfile {
        issues: usize,
        report: Box<AdapterReport>,
    },
    #[error("edited document changes {issues} properties without source correspondence")]
    UnmappedChanges {
        issues: usize,
        report: Box<AdapterReport>,
    },
    #[error("retentive SVG span for {pointer} is stale")]
    StaleSpan { pointer: String },
    #[error("synchronized SVG did not import to the edited document")]
    SynchronizationMismatch,
    #[error("canonical hashing failed: {0}")]
    Canonical(String),
}

pub(crate) fn entity_pointer(id: EntityId, suffix: &str) -> String {
    format!("/entities/{id}{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use nuif_core::{Entity, EntityId, EntityKind, Fidelity, ShapeKind};

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
    fn mapped_edits_preserve_unmapped_svg_bytes() {
        let document = profile_fixture();
        let source = export_document(&document)
            .unwrap()
            .source
            .replace(
                "  <g",
                "  <!-- retained comment -->\n  <metadata data-user-region=\"yes\">retained</metadata>\n  <g",
            );
        let imported = import_source(&source).unwrap();
        let mut edited = document;
        edited
            .entities
            .get_mut(&EntityId::new(0x21))
            .unwrap()
            .authored
            .position
            .x = 22.0;
        edited
            .entities
            .get_mut(&EntityId::new(0x22))
            .unwrap()
            .authored
            .width = nuif_core::SizeIntent::Fixed(30.0);
        edited
            .entities
            .get_mut(&EntityId::new(0x23))
            .unwrap()
            .authored
            .text
            .as_mut()
            .unwrap()
            .content = "NUIF <SVG> & source".to_owned();

        let first = synchronize(&imported.retentive, &edited).unwrap();
        let second = synchronize(&imported.retentive, &edited).unwrap();
        assert_eq!(first, second);
        assert_eq!(import_source(&first.source).unwrap().document, edited);
        assert!(first.source.contains("<!-- retained comment -->"));
        assert!(
            first
                .source
                .contains("<metadata data-user-region=\"yes\">retained</metadata>")
        );
        assert!(first.source.contains("NUIF &lt;SVG&gt; &amp; source"));
        assert_unchanged_outside_edits(&source, &first.source, &first.edits);
    }

    #[test]
    fn stale_and_structural_changes_fail_atomically() {
        let document = profile_fixture();
        let mut imported = import_source(&export_document(&document).unwrap().source).unwrap();
        imported.retentive.source = imported
            .retentive
            .source
            .replacen("x=\"16\"", "x=\"17\"", 1);
        assert!(matches!(
            synchronize(&imported.retentive, &document),
            Err(AdapterError::StaleSpan { .. })
        ));

        let imported = import_source(&export_document(&document).unwrap().source).unwrap();
        let mut structural = document;
        structural
            .entities
            .get_mut(&EntityId::new(0x20))
            .unwrap()
            .children
            .swap(0, 1);
        assert!(matches!(
            synchronize(&imported.retentive, &structural),
            Err(AdapterError::UnmappedChanges { .. })
        ));
    }

    #[test]
    fn unsupported_kind_has_property_attributed_fidelity() {
        let mut document = profile_fixture();
        let id = EntityId::new(0x24);
        let path = Entity::new(id, EntityKind::Shape(ShapeKind::Path));
        document
            .entities
            .get_mut(&EntityId::new(0x20))
            .unwrap()
            .children
            .push(id);
        document.entities.insert(id, path);
        let AdapterError::UnsupportedProfile { report, .. } =
            export_document(&document).unwrap_err()
        else {
            panic!("path export returned the wrong error");
        };
        assert!(report.fidelity.iter().any(|entry| {
            entry.target == CorrespondenceTarget::Entity { id }
                && entry.pointer.ends_with("/kind")
                && matches!(entry.status, Fidelity::Unsupported { .. })
        }));
    }

    #[test]
    fn parser_rejects_dtd_and_source_limit() {
        let source = export_document(&profile_fixture()).unwrap().source;
        let dtd = source.replacen("<svg", "<!DOCTYPE svg [<!ENTITY x \"expanded\">]>\n<svg", 1);
        assert!(matches!(
            import_source(&dtd),
            Err(AdapterError::XmlSyntax(_))
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
