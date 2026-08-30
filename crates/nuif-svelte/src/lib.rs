#![doc = "Retentive static Svelte source profile adapter with byte-span correspondence."]

mod export;
mod parse;
mod profile;
mod sync;
mod syntax;

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

pub const MAX_SOURCE_BYTES: usize = 1024 * 1024;
pub const MAX_SYNTAX_NODES: usize = 16_384;
pub const MAX_ELEMENT_DEPTH: usize = 128;

#[derive(Clone, Debug, Error)]
pub enum AdapterError {
    #[error("source exceeds the {MAX_SOURCE_BYTES}-byte Svelte profile limit")]
    SourceTooLarge,
    #[error("Svelte syntax is not valid for the bounded profile: {0}")]
    SvelteSyntax(String),
    #[error("Svelte profile marker is missing or duplicated: {0}")]
    ProfileMarker(String),
    #[error("Svelte profile value is invalid at {pointer}: {reason}")]
    InvalidValue { pointer: String, reason: String },
    #[error("Svelte syntax-node limit exceeded")]
    NodeLimit,
    #[error("Svelte mapped element depth exceeds {MAX_ELEMENT_DEPTH}")]
    DepthLimit,
    #[error("document is outside the declared Svelte profile ({issues} fidelity issues)")]
    UnsupportedProfile {
        issues: usize,
        report: Box<AdapterReport>,
    },
    #[error("edited document changes {issues} properties without source correspondence")]
    UnmappedChanges {
        issues: usize,
        report: Box<AdapterReport>,
    },
    #[error("retentive Svelte span for {pointer} is stale")]
    StaleSpan { pointer: String },
    #[error("synchronized Svelte did not import to the edited document")]
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
    use nuif_core::{EntityKind, SizeIntent};

    #[test]
    fn declared_profile_exports_and_imports_exactly() {
        let document = profile_fixture();
        let exported = export_document(&document).unwrap();
        let imported = import_source(&exported.source).unwrap();
        assert_eq!(imported.document, document);
        assert!(exported.report.is_lossless());
        assert_eq!(exported.report.correspondences.len(), 21);
        assert!(exported.source.contains("style=\"width:320px"));
    }

    #[test]
    fn mapped_edits_preserve_comments() {
        let before = profile_fixture();
        let exported = export_document(&before).unwrap();
        let source = format!("<!-- user header stays byte-exact -->\n{}", exported.source);
        let imported = import_source(&source).unwrap();
        let mut after = before;
        after
            .entities
            .get_mut(&EntityId::new(0x10))
            .unwrap()
            .authored
            .width = SizeIntent::Fixed(360.0);
        let text = after.entities.get_mut(&EntityId::new(0x11)).unwrap();
        text.name = Some("Edited copy".to_owned());
        text.authored.text.as_mut().unwrap().content = "Edited <Svelte> & {source}".to_owned();
        let synchronized = synchronize(&imported.retentive, &after).unwrap();
        assert_eq!(import_source(&synchronized.source).unwrap().document, after);
        assert!(
            synchronized
                .source
                .starts_with("<!-- user header stays byte-exact -->")
        );
        assert!(
            synchronized
                .source
                .contains("Edited &lt;Svelte&gt; &amp; &#123;source&#125;")
        );
        assert_eq!(synchronized.edits.len(), 3);
    }

    #[test]
    fn executable_svelte_constructs_are_rejected() {
        let source = export_document(&profile_fixture()).unwrap().source;
        for changed in [
            source.replace("width:320px", "width:{width}px"),
            format!("<script>let width = 320;</script>\n{source}"),
            format!("{{#if true}}{source}{{/if}}"),
            source.replace("data-nuif-name=\"Card\"", "on:click={{handler}}"),
            format!("<style>div {{ width: 320px; }}</style>\n{source}"),
            source.replace("<span data-nuif-id", "<Widget />\n  <span data-nuif-id"),
        ] {
            assert!(
                import_source(&changed).is_err(),
                "executable source was accepted: {changed}"
            );
        }
    }

    #[test]
    fn unsupported_and_stale_edits_are_typed() {
        let document = profile_fixture();
        let exported = export_document(&document).unwrap();
        let imported = import_source(&exported.source).unwrap();
        let mut unsupported = document.clone();
        unsupported
            .entities
            .get_mut(&EntityId::new(0x11))
            .unwrap()
            .kind = EntityKind::Container;
        assert!(matches!(
            synchronize(&imported.retentive, &unsupported),
            Err(AdapterError::UnmappedChanges { .. })
        ));

        let mut stale = imported.retentive;
        let record = stale
            .report
            .correspondences
            .iter()
            .find(|record| record.pointer.ends_with("/authored/text/content"))
            .unwrap();
        stale.source.replace_range(
            record.span.start..record.span.end,
            &"X".repeat(record.span.end - record.span.start),
        );
        assert!(matches!(
            synchronize(&stale, &document),
            Err(AdapterError::StaleSpan { .. })
        ));
    }
}
