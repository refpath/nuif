use nuif_codec::{
    CanonicalText, Decoder, Encoder, MAX_INPUT_BYTES, canonical_hash,
    read_bounded as read_bounded_stream,
};
use nuif_editor::{EditorDriver, EditorEvent, EditorInput};
use nuif_protocol::apply_patch;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

const MAX_SCRIPT_BYTES: usize = 8 * 1024 * 1024;
const MAX_SCRIPT_LINE_BYTES: usize = 64 * 1024;
const MAX_SCRIPT_COMMANDS: usize = 100_000;

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
        let mut file = fs::File::open(path).map_err(|error| error.to_string())?;
        read_bounded(&mut file, MAX_INPUT_BYTES, "document")?
    } else {
        read_bounded(&mut io::stdin(), MAX_INPUT_BYTES, "document")?
    };
    let document = CanonicalText
        .decode(&input)
        .map_err(|error| error.to_string())?;
    let replay_base = document.clone();
    let mut script_file = fs::File::open(script).map_err(|error| error.to_string())?;
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

fn execute_script(driver: &mut EditorDriver, commands: &str) -> Result<Vec<EditorEvent>, String> {
    let mut events = Vec::new();
    for (index, line) in commands.lines().enumerate() {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        if line.len() > MAX_SCRIPT_LINE_BYTES {
            return Err(format!(
                "script line {} exceeds the {MAX_SCRIPT_LINE_BYTES}-byte limit",
                index + 1
            ));
        }
        if events.len() >= MAX_SCRIPT_COMMANDS {
            return Err(format!(
                "editor script exceeds the {MAX_SCRIPT_COMMANDS}-command limit"
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
}
