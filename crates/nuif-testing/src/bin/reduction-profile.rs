use nuif_core::{EntityId, Severity, validate};
use nuif_protocol::Operation;
use nuif_testing::reduction::{minimize_document, write_reduced_fixture};
use serde_json::json;
use std::env;
use std::fs;
use std::path::PathBuf;

const TARGET: EntityId = EntityId::new(0x22);

fn main() {
    if let Err(error) = run() {
        eprintln!("reduction-profile: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut output = PathBuf::from("target/reduction-profile-report.json");
    let mut fixture = PathBuf::from("target/reduction-profile-fixture");
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--output" => {
                output = PathBuf::from(arguments.next().ok_or("--output requires a report path")?);
            }
            "--fixture" => {
                fixture = PathBuf::from(
                    arguments
                        .next()
                        .ok_or("--fixture requires a directory path")?,
                );
            }
            _ => return Err(format!("unknown argument: {argument}")),
        }
    }

    let mut document = nuif_testing::responsive_card_fixture();
    document
        .entities
        .get_mut(&TARGET)
        .expect("fixture target")
        .name = Some("trigger".to_owned());
    let reduction = minimize_document(&document, |candidate| {
        candidate
            .entities
            .get(&TARGET)
            .and_then(|entity| entity.name.as_deref())
            == Some("trigger")
    })
    .map_err(|error| error.to_string())?;
    if reduction.document.entities.len() != 3 {
        return Err(format!(
            "expected a three-entity ancestor path, observed {} entities",
            reduction.document.entities.len()
        ));
    }
    if validate(&reduction.document)
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        return Err("reduced document is invalid".to_owned());
    }
    let operations = [Operation::Rename {
        entity: TARGET,
        name: Some("trigger".to_owned()),
    }];
    let manifest = write_reduced_fixture(
        &fixture,
        "nuif:test:retained-trigger",
        Some(0x7265_6475_6365),
        &reduction,
        &operations,
    )?;
    let choices = nuif_testing::minimize_choice_bytes(&[9, 7, 200, 4], |candidate| {
        candidate.len() >= 2 && candidate.iter().copied().map(u16::from).sum::<u16>() >= 7
    });
    let report = json!({
        "schema_version": 1,
        "status": "passed",
        "source": {
            "revision": command_output("git", &["rev-parse", "HEAD"]),
            "dirty": command_output("git", &["status", "--porcelain"]).map(|value| !value.is_empty()),
        },
        "fixture": fixture,
        "manifest": manifest,
        "document_reduction": reduction.report,
        "choice_reduction": {
            "original_bytes": 4,
            "minimized_bytes": choices.len(),
            "minimized": choices,
        },
        "assertions": {
            "valid_candidates_only": true,
            "ancestor_path_retained": true,
            "existing_destination_rejected": write_reduced_fixture(
                &fixture,
                "nuif:test:retained-trigger",
                None,
                &reduction,
                &operations,
            ).is_err(),
        },
    });
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(
        output,
        serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn command_output(program: &str, arguments: &[&str]) -> Option<String> {
    let output = std::process::Command::new(program)
        .args(arguments)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
