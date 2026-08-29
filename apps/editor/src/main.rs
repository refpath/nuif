use nuif_codec::{
    CanonicalText, Decoder, Encoder, MAX_INPUT_BYTES, canonical_hash,
    read_bounded as read_bounded_stream,
};
use nuif_core::{Document, EntityId};
use nuif_editor::{EditorDriver, EditorEvent, EditorInput, SnapshotRaster};
use nuif_protocol::apply_patch;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

const MAX_SCRIPT_BYTES: usize = 8 * 1024 * 1024;
const MAX_SCRIPT_LINE_BYTES: usize = 64 * 1024;
const MAX_SCRIPT_COMMANDS: usize = 100_000;
const USAGE: &str = "usage: nuif-editor --headless --script <jsonl> [--document <nuif> | --new-document <id>] [--expect-document <nuif>] [--output <nuif>] [--snapshot-dir <dir>] [--report <json>]";

fn main() {
    let headless = env::args().nth(1).as_deref() == Some("--headless");
    let result = if headless {
        run()
    } else {
        nuif_editor::gui::run()
    };
    if let Err(error) = result {
        eprintln!("{{\"error\":{}}}", serde_json::to_string(&error).unwrap());
        std::process::exit(1);
    }
}

struct HeadlessOptions {
    script: PathBuf,
    document: Option<PathBuf>,
    new_document: Option<EntityId>,
    expected_document: Option<PathBuf>,
    output: Option<PathBuf>,
    snapshot_directory: Option<PathBuf>,
    report: Option<PathBuf>,
}

fn parse_options() -> Result<HeadlessOptions, String> {
    let mut args = env::args().skip(1);
    if args.next().as_deref() != Some("--headless") {
        return Err(USAGE.to_owned());
    }
    let mut script: Option<PathBuf> = None;
    let mut document_path: Option<PathBuf> = None;
    let mut new_document = None;
    let mut expected_document: Option<PathBuf> = None;
    let mut output_path: Option<PathBuf> = None;
    let mut snapshot_directory: Option<PathBuf> = None;
    let mut report_path: Option<PathBuf> = None;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--script" => script = Some(required_argument(&mut args, "--script")?.into()),
            "--document" => {
                document_path = Some(required_argument(&mut args, "--document")?.into());
            }
            "--new-document" => {
                new_document = Some(required_argument(&mut args, "--new-document")?);
            }
            "--expect-document" => {
                expected_document = Some(required_argument(&mut args, "--expect-document")?.into());
            }
            "--output" => output_path = Some(required_argument(&mut args, "--output")?.into()),
            "--snapshot-dir" => {
                snapshot_directory = Some(required_argument(&mut args, "--snapshot-dir")?.into());
            }
            "--report" => report_path = Some(required_argument(&mut args, "--report")?.into()),
            unknown => return Err(format!("unknown argument {unknown}")),
        }
    }
    let script = script.ok_or_else(|| "--script is required".to_owned())?;
    if document_path.is_some() && new_document.is_some() {
        return Err("--document and --new-document are mutually exclusive".to_owned());
    }
    let new_document = new_document
        .map(|id| {
            id.parse::<EntityId>()
                .map_err(|error| format!("invalid --new-document identifier: {error}"))
        })
        .transpose()?;
    Ok(HeadlessOptions {
        script,
        document: document_path,
        new_document,
        expected_document,
        output: output_path,
        snapshot_directory,
        report: report_path,
    })
}

fn load_initial_document(options: &HeadlessOptions) -> Result<Document, String> {
    match (&options.document, options.new_document) {
        (Some(path), None) => decode_document(path, "document"),
        (None, Some(id)) => Ok(Document::empty(id)),
        (None, None) => {
            let input = read_bounded(&mut io::stdin(), MAX_INPUT_BYTES, "document")?;
            CanonicalText
                .decode(&input)
                .map_err(|error| error.to_string())
        }
        (Some(_), Some(_)) => unreachable!("mutual exclusion checked above"),
    }
}

fn run() -> Result<(), String> {
    let options = parse_options()?;
    let document = load_initial_document(&options)?;
    let replay_base = document.clone();
    let mut script_file = fs::File::open(&options.script).map_err(|error| error.to_string())?;
    let commands = String::from_utf8(read_bounded(
        &mut script_file,
        MAX_SCRIPT_BYTES,
        "editor script",
    )?)
    .map_err(|error| format!("editor script is not UTF-8: {error}"))?;
    let mut driver = EditorDriver::new(document);
    let events = execute_script(&mut driver, &commands)?;
    let mut replayed = replay_base;
    for patch in driver.operation_log() {
        apply_patch(&mut replayed, patch).map_err(|error| error.to_string())?;
    }
    let final_hash = canonical_hash(driver.document()).map_err(|error| error.to_string())?;
    let replay_hash = canonical_hash(&replayed).map_err(|error| error.to_string())?;
    if final_hash != replay_hash {
        return Err(format!(
            "editor/replay hash mismatch: editor={final_hash}, replay={replay_hash}"
        ));
    }
    let canonical_document = CanonicalText
        .encode(driver.document())
        .map_err(|error| error.to_string())?;
    let (expected_hash, expected_exact_match) = if let Some(path) = &options.expected_document {
        let expected_bytes = read_file_bounded(path, "expected document")?;
        let expected = CanonicalText
            .decode(&expected_bytes)
            .map_err(|error| format!("expected document: {error}"))?;
        let expected_hash = canonical_hash(&expected).map_err(|error| error.to_string())?;
        let exact_match = expected_bytes == canonical_document;
        if !exact_match {
            return Err(format!(
                "authored document does not exactly match {} (authored={final_hash}, expected={expected_hash}, first_byte_difference={:?})",
                path.display(),
                first_difference(&canonical_document, &expected_bytes)
            ));
        }
        (Some(expected_hash), Some(true))
    } else {
        (None, None)
    };
    if let Some(path) = &options.output {
        fs::write(path, &canonical_document).map_err(|error| error.to_string())?;
    }
    let snapshot_artifacts = options
        .snapshot_directory
        .as_deref()
        .map_or_else(|| Ok(Vec::new()), |path| write_snapshots(path, &events))?;
    let summaries = events
        .iter()
        .map(|event| match event {
            EditorEvent::Snapshot { snapshot } => serde_json::json!({
                "event": "snapshot",
                "canonical_hash": snapshot.canonical_hash,
                "layout_boxes": snapshot.layout.boxes.len(),
                "render_commands": snapshot.scene.commands.len(),
                "raster": raster_summary(&snapshot.raster)
            }),
            event => serde_json::to_value(event).expect("editor events serialize"),
        })
        .collect::<Vec<_>>();
    let report = serde_json::json!({
        "schema_version": 1,
        "status": "passed",
        "canonical_hash": final_hash,
        "replay_hash": replay_hash,
        "expected_hash": expected_hash,
        "expected_exact_match": expected_exact_match,
        "events": summaries,
        "operations": driver.operation_log(),
        "snapshot_artifacts": snapshot_artifacts
    });
    if let Some(path) = &options.report {
        write_json(path, &report)?;
    }
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    Ok(())
}

fn required_argument(
    args: &mut impl Iterator<Item = String>,
    option: &str,
) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{option} requires a value"))
}

fn decode_document(path: &Path, label: &str) -> Result<Document, String> {
    CanonicalText
        .decode(&read_file_bounded(path, label)?)
        .map_err(|error| error.to_string())
}

fn read_file_bounded(path: &Path, label: &str) -> Result<Vec<u8>, String> {
    let mut file = fs::File::open(path).map_err(|error| error.to_string())?;
    read_bounded(&mut file, MAX_INPUT_BYTES, label)
}

fn first_difference(left: &[u8], right: &[u8]) -> Option<usize> {
    left.iter()
        .zip(right)
        .position(|(left, right)| left != right)
        .or_else(|| (left.len() != right.len()).then_some(left.len().min(right.len())))
}

fn raster_summary(raster: &SnapshotRaster) -> serde_json::Value {
    serde_json::json!({
        "width": raster.width,
        "height": raster.height,
        "rgba_sha256": raster.rgba_sha256,
        "png_sha256": format!("{:x}", Sha256::digest(&raster.png)),
        "png_bytes": raster.png.len()
    })
}

fn write_snapshots(
    directory: &Path,
    events: &[EditorEvent],
) -> Result<Vec<serde_json::Value>, String> {
    let snapshots = events
        .iter()
        .filter(|event| matches!(event, EditorEvent::Snapshot { .. }))
        .collect::<Vec<_>>();
    let mut reports = Vec::with_capacity(snapshots.len());
    for (index, event) in snapshots.iter().enumerate() {
        let EditorEvent::Snapshot { snapshot } = event else {
            unreachable!("snapshot events were filtered above");
        };
        let output = if snapshots.len() == 1 {
            directory.to_path_buf()
        } else {
            directory.join(format!("snapshot-{:04}", index + 1))
        };
        fs::create_dir_all(&output).map_err(|error| error.to_string())?;
        fs::write(output.join("input.nuif"), &snapshot.canonical_document)
            .map_err(|error| error.to_string())?;
        write_json(&output.join("context.json"), &snapshot.context)?;
        write_json(&output.join("expected.layout.json"), &snapshot.layout)?;
        write_json(&output.join("expected.scene.json"), &snapshot.scene)?;
        fs::write(output.join("expected.png"), &snapshot.raster.png)
            .map_err(|error| error.to_string())?;
        let report = serde_json::json!({
            "schema_version": 1,
            "status": "passed",
            "canonical_hash": snapshot.canonical_hash,
            "raster": raster_summary(&snapshot.raster),
            "fidelity": snapshot.scene.fidelity,
            "artifacts": [
                "input.nuif",
                "context.json",
                "expected.layout.json",
                "expected.scene.json",
                "expected.png",
                "expected.report.json"
            ]
        });
        write_json(&output.join("expected.report.json"), &report)?;
        reports.push(serde_json::json!({
            "directory": output,
            "canonical_hash": snapshot.canonical_hash,
            "report": "expected.report.json"
        }));
    }
    Ok(reports)
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn execute_script(driver: &mut EditorDriver, commands: &str) -> Result<Vec<EditorEvent>, String> {
    execute_script_with_limits(driver, commands, MAX_SCRIPT_LINE_BYTES, MAX_SCRIPT_COMMANDS)
}

fn execute_script_with_limits(
    driver: &mut EditorDriver,
    commands: &str,
    max_line_bytes: usize,
    max_commands: usize,
) -> Result<Vec<EditorEvent>, String> {
    let mut events = Vec::new();
    for (index, line) in commands.lines().enumerate() {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        if line.len() > max_line_bytes {
            return Err(format!(
                "script line {} exceeds the {max_line_bytes}-byte limit",
                index + 1
            ));
        }
        if events.len() >= max_commands {
            return Err(format!(
                "editor script exceeds the {max_commands}-command limit"
            ));
        }
        let input: EditorInput = serde_json::from_str(line)
            .map_err(|error| format!("script line {}: {error}", index + 1))?;
        events.push(driver.dispatch(input).map_err(|error| error.to_string())?);
    }
    Ok(events)
}

fn read_bounded(reader: &mut impl Read, limit: usize, label: &str) -> Result<Vec<u8>, String> {
    read_bounded_stream(reader, limit).map_err(|error| match error {
        nuif_codec::BoundedReadError::ResourceLimit { .. } => {
            format!("{label} exceeds the {limit}-byte limit")
        }
        error => error.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn bounded_reader_rejects_the_first_excess_byte() {
        assert_eq!(
            read_bounded(&mut Cursor::new([0_u8; 3]), 2, "probe").unwrap_err(),
            "probe exceeds the 2-byte limit"
        );
        assert_eq!(
            read_bounded(&mut Cursor::new([0_u8; 2]), 2, "probe").unwrap(),
            [0_u8; 2]
        );
    }

    #[test]
    fn script_limits_reject_the_first_excess_without_semantic_mutation() {
        let base = Document::empty(EntityId::new(1));
        let mut driver = EditorDriver::new(base.clone());
        let line_error =
            execute_script_with_limits(&mut driver, "{\"command\":\"clear_selection\"}", 8, 10)
                .unwrap_err();
        assert_eq!(line_error, "script line 1 exceeds the 8-byte limit");
        assert_eq!(driver.document(), &base);
        assert!(driver.operation_log().is_empty());

        let command_error = execute_script_with_limits(
            &mut driver,
            "# comment\n{\"command\":\"clear_selection\"}\n{\"command\":\"clear_selection\"}\n",
            64,
            1,
        )
        .unwrap_err();
        assert_eq!(command_error, "editor script exceeds the 1-command limit");
        assert_eq!(driver.document(), &base);
        assert!(driver.operation_log().is_empty());
    }

    #[test]
    fn malformed_script_reports_line_and_preserves_document() {
        let base = Document::empty(EntityId::new(1));
        let mut driver = EditorDriver::new(base.clone());
        let error = execute_script(&mut driver, "# ignored\n{not-json}\n").unwrap_err();
        assert!(error.starts_with("script line 2:"));
        assert_eq!(driver.document(), &base);
        assert!(driver.operation_log().is_empty());
    }
}
