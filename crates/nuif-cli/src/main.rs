use nuif_api::{Engine, ReferenceEngine, Session, profile_zero_context};
use nuif_codec::{
    BoundedReadError, CanonicalText, Canonicalizer, Decoder, DeterministicCbor, Encoder,
    MAX_INPUT_BYTES, MAX_SYNTAX_DEPTH, MAX_TEXT_BINARY_BYTES, canonical_hash,
    read_bounded as read_bounded_stream,
};
use nuif_core::{Document, EntityId, EntityKind, PROFILE0_RESOURCE_LIMITS, Severity, validate};
use nuif_dtcg::{
    AdapterError as DtcgAdapterError, export_document as export_dtcg_document,
    import_source as import_dtcg_source, synchronize as synchronize_dtcg,
};
use nuif_html::{
    AdapterError as HtmlAdapterError, export_document as export_html_document,
    export_v0_document as export_html_v0_document, import_source as import_html_source,
    import_v0_source as import_html_v0_source, synchronize as synchronize_html,
    synchronize_v0 as synchronize_html_v0,
};
use nuif_layout::EvaluationContext;
use nuif_protocol::{Patch, apply_patch};
use nuif_svg::{
    AdapterError as SvgAdapterError, export_document as export_svg_document,
    import_source as import_svg_source, synchronize as synchronize_svg,
};
use nuif_testing::{TrialConfig, run_trials};
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;

const COMMANDS: &[&str] = &[
    "version",
    "capabilities",
    "inspect",
    "query",
    "validate",
    "canonicalize",
    "diff",
    "patch",
    "layout",
    "render",
    "snapshot",
    "sync",
    "replay",
    "migrate",
    "import",
    "export",
    "fixture",
    "trial",
];

fn main() {
    match run() {
        Ok(()) => {}
        Err(error) => {
            if !error.reported {
                eprintln!(
                    "{}",
                    serde_json::to_string(&serde_json::json!({
                        "status": "failed",
                        "code": error.code,
                        "message": error.message
                    }))
                    .expect("error objects serialize")
                );
            }
            std::process::exit(error.exit_status);
        }
    }
}

fn run() -> Result<(), CliError> {
    let mut args = env::args().skip(1);
    let command = args.next().ok_or_else(usage)?;
    let rest = args.collect::<Vec<_>>();
    match command.as_str() {
        "version" => print_json(&serde_json::json!({
            "name": "nuif",
            "version": env!("CARGO_PKG_VERSION"),
            "protocol": "0.0.1"
        })),
        "capabilities" => print_capabilities(),
        "inspect" => inspect(&rest),
        "query" => query(&rest),
        "validate" => validate_command(&rest),
        "canonicalize" => canonicalize(&rest),
        "diff" => diff(&rest),
        "patch" | "replay" => replay(&rest),
        "layout" => layout(&rest),
        "render" => render(&rest),
        "snapshot" => snapshot(&rest),
        "sync" => sync(&rest),
        "migrate" => migrate(&rest),
        "import" => import(&rest),
        "export" => export(&rest),
        "fixture" => fixture(&rest),
        "trial" => trial(&rest),
        _ => Err(usage()),
    }
}

fn inspect(args: &[String]) -> Result<(), CliError> {
    let document = load_document(required(args, 0, "input document")?)?;
    let errors = validate(&document)
        .iter()
        .filter(|item| item.severity == Severity::Error)
        .count();
    print_json(&serde_json::json!({
        "status": if errors == 0 { "passed" } else { "failed" },
        "document": document.id,
        "schema_version": document.schema_version,
        "canonical_hash": canonical_hash(&document).ok(),
        "entities": document.entities.len(),
        "roots": document.roots,
        "tokens": document.tokens.len(),
        "relations": document.relations.len(),
        "extensions_used": document.extension_declarations.used,
        "errors": errors
    }))
}

fn query(args: &[String]) -> Result<(), CliError> {
    let document = load_document(required(args, 0, "input document")?)?;
    let selector = args.get(1).map_or("all", String::as_str);
    if !matches!(selector, "all" | "name" | "id" | "kind") {
        return Err(CliError::new(
            2,
            "QUERY_SELECTOR_INVALID",
            format!("unknown selector {selector}; expected all, name, id, or kind"),
        ));
    }
    let needle = (selector != "all")
        .then(|| required(args, 2, "selector value"))
        .transpose()?;
    let selected_id = if selector == "id" {
        Some(
            EntityId::from_str(needle.expect("id selector has a value"))
                .map_err(|error| CliError::new(2, "QUERY_ID_INVALID", error.to_string()))?,
        )
    } else {
        None
    };
    let entities = document
        .entities
        .values()
        .filter(|entity| match selector {
            "all" => true,
            "name" => entity.name.as_deref() == needle,
            "id" => selected_id == Some(entity.id),
            "kind" => needle.is_some_and(|value| kind_name(&entity.kind) == value),
            _ => unreachable!("selector was validated above"),
        })
        .map(|entity| {
            serde_json::json!({
                "id": entity.id,
                "name": entity.name,
                "kind": format!("{:?}", entity.kind),
                "children": entity.children
            })
        })
        .collect::<Vec<_>>();
    print_json(&serde_json::json!({"status": "passed", "entities": entities}))
}

fn kind_name(kind: &EntityKind) -> &'static str {
    match kind {
        EntityKind::Surface => "surface",
        EntityKind::Container => "container",
        EntityKind::Shape(_) => "shape",
        EntityKind::Text => "text",
        EntityKind::Image => "image",
        EntityKind::Component => "component",
        EntityKind::Instance { .. } => "instance",
        EntityKind::Unknown(_) => "unknown",
    }
}

fn validate_command(args: &[String]) -> Result<(), CliError> {
    let path = required(args, 0, "input document")?;
    let bytes = read_input(path)?;
    let document = if Path::new(path)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("cbor"))
    {
        DeterministicCbor
            .decode_for_validation(&bytes)
            .map_err(codec_error)?
    } else {
        CanonicalText
            .decode_for_validation(&bytes)
            .map_err(codec_error)?
    };
    let diagnostics = validate(&document);
    let errors = diagnostics
        .iter()
        .filter(|item| item.severity == Severity::Error)
        .count();
    print_json(&serde_json::json!({
        "status": if errors == 0 { "passed" } else { "failed" },
        "issues": {
            "errors": errors,
            "messages": diagnostics
        }
    }))?;
    if errors == 0 {
        Ok(())
    } else {
        Err(CliError::reported(
            1,
            "VALIDATION_FAILED",
            "document validation failed",
        ))
    }
}

fn canonicalize(args: &[String]) -> Result<(), CliError> {
    let input = read_input(required(args, 0, "input document")?)?;
    let output = args.get(1).map_or("-", String::as_str);
    let cbor = args.iter().any(|argument| argument == "--cbor");
    let bytes = if cbor {
        DeterministicCbor
            .canonicalize(&input)
            .or_else(|_| {
                CanonicalText
                    .decode(&input)
                    .and_then(|document| DeterministicCbor.encode(&document))
            })
            .map_err(codec_error)?
    } else {
        CanonicalText.canonicalize(&input).map_err(codec_error)?
    };
    write_output(output, &bytes)
}

fn diff(args: &[String]) -> Result<(), CliError> {
    let left = load_document(required(args, 0, "left document")?)?;
    let right = load_document(required(args, 1, "right document")?)?;
    let ids = left
        .entities
        .keys()
        .chain(right.entities.keys())
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let changes = ids
        .into_iter()
        .filter_map(
            |id| match (left.entities.get(&id), right.entities.get(&id)) {
                (None, Some(_)) => Some(serde_json::json!({"entity": id, "change": "inserted"})),
                (Some(_), None) => Some(serde_json::json!({"entity": id, "change": "removed"})),
                (Some(before), Some(after)) if before != after => {
                    Some(serde_json::json!({"entity": id, "change": "modified"}))
                }
                _ => None,
            },
        )
        .collect::<Vec<_>>();
    print_json(&serde_json::json!({
        "status": "passed",
        "equal": changes.is_empty(),
        "left_hash": canonical_hash(&left).map_err(codec_error)?,
        "right_hash": canonical_hash(&right).map_err(codec_error)?,
        "changes": changes
    }))
}

fn replay(args: &[String]) -> Result<(), CliError> {
    let mut document = load_document(required(args, 0, "input document")?)?;
    let patch_bytes = read_input(required(args, 1, "patch JSON")?)?;
    let patch: Patch = serde_json::from_slice(&patch_bytes)
        .map_err(|error| CliError::new(1, "PATCH_MALFORMED", error.to_string()))?;
    apply_patch(&mut document, &patch)
        .map_err(|error| CliError::new(1, "PATCH_APPLY_FAILED", error.to_string()))?;
    let output = args.get(2).map_or("-", String::as_str);
    let bytes = CanonicalText.encode(&document).map_err(codec_error)?;
    write_output(output, &bytes)
}

fn layout(args: &[String]) -> Result<(), CliError> {
    let document = load_document(required(args, 0, "input document")?)?;
    let width = number(args, 1, 360.0)?;
    let height = number(args, 2, 640.0)?;
    let snapshot = ReferenceEngine
        .layout(&document, &EvaluationContext::viewport(width, height))
        .map_err(|error| CliError::new(1, "LAYOUT_FAILED", error.to_string()))?;
    print_json(&serde_json::json!({"status": "passed", "layout": snapshot}))
}

fn render(args: &[String]) -> Result<(), CliError> {
    let document = load_document(required(args, 0, "input document")?)?;
    let output = required(args, 1, "output PNG")?;
    if output == "-" {
        return Err(CliError::new(
            2,
            "BINARY_OUTPUT_PATH_REQUIRED",
            "render requires a file path so machine-readable JSON remains on stdout",
        ));
    }
    let width = number(args, 2, 360.0)?;
    let height = number(args, 3, 640.0)?;
    let session = Session::new(document);
    let snapshot = session
        .snapshot(&profile_zero_context(width, height))
        .map_err(|error| CliError::new(1, "RENDER_FAILED", error.to_string()))?;
    let png = snapshot
        .raster
        .to_png()
        .map_err(|error| CliError::new(1, "PNG_FAILED", error.to_string()))?;
    write_output(output, &png)?;
    print_json(&serde_json::json!({
        "status": "passed",
        "canonical_hash": snapshot.canonical_hash,
        "width": width,
        "height": height,
        "bytes": png.len(),
        "fidelity": snapshot.scene.fidelity
    }))
}

fn snapshot(args: &[String]) -> Result<(), CliError> {
    let document = load_document(required(args, 0, "input document")?)?;
    let directory = PathBuf::from(required(args, 1, "output directory")?);
    let width = number(args, 2, 360.0)?;
    let height = number(args, 3, 640.0)?;
    fs::create_dir_all(&directory)
        .map_err(|error| CliError::new(1, "WRITE_FAILED", error.to_string()))?;
    let session = Session::new(document.clone());
    let snapshot = session
        .snapshot(&profile_zero_context(width, height))
        .map_err(|error| CliError::new(1, "SNAPSHOT_FAILED", error.to_string()))?;
    write_file(
        &directory.join("input.nuif"),
        &CanonicalText.encode(&document).map_err(codec_error)?,
    )?;
    write_file(
        &directory.join("expected.layout.json"),
        &serde_json::to_vec_pretty(&snapshot.layout).expect("layout serializes"),
    )?;
    write_file(
        &directory.join("expected.scene.json"),
        &serde_json::to_vec_pretty(&snapshot.scene).expect("scene serializes"),
    )?;
    write_file(
        &directory.join("expected.png"),
        &snapshot
            .raster
            .to_png()
            .map_err(|error| CliError::new(1, "PNG_FAILED", error.to_string()))?,
    )?;
    let report = serde_json::json!({
        "status": "passed",
        "canonical_hash": snapshot.canonical_hash,
        "context": {"viewport": [width, height], "scale": 1.0},
        "artifacts": ["input.nuif", "expected.layout.json", "expected.scene.json", "expected.png"]
    });
    write_file(
        &directory.join("expected.report.json"),
        &serde_json::to_vec_pretty(&report).expect("report serializes"),
    )?;
    print_json(&report)
}

fn migrate(args: &[String]) -> Result<(), CliError> {
    let document = load_document(required(args, 0, "input document")?)?;
    let output = args.get(1).map_or("-", String::as_str);
    write_output(
        output,
        &CanonicalText.encode(&document).map_err(codec_error)?,
    )
}

fn import(args: &[String]) -> Result<(), CliError> {
    if args
        .first()
        .is_some_and(|argument| matches!(argument.as_str(), "html-css-0" | "html-css-v0"))
    {
        return import_html(args);
    }
    if args
        .first()
        .is_some_and(|argument| matches!(argument.as_str(), "svg-0" | "nuif-svg-0"))
    {
        return import_svg(args);
    }
    if args
        .first()
        .is_some_and(|argument| matches!(argument.as_str(), "dtcg-scalar-0" | "nuif-dtcg-scalar-0"))
    {
        return import_dtcg(args);
    }
    let input = required(args, 0, "input document")?;
    let output = args.get(1).map_or("-", String::as_str);
    let document = load_document(input)?;
    write_output(
        output,
        &CanonicalText.encode(&document).map_err(codec_error)?,
    )?;
    eprintln!(
        "{}",
        serde_json::json!({"status":"passed","fidelity":[{"class":"lossless","reason":"native NUIF import"}]})
    );
    Ok(())
}

fn export(args: &[String]) -> Result<(), CliError> {
    let document = load_document(required(args, 0, "input document")?)?;
    let target = args.get(1).map_or("nuif-text-0", String::as_str);
    let output = args.get(2).map_or("-", String::as_str);
    match target {
        "nuif-text-0" => write_output(
            output,
            &CanonicalText.encode(&document).map_err(codec_error)?,
        ),
        "nuif-cbor-0" => write_output(
            output,
            &DeterministicCbor.encode(&document).map_err(codec_error)?,
        ),
        "html-css-0" => {
            let report_path = args.get(3).map(String::as_str);
            let exported = match export_html_document(&document) {
                Ok(exported) => exported,
                Err(error) => return adapter_failure(&error, report_path),
            };
            write_output(output, exported.source.as_bytes())?;
            emit_adapter_report(&exported.report, report_path)
        }
        "html-css-v0" => {
            let report_path = args.get(3).map(String::as_str);
            let exported = match export_html_v0_document(&document) {
                Ok(exported) => exported,
                Err(error) => return adapter_failure(&error, report_path),
            };
            write_output(output, exported.source.as_bytes())?;
            emit_adapter_report(&exported.report, report_path)
        }
        "svg-0" | "nuif-svg-0" => {
            let report_path = args.get(3).map(String::as_str);
            let exported = match export_svg_document(&document) {
                Ok(exported) => exported,
                Err(error) => return svg_adapter_failure(&error, report_path),
            };
            write_output(output, exported.source.as_bytes())?;
            emit_adapter_report(&exported.report, report_path)
        }
        "dtcg-scalar-0" | "nuif-dtcg-scalar-0" => {
            let report_path = args.get(3).map(String::as_str);
            let exported = match export_dtcg_document(&document) {
                Ok(exported) => exported,
                Err(error) => return dtcg_adapter_failure(&error, report_path),
            };
            write_output(output, exported.source.as_bytes())?;
            emit_adapter_report(&exported.report, report_path)
        }
        _ => Err(CliError::new(
            3,
            "EXPORT_TARGET_UNSUPPORTED",
            format!("target {target} is unsupported; no data was written"),
        )),
    }
}

fn import_html(args: &[String]) -> Result<(), CliError> {
    let target = required(args, 0, "HTML target")?;
    let input = required(args, 1, "HTML input")?;
    let output = args.get(2).map_or("-", String::as_str);
    let report_path = args.get(3).map(String::as_str);
    let bytes = read_input(input)?;
    let source = String::from_utf8(bytes)
        .map_err(|error| CliError::new(1, "HTML_UTF8_INVALID", error.to_string()))?;
    let imported = match target {
        "html-css-0" => import_html_source(&source),
        "html-css-v0" => import_html_v0_source(&source),
        _ => unreachable!("HTML target was checked by import"),
    };
    let imported = match imported {
        Ok(imported) => imported,
        Err(error) => return adapter_failure(&error, report_path),
    };
    write_output(
        output,
        &CanonicalText
            .encode(&imported.document)
            .map_err(codec_error)?,
    )?;
    emit_adapter_report(&imported.retentive.report, report_path)
}

fn import_svg(args: &[String]) -> Result<(), CliError> {
    let input = required(args, 1, "SVG input")?;
    let output = args.get(2).map_or("-", String::as_str);
    let report_path = args.get(3).map(String::as_str);
    let source = String::from_utf8(read_input(input)?)
        .map_err(|error| CliError::new(1, "SVG_UTF8_INVALID", error.to_string()))?;
    let imported = match import_svg_source(&source) {
        Ok(imported) => imported,
        Err(error) => return svg_adapter_failure(&error, report_path),
    };
    write_output(
        output,
        &CanonicalText
            .encode(&imported.document)
            .map_err(codec_error)?,
    )?;
    emit_adapter_report(&imported.retentive.report, report_path)
}

fn import_dtcg(args: &[String]) -> Result<(), CliError> {
    let input = required(args, 1, "DTCG input")?;
    let output = args.get(2).map_or("-", String::as_str);
    let report_path = args.get(3).map(String::as_str);
    let source = String::from_utf8(read_input(input)?)
        .map_err(|error| CliError::new(1, "DTCG_UTF8_INVALID", error.to_string()))?;
    let imported = match import_dtcg_source(&source) {
        Ok(imported) => imported,
        Err(error) => return dtcg_adapter_failure(&error, report_path),
    };
    write_output(
        output,
        &CanonicalText
            .encode(&imported.document)
            .map_err(codec_error)?,
    )?;
    emit_adapter_report(&imported.retentive.report, report_path)
}

fn sync(args: &[String]) -> Result<(), CliError> {
    let target = required(args, 0, "synchronization target")?;
    if !matches!(
        target,
        "html-css-0"
            | "html-css-v0"
            | "svg-0"
            | "nuif-svg-0"
            | "dtcg-scalar-0"
            | "nuif-dtcg-scalar-0"
    ) {
        return Err(CliError::new(
            3,
            "SYNC_TARGET_UNSUPPORTED",
            format!("target {target} is unsupported"),
        ));
    }
    let source_path = required(args, 1, "retentive foreign source")?;
    let edited_path = required(args, 2, "edited NUIF document")?;
    let output = required(args, 3, "synchronized foreign output")?;
    let report_path = args.get(4).map(String::as_str);
    let edited = load_document(edited_path)?;
    let source = String::from_utf8(read_input(source_path)?)
        .map_err(|error| CliError::new(1, "ADAPTER_UTF8_INVALID", error.to_string()))?;
    let synchronized = match target {
        "html-css-0" | "html-css-v0" => {
            let imported = match target {
                "html-css-0" => import_html_source(&source),
                "html-css-v0" => import_html_v0_source(&source),
                _ => unreachable!("HTML synchronization target was checked above"),
            };
            let imported = match imported {
                Ok(imported) => imported,
                Err(error) => return adapter_failure(&error, report_path),
            };
            let synchronized = match target {
                "html-css-0" => synchronize_html(&imported.retentive, &edited),
                "html-css-v0" => synchronize_html_v0(&imported.retentive, &edited),
                _ => unreachable!("HTML synchronization target was checked above"),
            };
            match synchronized {
                Ok(synchronized) => synchronized,
                Err(error) => return adapter_failure(&error, report_path),
            }
        }
        "svg-0" | "nuif-svg-0" => {
            let imported = match import_svg_source(&source) {
                Ok(imported) => imported,
                Err(error) => return svg_adapter_failure(&error, report_path),
            };
            match synchronize_svg(&imported.retentive, &edited) {
                Ok(synchronized) => synchronized,
                Err(error) => return svg_adapter_failure(&error, report_path),
            }
        }
        "dtcg-scalar-0" | "nuif-dtcg-scalar-0" => {
            let imported = match import_dtcg_source(&source) {
                Ok(imported) => imported,
                Err(error) => return dtcg_adapter_failure(&error, report_path),
            };
            match synchronize_dtcg(&imported.retentive, &edited) {
                Ok(synchronized) => synchronized,
                Err(error) => return dtcg_adapter_failure(&error, report_path),
            }
        }
        _ => unreachable!("synchronization target was checked above"),
    };
    write_output(output, synchronized.source.as_bytes())?;
    let report = serde_json::json!({
        "status": "passed",
        "adapter": synchronized.report,
        "edits": synchronized.edits
    });
    emit_adapter_report(&report, report_path)
}

fn adapter_failure<T>(error: &HtmlAdapterError, report_path: Option<&str>) -> Result<T, CliError> {
    let report = match &error {
        HtmlAdapterError::UnsupportedProfile { report, .. }
        | HtmlAdapterError::UnmappedChanges { report, .. } => Some(report.as_ref()),
        _ => None,
    };
    if let Some(report) = report {
        emit_adapter_report(report, report_path)?;
    }
    Err(CliError::new(1, "ADAPTER_FAILED", error.to_string()))
}

fn svg_adapter_failure<T>(
    error: &SvgAdapterError,
    report_path: Option<&str>,
) -> Result<T, CliError> {
    let report = match error {
        SvgAdapterError::UnsupportedProfile { report, .. }
        | SvgAdapterError::UnmappedChanges { report, .. } => Some(report.as_ref()),
        _ => None,
    };
    if let Some(report) = report {
        emit_adapter_report(report, report_path)?;
    }
    Err(CliError::new(1, "ADAPTER_FAILED", error.to_string()))
}

fn dtcg_adapter_failure<T>(
    error: &DtcgAdapterError,
    report_path: Option<&str>,
) -> Result<T, CliError> {
    let report = match error {
        DtcgAdapterError::UnsupportedProfile { report, .. }
        | DtcgAdapterError::UnmappedChanges { report, .. } => Some(report.as_ref()),
        _ => None,
    };
    if let Some(report) = report {
        emit_adapter_report(report, report_path)?;
    }
    Err(CliError::new(1, "ADAPTER_FAILED", error.to_string()))
}

fn emit_adapter_report(report: &impl serde::Serialize, path: Option<&str>) -> Result<(), CliError> {
    let bytes = serde_json::to_vec_pretty(report)
        .map_err(|error| CliError::new(1, "JSON_FAILED", error.to_string()))?;
    if let Some(path) = path {
        write_output(path, &bytes)
    } else {
        eprintln!("{}", String::from_utf8_lossy(&bytes));
        Ok(())
    }
}

fn trial(args: &[String]) -> Result<(), CliError> {
    let seed = args.first().map_or(Ok(1), |value| {
        value
            .parse::<u64>()
            .map_err(|error| CliError::new(2, "ARGUMENT_INVALID", error.to_string()))
    })?;
    let iterations = args.get(1).map_or(Ok(100), |value| {
        value
            .parse::<u32>()
            .map_err(|error| CliError::new(2, "ARGUMENT_INVALID", error.to_string()))
    })?;
    let snapshot_interval = args.get(2).map_or(Ok(1), |value| {
        value
            .parse::<u32>()
            .map_err(|error| CliError::new(2, "ARGUMENT_INVALID", error.to_string()))
    })?;
    let report = run_trials(&TrialConfig {
        seed,
        iterations,
        snapshot_interval: snapshot_interval.max(1),
        ..TrialConfig::default()
    });
    if let Some(output) = args.get(3) {
        write_output(
            output,
            &serde_json::to_vec_pretty(&report)
                .map_err(|error| CliError::new(1, "JSON_FAILED", error.to_string()))?,
        )?;
    }
    print_json(&report)?;
    if report.passed() {
        Ok(())
    } else {
        Err(CliError::reported(1, "TRIAL_FAILED", "seeded trial failed"))
    }
}

fn fixture(args: &[String]) -> Result<(), CliError> {
    let fixture = args.first().map_or("v0-responsive-card", String::as_str);
    let output = args.get(1).map_or("-", String::as_str);
    let document = match fixture {
        "v0-responsive-card" | "v0" => nuif_testing::responsive_card_fixture(),
        "html-css-profile" => nuif_html::profile_fixture(),
        "svg-profile" => nuif_svg::profile_fixture(),
        "dtcg-profile" => nuif_dtcg::profile_fixture(),
        _ => {
            return Err(CliError::new(
                2,
                "FIXTURE_UNKNOWN",
                format!("unknown fixture {fixture}"),
            ));
        }
    };
    write_output(
        output,
        &CanonicalText.encode(&document).map_err(codec_error)?,
    )
}

fn print_capabilities() -> Result<(), CliError> {
    let engine = ReferenceEngine;
    print_json(&serde_json::json!({
        "protocol": "0.0.1",
        "status": "executable",
        "commands": COMMANDS,
        "adapters": ["html-css-0", "html-css-v0", "svg-0", "dtcg-scalar-0"],
        "engine": engine.capabilities(),
        "resource_limits": {
            "input_bytes": MAX_INPUT_BYTES,
            "syntax_depth": MAX_SYNTAX_DEPTH,
            "text_binary_bytes": MAX_TEXT_BINARY_BYTES,
            "entities": PROFILE0_RESOURCE_LIMITS.entities,
            "roots": PROFILE0_RESOURCE_LIMITS.roots,
            "tokens": PROFILE0_RESOURCE_LIMITS.tokens,
            "relations": PROFILE0_RESOURCE_LIMITS.relations,
            "child_references": PROFILE0_RESOURCE_LIMITS.child_references,
            "responsive_overrides": PROFILE0_RESOURCE_LIMITS.responsive_overrides,
            "property_values": PROFILE0_RESOURCE_LIMITS.property_values,
            "property_depth": PROFILE0_RESOURCE_LIMITS.property_depth,
            "containment_depth": PROFILE0_RESOURCE_LIMITS.containment_depth,
            "binary_bytes": PROFILE0_RESOURCE_LIMITS.binary_bytes,
            "string_bytes": PROFILE0_RESOURCE_LIMITS.string_bytes,
            "single_string_bytes": PROFILE0_RESOURCE_LIMITS.single_string_bytes
        }
    }))
}

fn load_document(path: &str) -> Result<Document, CliError> {
    let bytes = read_input(path)?;
    if Path::new(path)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("cbor"))
    {
        DeterministicCbor.decode(&bytes).map_err(codec_error)
    } else {
        CanonicalText.decode(&bytes).map_err(codec_error)
    }
}

fn read_input(path: &str) -> Result<Vec<u8>, CliError> {
    if path == "-" {
        read_bounded(&mut io::stdin())
    } else {
        let mut input = fs::File::open(path)
            .map_err(|error| CliError::new(1, "READ_FAILED", error.to_string()))?;
        read_bounded(&mut input)
    }
}

fn read_bounded(reader: &mut impl Read) -> Result<Vec<u8>, CliError> {
    read_bounded_with_limit(reader, MAX_INPUT_BYTES)
}

fn read_bounded_with_limit(reader: &mut impl Read, limit: usize) -> Result<Vec<u8>, CliError> {
    read_bounded_stream(reader, limit).map_err(|error| match error {
        BoundedReadError::ResourceLimit { .. } => CliError::new(
            1,
            "INPUT_RESOURCE_LIMIT",
            format!("input exceeds the {limit}-byte profile limit"),
        ),
        error => CliError::new(1, "READ_FAILED", error.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn bounded_reader_stops_before_unbounded_allocation() {
        let mut oversized = Cursor::new([0_u8; 10]);
        let error = read_bounded_with_limit(&mut oversized, 3).unwrap_err();
        assert_eq!(error.code, "INPUT_RESOURCE_LIMIT");
        assert_eq!(oversized.position(), 4);
        assert_eq!(
            read_bounded_with_limit(&mut Cursor::new([0_u8; 3]), 3).unwrap(),
            [0_u8; 3]
        );
    }
}

fn write_output(path: &str, bytes: &[u8]) -> Result<(), CliError> {
    if path == "-" {
        io::stdout()
            .write_all(bytes)
            .map_err(|error| CliError::new(1, "WRITE_FAILED", error.to_string()))
    } else {
        write_file(Path::new(path), bytes)
    }
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), CliError> {
    fs::write(path, bytes).map_err(|error| CliError::new(1, "WRITE_FAILED", error.to_string()))
}

fn print_json(value: &impl serde::Serialize) -> Result<(), CliError> {
    println!(
        "{}",
        serde_json::to_string_pretty(value).map_err(|error| CliError::new(
            1,
            "JSON_FAILED",
            error.to_string()
        ))?
    );
    Ok(())
}

fn required<'a>(args: &'a [String], index: usize, label: &str) -> Result<&'a str, CliError> {
    args.get(index)
        .map(String::as_str)
        .ok_or_else(|| CliError::new(2, "ARGUMENT_MISSING", format!("missing {label}")))
}

fn number(args: &[String], index: usize, default: f64) -> Result<f64, CliError> {
    args.get(index).map_or(Ok(default), |value| {
        value
            .parse::<f64>()
            .map_err(|error| CliError::new(2, "ARGUMENT_INVALID", error.to_string()))
    })
}

fn codec_error(error: impl std::fmt::Display) -> CliError {
    CliError::new(1, "CODEC_FAILED", error.to_string())
}

fn usage() -> CliError {
    CliError::new(2, "USAGE", format!("usage: nuif <{}>", COMMANDS.join("|")))
}

#[derive(Debug)]
struct CliError {
    exit_status: i32,
    code: String,
    message: String,
    reported: bool,
}

impl CliError {
    fn new(exit_status: i32, code: &str, message: impl Into<String>) -> Self {
        Self {
            exit_status,
            code: code.to_owned(),
            message: message.into(),
            reported: false,
        }
    }

    fn reported(exit_status: i32, code: &str, message: impl Into<String>) -> Self {
        Self {
            exit_status,
            code: code.to_owned(),
            message: message.into(),
            reported: true,
        }
    }
}
