use nuif_codec::{CanonicalText, Encoder, canonical_hash};
use nuif_core::{Align, EntityId, Fidelity, FlowDirection, SizeIntent};
use nuif_svelte::{
    AdapterError, CorrespondenceTarget, MAX_ELEMENT_DEPTH, MAX_SOURCE_BYTES, MAX_SYNTAX_NODES,
    SourceEdit, export_document, import_source, profile_fixture, synchronize,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    if let Err(error) = run() {
        eprintln!("svelte-sync-profile: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let (report_path, source_path, document_path) = output_paths()?;
    let before = profile_fixture();
    let exported = export_document(&before).map_err(|error| error.to_string())?;
    let retained_source = format!(
        "<!-- user header stays byte-exact -->\n{}<!-- user tail stays byte-exact -->\n",
        exported.source
    );
    let imported = import_source(&retained_source).map_err(|error| error.to_string())?;
    let after = edited_document(&before);
    let synchronized = synchronize(&imported.retentive, &after)
        .map_err(|error| format!("mapped synchronization failed: {error}"))?;
    let repeated = synchronize(&imported.retentive, &after)
        .map_err(|error| format!("repeat synchronization failed: {error}"))?;
    let reimported = import_source(&synchronized.source).map_err(|error| error.to_string())?;
    let negative = negative_trials(&exported.source);
    let checks = json!({
        "export_import_exact": imported.document == before,
        "synchronized_import_exact": reimported.document == after,
        "repeat_source_exact": repeated.source == synchronized.source,
        "repeat_edits_exact": repeated.edits == synchronized.edits,
        "mapped_edit_count_exact": synchronized.edits.len() == 11,
        "unmapped_bytes_exact": unchanged_outside_edits(&retained_source, &synchronized.source, &synchronized.edits),
        "comment_header_preserved": synchronized.source.starts_with("<!-- user header stays byte-exact -->"),
        "comment_tail_preserved": synchronized.source.ends_with("<!-- user tail stays byte-exact -->\n"),
        "escaped_text_exact": synchronized.source.contains("Edited &lt;Svelte&gt; &amp; &#123;source&#125;"),
        "fidelity_lossless": synchronized.report.is_lossless(),
        "unsupported_edit_typed": unsupported_edit_typed(&imported.retentive),
        "structural_edit_typed": structural_edit_typed(&imported.retentive),
        "stale_span_typed": stale_span_typed(&imported.retentive),
        "negative_trials_pass": negative.iter().all(passed_trial),
    });
    let passed = checks
        .as_object()
        .expect("checks object")
        .values()
        .all(|value| value == &Value::Bool(true));
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:svelte-static-retentive-sync",
        "status": if passed { "passed" } else { "failed" },
        "source": source_identity(),
        "profile": {
            "name": nuif_svelte::PROFILE_NAME,
            "syntax_parser": "tree-sitter-svelte-next 0.1.1 (bdea454a8ae7272498b8fe9d4b6b24fbd3dfe7b6)",
            "foreign_oracle": "official svelte/compiler 5.57.0, executed by cargo xtask gate-svelte",
            "source_limit_bytes": MAX_SOURCE_BYTES,
            "syntax_node_limit": MAX_SYNTAX_NODES,
            "element_depth_limit": MAX_ELEMENT_DEPTH,
            "mapped_semantics": ["stable identity", "containment", "fixed container box", "stack direction gap padding alignment", "literal text", "pinned font identity and metrics"],
            "execution": "none; marked regular elements and literal inline styles are parsed without executing JavaScript",
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
            "negative_trials": negative.len(),
            "blocking_failures": u8::from(!passed),
        },
        "checks": checks,
        "negative_trials": negative,
        "edits": synchronized.edits,
        "fidelity": synchronized.report.fidelity,
        "non_claims": [
            "no scripts runes expressions blocks directives components events or preprocessor execution",
            "no arbitrary Svelte runtime-equivalence claim",
            "no component CSS cascade scope hash server rendering hydration or browser-layout claim",
            "no TypeScript source profile",
        ],
    });
    write(
        &report_path,
        &serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )?;
    write(&source_path, synchronized.source.as_bytes())?;
    write(
        &document_path,
        &CanonicalText
            .encode(&after)
            .map_err(|error| error.to_string())?,
    )?;
    println!(
        "Svelte retentive sync: {} mapped edits, {} negative trials, status {}",
        synchronized.edits.len(),
        negative.len(),
        report["status"]
    );
    if passed {
        Ok(())
    } else {
        Err(format!("report failed; inspect {}", report_path.display()))
    }
}

fn edited_document(before: &nuif_core::Document) -> nuif_core::Document {
    let mut after = before.clone();
    let root = after.entities.get_mut(&EntityId::new(0x10)).unwrap();
    root.name = Some("Edited card".to_owned());
    root.authored.width = SizeIntent::Fixed(360.0);
    root.authored.layout.direction = FlowDirection::Row;
    root.authored.layout.gap = 20.0;
    root.authored.layout.padding.top = 25.0;
    root.authored.layout.padding.right = 26.0;
    root.authored.layout.align = Align::Center;
    let text = after.entities.get_mut(&EntityId::new(0x11)).unwrap();
    text.name = Some("Edited copy".to_owned());
    let text = text.authored.text.as_mut().unwrap();
    "Edited <Svelte> & {source}".clone_into(&mut text.content);
    text.size = 20.0;
    text.line_height = 28.0;
    after
}

fn negative_trials(source: &str) -> Vec<Value> {
    let oversized = "x".repeat(MAX_SOURCE_BYTES + 1);
    let node_heavy = format!(
        "{}{}",
        "<!-- node -->\n".repeat(MAX_SYNTAX_NODES + 1),
        source
    );
    [
        (
            "dynamic_style",
            source.replace("width:320px", "width:{width}px"),
        ),
        (
            "script",
            format!("<script>let width = 320;</script>\n{source}"),
        ),
        ("if_block", format!("{{#if true}}{source}{{/if}}")),
        (
            "directive",
            source.replace("data-nuif-name=\"Card\"", "on:click={{handler}}"),
        ),
        (
            "component_css",
            format!("<style>div {{ width: 320px; }}</style>\n{source}"),
        ),
        (
            "component_child",
            source.replace("  <span data-nuif-id", "  <Widget />\n  <span data-nuif-id"),
        ),
        (
            "extra_style",
            source.replace("width:320px", "opacity:1;width:320px"),
        ),
        (
            "wrong_profile",
            source.replace(nuif_svelte::PROFILE_NAME, "nuif-svelte-static-9"),
        ),
        (
            "duplicate_entity",
            source.replace(
                "00000000000000000000000000000011",
                "00000000000000000000000000000010",
            ),
        ),
        ("syntax_error", source.replace("</span>", "</div>")),
        ("source_limit", oversized),
        ("syntax_node_limit", node_heavy),
        ("element_depth_limit", deep_source(MAX_ELEMENT_DEPTH + 2)),
    ]
    .into_iter()
    .map(|(name, candidate)| json!({"name": name, "passed": import_source(&candidate).is_err()}))
    .collect()
}

fn deep_source(elements: usize) -> String {
    const STYLE: &str = "style=\"width:1px;height:1px;box-sizing:border-box;display:flex;flex-direction:column;gap:0px;padding-top:0px;padding-right:0px;padding-bottom:0px;padding-left:0px;align-items:stretch\"";
    let mut source = String::new();
    for index in 0..elements {
        let id = format!("{:032x}", index + 0x100);
        source.push_str(&"  ".repeat(index));
        write!(
            source,
            "<div data-nuif-id=\"{id}\" data-nuif-kind=\"container\" data-nuif-name=\"Depth {index}\""
        )
        .expect("writing into a String cannot fail");
        if index == 0 {
            write!(
                source,
                " data-nuif-profile=\"{}\" data-nuif-document=\"{:032x}\"",
                nuif_svelte::PROFILE_NAME,
                1
            )
            .expect("writing into a String cannot fail");
        }
        source.push(' ');
        source.push_str(STYLE);
        source.push_str(">\n");
    }
    for index in (0..elements).rev() {
        source.push_str(&"  ".repeat(index));
        source.push_str("</div>\n");
    }
    source
}

fn unsupported_edit_typed(retentive: &nuif_svelte::RetentiveSource) -> bool {
    let mut unsupported = retentive.document.clone();
    unsupported
        .entities
        .get_mut(&EntityId::new(0x10))
        .unwrap()
        .authored
        .fill = Some(nuif_core::Color {
        space: nuif_core::ColorSpace::Srgb,
        red: 1.0,
        green: 0.0,
        blue: 0.0,
        alpha: 1.0,
    });
    let Err(AdapterError::UnmappedChanges { report, .. }) = synchronize(retentive, &unsupported)
    else {
        return false;
    };
    report.fidelity.iter().any(|entry| {
        entry.target
            == CorrespondenceTarget::Entity {
                id: EntityId::new(0x10),
            }
            && matches!(entry.status, Fidelity::Unsupported { .. })
    })
}

fn structural_edit_typed(retentive: &nuif_svelte::RetentiveSource) -> bool {
    let mut structural = retentive.document.clone();
    structural
        .entities
        .get_mut(&EntityId::new(0x10))
        .unwrap()
        .children
        .clear();
    matches!(
        synchronize(retentive, &structural),
        Err(AdapterError::UnmappedChanges { .. })
    )
}

fn stale_span_typed(retentive: &nuif_svelte::RetentiveSource) -> bool {
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

fn passed_trial(value: &Value) -> bool {
    value["passed"] == true
}

fn unchanged_outside_edits(before: &str, after: &str, edits: &[SourceEdit]) -> bool {
    let mut before_cursor = 0;
    let mut after_cursor = 0;
    for edit in edits {
        let unchanged = &before[before_cursor..edit.span.start];
        if after.get(after_cursor..after_cursor + unchanged.len()) != Some(unchanged) {
            return false;
        }
        before_cursor = edit.span.end;
        after_cursor += unchanged.len() + edit.replacement.len();
    }
    after.get(after_cursor..) == Some(&before[before_cursor..])
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn source_identity() -> Value {
    json!({
        "revision": command_text("git", &["rev-parse", "HEAD"]),
        "dirty": command_text("git", &["status", "--porcelain"]).map(|value| !value.is_empty()),
        "toolchain": command_text("rustc", &["--version"]),
        "os": env::consts::OS,
        "architecture": env::consts::ARCH,
    })
}

fn command_text(program: &str, arguments: &[&str]) -> Option<String> {
    Command::new(program)
        .args(arguments)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_owned())
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(path, bytes).map_err(|error| error.to_string())
}

fn output_paths() -> Result<(PathBuf, PathBuf, PathBuf), String> {
    let mut args = env::args().skip(1);
    let report = args.next().map_or_else(
        || PathBuf::from("target/svelte-sync-report.json"),
        PathBuf::from,
    );
    let source = args.next().map_or_else(
        || PathBuf::from("target/svelte-sync-output.svelte"),
        PathBuf::from,
    );
    let document = args.next().map_or_else(
        || PathBuf::from("target/svelte-sync-edited.nuif.json"),
        PathBuf::from,
    );
    if args.next().is_some() {
        return Err(
            "usage: svelte-sync-profile [report.json output.svelte edited.nuif.json]".to_owned(),
        );
    }
    Ok((report, source, document))
}
