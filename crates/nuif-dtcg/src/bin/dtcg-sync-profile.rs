use nuif_codec::{CanonicalText, Encoder, canonical_hash};
use nuif_core::{EntityId, PropertyValue};
use nuif_dtcg::{
    AdapterError, MAX_JSON_DEPTH, MAX_SOURCE_BYTES, MAX_TOKENS, SourceEdit, export_document,
    import_source, profile_fixture, synchronize,
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
        eprintln!("dtcg-sync-profile: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let outputs = output_paths()?;
    let before = profile_fixture();
    let exported = export_document(&before).map_err(|error| error.to_string())?;
    let retained_source = add_unknown_extensions(&exported.source);
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
        "mapped_edit_count_exact": synchronized.edits.len() == 8,
        "mapped_edit_pointers_exact": observed_pointers == expected_pointers(),
        "unmapped_bytes_exact": unchanged_outside_edits(&retained_source, &synchronized.source, &synchronized.edits),
        "root_extension_preserved": synchronized.source.contains("\"com.example.root\": {\"opaque\": [1, 2, 3]}"),
        "token_extension_preserved": synchronized.source.contains("\"com.example.token\": {\"opaque\": true}"),
        "fidelity_lossless": synchronized.report.is_lossless(),
        "structural_edit_typed": structural_edit_typed(&imported.retentive),
        "unsupported_edit_typed": unsupported_edit_typed(&imported.retentive),
        "stale_span_typed": stale_span_typed(&imported.retentive),
        "duplicate_member_typed": duplicate_member_typed(&exported.source),
        "alias_typed": alias_typed(&exported.source),
        "standard_member_typed": standard_member_typed(&exported.source),
        "depth_limit_typed": depth_limit_typed(&exported.source),
        "token_limit_typed": token_limit_typed(),
        "source_limit_typed": matches!(import_source(&" ".repeat(MAX_SOURCE_BYTES + 1)), Err(AdapterError::SourceTooLarge)),
    });
    let passed = all_checks_pass(&checks);
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:dtcg-scalar-retentive-sync",
        "status": if passed { "passed" } else { "failed" },
        "source": {
            "revision": command_text("git", &["rev-parse", "HEAD"]),
            "dirty": command_text("git", &["status", "--porcelain"]).map(|value| !value.is_empty()),
            "toolchain": command_text("rustc", &["--version"]),
            "os": env::consts::OS,
            "architecture": env::consts::ARCH,
        },
        "profile": {
            "name": nuif_dtcg::PROFILE_NAME,
            "foreign_format": "Design Tokens Format Module 2025.10 JSON",
            "json_parser": "serde_json 1.0.151",
            "source_limit_bytes": MAX_SOURCE_BYTES,
            "json_depth_limit": MAX_JSON_DEPTH,
            "token_limit": MAX_TOKENS,
            "mapped_semantics": ["document identity", "token identity", "flat token name", "explicit DTCG type", "boolean", "string", "integer-discriminated number", "real-discriminated number", "unknown extension retention"],
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
        "DTCG scalar retentive sync: {} mapped edits, status {}",
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

fn add_unknown_extensions(source: &str) -> String {
    source
        .replacen(
            "\"org.nuif\": {",
            "\"com.example.root\": {\"opaque\": [1, 2, 3]}, \"org.nuif\": {",
            1,
        )
        .replacen(
            "\"org.nuif\": {\"id\"",
            "\"com.example.token\": {\"opaque\": true}, \"org.nuif\": {\"id\"",
            1,
        )
}

fn edited_document(before: &nuif_core::Document) -> nuif_core::Document {
    let mut after = before.clone();
    after.tokens.get_mut(&EntityId::new(0x100)).unwrap().value = PropertyValue::Boolean(false);
    let label = after.tokens.get_mut(&EntityId::new(0x101)).unwrap();
    "title".clone_into(&mut label.name);
    label.value = PropertyValue::String("Secondary".to_owned());
    after.tokens.get_mut(&EntityId::new(0x102)).unwrap().value = PropertyValue::Real(7.0);
    after.tokens.get_mut(&EntityId::new(0x103)).unwrap().value = PropertyValue::Boolean(false);
    after
}

fn expected_pointers() -> Vec<&'static str> {
    vec![
        "/tokens/00000000000000000000000000000100/value",
        "/tokens/00000000000000000000000000000101/name",
        "/tokens/00000000000000000000000000000101/value",
        "/tokens/00000000000000000000000000000102/value",
        "/tokens/00000000000000000000000000000102/value",
        "/tokens/00000000000000000000000000000103/value",
        "/tokens/00000000000000000000000000000103/value",
        "/tokens/00000000000000000000000000000103/value",
    ]
}

fn structural_edit_typed(retentive: &nuif_dtcg::RetentiveSource) -> bool {
    let mut edited = retentive.document.clone();
    edited.tokens.remove(&EntityId::new(0x100));
    matches!(
        synchronize(retentive, &edited),
        Err(AdapterError::UnmappedChanges { .. })
    )
}

fn unsupported_edit_typed(retentive: &nuif_dtcg::RetentiveSource) -> bool {
    let mut edited = retentive.document.clone();
    edited.tokens.get_mut(&EntityId::new(0x100)).unwrap().value = PropertyValue::Array(Vec::new());
    matches!(
        synchronize(retentive, &edited),
        Err(AdapterError::UnmappedChanges { .. })
    )
}

fn stale_span_typed(retentive: &nuif_dtcg::RetentiveSource) -> bool {
    let mut stale = retentive.clone();
    stale.source = stale
        .source
        .replacen("\"$value\": true", "\"$value\":false", 1);
    matches!(
        synchronize(&stale, &retentive.document),
        Err(AdapterError::StaleSpan { .. })
    )
}

fn duplicate_member_typed(source: &str) -> bool {
    matches!(
        import_source(&source.replacen(
            "\"$type\": \"boolean\"",
            "\"$type\": \"boolean\", \"$type\": \"boolean\"",
            1,
        )),
        Err(AdapterError::JsonSyntax(_))
    )
}

fn alias_typed(source: &str) -> bool {
    matches!(
        import_source(&source.replacen("\"$value\": \"Primary\"", "\"$value\": \"{enabled}\"", 1)),
        Err(AdapterError::InvalidValue { .. })
    )
}

fn standard_member_typed(source: &str) -> bool {
    matches!(
        import_source(&source.replacen(
            "\"$type\": \"boolean\"",
            "\"$description\": \"outside profile\", \"$type\": \"boolean\"",
            1,
        )),
        Err(AdapterError::JsonSyntax(_))
    )
}

fn depth_limit_typed(source: &str) -> bool {
    let nested = "[".repeat(MAX_JSON_DEPTH + 1) + &"]".repeat(MAX_JSON_DEPTH + 1);
    matches!(
        import_source(&source.replacen(
            "\"org.nuif\": {",
            &format!("\"com.example.deep\": {nested}, \"org.nuif\": {{"),
            1,
        )),
        Err(AdapterError::JsonSyntax(_))
    )
}

fn token_limit_typed() -> bool {
    let mut source = String::from(
        "{\"$extensions\":{\"org.nuif\":{\"profile\":\"nuif-dtcg-scalar-0\",\"document\":\"00000000000000000000000000000001\"}}",
    );
    for index in 0..=MAX_TOKENS {
        write!(
            source,
            ",\"t{index}\":{{\"$type\":\"boolean\",\"$value\":true,\"$extensions\":{{\"org.nuif\":{{\"id\":\"{index:032x}\",\"value_kind\":\"boolean\"}}}}}}"
        )
        .expect("writing to a string cannot fail");
    }
    source.push('}');
    source.len() <= MAX_SOURCE_BYTES
        && matches!(
            import_source(&source),
            Err(AdapterError::InvalidValue { .. })
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
        report: PathBuf::from("target/dtcg-sync-report.json"),
        source: PathBuf::from("target/dtcg-sync-output.tokens.json"),
        edited: PathBuf::from("target/dtcg-sync-edited.nuif"),
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
