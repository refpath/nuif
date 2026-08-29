use nuif_codec::canonical_hash;
use nuif_core::{EntityId, Fidelity, FlowDirection, PropertyValue};
use nuif_html::{
    AdapterError, CorrespondenceTarget, SourceEdit, export_v0_document, import_v0_source,
    synchronize_v0,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const CARD: EntityId = EntityId::new(0x20);
const COPY: EntityId = EntityId::new(0x22);
const BUTTON_INSTANCE: EntityId = EntityId::new(0x24);
const UNKNOWN: EntityId = EntityId::new(0x25);
const ICON: EntityId = EntityId::new(0x26);
const SPACE_TOKEN: EntityId = EntityId::new(0x101);

fn main() {
    if let Err(error) = run() {
        eprintln!("html-sync-v0: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let (report_path, source_path) = output_paths()?;
    let before = nuif_testing::responsive_card_fixture();
    let exported = export_v0_document(&before).map_err(|error| error.to_string())?;
    let retained_source = add_unmapped_regions(&exported.source);
    let imported = import_v0_source(&retained_source).map_err(|error| error.to_string())?;
    let after = edited_document(&before);
    let synchronized = synchronize_v0(&imported.retentive, &after)
        .map_err(|error| format!("mapped synchronization failed: {error}"))?;
    let repeated = synchronize_v0(&imported.retentive, &after)
        .map_err(|error| format!("repeat synchronization failed: {error}"))?;
    let reimported = import_v0_source(&synchronized.source).map_err(|error| error.to_string())?;
    let observed_pointers = synchronized
        .edits
        .iter()
        .map(|edit| edit.pointer.clone())
        .collect::<Vec<_>>();
    let checks = json!({
        "export_import_exact": imported.document == before,
        "synchronized_import_exact": reimported.document == after,
        "repeat_source_exact": repeated.source == synchronized.source,
        "repeat_edits_exact": repeated.edits == synchronized.edits,
        "mapped_edit_count_exact": synchronized.edits.len() == 8,
        "mapped_edit_pointers_exact": observed_pointers == expected_pointers(),
        "unmapped_bytes_exact": unchanged_outside_edits(&retained_source, &synchronized.source, &synchronized.edits),
        "css_comment_preserved": synchronized.source.contains("/* user CSS comment stays byte-exact */"),
        "unmapped_css_rule_preserved": synchronized.source.contains("body { margin: 0; }") ,
        "html_comment_preserved": synchronized.source.contains("<!-- user HTML comment stays byte-exact -->"),
        "unmapped_element_preserved": synchronized.source.contains("<aside data-user-region=\"true\">untouched</aside>"),
        "responsive_css_consistent": responsive_css_consistent(&synchronized.source),
        "opaque_unknown_exact": unknown_payload_exact(&before, &reimported.document),
        "all_correspondences_lossless": correspondences_are_lossless(&synchronized.report),
        "target_limits_explicit": target_limits_explicit(&synchronized.report),
        "unsupported_token_edit_typed": unsupported_token_edit_typed(&imported.retentive),
        "structural_edit_typed": structural_edit_typed(&imported.retentive),
        "stale_span_typed": stale_span_typed(&imported.retentive),
        "derived_css_drift_typed": derived_css_drift_typed(&imported.retentive),
        "source_limit_typed": matches!(import_v0_source(&"x".repeat(nuif_html::MAX_SOURCE_BYTES + 1)), Err(AdapterError::SourceTooLarge)),
    });
    let passed = all_checks_pass(&checks);
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:html-css-v0-responsive-card",
        "status": if passed { "passed" } else { "failed" },
        "source": {
            "revision": command_text("git", &["rev-parse", "HEAD"]),
            "dirty": command_text("git", &["status", "--porcelain"]).map(|value| !value.is_empty()),
            "toolchain": command_text("rustc", &["--version"]),
            "os": env::consts::OS,
            "architecture": env::consts::ARCH,
        },
        "profile": {
            "name": nuif_html::V0_PROFILE_NAME,
            "tree_sitter": "0.26.10",
            "tree_sitter_html": "0.23.2",
            "tree_sitter_css": "0.25.0",
            "source_limit_bytes": nuif_html::MAX_SOURCE_BYTES,
            "mapped_semantics": [
                "document metadata", "identity", "DOM containment", "entity kinds",
                "tokens", "size intents", "position", "layout", "fill", "text",
                "responsive width rules", "property values", "semantics", "opaque extensions"
            ],
            "target_limits": [
                "path geometry is preserved but not rendered",
                "component instances are preserved but not materialized",
                "unknown kinds and extensions are preserved but not rendered"
            ],
        },
        "before": {
            "canonical_hash": canonical_hash(&before).map_err(|error| error.to_string())?,
            "source_sha256": sha256(exported.source.as_bytes()),
        },
        "after": {
            "canonical_hash": canonical_hash(&after).map_err(|error| error.to_string())?,
            "source_sha256": sha256(synchronized.source.as_bytes()),
        },
        "summary": {
            "mapped_edits": synchronized.edits.len(),
            "correspondences": synchronized.report.correspondences.len(),
            "fidelity_entries": synchronized.report.fidelity.len(),
            "blocking_failures": u8::from(!passed),
        },
        "checks": checks,
        "edits": synchronized.edits,
        "fidelity": synchronized.report.fidelity,
    });
    write_file(
        &report_path,
        &serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )?;
    write_file(&source_path, synchronized.source.as_bytes())?;
    println!(
        "full v0 HTML/CSS sync: {} mapped edits, status {}",
        report["summary"]["mapped_edits"], report["status"]
    );
    if passed {
        Ok(())
    } else {
        Err(format!("report failed; inspect {}", report_path.display()))
    }
}

fn add_unmapped_regions(source: &str) -> String {
    source
        .replace(
            "    :root {",
            "    /* user CSS comment stays byte-exact */\n    body { margin: 0; }\n    :root {",
        )
        .replace(
            "</body>",
            "  <!-- user HTML comment stays byte-exact -->\n  <aside data-user-region=\"true\">untouched</aside>\n</body>",
        )
}

fn all_checks_pass(checks: &Value) -> bool {
    checks
        .as_object()
        .expect("checks is an object")
        .values()
        .all(|value| value == &Value::Bool(true))
}

fn edited_document(before: &nuif_core::Document) -> nuif_core::Document {
    let mut after = before.clone();
    after.tokens.get_mut(&SPACE_TOKEN).unwrap().value = PropertyValue::Real(28.0);
    let card = after.entities.get_mut(&CARD).unwrap();
    card.authored.layout.padding.top = 28.0;
    card.authored.layout.padding.right = 29.0;
    card.authored.layout.padding.bottom = 30.0;
    card.authored.layout.padding.left = 31.0;
    let responsive = card.authored.responsive.first_mut().unwrap();
    responsive.when.min_width = Some(800.0);
    responsive.direction = Some(FlowDirection::Column);
    responsive.gap = Some(28.0);
    "Portable & verified <intent>".clone_into(
        &mut after
            .entities
            .get_mut(&COPY)
            .unwrap()
            .authored
            .text
            .as_mut()
            .unwrap()
            .content,
    );
    after
}

fn expected_pointers() -> Vec<String> {
    [
        "/tokens/00000000000000000000000000000101/value",
        "/entities/00000000000000000000000000000020/authored/layout/padding/top",
        "/entities/00000000000000000000000000000020/authored/layout/padding/right",
        "/entities/00000000000000000000000000000020/authored/layout/padding/bottom",
        "/entities/00000000000000000000000000000020/authored/layout/padding/left",
        "/entities/00000000000000000000000000000020/authored/responsive/rendered/0",
        "/entities/00000000000000000000000000000020/authored/responsive",
        "/entities/00000000000000000000000000000022/authored/text/content",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn responsive_css_consistent(source: &str) -> bool {
    source.contains("@media (min-width: 800px)")
        && source.contains("flex-direction: column;")
        && source.contains("gap: 28px;")
}

fn unknown_payload_exact(before: &nuif_core::Document, after: &nuif_core::Document) -> bool {
    let before = &before.entities[&UNKNOWN];
    let after = &after.entities[&UNKNOWN];
    before.kind == after.kind && before.extensions == after.extensions
}

fn correspondences_are_lossless(report: &nuif_html::AdapterReport) -> bool {
    report.correspondences.iter().all(|record| {
        report.fidelity.iter().any(|entry| {
            entry.target == record.target
                && entry.pointer == record.pointer
                && entry.status == Fidelity::Lossless
        })
    })
}

fn target_limits_explicit(report: &nuif_html::AdapterReport) -> bool {
    let has = |id, suffix: &str, class: fn(&Fidelity) -> bool| {
        report.fidelity.iter().any(|entry| {
            entry.target == (CorrespondenceTarget::Entity { id })
                && entry.pointer.ends_with(suffix)
                && class(&entry.status)
        })
    };
    has(ICON, "/kind", |status| {
        matches!(status, Fidelity::Unsupported { .. })
    }) && has(BUTTON_INSTANCE, "/kind", |status| {
        matches!(status, Fidelity::Unsupported { .. })
    }) && has(UNKNOWN, "/kind", |status| {
        matches!(status, Fidelity::PreservedUnrenderable { .. })
    }) && has(UNKNOWN, "/extensions/vendor.probe", |status| {
        matches!(status, Fidelity::PreservedUnrenderable { .. })
    })
}

fn unsupported_token_edit_typed(retentive: &nuif_html::RetentiveSource) -> bool {
    let mut unsupported = retentive.document.clone();
    unsupported.tokens.get_mut(&SPACE_TOKEN).unwrap().value = PropertyValue::Boolean(true);
    let Err(AdapterError::UnmappedChanges { report, .. }) = synchronize_v0(retentive, &unsupported)
    else {
        return false;
    };
    report.fidelity.iter().any(|entry| {
        entry.target == (CorrespondenceTarget::Token { id: SPACE_TOKEN })
            && entry.pointer.ends_with("/value")
            && matches!(entry.status, Fidelity::Unsupported { .. })
    })
}

fn structural_edit_typed(retentive: &nuif_html::RetentiveSource) -> bool {
    let mut structural = retentive.document.clone();
    structural
        .entities
        .get_mut(&CARD)
        .unwrap()
        .children
        .swap(0, 1);
    let Err(AdapterError::UnmappedChanges { report, .. }) = synchronize_v0(retentive, &structural)
    else {
        return false;
    };
    report.fidelity.iter().any(|entry| {
        entry.target == (CorrespondenceTarget::Entity { id: CARD })
            && entry.pointer.ends_with("/children")
            && matches!(entry.status, Fidelity::Unsupported { .. })
    })
}

fn stale_span_typed(retentive: &nuif_html::RetentiveSource) -> bool {
    let mut stale = retentive.clone();
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
    matches!(
        synchronize_v0(&stale, &retentive.document),
        Err(AdapterError::StaleSpan { .. })
    )
}

fn derived_css_drift_typed(retentive: &nuif_html::RetentiveSource) -> bool {
    let mut stale = retentive.clone();
    let record = stale
        .report
        .correspondences
        .iter()
        .find(|record| record.pointer.ends_with("/authored/responsive/rendered/0"))
        .unwrap();
    stale
        .source
        .replace_range(record.span.start..=record.span.start, " ");
    matches!(
        import_v0_source(&stale.source),
        Err(AdapterError::InvalidValue { .. })
    ) && matches!(
        synchronize_v0(&stale, &retentive.document),
        Err(AdapterError::StaleSpan { .. })
    )
}

fn unchanged_outside_edits(before: &str, after: &str, edits: &[SourceEdit]) -> bool {
    let mut before_cursor = 0;
    let mut after_cursor = 0;
    for edit in edits {
        let unchanged = &before[before_cursor..edit.span.start];
        let Some(observed) = after.get(after_cursor..after_cursor + unchanged.len()) else {
            return false;
        };
        if observed != unchanged {
            return false;
        }
        before_cursor = edit.span.end;
        after_cursor += unchanged.len() + edit.replacement.len();
    }
    before[before_cursor..] == after[after_cursor..]
}

fn output_paths() -> Result<(PathBuf, PathBuf), String> {
    let mut args = env::args().skip(1);
    let mut report = PathBuf::from("target/html-sync-v0-report.json");
    let mut source = PathBuf::from("target/html-sync-v0-output.html");
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--output" => report = PathBuf::from(args.next().ok_or("--output requires a path")?),
            "--source-output" => {
                source = PathBuf::from(args.next().ok_or("--source-output requires a path")?);
            }
            _ => return Err(format!("unknown argument {argument}")),
        }
    }
    Ok((report, source))
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(path, bytes).map_err(|error| error.to_string())
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn command_text(program: &str, arguments: &[&str]) -> Option<String> {
    let output = Command::new(program).args(arguments).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
