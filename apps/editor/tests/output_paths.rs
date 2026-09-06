use std::{fs, process::Command};

#[test]
fn headless_report_aliases_leave_document_and_script_bytes_unchanged() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("input.nuif");
    let output = directory.path().join("output.nuif");
    let script = directory.path().join("script.jsonl");
    let expected = directory.path().join("expected.nuif");
    for path in [&input, &output, &script, &expected] {
        fs::write(path, b"sentinel").unwrap();
    }
    let indirect = directory
        .path()
        .join("missing")
        .join("..")
        .join("output.nuif");
    for report in [&input, &output, &script, &expected, &indirect] {
        let result = Command::new(env!("CARGO_BIN_EXE_nuif-editor"))
            .arg("--headless")
            .arg("--document")
            .arg(&input)
            .arg("--script")
            .arg(&script)
            .arg("--expect-document")
            .arg(&expected)
            .arg("--output")
            .arg(&output)
            .arg("--report")
            .arg(report)
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("--report must be separate"));
        for path in [&input, &output, &script, &expected] {
            assert_eq!(fs::read(path).unwrap(), b"sentinel");
        }
        assert!(!directory.path().join("missing").exists());
    }
}

#[test]
fn headless_document_output_cannot_overwrite_the_script_or_expectation() {
    let directory = tempfile::tempdir().unwrap();
    let script = directory.path().join("script.jsonl");
    let expected = directory.path().join("expected.nuif");
    for path in [&script, &expected] {
        fs::write(path, b"sentinel").unwrap();
    }
    for output in [&script, &expected] {
        let result = Command::new(env!("CARGO_BIN_EXE_nuif-editor"))
            .arg("--headless")
            .args(["--new-document", "00000000000000000000000000000001"])
            .arg("--script")
            .arg(&script)
            .arg("--expect-document")
            .arg(&expected)
            .arg("--output")
            .arg(output)
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("--output must be separate"));
        for path in [&script, &expected] {
            assert_eq!(fs::read(path).unwrap(), b"sentinel");
        }
    }
}
