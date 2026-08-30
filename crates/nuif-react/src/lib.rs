#![doc = "Retentive static React JSX profile adapter with byte-span correspondence."]

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
pub const MAX_JSX_DEPTH: usize = 128;

#[derive(Clone, Debug, Error)]
pub enum AdapterError {
    #[error("source exceeds the {MAX_SOURCE_BYTES}-byte React JSX profile limit")]
    SourceTooLarge,
    #[error("React JSX syntax is not valid for the bounded profile: {0}")]
    JsxSyntax(String),
    #[error("React JSX profile marker is missing or duplicated: {0}")]
    ProfileMarker(String),
    #[error("React JSX profile value is invalid at {pointer}: {reason}")]
    InvalidValue { pointer: String, reason: String },
    #[error("React JSX syntax-node limit exceeded")]
    NodeLimit,
    #[error("React JSX mapped element depth exceeds {MAX_JSX_DEPTH}")]
    DepthLimit,
    #[error("document is outside the declared React JSX profile ({issues} fidelity issues)")]
    UnsupportedProfile {
        issues: usize,
        report: Box<AdapterReport>,
    },
    #[error("edited document changes {issues} properties without source correspondence")]
    UnmappedChanges {
        issues: usize,
        report: Box<AdapterReport>,
    },
    #[error("retentive React JSX span for {pointer} is stale")]
    StaleSpan { pointer: String },
    #[error("synchronized React JSX did not import to the edited document")]
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
    use nuif_core::{EntityKind, Fidelity, SizeIntent};
    use std::fmt::Write as _;

    #[test]
    fn declared_profile_exports_and_imports_exactly() {
        let document = profile_fixture();
        let exported = export_document(&document).unwrap();
        let imported = import_source(&exported.source).unwrap();
        assert_eq!(imported.document, document);
        assert!(exported.report.is_lossless());
        assert_eq!(exported.report.correspondences.len(), 21);
        assert!(exported.source.contains("style={{ width: 320"));
    }

    #[test]
    fn mapped_edits_preserve_unmapped_javascript_and_comments() {
        let before = profile_fixture();
        let exported = export_document(&before).unwrap();
        let source = format!(
            "// user module comment stays byte-exact\n{}\nexport const untouched = 42;\n",
            exported.source
        );
        let imported = import_source(&source).unwrap();
        let mut after = before;
        let root = after.entities.get_mut(&EntityId::new(0x10)).unwrap();
        root.authored.width = SizeIntent::Fixed(360.0);
        root.authored.layout.gap = 20.0;
        let text = after.entities.get_mut(&EntityId::new(0x11)).unwrap();
        text.name = Some("Edited copy".to_owned());
        let content = text.authored.text.as_mut().unwrap();
        content.content = "Edited <React> & {source}".to_owned();
        content.size = 20.0;
        let synchronized = synchronize(&imported.retentive, &after).unwrap();
        assert_eq!(import_source(&synchronized.source).unwrap().document, after);
        assert!(
            synchronized
                .source
                .starts_with("// user module comment stays byte-exact")
        );
        assert!(
            synchronized
                .source
                .ends_with("export const untouched = 42;\n")
        );
        assert!(
            synchronized
                .source
                .contains("Edited &lt;React&gt; &amp; &#123;source&#125;")
        );
        assert_eq!(synchronized.edits.len(), 5);
        assert_unchanged_outside_edits(&source, &synchronized.source, &synchronized.edits);
    }

    #[test]
    fn runtime_jsx_constructs_are_rejected() {
        let source = export_document(&profile_fixture()).unwrap().source;
        for changed in [
            source.replace("width: 320", "width: props.width"),
            source.replace("data-nuif-name=\"Card\"", "{...props}"),
            source.replace(
                "    <span data-nuif-id",
                "    <Card />\n    <span data-nuif-id",
            ),
            source
                .replace("return (", "return condition ? (")
                .replace("  );", "  ) : null;"),
            source.replace("export default function", "export /* default */ function"),
            source.replace("export default function", "export default async function"),
        ] {
            assert!(
                import_source(&changed).is_err(),
                "dynamic source was accepted: {changed}"
            );
        }
    }

    #[test]
    fn mapped_depth_limit_is_typed() {
        let source = deep_source(MAX_JSX_DEPTH + 2);
        assert!(matches!(
            import_source(&source),
            Err(AdapterError::DepthLimit)
        ));
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
        assert!(imported_report_has_unsupported_fidelity(&unsupported));
    }

    fn imported_report_has_unsupported_fidelity(document: &nuif_core::Document) -> bool {
        let Err(AdapterError::UnsupportedProfile { report, .. }) = export_document(document) else {
            return false;
        };
        report
            .fidelity
            .iter()
            .any(|entry| matches!(entry.status, Fidelity::Unsupported { .. }))
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

    fn deep_source(elements: usize) -> String {
        const STYLE: &str = "style={{ width: 1, height: 1, boxSizing: \"border-box\", display: \"flex\", flexDirection: \"column\", gap: 0, paddingTop: 0, paddingRight: 0, paddingBottom: 0, paddingLeft: 0, alignItems: \"stretch\" }}";
        let mut source = String::from("export default function NuifDocument() {\n  return (\n");
        for index in 0..elements {
            let id = format!("{:032x}", index + 0x100);
            source.push_str(&"  ".repeat(index + 2));
            write!(
                source,
                "<div data-nuif-id=\"{id}\" data-nuif-kind=\"container\" data-nuif-name=\"Depth {index}\""
            )
            .expect("writing into a String cannot fail");
            if index == 0 {
                write!(
                    source,
                    " data-nuif-profile=\"{PROFILE_NAME}\" data-nuif-document=\"{:032x}\"",
                    1
                )
                .expect("writing into a String cannot fail");
            }
            source.push(' ');
            source.push_str(STYLE);
            source.push_str(">\n");
        }
        for index in (0..elements).rev() {
            source.push_str(&"  ".repeat(index + 2));
            source.push_str("</div>\n");
        }
        source.push_str("  );\n}\n");
        source
    }
}
