use nuif_codec::{CanonicalText, Decoder, Encoder, canonical_hash};
use nuif_editor::{EditorDriver, EditorEvent, EditorInput};
use nuif_protocol::apply_patch;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

fn main() {
    if let Err(error) = run() {
        eprintln!("{{\"error\":{}}}", serde_json::to_string(&error).unwrap());
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    if args.next().as_deref() != Some("--headless") {
        return Err(
            "usage: nuif-editor --headless --script <jsonl> [--document <nuif>] [--output <nuif>]"
                .to_owned(),
        );
    }
    let mut script = None;
    let mut document_path = None;
    let mut output_path = None;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--script" => script = args.next().map(PathBuf::from),
            "--document" => document_path = args.next().map(PathBuf::from),
            "--output" => output_path = args.next().map(PathBuf::from),
            unknown => return Err(format!("unknown argument {unknown}")),
        }
    }
    let script = script.ok_or_else(|| "--script is required".to_owned())?;
    let input = if let Some(path) = document_path {
        fs::read(path).map_err(|error| error.to_string())?
    } else {
        let mut bytes = Vec::new();
        io::stdin()
            .read_to_end(&mut bytes)
            .map_err(|error| error.to_string())?;
        bytes
    };
    let document = CanonicalText
        .decode(&input)
        .map_err(|error| error.to_string())?;
    let replay_base = document.clone();
    let commands = fs::read_to_string(script).map_err(|error| error.to_string())?;
    let mut driver = EditorDriver::new(document);
    let mut events = Vec::new();
    for (index, line) in commands.lines().enumerate() {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let input: EditorInput = serde_json::from_str(line)
            .map_err(|error| format!("script line {}: {error}", index + 1))?;
        events.push(driver.dispatch(input).map_err(|error| error.to_string())?);
    }
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
    if let Some(path) = output_path {
        fs::write(
            path,
            CanonicalText
                .encode(driver.document())
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
    }
    let summaries = events
        .into_iter()
        .map(|event| match event {
            EditorEvent::Snapshot {
                canonical_hash,
                layout_boxes,
                render_commands,
                png,
            } => serde_json::json!({
                "event": "snapshot",
                "canonical_hash": canonical_hash,
                "layout_boxes": layout_boxes,
                "render_commands": render_commands,
                "png_bytes": png.len()
            }),
            event => serde_json::to_value(event).expect("editor events serialize"),
        })
        .collect::<Vec<_>>();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "passed",
            "canonical_hash": final_hash,
            "replay_hash": replay_hash,
            "events": summaries,
            "operations": driver.operation_log()
        }))
        .unwrap()
    );
    Ok(())
}
