use nuif_codec::canonical_hash;
use nuif_core::{Color, ColorSpace, EntityId, Fidelity, PropertyValue};
use nuif_html::{
    AdapterError, CorrespondenceTarget, SourceEdit, export_document, import_source,
    profile_fixture, synchronize,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    if let Err(error) = run() {
        eprintln!("html-sync-profile: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let (report_path, source_path) = output_paths()?;
    let before = profile_fixture();
    let exported = export_document(&before).map_err(|error| error.to_string())?;
    let retained_source = add_unmapped_regions(&exported.source);
    let imported = import_source(&retained_source).map_err(|error| error.to_string())?;
    let after = edited_document(&before);

    let synchronized = synchronize(&imported.retentive, &after)
        .map_err(|error| format!("mapped synchronization failed: {error}"))?;
    let repeated = synchronize(&imported.retentive, &after)
        .map_err(|error| format!("repeat synchronization failed: {error}"))?;
    let reimported = import_source(&synchronized.source).map_err(|error| error.to_string())?;
    let expected_pointers = expected_pointers();
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
        "mapped_edit_count_exact": synchronized.edits.len() == 6,
        "mapped_edit_pointers_exact": observed_pointers == expected_pointers,
        "unmapped_bytes_exact": unchanged_outside_edits(&retained_source, &synchronized.source, &synchronized.edits),
        "css_comment_preserved": synchronized.source.contains("/* user CSS comment stays byte-exact */"),
        "html_comment_preserved": synchronized.source.contains("<!-- user HTML comment stays byte-exact -->"),
        "unmapped_element_preserved": synchronized.source.contains("<aside data-user-region>untouched</aside>"),
        "fidelity_lossless": synchronized.report.is_lossless(),
        "unsupported_edit_typed": unsupported_edit_typed(&imported.retentive),
        "stale_span_typed": stale_span_typed(&imported.retentive),
        "source_limit_typed": matches!(import_source(&"x".repeat(nuif_html::MAX_SOURCE_BYTES + 1)), Err(AdapterError::SourceTooLarge)),
    });
    let passed = checks
        .as_object()
        .expect("checks is an object")
        .values()
        .all(|value| value == &Value::Bool(true));
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:html-css-retentive-sync",
        "status": if passed { "passed" } else { "failed" },
        "source": {
            "revision": command_text("git", &["rev-parse", "HEAD"]),
            "dirty": command_text("git", &["status", "--porcelain"]).map(|value| !value.is_empty()),
            "toolchain": command_text("rustc", &["--version"]),
            "os": env::consts::OS,
            "architecture": env::consts::ARCH,
        },
        "profile": {
            "name": nuif_html::PROFILE_NAME,
            "tree_sitter": "0.26.10",
            "tree_sitter_html": "0.23.2",
            "tree_sitter_css": "0.25.0",
            "source_limit_bytes": nuif_html::MAX_SOURCE_BYTES,
            "mapped_semantics": ["identity", "containment", "finite length token", "fixed size", "stack direction", "gap", "padding edges", "alignment", "text content and pinned font metadata"],
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
        "HTML/CSS retentive sync: {} mapped edits, status {}",
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
            "    /* user CSS comment stays byte-exact */\n    :root {",
        )
        .replace(
            "</body>",
            "  <!-- user HTML comment stays byte-exact -->\n  <aside data-user-region>untouched</aside>\n</body>",
        )
}

fn edited_document(before: &nuif_core::Document) -> nuif_core::Document {
    let mut after = before.clone();
    after.tokens.get_mut(&EntityId::new(0x100)).unwrap().value = PropertyValue::Real(32.0);
    let layout = &mut after
        .entities
        .get_mut(&EntityId::new(0x10))
        .unwrap()
        .authored
        .layout;
    layout.padding.top = 30.0;
    layout.padding.right = 31.0;
    layout.padding.bottom = 32.0;
    layout.padding.left = 33.0;
    "Edited & verified".clone_into(
        &mut after
            .entities
            .get_mut(&EntityId::new(0x11))
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
        "/tokens/00000000000000000000000000000100/value",
        "/entities/00000000000000000000000000000010/authored/layout/padding/top",
        "/entities/00000000000000000000000000000010/authored/layout/padding/right",
        "/entities/00000000000000000000000000000010/authored/layout/padding/bottom",
        "/entities/00000000000000000000000000000010/authored/layout/padding/left",
        "/entities/00000000000000000000000000000011/authored/text/content",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn unsupported_edit_typed(retentive: &nuif_html::RetentiveSource) -> bool {
    let mut unsupported = retentive.document.clone();
    unsupported
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
    let Err(AdapterError::UnmappedChanges { report, .. }) = synchronize(retentive, &unsupported)
    else {
        return false;
    };
    report.fidelity.iter().any(|entry| {
        entry.target
            == (CorrespondenceTarget::Entity {
                id: EntityId::new(0x10),
            })
            && entry.pointer == "/entities/00000000000000000000000000000010/authored"
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
        synchronize(&stale, &retentive.document),
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
    let mut report = PathBuf::from("target/html-sync-report.json");
    let mut source = PathBuf::from("target/html-sync-output.html");
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--output" => {
                report = PathBuf::from(args.next().ok_or("--output requires a path")?);
            }
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
