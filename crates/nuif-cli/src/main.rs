use nuif_api::{Engine, ReferenceEngine, Session, profile_zero_context};
use nuif_capture::{
    BrowserCapture, OcrSpan, SCREENSHOT_CAPTURE_PROFILE, ScreenshotCapture, Viewport,
    analyze_screenshot, normalize_browser_capture,
};
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
use nuif_package::{MAX_PACKAGE_BYTES, MAX_RESOURCE_BYTES, NuifPackage, PackageMode};
use nuif_penpot::{
    AdapterError as PenpotAdapterError, export_document as export_penpot_document,
    import_package as import_penpot_package, synchronize as synchronize_penpot,
};
use nuif_protocol::{Patch, apply_patch};
use nuif_reconstruct::{ObservationBundle, Proposal, ProposalPolicy, apply_proposal};
use nuif_svg::{
    AdapterError as SvgAdapterError, export_document as export_svg_document,
    import_source as import_svg_source, synchronize as synchronize_svg,
};
use nuif_testing::{TrialConfig, run_trials};
use serde::Deserialize;
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
    "pack",
    "unpack",
    "capture",
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
        "pack" => pack(&rest),
        "unpack" => unpack(&rest),
        "capture" => capture(&rest),
        _ => Err(usage()),
    }
}

fn inspect(args: &[String]) -> Result<(), CliError> {
    let loaded = load_nuif(required(args, 0, "input document")?)?;
    let document = &loaded.document;
    let errors = validate(document)
        .iter()
        .filter(|item| item.severity == Severity::Error)
        .count();
    print_json(&serde_json::json!({
        "status": if errors == 0 { "passed" } else { "failed" },
        "document": document.id,
        "schema_version": document.schema_version,
        "canonical_hash": canonical_hash(document).ok(),
        "entities": document.entities.len(),
        "roots": document.roots,
        "tokens": document.tokens.len(),
        "relations": document.relations.len(),
        "assets": document.assets.len(),
        "file_profile": loaded.profile,
        "package": loaded.package.as_ref().map(|package| serde_json::json!({
            "mode": package.mode,
            "resources": package.resources.len(),
            "package_hash": package.package_hash().ok()
        })),
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
    let bytes = read_input_with_limit(path, MAX_PACKAGE_BYTES)?;
    let document = if bytes.starts_with(b"PK\x03\x04") {
        NuifPackage::decode(&bytes).map_err(package_error)?.document
    } else if Path::new(path)
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
    let mut loaded = load_nuif(required(args, 0, "input document")?)?;
    let patch_bytes = read_input(required(args, 1, "patch JSON")?)?;
    let patch: Patch = serde_json::from_slice(&patch_bytes)
        .map_err(|error| CliError::new(1, "PATCH_MALFORMED", error.to_string()))?;
    apply_patch(&mut loaded.document, &patch)
        .map_err(|error| CliError::new(1, "PATCH_APPLY_FAILED", error.to_string()))?;
    let output = args.get(2).map_or("-", String::as_str);
    write_loaded_document(output, &mut loaded)
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
    write_document(output, &document)
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
    if args
        .first()
        .is_some_and(|argument| matches!(argument.as_str(), "penpot-v3-0" | "nuif-penpot-v3-0"))
    {
        return import_penpot(args);
    }
    let input = required(args, 0, "input document")?;
    let output = args.get(1).map_or("-", String::as_str);
    let document = load_document(input)?;
    write_document(output, &document)?;
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
        "nuif-package-0" => write_output(
            output,
            &NuifPackage::new(document, PackageMode::Portable)
                .encode()
                .map_err(package_error)?,
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
        "penpot-v3-0" | "nuif-penpot-v3-0" => {
            let report_path = args.get(3).map(String::as_str);
            let exported = match export_penpot_document(&document) {
                Ok(exported) => exported,
                Err(error) => return penpot_adapter_failure(&error, report_path),
            };
            write_output(output, &exported.bytes)?;
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
    write_document(output, &imported.document)?;
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
    write_document(output, &imported.document)?;
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
    write_document(output, &imported.document)?;
    emit_adapter_report(&imported.retentive.report, report_path)
}

fn import_penpot(args: &[String]) -> Result<(), CliError> {
    let input = required(args, 1, "Penpot package input")?;
    let output = args.get(2).map_or("-", String::as_str);
    let report_path = args.get(3).map(String::as_str);
    let imported = match import_penpot_package(&read_input(input)?) {
        Ok(imported) => imported,
        Err(error) => return penpot_adapter_failure(&error, report_path),
    };
    write_document(output, &imported.document)?;
    emit_adapter_report(imported.retentive.report(), report_path)
}

fn sync(args: &[String]) -> Result<(), CliError> {
    let target = required(args, 0, "synchronization target")?;
    if matches!(target, "penpot-v3-0" | "nuif-penpot-v3-0") {
        return sync_penpot(args);
    }
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

fn sync_penpot(args: &[String]) -> Result<(), CliError> {
    let source_path = required(args, 1, "retentive Penpot package")?;
    let edited_path = required(args, 2, "edited NUIF document")?;
    let output = required(args, 3, "synchronized Penpot output")?;
    let report_path = args.get(4).map(String::as_str);
    let imported = match import_penpot_package(&read_input(source_path)?) {
        Ok(imported) => imported,
        Err(error) => return penpot_adapter_failure(&error, report_path),
    };
    let edited = load_document(edited_path)?;
    let synchronized = match synchronize_penpot(&imported.retentive, &edited) {
        Ok(synchronized) => synchronized,
        Err(error) => return penpot_adapter_failure(&error, report_path),
    };
    write_output(output, &synchronized.bytes)?;
    emit_adapter_report(
        &serde_json::json!({
            "status": "passed",
            "adapter": synchronized.report,
            "edits": synchronized.edits,
        }),
        report_path,
    )
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

fn penpot_adapter_failure<T>(
    error: &PenpotAdapterError,
    report_path: Option<&str>,
) -> Result<T, CliError> {
    let report = match error {
        PenpotAdapterError::UnsupportedProfile { report, .. }
        | PenpotAdapterError::UnmappedChanges { report, .. } => Some(report.as_ref()),
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

fn pack(args: &[String]) -> Result<(), CliError> {
    let input = required(args, 0, "input NUIF document")?;
    let output = required(args, 1, "output .nuif package")?;
    if !has_extension(output, "nuif") {
        return Err(CliError::new(
            2,
            "PACKAGE_EXTENSION_REQUIRED",
            "pack output must use the .nuif extension",
        ));
    }
    let loaded = load_nuif(input)?;
    let requested_mode = if args.iter().any(|argument| argument == "--authoring") {
        Some(PackageMode::Authoring)
    } else if args.iter().any(|argument| argument == "--portable") {
        Some(PackageMode::Portable)
    } else {
        None
    };
    let mut package = loaded
        .package
        .unwrap_or_else(|| NuifPackage::new(loaded.document.clone(), PackageMode::Portable));
    package.document = loaded.document;
    if let Some(mode) = requested_mode {
        package.mode = mode;
    }
    let bytes = package.encode().map_err(package_error)?;
    write_output(output, &bytes)?;
    print_json(&serde_json::json!({
        "status": "passed",
        "profile": nuif_package::PROFILE,
        "mode": package.mode,
        "document_hash": canonical_hash(&package.document).map_err(codec_error)?,
        "package_hash": package.package_hash().map_err(package_error)?,
        "resources": package.resources.len(),
        "bytes": bytes.len()
    }))
}

fn unpack(args: &[String]) -> Result<(), CliError> {
    let input = required(args, 0, "input .nuif package")?;
    let output = required(args, 1, "output .nuif.json or .nuif.cbor")?;
    if has_extension(output, "nuif") {
        return Err(CliError::new(
            2,
            "BARE_EXTENSION_REQUIRED",
            "unpack output must use .json or .cbor, not .nuif",
        ));
    }
    let bytes = read_input_with_limit(input, MAX_PACKAGE_BYTES)?;
    let package = NuifPackage::decode(&bytes).map_err(package_error)?;
    let document_bytes = if has_extension(output, "cbor") {
        DeterministicCbor
            .encode(&package.document)
            .map_err(codec_error)?
    } else {
        CanonicalText
            .encode(&package.document)
            .map_err(codec_error)?
    };
    write_output(output, &document_bytes)?;
    print_json(&serde_json::json!({
        "status": "passed",
        "source_profile": nuif_package::PROFILE,
        "output_profile": if has_extension(output, "cbor") { "nuif-cbor-0" } else { "nuif-text-0" },
        "document_hash": canonical_hash(&package.document).map_err(codec_error)?
    }))
}

fn capture(args: &[String]) -> Result<(), CliError> {
    match required(args, 0, "capture kind")? {
        "browser" => capture_browser(args),
        "screenshot" => capture_screenshot(args),
        kind => Err(CliError::new(
            2,
            "CAPTURE_KIND_UNSUPPORTED",
            format!("capture kind {kind} is unsupported"),
        )),
    }
}

fn capture_browser(args: &[String]) -> Result<(), CliError> {
    let input = required(args, 1, "browser capture JSON")?;
    let output = required(args, 2, "output .nuif package")?;
    let observations = required(args, 3, "output observation CBOR")?;
    let proposal = required(args, 4, "output proposal JSON")?;
    let capture: BrowserCapture = serde_json::from_slice(&read_input(input)?)
        .map_err(|error| CliError::new(1, "CAPTURE_JSON_INVALID", error.to_string()))?;
    let mut package = NuifPackage::new(Document::empty(EntityId::new(1)), PackageMode::Authoring);
    let result = normalize_browser_capture(&capture, &mut package).map_err(capture_error)?;
    finish_capture(
        output,
        observations,
        proposal,
        &mut package,
        &result.observations,
        &result.proposal,
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ScreenshotMetadata {
    schema_version: u32,
    profile: String,
    capture_id: String,
    viewport: Viewport,
    #[serde(default)]
    ocr: Vec<OcrSpan>,
}

fn capture_screenshot(args: &[String]) -> Result<(), CliError> {
    let png = required(args, 1, "input screenshot PNG")?;
    let metadata = required(args, 2, "screenshot metadata JSON")?;
    let output = required(args, 3, "output .nuif package")?;
    let observations = required(args, 4, "output observation CBOR")?;
    let proposal = required(args, 5, "output proposal JSON")?;
    let metadata: ScreenshotMetadata = serde_json::from_slice(&read_input(metadata)?)
        .map_err(|error| CliError::new(1, "CAPTURE_JSON_INVALID", error.to_string()))?;
    if metadata.profile != SCREENSHOT_CAPTURE_PROFILE {
        return Err(CliError::new(
            2,
            "CAPTURE_PROFILE_UNSUPPORTED",
            "screenshot metadata does not declare nuif-screenshot-baseline-0",
        ));
    }
    let capture = ScreenshotCapture {
        schema_version: metadata.schema_version,
        profile: metadata.profile,
        capture_id: metadata.capture_id,
        viewport: metadata.viewport,
        png: read_input_with_limit(png, MAX_RESOURCE_BYTES)?,
        ocr: metadata.ocr,
    };
    let mut package = NuifPackage::new(Document::empty(EntityId::new(1)), PackageMode::Authoring);
    let result = analyze_screenshot(&capture, &mut package).map_err(capture_error)?;
    finish_capture(
        output,
        observations,
        proposal,
        &mut package,
        &result.observations,
        &result.proposal,
    )
}

fn finish_capture(
    output: &str,
    observations_path: &str,
    proposal_path: &str,
    package: &mut NuifPackage,
    observations: &ObservationBundle,
    proposal: &Proposal,
) -> Result<(), CliError> {
    if !has_extension(output, "nuif") {
        return Err(CliError::new(
            2,
            "PACKAGE_EXTENSION_REQUIRED",
            "capture output must use the .nuif extension",
        ));
    }
    apply_proposal(
        &mut package.document,
        observations,
        proposal,
        &ProposalPolicy::default(),
    )
    .map_err(|error| CliError::new(1, "PROPOSAL_APPLY_FAILED", error.to_string()))?;
    let package_bytes = package.encode().map_err(package_error)?;
    let observation_bytes = observations
        .encode()
        .map_err(|error| CliError::new(1, "OBSERVATION_ENCODE_FAILED", error.to_string()))?;
    let proposal_bytes = serde_json::to_vec_pretty(proposal)
        .map_err(|error| CliError::new(1, "PROPOSAL_JSON_FAILED", error.to_string()))?;
    write_output(output, &package_bytes)?;
    write_output(observations_path, &observation_bytes)?;
    write_output(proposal_path, &proposal_bytes)?;
    print_json(&serde_json::json!({
        "status": "passed",
        "package": output,
        "observations": observations_path,
        "proposal": proposal_path,
        "document_hash": canonical_hash(&package.document).map_err(codec_error)?,
        "package_hash": package.package_hash().map_err(package_error)?,
        "observation_count": observations.observations.len(),
        "omission_count": observations.omissions.len()
    }))
}

fn fixture(args: &[String]) -> Result<(), CliError> {
    let fixture = args.first().map_or("v0-responsive-card", String::as_str);
    let output = args.get(1).map_or("-", String::as_str);
    let document = match fixture {
        "v0-responsive-card" | "v0" => nuif_testing::responsive_card_fixture(),
        "html-css-profile" => nuif_html::profile_fixture(),
        "svg-profile" => nuif_svg::profile_fixture(),
        "dtcg-profile" => nuif_dtcg::profile_fixture(),
        "penpot-profile" => nuif_penpot::profile_fixture(),
        _ => {
            return Err(CliError::new(
                2,
                "FIXTURE_UNKNOWN",
                format!("unknown fixture {fixture}"),
            ));
        }
    };
    write_document(output, &document)
}

fn print_capabilities() -> Result<(), CliError> {
    let engine = ReferenceEngine;
    print_json(&serde_json::json!({
        "protocol": "0.0.1",
        "status": "executable",
        "commands": COMMANDS,
        "adapters": ["html-css-0", "html-css-v0", "svg-0", "dtcg-scalar-0", "penpot-v3-0"],
        "containers": ["nuif-package-0", "nuif-cbor-0", "nuif-text-0"],
        "capture_profiles": ["nuif-browser-capture-0", "nuif-screenshot-baseline-0"],
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
            "single_string_bytes": PROFILE0_RESOURCE_LIMITS.single_string_bytes,
            "assets": PROFILE0_RESOURCE_LIMITS.assets,
            "package_bytes": MAX_PACKAGE_BYTES,
            "single_resource_bytes": MAX_RESOURCE_BYTES
        }
    }))
}

fn load_document(path: &str) -> Result<Document, CliError> {
    load_nuif(path).map(|loaded| loaded.document)
}

struct LoadedNuif {
    document: Document,
    package: Option<NuifPackage>,
    profile: &'static str,
}

fn load_nuif(path: &str) -> Result<LoadedNuif, CliError> {
    let bytes = read_input_with_limit(path, MAX_PACKAGE_BYTES)?;
    if bytes.starts_with(b"PK\x03\x04") {
        let package = NuifPackage::decode(&bytes).map_err(package_error)?;
        return Ok(LoadedNuif {
            document: package.document.clone(),
            package: Some(package),
            profile: nuif_package::PROFILE,
        });
    }
    if has_extension(path, "cbor") {
        return Ok(LoadedNuif {
            document: DeterministicCbor.decode(&bytes).map_err(codec_error)?,
            package: None,
            profile: "nuif-cbor-0",
        });
    }
    if let Ok(document) = CanonicalText.decode(&bytes) {
        return Ok(LoadedNuif {
            document,
            package: None,
            profile: "nuif-text-0-legacy",
        });
    }
    Ok(LoadedNuif {
        document: DeterministicCbor.decode(&bytes).map_err(codec_error)?,
        package: None,
        profile: "nuif-cbor-0-legacy",
    })
}

fn read_input(path: &str) -> Result<Vec<u8>, CliError> {
    read_input_with_limit(path, MAX_INPUT_BYTES)
}

fn read_input_with_limit(path: &str, limit: usize) -> Result<Vec<u8>, CliError> {
    if path == "-" {
        read_bounded_with_limit(&mut io::stdin(), limit)
    } else {
        let mut input = fs::File::open(path)
            .map_err(|error| CliError::new(1, "READ_FAILED", error.to_string()))?;
        read_bounded_with_limit(&mut input, limit)
    }
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

fn write_document(path: &str, document: &Document) -> Result<(), CliError> {
    let bytes = if path != "-" && has_extension(path, "nuif") {
        NuifPackage::new(document.clone(), PackageMode::Portable)
            .encode()
            .map_err(package_error)?
    } else if has_extension(path, "cbor") {
        DeterministicCbor.encode(document).map_err(codec_error)?
    } else {
        CanonicalText.encode(document).map_err(codec_error)?
    };
    write_output(path, &bytes)
}

fn write_loaded_document(path: &str, loaded: &mut LoadedNuif) -> Result<(), CliError> {
    if path != "-" && has_extension(path, "nuif") {
        let mut package = loaded
            .package
            .take()
            .unwrap_or_else(|| NuifPackage::new(loaded.document.clone(), PackageMode::Portable));
        package.document.clone_from(&loaded.document);
        return write_output(path, &package.encode().map_err(package_error)?);
    }
    write_document(path, &loaded.document)
}

fn has_extension(path: &str, extension: &str) -> bool {
    Path::new(path)
        .extension()
        .is_some_and(|value| value.eq_ignore_ascii_case(extension))
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

fn package_error(error: impl std::fmt::Display) -> CliError {
    CliError::new(1, "PACKAGE_FAILED", error.to_string())
}

fn capture_error(error: impl std::fmt::Display) -> CliError {
    CliError::new(1, "CAPTURE_FAILED", error.to_string())
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
