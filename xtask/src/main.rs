use std::env;
use std::fs;
use std::path::Path;
use std::process::{Command, ExitStatus};

use sha2::{Digest, Sha256};

type Step = (&'static str, fn() -> Result<(), String>);

const ALL_STEPS: &[Step] = &[
    ("research", research),
    ("verify", verify),
    ("gate-b", gate_b),
    ("hostile-inputs", hostile_inputs),
    ("performance", performance),
    ("browser-install", browser_install),
    ("gate-c", gate_c),
    ("gate-d", gate_d),
    ("editor-trial", editor_trial),
    ("editor-gui-trial", editor_gui_trial),
    ("gate-f", gate_f),
    ("gate-f-v0", gate_f_v0),
    ("gate-svg", gate_svg),
    ("gate-dtcg", gate_dtcg),
    ("gate-g", gate_g),
    ("gate-h", gate_h),
];

const VERIFICATION_ARTIFACTS: &[&str] = &[
    "target/gate-b-report.json",
    "target/hostile-input-report.json",
    "target/performance-profile-report.json",
    "target/layout-differential-report.json",
    "target/text-pinning-report.json",
    "target/render-profile-report.json",
    "target/editor-authoring-report.json",
    "target/editor-authoring-snapshot",
    "target/editor-gui-trial",
    "target/html-sync-report.json",
    "target/html-sync-output.html",
    "target/html-sync-v0-report.json",
    "target/html-sync-v0-output.html",
    "target/html-sync-v0-editor-report.json",
    "target/html-sync-v0-editor-output.html",
    "target/svg-sync-report.json",
    "target/svg-sync-output.svg",
    "target/svg-sync-edited.nuif",
    "target/svg-sync-cli-report.json",
    "target/svg-sync-cli-output.svg",
    "target/dtcg-sync-report.json",
    "target/dtcg-sync-output.tokens.json",
    "target/dtcg-sync-edited.nuif",
    "target/dtcg-sync-cli-report.json",
    "target/dtcg-sync-cli-output.tokens.json",
    "target/gate-g-report.json",
    "target/gate-g-independent",
    "target/collaboration-report.json",
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
        Some("gate-svg") => gate_svg(),
        Some("gate-dtcg") => gate_dtcg(),
        Some("gate-g") => gate_g(),
        Some("gate-h") => gate_h(),
        Some("browser-install") => browser_install(),
        Some("hostile-inputs") => hostile_inputs(),
        Some("performance") => performance(),
        Some("research") => research(),
        Some("editor-trial") => editor_trial(),
        Some("editor-gui-trial") => editor_gui_trial(),
        Some("editor-package") => editor_package(),
        Some("editor-launch") => editor_launch(),
        Some("manifest") => standalone_manifest(),
        Some("all") => all(),
        _ => Err(
            "usage: cargo xtask <research|verify|trial [seed iterations snapshot-interval report-path]|gate-b|gate-c|gate-d|gate-d-text|gate-d-render|gate-f|gate-f-v0|gate-svg|gate-dtcg|gate-g|gate-h|browser-install|hostile-inputs|performance|editor-trial|editor-gui-trial|editor-package|editor-launch|manifest|all>"
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

fn performance() -> Result<(), String> {
    cargo(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "nuif-testing",
        "--bin",
        "performance-profile",
        "--",
        "--output",
        "target/performance-profile-report.json",
    ])?;
    cargo(&[
        "bench",
        "--locked",
        "-p",
        "nuif-conformance",
        "--bench",
        "profile_zero",
        "--no-run",
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

fn gate_svg() -> Result<(), String> {
    cargo(&[
        "run",
        "--locked",
        "-p",
        "nuif-svg",
        "--bin",
        "svg-sync-profile",
        "--",
        "--output",
        "target/svg-sync-report.json",
        "--source-output",
        "target/svg-sync-output.svg",
        "--edited-output",
        "target/svg-sync-edited.nuif",
    ])?;
    gate_svg_cli_bridge()
}

fn gate_svg_cli_bridge() -> Result<(), String> {
    let directory = env::temp_dir().join(format!("nuif-svg-trial-{}", std::process::id()));
    if directory.exists() {
        return Err(format!(
            "temporary path already exists: {}",
            directory.display()
        ));
    }
    fs::create_dir(&directory).map_err(|error| error.to_string())?;
    let input = directory.join("input.nuif");
    let exported = directory.join("exported.svg");
    let export_report = directory.join("export-report.json");
    let imported = directory.join("imported.nuif");
    let import_report = directory.join("import-report.json");
    let reimported = directory.join("reimported.nuif");
    let reimport_report = directory.join("reimport-report.json");
    let synchronized = Path::new("target/svg-sync-cli-output.svg");
    let sync_report = Path::new("target/svg-sync-cli-report.json");
    let edited = Path::new("target/svg-sync-edited.nuif");
    nuif(&["fixture", "svg-profile", path(&input)?])?;
    nuif(&[
        "export",
        path(&input)?,
        "svg-0",
        path(&exported)?,
        path(&export_report)?,
    ])?;
    nuif(&[
        "import",
        "svg-0",
        path(&exported)?,
        path(&imported)?,
        path(&import_report)?,
    ])?;
    if fs::read(&input).map_err(|error| error.to_string())?
        != fs::read(&imported).map_err(|error| error.to_string())?
    {
        return Err("CLI SVG export/import changed canonical NUIF bytes".to_owned());
    }
    nuif(&[
        "sync",
        "svg-0",
        path(&exported)?,
        path(edited)?,
        path(synchronized)?,
        path(sync_report)?,
    ])?;
    nuif(&[
        "import",
        "svg-0",
        path(synchronized)?,
        path(&reimported)?,
        path(&reimport_report)?,
    ])?;
    if fs::read(edited).map_err(|error| error.to_string())?
        != fs::read(&reimported).map_err(|error| error.to_string())?
    {
        return Err("CLI SVG synchronization changed edited canonical NUIF bytes".to_owned());
    }
    let report = read_json(sync_report)?;
    if report["status"] != "passed" || report["edits"].as_array().map(Vec::len) != Some(7) {
        return Err("CLI SVG bridge did not produce the expected seven source edits".to_owned());
    }
    fs::remove_dir_all(&directory).map_err(|error| {
        format!(
            "trial passed but temporary directory {} could not be removed: {error}",
            directory.display()
        )
    })
}

fn gate_dtcg() -> Result<(), String> {
    cargo(&[
        "run",
        "--locked",
        "-p",
        "nuif-dtcg",
        "--bin",
        "dtcg-sync-profile",
        "--",
        "--output",
        "target/dtcg-sync-report.json",
        "--source-output",
        "target/dtcg-sync-output.tokens.json",
        "--edited-output",
        "target/dtcg-sync-edited.nuif",
    ])?;
    gate_dtcg_cli_bridge()
}

fn gate_dtcg_cli_bridge() -> Result<(), String> {
    let directory = env::temp_dir().join(format!("nuif-dtcg-trial-{}", std::process::id()));
    if directory.exists() {
        return Err(format!(
            "temporary path already exists: {}",
            directory.display()
        ));
    }
    fs::create_dir(&directory).map_err(|error| error.to_string())?;
    let input = directory.join("input.nuif");
    let exported = directory.join("exported.tokens.json");
    let export_report = directory.join("export-report.json");
    let imported = directory.join("imported.nuif");
    let import_report = directory.join("import-report.json");
    let reimported = directory.join("reimported.nuif");
    let reimport_report = directory.join("reimport-report.json");
    let synchronized = Path::new("target/dtcg-sync-cli-output.tokens.json");
    let sync_report = Path::new("target/dtcg-sync-cli-report.json");
    let edited = Path::new("target/dtcg-sync-edited.nuif");
    nuif(&["fixture", "dtcg-profile", path(&input)?])?;
    nuif(&[
        "export",
        path(&input)?,
        "dtcg-scalar-0",
        path(&exported)?,
        path(&export_report)?,
    ])?;
    nuif(&[
        "import",
        "dtcg-scalar-0",
        path(&exported)?,
        path(&imported)?,
        path(&import_report)?,
    ])?;
    if fs::read(&input).map_err(|error| error.to_string())?
        != fs::read(&imported).map_err(|error| error.to_string())?
    {
        return Err("CLI DTCG export/import changed canonical NUIF bytes".to_owned());
    }
    nuif(&[
        "sync",
        "dtcg-scalar-0",
        path(&exported)?,
        path(edited)?,
        path(synchronized)?,
        path(sync_report)?,
    ])?;
    nuif(&[
        "import",
        "dtcg-scalar-0",
        path(synchronized)?,
        path(&reimported)?,
        path(&reimport_report)?,
    ])?;
    if fs::read(edited).map_err(|error| error.to_string())?
        != fs::read(&reimported).map_err(|error| error.to_string())?
    {
        return Err("CLI DTCG synchronization changed edited canonical NUIF bytes".to_owned());
    }
    let report = read_json(sync_report)?;
    if report["status"] != "passed" || report["edits"].as_array().map(Vec::len) != Some(8) {
        return Err("CLI DTCG bridge did not produce the expected eight source edits".to_owned());
    }
    fs::remove_dir_all(&directory).map_err(|error| {
        format!(
            "trial passed but temporary directory {} could not be removed: {error}",
            directory.display()
        )
    })
}

fn nuif(arguments: &[&str]) -> Result<(), String> {
    let mut command = vec!["run", "--quiet", "--locked", "-p", "nuif-cli", "--"];
    command.extend_from_slice(arguments);
    cargo(&command)
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

fn gate_h() -> Result<(), String> {
    cargo(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "nuif-conformance",
        "--bin",
        "collaboration-registers",
        "--",
        "--output",
        "target/collaboration-report.json",
    ])
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

fn editor_gui_trial() -> Result<(), String> {
    let artifacts = Path::new("target/editor-gui-trial");
    if artifacts.exists() {
        fs::remove_dir_all(artifacts).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(artifacts).map_err(|error| error.to_string())?;
    let input = artifacts.join("input.nuif");
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
    run_editor_gui_automation(&input, artifacts)?;
    cargo(&[
        "run",
        "--quiet",
        "--locked",
        "-p",
        "nuif-cli",
        "--",
        "validate",
        "target/editor-gui-trial/output.nuif",
    ])?;

    let reproduction = env::temp_dir().join(format!(
        "nuif-editor-gui-reproduction-{}",
        std::process::id()
    ));
    if reproduction.exists() {
        return Err(format!(
            "temporary path already exists: {}",
            reproduction.display()
        ));
    }
    run_editor_gui_automation(&input, &reproduction)?;
    let first = read_json(&artifacts.join("report.json"))?;
    let second = read_json(&reproduction.join("report.json"))?;
    for field in [
        "canonical_hash",
        "replay_hash",
        "shell_rgba_sha256",
        "document_rgba_sha256",
    ] {
        if first[field] != second[field] {
            return Err(format!(
                "native editor trial is not reproducible for {field}: first={}, second={}",
                first[field], second[field]
            ));
        }
    }
    if first["status"] != "passed"
        || first["window"] != serde_json::json!([1280, 800])
        || first["semantic_nodes"] != 19
        || first["operations"] != 7
    {
        return Err("native editor trial report failed its evidence assertions".to_owned());
    }
    fs::remove_dir_all(&reproduction).map_err(|error| {
        format!(
            "trial passed but temporary directory {} could not be removed: {error}",
            reproduction.display()
        )
    })
}

fn run_editor_gui_automation(input: &Path, artifacts: &Path) -> Result<(), String> {
    cargo(&[
        "run",
        "--quiet",
        "--locked",
        "-p",
        "nuif-editor",
        "--bin",
        "nuif-editor-automation",
        "--features",
        "editor-automation",
        "--",
        "--document",
        path(input)?,
        "--scenario",
        "conformance/fixtures/editor-native-trial.json",
        "--artifact-dir",
        path(artifacts)?,
    ])
}

#[derive(Debug)]
struct EditorPackage {
    package_root: std::path::PathBuf,
    binary: std::path::PathBuf,
    app_bundle: Option<std::path::PathBuf>,
    archive: std::path::PathBuf,
}

fn editor_package() -> Result<(), String> {
    let package = build_editor_package()?;
    println!("packaged native editor: {}", package.package_root.display());
    Ok(())
}

fn build_editor_package() -> Result<EditorPackage, String> {
    cargo(&[
        "build",
        "--release",
        "--locked",
        "-p",
        "nuif-editor",
        "--bin",
        "nuif-editor-app",
    ])?;
    let target_root = env::var_os("CARGO_TARGET_DIR").map_or_else(
        || std::path::PathBuf::from("target"),
        std::path::PathBuf::from,
    );
    let executable_suffix = if cfg!(windows) { ".exe" } else { "" };
    let source_binary = target_root
        .join("release")
        .join(format!("nuif-editor-app{executable_suffix}"));
    if !source_binary.is_file() {
        return Err(format!(
            "release editor binary is absent: {}",
            source_binary.display()
        ));
    }
    let package_name = format!("nuif-editor-{}-{}", env::consts::OS, env::consts::ARCH);
    let dist = target_root.join("dist");
    let package_root = dist.join(&package_name);
    if package_root.exists() {
        fs::remove_dir_all(&package_root).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(&package_root).map_err(|error| error.to_string())?;
    let version = editor_version()?;

    let (binary, app_bundle, launch) = match env::consts::OS {
        "macos" => package_macos(&source_binary, &package_root, &version)?,
        "windows" => package_windows(&source_binary, &package_root)?,
        "linux" => package_linux(&source_binary, &package_root)?,
        platform => {
            return Err(format!(
                "native editor packaging is unsupported on {platform}"
            ));
        }
    };
    for license in ["LICENSE-APACHE", "LICENSE-MIT"] {
        fs::copy(license, package_root.join(license)).map_err(|error| error.to_string())?;
    }
    write_package_readme(&package_root.join("README.txt"), &version)?;
    let archive = verify_editor_package(
        &binary,
        &package_root,
        &dist,
        &package_name,
        &version,
        &launch,
    )?;
    Ok(EditorPackage {
        package_root,
        binary,
        app_bundle,
        archive,
    })
}

fn verify_editor_package(
    binary: &Path,
    package_root: &Path,
    dist: &Path,
    package_name: &str,
    version: &str,
    launch: &[String],
) -> Result<std::path::PathBuf, String> {
    let smoke = Command::new(binary)
        .arg("--help")
        .output()
        .map_err(|error| format!("could not execute packaged editor: {error}"))?;
    check_status(smoke.status, path(binary)?, &["--help"])?;
    #[cfg(not(windows))]
    if !String::from_utf8_lossy(&smoke.stdout).contains("usage: nuif-editor") {
        return Err("packaged editor help smoke test returned unexpected output".to_owned());
    }

    let bytes = fs::read(binary).map_err(|error| error.to_string())?;
    let smoke_command = [binary.as_os_str(), std::ffi::OsStr::new("--help")]
        .iter()
        .map(|value| value.to_string_lossy())
        .collect::<Vec<_>>();
    let mut manifest = serde_json::json!({
        "schema_version": 1,
        "status": "passed",
        "name": "NUIF Editor",
        "version": version,
        "platform": env::consts::OS,
        "architecture": env::consts::ARCH,
        "package_root": package_root,
        "binary": binary,
        "binary_bytes": bytes.len(),
        "binary_sha256": format!("{:x}", Sha256::digest(&bytes)),
        "source_revision": command_text("git", &["rev-parse", "HEAD"]),
        "source_dirty": command_text("git", &["status", "--porcelain"])
            .map(|value| !value.is_empty()),
        "launch": launch,
        "smoke_test": {
            "command": smoke_command,
            "status": "passed"
        },
        "signing": {
            "status": "unsigned",
            "note": "development package; release signing requires platform credentials"
        }
    });
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?;
    fs::write(package_root.join("manifest.json"), &manifest_bytes)
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(dist).map_err(|error| error.to_string())?;
    let archive = create_editor_archive(dist, package_root, package_name)?;
    let archive_bytes = fs::read(&archive).map_err(|error| error.to_string())?;
    manifest["archive"] = serde_json::json!({
        "path": archive,
        "bytes": archive_bytes.len(),
        "sha256": format!("{:x}", Sha256::digest(&archive_bytes))
    });
    fs::write(
        dist.join("editor-package-manifest.json"),
        serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    Ok(archive)
}

fn create_editor_archive(
    dist: &Path,
    package_root: &Path,
    package_name: &str,
) -> Result<std::path::PathBuf, String> {
    let extension = if cfg!(windows) { "zip" } else { "tar.gz" };
    let archive = dist.join(format!("{package_name}.{extension}"));
    if archive.exists() {
        fs::remove_file(&archive).map_err(|error| error.to_string())?;
    }
    let archive_path = path(&archive)?;
    let dist_path = path(dist)?;
    if cfg!(windows) {
        command(
            "tar",
            &[
                "-a",
                "-c",
                "-f",
                archive_path,
                "-C",
                dist_path,
                package_name,
            ],
        )?;
    } else {
        command(
            "tar",
            &[
                "-c",
                "-z",
                "-f",
                archive_path,
                "-C",
                dist_path,
                package_name,
            ],
        )?;
    }
    if !archive.is_file()
        || fs::metadata(&archive)
            .map_err(|error| error.to_string())?
            .len()
            == 0
    {
        return Err(format!(
            "native editor archive is absent or empty: {}",
            archive.display()
        ));
    }
    debug_assert_eq!(
        package_root.file_name().and_then(|name| name.to_str()),
        Some(package_name)
    );
    Ok(archive)
}

fn package_macos(
    source_binary: &Path,
    package_root: &Path,
    version: &str,
) -> Result<(std::path::PathBuf, Option<std::path::PathBuf>, Vec<String>), String> {
    let app = package_root.join("NUIF Editor.app");
    let contents = app.join("Contents");
    let macos = contents.join("MacOS");
    let resources = contents.join("Resources");
    fs::create_dir_all(&macos).map_err(|error| error.to_string())?;
    fs::create_dir_all(&resources).map_err(|error| error.to_string())?;
    let binary = macos.join("NUIF Editor");
    fs::copy(source_binary, &binary).map_err(|error| error.to_string())?;
    let plist = fs::read_to_string("apps/editor/packaging/macos/Info.plist.in")
        .map_err(|error| error.to_string())?
        .replace("@VERSION@", version);
    fs::write(contents.join("Info.plist"), plist).map_err(|error| error.to_string())?;
    write_package_readme(&resources.join("README.txt"), version)?;
    Ok((
        binary,
        Some(app.clone()),
        vec!["open".to_owned(), app.display().to_string()],
    ))
}

fn package_windows(
    source_binary: &Path,
    package_root: &Path,
) -> Result<(std::path::PathBuf, Option<std::path::PathBuf>, Vec<String>), String> {
    let binary = package_root.join("NUIF Editor.exe");
    fs::copy(source_binary, &binary).map_err(|error| error.to_string())?;
    Ok((binary.clone(), None, vec![binary.display().to_string()]))
}

fn package_linux(
    source_binary: &Path,
    package_root: &Path,
) -> Result<(std::path::PathBuf, Option<std::path::PathBuf>, Vec<String>), String> {
    let binary_directory = package_root.join("bin");
    let applications = package_root.join("share/applications");
    let icons = package_root.join("share/icons/hicolor/scalable/apps");
    for directory in [&binary_directory, &applications, &icons] {
        fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    }
    let binary = binary_directory.join("nuif-editor");
    fs::copy(source_binary, &binary).map_err(|error| error.to_string())?;
    fs::copy(
        "apps/editor/packaging/linux/org.nuif.Editor.desktop",
        applications.join("org.nuif.Editor.desktop"),
    )
    .map_err(|error| error.to_string())?;
    fs::copy(
        "apps/editor/packaging/linux/nuif-editor.svg",
        icons.join("nuif-editor.svg"),
    )
    .map_err(|error| error.to_string())?;
    Ok((binary.clone(), None, vec![binary.display().to_string()]))
}

fn editor_version() -> Result<String, String> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()
        .map_err(|error| error.to_string())?;
    check_status(
        output.status,
        "cargo",
        &["metadata", "--format-version", "1", "--no-deps"],
    )?;
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).map_err(|error| error.to_string())?;
    metadata["packages"]
        .as_array()
        .and_then(|packages| {
            packages
                .iter()
                .find(|package| package["name"] == "nuif-editor")
        })
        .and_then(|package| package["version"].as_str())
        .map(str::to_owned)
        .ok_or_else(|| "cargo metadata did not contain the nuif-editor version".to_owned())
}

fn write_package_readme(destination: &Path, version: &str) -> Result<(), String> {
    let readme = fs::read_to_string("apps/editor/packaging/README.txt")
        .map_err(|error| error.to_string())?
        .replace("@VERSION@", version);
    fs::write(destination, readme).map_err(|error| error.to_string())
}

fn editor_launch() -> Result<(), String> {
    let package = build_editor_package()?;
    println!(
        "launching package archive source: {}",
        package.archive.display()
    );
    if let Some(app) = package.app_bundle {
        return command("open", &["-n", path(&app)?]);
    }
    Command::new(&package.binary)
        .spawn()
        .map_err(|error| format!("could not launch {}: {error}", package.binary.display()))?;
    Ok(())
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
