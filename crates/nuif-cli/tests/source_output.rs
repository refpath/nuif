use std::fs;
use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_nuif"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn in_place_source_sync_preserves_foreign_bytes_and_failed_output() {
    let directory = tempfile::tempdir().unwrap();
    let source_path = directory.path().join("source.html");
    let edited_path = directory.path().join("edited.nuif");
    let report_path = directory.path().join("report.json");
    let document = nuif_html::profile_fixture();
    let source = nuif_html::export_document(&document)
        .unwrap()
        .source
        .replace("</body>", "<!-- retained: α🙂 -->\n</body>");
    fs::write(&source_path, &source).unwrap();
    let mut edited = document.clone();
    edited
        .entities
        .get_mut(&nuif_core::EntityId::new(0x11))
        .unwrap()
        .authored
        .text
        .as_mut()
        .unwrap()
        .content = "edited <&> β".to_owned();
    fs::write(
        &edited_path,
        nuif_codec::Encoder::encode(&nuif_codec::CanonicalText, &edited).unwrap(),
    )
    .unwrap();
    let args = [
        "sync",
        "html-css-0",
        source_path.to_str().unwrap(),
        edited_path.to_str().unwrap(),
        source_path.to_str().unwrap(),
        report_path.to_str().unwrap(),
    ];
    let output = run(&args);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let synchronized = fs::read_to_string(&source_path).unwrap();
    assert!(synchronized.contains("<!-- retained: α🙂 -->"));
    assert_eq!(
        nuif_html::import_source(&synchronized).unwrap().document,
        edited
    );
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(&report_path).unwrap()).unwrap();
    assert_eq!(report["adapter"]["unmapped_source_preserved"], true);

    edited
        .entities
        .get_mut(&nuif_core::EntityId::new(0x10))
        .unwrap()
        .authored
        .fill = Some(nuif_core::Color {
        space: nuif_core::ColorSpace::Srgb,
        red: 0.0,
        green: 0.0,
        blue: 0.0,
        alpha: 1.0,
    });
    fs::write(
        &edited_path,
        nuif_codec::Encoder::encode(&nuif_codec::CanonicalText, &edited).unwrap(),
    )
    .unwrap();
    assert!(!run(&args).status.success());
    assert_eq!(fs::read_to_string(&source_path).unwrap(), synchronized);
}

#[test]
fn aliased_report_destinations_are_rejected_before_writing() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source.html");
    let document = directory.path().join("document.nuif");
    fs::write(&source, b"sentinel source").unwrap();
    fs::write(&document, b"sentinel document").unwrap();
    let source_alias = directory.path().join(".").join("source.html");
    for args in [
        vec![
            "import",
            "html-css-0",
            source.to_str().unwrap(),
            document.to_str().unwrap(),
            document.to_str().unwrap(),
        ],
        vec![
            "export",
            document.to_str().unwrap(),
            "html-css-0",
            source.to_str().unwrap(),
            source_alias.to_str().unwrap(),
        ],
        vec![
            "sync",
            "html-css-0",
            source.to_str().unwrap(),
            document.to_str().unwrap(),
            "-",
            source_alias.to_str().unwrap(),
        ],
        vec!["export", document.to_str().unwrap(), "html-css-0", "-", "-"],
    ] {
        let output = run(&args);
        assert_eq!(output.status.code(), Some(2));
        let diagnostic: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(diagnostic["code"], "ARGUMENT_INVALID");
        assert_eq!(fs::read(&source).unwrap(), b"sentinel source");
        assert_eq!(fs::read(&document).unwrap(), b"sentinel document");
        assert!(output.stdout.is_empty());
    }
}
