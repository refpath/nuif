use nuif_codec::{CanonicalText, Encoder, canonical_hash};
use nuif_core::{Color, ColorSpace, EntityId, Fidelity};
use nuif_svg::{
    AdapterError, CorrespondenceTarget, MAX_SOURCE_BYTES, MAX_XML_NODES, SourceEdit,
    export_document, import_source, profile_fixture, synchronize,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    if let Err(error) = run() {
        eprintln!("svg-sync-profile: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let outputs = output_paths()?;
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
    let observed_pointers = synchronized
        .edits
        .iter()
        .map(|edit| edit.pointer.as_str())
        .collect::<Vec<_>>();
    let checks = json!({
        "export_import_exact": imported.document == before,
        "synchronized_import_exact": reimported.document == after,
        "repeat_source_exact": repeated.source == synchronized.source,
        "repeat_edits_exact": repeated.edits == synchronized.edits,
        "mapped_edit_count_exact": synchronized.edits.len() == 7,
        "mapped_edit_pointers_exact": observed_pointers == expected_pointers(),
        "unmapped_bytes_exact": unchanged_outside_edits(&retained_source, &synchronized.source, &synchronized.edits),
        "comment_preserved": synchronized.source.contains("<!-- user SVG comment stays byte-exact -->"),
        "metadata_preserved": synchronized.source.contains("<metadata data-user-region=\"yes\">untouched</metadata>"),
        "escaped_text_exact": synchronized.source.contains("NUIF &lt;SVG&gt; &amp; source"),
        "fidelity_lossless": synchronized.report.is_lossless(),
        "unsupported_edit_typed": unsupported_edit_typed(&imported.retentive),
        "structural_edit_typed": structural_edit_typed(&imported.retentive),
        "stale_span_typed": stale_span_typed(&imported.retentive),
        "derived_geometry_typed": derived_geometry_typed(&exported.source),
        "dtd_rejected": dtd_rejected(&exported.source),
        "node_limit_typed": node_limit_typed(&exported.source),
        "source_limit_typed": matches!(import_source(&"x".repeat(MAX_SOURCE_BYTES + 1)), Err(AdapterError::SourceTooLarge)),
    });
    let passed = all_checks_pass(&checks);
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:svg-retentive-sync",
        "status": if passed { "passed" } else { "failed" },
        "source": {
            "revision": command_text("git", &["rev-parse", "HEAD"]),
            "dirty": command_text("git", &["status", "--porcelain"]).map(|value| !value.is_empty()),
            "toolchain": command_text("rustc", &["--version"]),
            "os": env::consts::OS,
            "architecture": env::consts::ARCH,
        },
        "profile": {
            "name": nuif_svg::PROFILE_NAME,
            "xml_parser": "roxmltree 0.21.1",
            "dtd": "disabled",
            "source_limit_bytes": MAX_SOURCE_BYTES,
            "xml_node_limit": MAX_XML_NODES,
            "mapped_semantics": ["identity", "containment", "rectangle geometry", "ellipse bounding geometry", "literal text", "pinned font metadata", "opaque quantized sRGB fill", "role", "accessible name"],
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
        &outputs.report,
        &serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )?;
    write_file(&outputs.source, synchronized.source.as_bytes())?;
    write_file(
        &outputs.edited,
        &CanonicalText
            .encode(&after)
            .map_err(|error| error.to_string())?,
    )?;
    println!(
        "SVG retentive sync: {} mapped edits, status {}",
        report["summary"]["mapped_edits"], report["status"]
    );
    if passed {
        Ok(())
    } else {
        Err(format!(
            "report failed; inspect {}",
            outputs.report.display()
        ))
    }
}

fn all_checks_pass(checks: &Value) -> bool {
    checks
        .as_object()
        .expect("checks is an object")
        .values()
        .all(|value| value == &Value::Bool(true))
}

fn add_unmapped_regions(source: &str) -> String {
    source.replace(
        "  <g",
        "  <!-- user SVG comment stays byte-exact -->\n  <metadata data-user-region=\"yes\">untouched</metadata>\n  <g",
    )
}

fn edited_document(before: &nuif_core::Document) -> nuif_core::Document {
    let mut after = before.clone();
    let rectangle = after.entities.get_mut(&EntityId::new(0x21)).unwrap();
    rectangle.authored.position.x = 22.0;
    rectangle.authored.fill = Some(Color {
        space: ColorSpace::Srgb,
        red: 1.0,
        green: 128.0 / 255.0,
        blue: 0.0,
        alpha: 1.0,
    });
    after
        .entities
        .get_mut(&EntityId::new(0x22))
        .unwrap()
        .authored
        .width = nuif_core::SizeIntent::Fixed(30.0);
    "NUIF <SVG> & source".clone_into(
        &mut after
            .entities
            .get_mut(&EntityId::new(0x23))
            .unwrap()
            .authored
            .text
            .as_mut()
            .unwrap()
            .content,
    );
    after
        .entities
        .get_mut(&EntityId::new(0x20))
        .unwrap()
        .semantics
        .accessible_name = Some("Edited artwork".to_owned());
    after
}

fn expected_pointers() -> Vec<&'static str> {
    vec![
        "/entities/00000000000000000000000000000020/semantics/accessible_name",
        "/entities/00000000000000000000000000000021/authored/position/x",
        "/entities/00000000000000000000000000000021/authored/fill",
        "/entities/00000000000000000000000000000022/authored/width",
        "/entities/00000000000000000000000000000022/authored",
        "/entities/00000000000000000000000000000022/authored",
        "/entities/00000000000000000000000000000023/authored/text/content",
    ]
}

fn unsupported_edit_typed(retentive: &nuif_svg::RetentiveSource) -> bool {
    let mut unsupported = retentive.document.clone();
    unsupported
        .entities
        .get_mut(&EntityId::new(0x21))
        .unwrap()
        .authored
        .fill
        .as_mut()
        .unwrap()
        .alpha = 0.5;
    let Err(AdapterError::UnmappedChanges { report, .. }) = synchronize(retentive, &unsupported)
    else {
        return false;
    };
    report.fidelity.iter().any(|entry| {
        entry.target
            == CorrespondenceTarget::Entity {
                id: EntityId::new(0x21),
            }
            && entry.pointer.ends_with("/authored/fill")
            && matches!(entry.status, Fidelity::Unsupported { .. })
    })
}

fn structural_edit_typed(retentive: &nuif_svg::RetentiveSource) -> bool {
    let mut structural = retentive.document.clone();
    structural
        .entities
        .get_mut(&EntityId::new(0x20))
        .unwrap()
        .children
        .swap(0, 1);
    matches!(
        synchronize(retentive, &structural),
        Err(AdapterError::UnmappedChanges { .. })
    )
}

fn stale_span_typed(retentive: &nuif_svg::RetentiveSource) -> bool {
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

fn derived_geometry_typed(source: &str) -> bool {
    matches!(
        import_source(&source.replacen("cx=\"48\"", "cx=\"49\"", 1)),
        Err(AdapterError::InvalidValue { .. })
    )
}

fn dtd_rejected(source: &str) -> bool {
    matches!(
        import_source(&source.replacen(
            "<svg",
            "<!DOCTYPE svg [<!ENTITY expansion \"blocked\">]>\n<svg",
            1,
        )),
        Err(AdapterError::XmlSyntax(_))
    )
}

fn node_limit_typed(source: &str) -> bool {
    let nodes = "<metadata/>".repeat(MAX_XML_NODES as usize);
    matches!(
        import_source(&source.replacen("  <g", &format!("{nodes}  <g"), 1)),
        Err(AdapterError::XmlSyntax(_))
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

struct OutputPaths {
    report: PathBuf,
    source: PathBuf,
    edited: PathBuf,
}

fn output_paths() -> Result<OutputPaths, String> {
    let mut args = env::args().skip(1);
    let mut outputs = OutputPaths {
        report: PathBuf::from("target/svg-sync-report.json"),
        source: PathBuf::from("target/svg-sync-output.svg"),
        edited: PathBuf::from("target/svg-sync-edited.nuif.json"),
    };
    while let Some(argument) = args.next() {
        let mut value = || {
            args.next()
                .ok_or_else(|| format!("{argument} requires a path"))
        };
        match argument.as_str() {
            "--output" => outputs.report = PathBuf::from(value()?),
            "--source-output" => outputs.source = PathBuf::from(value()?),
            "--edited-output" => outputs.edited = PathBuf::from(value()?),
            _ => return Err(format!("unknown argument {argument}")),
        }
    }
    Ok(outputs)
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
