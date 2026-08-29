use std::env;
use std::fs;
use std::path::Path;
use std::process::{Command, ExitStatus};

type Step = (&'static str, fn() -> Result<(), String>);

const ALL_STEPS: &[Step] = &[
    ("research", research),
    ("verify", verify),
    ("gate-b", gate_b),
    ("hostile-inputs", hostile_inputs),
    ("browser-install", browser_install),
    ("gate-c", gate_c),
    ("gate-d", gate_d),
    ("editor-trial", editor_trial),
    ("gate-f", gate_f),
    ("gate-f-v0", gate_f_v0),
    ("gate-g", gate_g),
];

const VERIFICATION_ARTIFACTS: &[&str] = &[
    "target/gate-b-report.json",
    "target/hostile-input-report.json",
    "target/layout-differential-report.json",
    "target/text-pinning-report.json",
    "target/render-profile-report.json",
    "target/editor-authoring-report.json",
    "target/editor-authoring-snapshot",
    "target/html-sync-report.json",
    "target/html-sync-output.html",
    "target/html-sync-v0-report.json",
    "target/html-sync-v0-output.html",
    "target/html-sync-v0-editor-report.json",
    "target/html-sync-v0-editor-output.html",
    "target/gate-g-report.json",
    "target/gate-g-independent",
];

fn main() {
    if let Err(error) = run() {
        eprintln!("xtask: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("verify") => verify(),
        Some("trial") => {
            let seed = args.next().unwrap_or_else(|| "24301".to_owned());
            let iterations = args.next().unwrap_or_else(|| "100".to_owned());
            let snapshot_interval = args.next().unwrap_or_else(|| "1".to_owned());
            let report = args.next();
            let mut command = vec![
                "run",
                "--locked",
                "-p",
                "nuif-cli",
                "--",
                "trial",
                &seed,
                &iterations,
                &snapshot_interval,
            ];
            if let Some(report) = report.as_deref() {
                command.push(report);
            }
            cargo(&command)
        }
        Some("gate-b") => gate_b(),
        Some("gate-c") => gate_c(),
        Some("gate-d") => gate_d(),
        Some("gate-d-text") => gate_d_text(),
        Some("gate-d-render") => gate_d_render(),
        Some("gate-f") => gate_f(),
        Some("gate-f-v0") => gate_f_v0(),
        Some("gate-g") => gate_g(),
        Some("browser-install") => browser_install(),
        Some("hostile-inputs") => hostile_inputs(),
        Some("research") => research(),
        Some("editor-trial") => editor_trial(),
        Some("manifest") => standalone_manifest(),
        Some("all") => all(),
        _ => Err(
            "usage: cargo xtask <research|verify|trial [seed iterations snapshot-interval report-path]|gate-b|gate-c|gate-d|gate-d-text|gate-d-render|gate-f|gate-f-v0|gate-g|browser-install|hostile-inputs|editor-trial|manifest|all>"
                .to_owned(),
        ),
    }
}

fn all() -> Result<(), String> {
    let mut completed = Vec::new();
    for (name, step) in ALL_STEPS {
        if let Err(error) = step() {
            let manifest_error = verification_manifest(
                "complete-run",
                "cargo xtask all",
                &completed,
                Some((name, &error)),
            )
            .err()
            .map(|failure| format!("; manifest error: {failure}"))
            .unwrap_or_default();
            return Err(format!("{name}: {error}{manifest_error}"));
        }
        completed.push(*name);
    }
    verification_manifest("complete-run", "cargo xtask all", &completed, None)
}

fn standalone_manifest() -> Result<(), String> {
    let missing = VERIFICATION_ARTIFACTS
        .iter()
        .filter(|path| !Path::new(path).exists())
        .copied()
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return verification_manifest("artifact-index", "cargo xtask manifest", &[], None);
    }

    let message = format!("missing expected artifacts: {}", missing.join(", "));
    verification_manifest(
        "artifact-index",
        "cargo xtask manifest",
        &[],
        Some(("artifact-index", &message)),
    )?;
    Err(message)
}

fn gate_b() -> Result<(), String> {
    cargo(&[
        "run",
        "--locked",
        "-p",
        "nuif-cli",
        "--",
        "trial",
        "24301",
        "10000",
        "100",
        "target/gate-b-report.json",
    ])
}

fn hostile_inputs() -> Result<(), String> {
    cargo(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "nuif-testing",
        "--bin",
        "hostile-inputs",
        "--",
        "--output",
        "target/hostile-input-report.json",
    ])
}

fn browser_install() -> Result<(), String> {
    command("sh", &["tools/browser/install-chrome-for-testing.sh"])
}

fn gate_c() -> Result<(), String> {
    cargo(&[
        "run",
        "--locked",
        "-p",
        "nuif-testing",
        "--bin",
        "layout-differential",
        "--",
        "--output",
        "target/layout-differential-report.json",
    ])
}

fn gate_d_text() -> Result<(), String> {
    cargo(&[
        "run",
        "--locked",
        "-p",
        "nuif-testing",
        "--bin",
        "text-pinning",
        "--",
        "--output",
        "target/text-pinning-report.json",
    ])
}

fn gate_d_render() -> Result<(), String> {
    cargo(&[
        "run",
        "--locked",
        "-p",
        "nuif-testing",
        "--bin",
        "render-profile",
        "--",
        "--output",
        "target/render-profile-report.json",
    ])
}

fn gate_d() -> Result<(), String> {
    gate_d_text()?;
    gate_d_render()
}

fn gate_f() -> Result<(), String> {
    cargo(&[
        "run",
        "--locked",
        "-p",
        "nuif-html",
        "--bin",
        "html-sync-profile",
        "--",
        "--output",
        "target/html-sync-report.json",
        "--source-output",
        "target/html-sync-output.html",
    ])
}

fn gate_f_v0() -> Result<(), String> {
    cargo(&[
        "run",
        "--locked",
        "-p",
        "nuif-testing",
        "--bin",
        "html-sync-v0",
        "--",
        "--output",
        "target/html-sync-v0-report.json",
        "--source-output",
        "target/html-sync-v0-output.html",
    ])?;
    gate_f_v0_editor_bridge()
}

fn gate_f_v0_editor_bridge() -> Result<(), String> {
    let directory = env::temp_dir().join(format!("nuif-html-v0-trial-{}", std::process::id()));
    if directory.exists() {
        return Err(format!(
            "temporary path already exists: {}",
            directory.display()
        ));
    }
    fs::create_dir(&directory).map_err(|error| error.to_string())?;
    let input = directory.join("input.nuif");
    let exported = directory.join("exported.html");
    let export_report = directory.join("export-report.json");
    let edited = directory.join("editor-output.nuif");
    let reimported = directory.join("reimported.nuif");
    let import_report = directory.join("import-report.json");
    let synchronized = Path::new("target/html-sync-v0-editor-output.html");
    let sync_report = Path::new("target/html-sync-v0-editor-report.json");
    cargo(&[
        "run",
        "--quiet",
        "--locked",
        "-p",
        "nuif-cli",
        "--",
        "fixture",
        "v0-responsive-card",
        path(&input)?,
    ])?;
    cargo(&[
        "run",
        "--quiet",
        "--locked",
        "-p",
        "nuif-cli",
        "--",
        "export",
        path(&input)?,
        "html-css-v0",
        path(&exported)?,
        path(&export_report)?,
    ])?;
    cargo(&[
        "run",
        "--quiet",
        "--locked",
        "-p",
        "nuif-editor",
        "--",
        "--headless",
        "--script",
        "conformance/fixtures/v0-responsive-card/editor-trial.jsonl",
        "--document",
        path(&input)?,
        "--output",
        path(&edited)?,
    ])?;
    cargo(&[
        "run",
        "--quiet",
        "--locked",
        "-p",
        "nuif-cli",
        "--",
        "sync",
        "html-css-v0",
        path(&exported)?,
        path(&edited)?,
        path(synchronized)?,
        path(sync_report)?,
    ])?;
    cargo(&[
        "run",
        "--quiet",
        "--locked",
        "-p",
        "nuif-cli",
        "--",
        "import",
        "html-css-v0",
        path(synchronized)?,
        path(&reimported)?,
        path(&import_report)?,
    ])?;
    if fs::read(&edited).map_err(|error| error.to_string())?
        != fs::read(&reimported).map_err(|error| error.to_string())?
    {
        return Err(
            "editor-authored NUIF changed during full-v0 source synchronization".to_owned(),
        );
    }
    let report = read_json(sync_report)?;
    if report["status"] != "passed" || report["edits"].as_array().map(Vec::len) != Some(2) {
        return Err("editor bridge did not produce the expected name and width edits".to_owned());
    }
    fs::remove_dir_all(&directory).map_err(|error| {
        format!(
            "trial passed but temporary directory {} could not be removed: {error}",
            directory.display()
        )
    })
}

fn read_json(path: &Path) -> Result<serde_json::Value, String> {
    serde_json::from_slice(&fs::read(path).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())
}

fn gate_g() -> Result<(), String> {
    let reference = Path::new("target/gate-g-reference");
    let independent = Path::new("target/gate-g-independent");
    for directory in [reference, independent] {
        if directory.exists() {
            fs::remove_dir_all(directory).map_err(|error| error.to_string())?;
        }
    }
    fs::create_dir_all(reference).map_err(|error| error.to_string())?;
    let input = reference.join("input.nuif");
    cargo(&[
        "run",
        "--quiet",
        "--locked",
        "-p",
        "nuif-cli",
        "--",
        "fixture",
        "v0-responsive-card",
        path(&input)?,
    ])?;
    for (name, width, height) in [
        ("360x640", "360", "640"),
        ("768x768", "768", "768"),
        ("1440x900", "1440", "900"),
    ] {
        let output = reference.join(name);
        cargo(&[
            "run",
            "--quiet",
            "--locked",
            "-p",
            "nuif-cli",
            "--",
            "snapshot",
            path(&input)?,
            path(&output)?,
            width,
            height,
        ])?;
    }
    command(
        "python3",
        &[
            "-m",
            "unittest",
            "discover",
            "-s",
            "implementations/python/tests",
            "-p",
            "test_*.py",
        ],
    )?;
    command(
        "python3",
        &[
            "implementations/python/nuif_profile0.py",
            "verify",
            "--input",
            path(&input)?,
            "--case",
            "360x640=target/gate-g-reference/360x640",
            "--case",
            "768x768=target/gate-g-reference/768x768",
            "--case",
            "1440x900=target/gate-g-reference/1440x900",
            "--output",
            "target/gate-g-report.json",
            "--artifact-dir",
            path(independent)?,
        ],
    )
}

fn research() -> Result<(), String> {
    let environment = Path::new("target/research-validator-venv");
    #[cfg(windows)]
    let (python, pip) = (
        environment.join("Scripts/python.exe"),
        environment.join("Scripts/pip.exe"),
    );
    #[cfg(not(windows))]
    let (python, pip) = (environment.join("bin/python"), environment.join("bin/pip"));
    if !python.exists() {
        command("python3", &["-m", "venv", path(environment)?])?;
    }
    command(
        path(&pip)?,
        &[
            "install",
            "--quiet",
            "--disable-pip-version-check",
            "--requirement",
            "tools/research/requirements.txt",
        ],
    )?;
    command(path(&python)?, &["tools/research/validate.py"])
}

fn verify() -> Result<(), String> {
    cargo(&["fmt", "--all", "--", "--check"])?;
    cargo(&["check", "--workspace", "--all-targets", "--locked"])?;
    cargo(&["test", "--workspace", "--locked"])?;
    cargo(&[
        "clippy",
        "--workspace",
        "--all-targets",
        "--locked",
        "--",
        "-D",
        "warnings",
    ])?;
    cargo(&[
        "run", "--locked", "-p", "nuif-cli", "--", "trial", "24301", "100",
    ])
}

fn editor_trial() -> Result<(), String> {
    let directory = env::temp_dir().join(format!("nuif-editor-trial-{}", std::process::id()));
    if directory.exists() {
        return Err(format!(
            "temporary path already exists: {}",
            directory.display()
        ));
    }
    fs::create_dir(&directory).map_err(|error| error.to_string())?;
    let input = directory.join("v0.nuif");
    let edited = directory.join("edited.nuif");
    let authored = directory.join("authored.nuif");
    let report = Path::new("target/editor-authoring-report.json");
    let snapshots = Path::new("target/editor-authoring-snapshot");
    cargo(&[
        "run",
        "--locked",
        "-p",
        "nuif-cli",
        "--",
        "fixture",
        "v0-responsive-card",
        path(&input)?,
    ])?;
    cargo(&[
        "run",
        "--locked",
        "-p",
        "nuif-editor",
        "--",
        "--headless",
        "--script",
        "conformance/fixtures/v0-responsive-card/editor-trial.jsonl",
        "--document",
        path(&input)?,
        "--output",
        path(&edited)?,
    ])?;
    cargo(&[
        "run",
        "--locked",
        "-p",
        "nuif-cli",
        "--",
        "validate",
        path(&edited)?,
    ])?;
    cargo(&[
        "run",
        "--locked",
        "-p",
        "nuif-editor",
        "--",
        "--headless",
        "--script",
        "conformance/fixtures/v0-responsive-card/editor-authoring.jsonl",
        "--new-document",
        "00000000000000000000000000000001",
        "--expect-document",
        path(&input)?,
        "--output",
        path(&authored)?,
        "--snapshot-dir",
        path(snapshots)?,
        "--report",
        path(report)?,
    ])?;
    let expected = fs::read(&input).map_err(|error| error.to_string())?;
    let observed = fs::read(&authored).map_err(|error| error.to_string())?;
    if observed != expected {
        return Err(
            "semantic editor authoring did not exactly reproduce the v0 fixture".to_owned(),
        );
    }
    cargo(&[
        "run",
        "--locked",
        "-p",
        "nuif-cli",
        "--",
        "validate",
        path(&authored)?,
    ])?;
    fs::remove_dir_all(&directory).map_err(|error| {
        format!(
            "trial passed but temporary directory {} could not be removed: {error}",
            directory.display()
        )
    })
}

fn verification_manifest(
    mode: &str,
    entrypoint: &str,
    completed: &[&str],
    failure: Option<(&str, &str)>,
) -> Result<(), String> {
    let status = if failure.is_none() {
        "passed"
    } else {
        "failed"
    };
    let failed_step = failure.as_ref().map(|(step, _)| *step);
    let failure_message = failure.as_ref().map(|(_, message)| message);
    let artifacts = VERIFICATION_ARTIFACTS
        .iter()
        .map(|path| {
            let path = Path::new(path);
            serde_json::json!({
                "path": path,
                "present": path.exists(),
                "kind": if path.is_dir() { "directory" } else { "file" }
            })
        })
        .collect::<Vec<_>>();
    let report = serde_json::json!({
        "schema_version": 1,
        "mode": mode,
        "status": status,
        "source": {
            "revision": command_text("git", &["rev-parse", "HEAD"]),
            "dirty": command_text("git", &["status", "--porcelain"]).map(|value| !value.is_empty()),
            "toolchain": command_text("rustc", &["--version"]),
            "os": env::consts::OS,
            "architecture": env::consts::ARCH,
        },
        "entrypoint": entrypoint,
        "completed_steps": completed,
        "failed_step": failed_step,
        "failure": failure_message,
        "artifacts": artifacts
    });
    let path = Path::new("target/verification-manifest.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn cargo(arguments: &[&str]) -> Result<(), String> {
    command("cargo", arguments)
}

fn command(program: &str, arguments: &[&str]) -> Result<(), String> {
    let status = Command::new(program)
        .args(arguments)
        .status()
        .map_err(|error| error.to_string())?;
    check_status(status, program, arguments)
}

fn command_text(program: &str, arguments: &[&str]) -> Option<String> {
    let output = Command::new(program).args(arguments).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn check_status(status: ExitStatus, program: &str, arguments: &[&str]) -> Result<(), String> {
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "{program} {} failed with {status}",
            arguments.join(" ")
        ))
    }
}

fn path(path: &Path) -> Result<&str, String> {
    path.to_str()
        .ok_or_else(|| format!("path is not valid UTF-8: {}", path.display()))
}
