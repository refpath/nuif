use nuif_core::{
    Document, Entity, EntityId, EntityKind, GridTrack, LayoutFamily, Point, SizeIntent, TextContent,
};
use nuif_editor::{
    AccessibilityAction, EditorCommand, EditorDriver, EditorError, EditorEvent, MAX_SNAPSHOT_EDGE,
    MAX_SNAPSHOT_PIXELS, encode_editor_file,
};
use nuif_package::{NuifPackage, PackageMode};
use nuif_protocol::{Operation, apply_patch};
use serde_json::{Value, json};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

fn main() {
    if let Err(error) = run() {
        eprintln!("editor-hostile-inputs: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let output = output_path()?;
    let started = Instant::now();
    let checks = json!({
        "snapshot_dimensions_typed": snapshot_dimensions_typed(),
        "snapshot_edge_boundary_accepted": snapshot_edge_boundary_accepted(),
        "accessibility_nonfinite_atomic": accessibility_nonfinite_atomic(),
        "accessibility_fill_atomic": accessibility_fill_atomic(),
        "accessibility_grid_atomic": accessibility_grid_atomic(),
        "missing_selection_atomic": missing_selection_atomic(),
        "missing_semantic_node_typed": missing_semantic_node_typed(),
        "multi_operation_failure_atomic": multi_operation_failure_atomic(),
        "empty_history_atomic": empty_history_atomic(),
        "redo_invalidation_exact": redo_invalidation_exact(),
        "history_log_replays_exactly": history_log_replays_exactly(),
        "unsupported_package_capability_is_read_only": unsupported_package_capability_is_read_only(),
    });
    let elapsed_micros = started.elapsed().as_micros();
    let passed = checks
        .as_object()
        .expect("checks is an object")
        .values()
        .all(|value| value == &Value::Bool(true));
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:editor-hostile-interactions",
        "status": if passed { "passed" } else { "failed" },
        "source": {
            "revision": command_text("git", &["rev-parse", "HEAD"]),
            "dirty": command_text("git", &["status", "--porcelain"]).map(|value| !value.is_empty()),
            "toolchain": command_text("rustc", &["--version"]),
            "profile": "release",
            "os": env::consts::OS,
            "architecture": env::consts::ARCH,
        },
        "limits": {
            "snapshot_edge": MAX_SNAPSHOT_EDGE,
            "snapshot_pixels": MAX_SNAPSHOT_PIXELS,
        },
        "summary": {
            "cases": checks.as_object().map_or(0, serde_json::Map::len),
            "blocking_failures": u8::from(!passed),
            "elapsed_micros": elapsed_micros,
        },
        "checks": checks,
    });
    write_json(&output, &report)?;
    println!(
        "Editor hostile interactions: {} cases, status {}",
        report["summary"]["cases"], report["status"]
    );
    if passed {
        Ok(())
    } else {
        Err(format!("report failed; inspect {}", output.display()))
    }
}

fn snapshot_dimensions_typed() -> bool {
    let base = editor_fixture();
    let mut driver = EditorDriver::new(base.clone());
    [
        (0, 1),
        (1, 0),
        (MAX_SNAPSHOT_EDGE + 1, 1),
        (u32::MAX, u32::MAX),
    ]
    .into_iter()
    .all(|(width, height)| {
        matches!(
            driver.execute(EditorCommand::Snapshot { width, height }),
            Err(EditorError::SnapshotDimensions { .. })
        ) && driver.document() == &base
            && driver.operation_log().is_empty()
    })
}

fn snapshot_edge_boundary_accepted() -> bool {
    let mut driver = EditorDriver::new(editor_fixture());
    matches!(
        driver.execute(EditorCommand::Snapshot {
            width: MAX_SNAPSHOT_EDGE,
            height: 1,
        }),
        Ok(EditorEvent::Snapshot { snapshot })
            if snapshot.raster.width == MAX_SNAPSHOT_EDGE && snapshot.raster.height == 1
    )
}

fn accessibility_nonfinite_atomic() -> bool {
    let base = editor_fixture();
    let mut driver = EditorDriver::new(base.clone());
    let card = EntityId::new(0x20);
    let text = EntityId::new(0x21);
    [
        (card, "width", "NaN"),
        (card, "width", "NaN%"),
        (card, "height", "fit-content(inf)"),
        (card, "x", "-inf"),
        (card, "gap", "1e999"),
        (text, "font_size", "NaN"),
        (text, "line_height", "inf"),
    ]
    .into_iter()
    .all(|(author_id, label, value)| {
        matches!(
            driver.dispatch_accessibility_action(AccessibilityAction::SetValue {
                author_id,
                label: label.to_owned(),
                value: value.to_owned(),
            }),
            Err(EditorError::AccessibilityValueInvalid { .. })
        ) && driver.document() == &base
            && driver.operation_log().is_empty()
    })
}

fn accessibility_fill_atomic() -> bool {
    let base = editor_fixture();
    let mut driver = EditorDriver::new(base.clone());
    matches!(
        driver.dispatch_accessibility_action(AccessibilityAction::SetValue {
            author_id: EntityId::new(0x20),
            label: "fill".to_owned(),
            value: "#12345g".to_owned(),
        }),
        Err(EditorError::AccessibilityValueInvalid { .. })
    ) && driver.document() == &base
        && driver.operation_log().is_empty()
}

fn accessibility_grid_atomic() -> bool {
    let base = editor_fixture();
    let mut driver = EditorDriver::new(base.clone());
    [
        (EntityId::new(0x20), "grid_columns", ""),
        (EntityId::new(0x20), "grid_rows", "0fr"),
        (EntityId::new(0x21), "grid_position", "1"),
        (EntityId::new(0x21), "grid_position", "0 1"),
        (EntityId::new(0x21), "grid_column_span", "0"),
        (EntityId::new(0x21), "grid_row_span", "257"),
    ]
    .into_iter()
    .all(|(author_id, label, value)| {
        matches!(
            driver.dispatch_accessibility_action(AccessibilityAction::SetValue {
                author_id,
                label: label.to_owned(),
                value: value.to_owned(),
            }),
            Err(EditorError::AccessibilityValueInvalid { .. })
        ) && driver.document() == &base
            && driver.operation_log().is_empty()
            && !driver.can_undo()
    })
}

fn missing_selection_atomic() -> bool {
    let base = editor_fixture();
    let mut driver = EditorDriver::new(base.clone());
    let card = EntityId::new(0x20);
    if driver
        .execute(EditorCommand::Select { entity: card })
        .is_err()
    {
        return false;
    }
    matches!(
        driver.execute(EditorCommand::Select {
            entity: EntityId::new(u128::MAX),
        }),
        Err(EditorError::EntityMissing(_))
    ) && driver.document() == &base
        && driver.selection() == [card]
        && driver.operation_log().is_empty()
}

fn missing_semantic_node_typed() -> bool {
    let base = editor_fixture();
    let mut driver = EditorDriver::new(base.clone());
    matches!(
        driver.dispatch_accessibility_action(AccessibilityAction::SetValue {
            author_id: EntityId::new(0x20),
            label: "not_a_control".to_owned(),
            value: "1".to_owned(),
        }),
        Err(EditorError::AccessibilityNodeMissing { .. })
    ) && driver.document() == &base
        && driver.operation_log().is_empty()
}

fn multi_operation_failure_atomic() -> bool {
    let base = editor_fixture();
    let mut driver = EditorDriver::new(base.clone());
    matches!(
        driver.execute(EditorCommand::Apply {
            operations: vec![
                Operation::Rename {
                    entity: EntityId::new(0x20),
                    name: Some("must not commit".to_owned()),
                },
                Operation::SetPosition {
                    entity: EntityId::new(u128::MAX),
                    value: Point { x: 1.0, y: 2.0 },
                },
            ],
        }),
        Err(EditorError::Engine(_))
    ) && driver.document() == &base
        && driver.operation_log().is_empty()
        && !driver.can_undo()
}

fn empty_history_atomic() -> bool {
    let base = editor_fixture();
    let mut driver = EditorDriver::new(base.clone());
    driver.execute(EditorCommand::Undo).is_err()
        && driver.execute(EditorCommand::Redo).is_err()
        && driver.document() == &base
        && driver.operation_log().is_empty()
}

fn redo_invalidation_exact() -> bool {
    let mut driver = EditorDriver::new(editor_fixture());
    let card = EntityId::new(0x20);
    if driver
        .execute(EditorCommand::Rename {
            entity: card,
            name: "first".to_owned(),
        })
        .is_err()
        || driver.execute(EditorCommand::Undo).is_err()
        || !driver.can_redo()
        || driver
            .execute(EditorCommand::Rename {
                entity: card,
                name: "replacement".to_owned(),
            })
            .is_err()
    {
        return false;
    }
    !driver.can_redo() && driver.document().entities[&card].name.as_deref() == Some("replacement")
}

fn history_log_replays_exactly() -> bool {
    let base = editor_fixture();
    let mut driver = EditorDriver::new(base.clone());
    let card = EntityId::new(0x20);
    for command in [
        EditorCommand::Rename {
            entity: card,
            name: "one".to_owned(),
        },
        EditorCommand::Undo,
        EditorCommand::Redo,
        EditorCommand::SetPosition {
            entity: card,
            value: Point { x: 12.0, y: 18.0 },
        },
    ] {
        if driver.execute(command).is_err() {
            return false;
        }
    }
    let mut replayed = base;
    for patch in driver.operation_log() {
        if apply_patch(&mut replayed, patch).is_err() {
            return false;
        }
    }
    &replayed == driver.document()
}

fn unsupported_package_capability_is_read_only() -> bool {
    let base = editor_fixture();
    let mut package = NuifPackage::new(base.clone(), PackageMode::Portable);
    package
        .required_capabilities
        .insert("feature.example".to_owned());
    let Ok(encoded) = package.encode() else {
        return false;
    };
    let Ok(mut driver) = EditorDriver::new_with_package(base.clone(), Some(&package)) else {
        return false;
    };
    let selected = driver
        .execute(EditorCommand::Select {
            entity: EntityId::new(0x20),
        })
        .is_ok();
    let rejected = matches!(
        driver.execute(EditorCommand::Rename {
            entity: EntityId::new(0x20),
            name: "must not commit".to_owned(),
        }),
        Err(EditorError::PackageReadOnly { capabilities })
            if capabilities == std::collections::BTreeSet::from(["feature.example".to_owned()])
    );
    let mut no_op_package = Some(package.clone());
    let no_op_exact =
        encode_editor_file(&base, &mut no_op_package).is_ok_and(|bytes| bytes == encoded);
    let mut changed = base.clone();
    changed
        .entities
        .get_mut(&EntityId::new(0x20))
        .expect("fixture card exists")
        .name = Some("must not save".to_owned());
    let mut rejected_package = Some(package.clone());
    let save_rejected = encode_editor_file(&changed, &mut rejected_package).is_err();
    let mismatched_document_rejected = matches!(
        EditorDriver::new_with_package(Document::empty(EntityId::new(0xff)), Some(&package)),
        Err(EditorError::PackageDocumentMismatch)
    );

    selected
        && driver.is_read_only()
        && rejected
        && driver.document() == &base
        && driver.operation_log().is_empty()
        && no_op_exact
        && save_rejected
        && mismatched_document_rejected
        && rejected_package == Some(package)
}

fn output_path() -> Result<PathBuf, String> {
    let mut args = env::args().skip(1);
    let mut output = PathBuf::from("target/editor-hostile-input-report.json");
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--output" => {
                output = PathBuf::from(args.next().ok_or("--output requires a path")?);
            }
            _ => return Err(format!("unknown argument {argument}")),
        }
    }
    Ok(output)
}

fn editor_fixture() -> Document {
    let mut document = Document::empty(EntityId::new(1));
    let surface_id = EntityId::new(0x10);
    let card_id = EntityId::new(0x20);
    let text_id = EntityId::new(0x21);
    let mut surface = Entity::new(surface_id, EntityKind::Surface);
    surface.name = Some("Hostile trial surface".to_owned());
    surface.authored.width = SizeIntent::Fixed(320.0);
    surface.authored.height = SizeIntent::Fixed(200.0);
    surface.children.push(card_id);
    let mut card = Entity::new(card_id, EntityKind::Container);
    card.name = Some("Card".to_owned());
    card.authored.position = Point { x: 16.0, y: 20.0 };
    card.authored.width = SizeIntent::Fixed(288.0);
    card.authored.height = SizeIntent::Fixed(160.0);
    card.authored.layout.family = LayoutFamily::Grid;
    card.authored.layout.grid.columns = vec![GridTrack::Fraction(1.0)];
    card.authored.layout.grid.rows = vec![GridTrack::Fraction(1.0)];
    card.children.push(text_id);
    let mut text = Entity::new(text_id, EntityKind::Text);
    text.name = Some("Label".to_owned());
    text.authored.position = Point { x: 24.0, y: 32.0 };
    text.authored.width = SizeIntent::Fixed(160.0);
    text.authored.height = SizeIntent::Fixed(24.0);
    text.authored.text = Some(TextContent {
        content: "Hostile trial".to_owned(),
        font: nuif_text::PINNED_FONT_NAME.to_owned(),
        font_sha256: nuif_text::PINNED_FONT_SHA256.to_owned(),
        font_asset: None,
        size: 16.0,
        line_height: 24.0,
    });
    document.roots.push(surface_id);
    document.entities.insert(surface_id, surface);
    document.entities.insert(card_id, card);
    document.entities.insert(text_id, text);
    document
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn command_text(program: &str, arguments: &[&str]) -> Option<String> {
    let output = Command::new(program).args(arguments).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
