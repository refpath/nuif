use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::Path;
use std::process::{Command, ExitStatus};

use sha2::{Digest, Sha256};

mod documentation;
mod editor_install;

type Step = (&'static str, fn() -> Result<(), String>);

const ALL_STEPS: &[Step] = &[
    ("research", research),
    ("adapter-audit", adapter_audit),
    ("dependency-audit", dependency_audit),
    ("docs-check", documentation::check),
    ("verify", verify),
    ("gate-b", gate_b),
    ("hostile-inputs", hostile_inputs),
    ("editor-hostile-inputs", editor_hostile_inputs),
    ("performance", performance),
    ("browser-install", browser_install),
    ("gate-c", gate_c),
    ("gate-d", gate_d),
    ("editor-trial", editor_trial),
    ("editor-gui-trial", editor_gui_trial),
    ("editor-install-trial", editor_install_trial),
    ("gate-f", gate_f),
    ("gate-f-v0", gate_f_v0),
    ("gate-svg", gate_svg),
    ("gate-dtcg", gate_dtcg),
    ("gate-penpot", gate_penpot),
    ("gate-g", gate_g),
    ("gate-h", gate_h),
];

const VERIFICATION_ARTIFACTS: &[&str] = &[
    "target/adapter-coverage-report.json",
    "target/dependency-audit-report.json",
    "target/documentation-catalog.json",
    "target/documentation-report.json",
    "target/gate-b-report.json",
    "target/hostile-input-report.json",
    "target/editor-hostile-input-report.json",
    "target/performance-profile-report.json",
    "target/layout-differential-report.json",
    "target/text-pinning-report.json",
    "target/render-profile-report.json",
    "target/editor-authoring-report.json",
    "target/editor-authoring-snapshot",
    "target/editor-gui-trial",
    "target/editor-install-trial.json",
    "target/html-sync-report.json",
    "target/html-sync-output.html",
    "target/html-sync-v0-report.json",
    "target/html-sync-v0-output.html",
    "target/html-sync-v0-editor-report.json",
    "target/html-sync-v0-editor-output.html",
    "target/svg-sync-report.json",
    "target/svg-sync-output.svg",
    "target/svg-sync-edited.nuif.json",
    "target/svg-sync-cli-report.json",
    "target/svg-sync-cli-output.svg",
    "target/dtcg-sync-report.json",
    "target/dtcg-sync-output.tokens.json",
    "target/dtcg-sync-edited.nuif.json",
    "target/dtcg-sync-cli-report.json",
    "target/dtcg-sync-cli-output.tokens.json",
    "target/penpot-sync-report.json",
    "target/penpot-sync-output.penpot",
    "target/penpot-sync-edited.nuif.json",
    "target/penpot-sync-cli-report.json",
    "target/penpot-sync-cli-output.penpot",
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
        Some("gate-penpot") => gate_penpot(),
        Some("gate-g") => gate_g(),
        Some("gate-h") => gate_h(),
        Some("browser-install") => browser_install(),
        Some("hostile-inputs") => hostile_inputs(),
        Some("editor-hostile-inputs") => editor_hostile_inputs(),
        Some("performance") => performance(),
        Some("research") => research(),
        Some("adapter-audit") => adapter_audit(),
        Some("dependency-audit") => dependency_audit(),
        Some("docs-check") => documentation::check(),
        Some("docs-build") => documentation::build(),
        Some("docs-paper") => documentation::paper(),
        Some("docs-serve") => documentation::serve(),
        Some("docs-setup") => documentation::setup(),
        Some("editor-trial") => editor_trial(),
        Some("editor-gui-trial") => editor_gui_trial(),
        Some("editor-install-trial") => editor_install::trial(&args.collect::<Vec<_>>()),
        Some("editor-package") => editor_package(),
        Some("editor-launch") => editor_launch(),
        Some("editor-install") => editor_install::install(&args.collect::<Vec<_>>()),
        Some("editor-doctor") => editor_install::doctor(&args.collect::<Vec<_>>()),
        Some("editor-rollback") => editor_install::rollback(&args.collect::<Vec<_>>()),
        Some("editor-uninstall") => editor_install::uninstall(&args.collect::<Vec<_>>()),
        Some("editor-update") => editor_install::update(&args.collect::<Vec<_>>()),
        Some("release-check") => release_check(args.next().as_deref()),
        Some("manifest") => standalone_manifest(),
        Some("all") => all(),
        _ => Err(
            "usage: cargo xtask <research|adapter-audit|dependency-audit|docs-check|docs-build|docs-paper|docs-serve|docs-setup|verify|trial [seed iterations snapshot-interval report-path]|gate-b|gate-c|gate-d|gate-d-text|gate-d-render|gate-f|gate-f-v0|gate-svg|gate-dtcg|gate-penpot|gate-g|gate-h|browser-install|hostile-inputs|editor-hostile-inputs|performance|editor-trial|editor-gui-trial|editor-install-trial|editor-package|editor-launch|editor-install|editor-doctor|editor-rollback|editor-uninstall|editor-update|release-check <tag>|manifest|all>"
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

fn editor_hostile_inputs() -> Result<(), String> {
    cargo(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "nuif-editor",
        "--bin",
        "editor-hostile-inputs",
        "--",
        "--output",
        "target/editor-hostile-input-report.json",
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
        "--benches",
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
        "target/svg-sync-edited.nuif.json",
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
    let reimported = directory.join("reimported.nuif.json");
    let reimport_report = directory.join("reimport-report.json");
    let synchronized = Path::new("target/svg-sync-cli-output.svg");
    let sync_report = Path::new("target/svg-sync-cli-report.json");
    let edited = Path::new("target/svg-sync-edited.nuif.json");
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
        return Err("CLI SVG synchronization changed edited canonical document bytes".to_owned());
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
        "target/dtcg-sync-edited.nuif.json",
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
    let reimported = directory.join("reimported.nuif.json");
    let reimport_report = directory.join("reimport-report.json");
    let synchronized = Path::new("target/dtcg-sync-cli-output.tokens.json");
    let sync_report = Path::new("target/dtcg-sync-cli-report.json");
    let edited = Path::new("target/dtcg-sync-edited.nuif.json");
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
        return Err("CLI DTCG synchronization changed edited canonical document bytes".to_owned());
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

fn gate_penpot() -> Result<(), String> {
    cargo(&[
        "run",
        "--locked",
        "-p",
        "nuif-penpot",
        "--bin",
        "penpot-sync-profile",
        "--",
        "--output",
        "target/penpot-sync-report.json",
        "--package-output",
        "target/penpot-sync-output.penpot",
        "--edited-output",
        "target/penpot-sync-edited.nuif.json",
    ])?;
    gate_penpot_cli_bridge()
}

fn gate_penpot_cli_bridge() -> Result<(), String> {
    let directory = env::temp_dir().join(format!("nuif-penpot-trial-{}", std::process::id()));
    if directory.exists() {
        return Err(format!(
            "temporary path already exists: {}",
            directory.display()
        ));
    }
    fs::create_dir(&directory).map_err(|error| error.to_string())?;
    let input = directory.join("input.nuif");
    let exported = directory.join("exported.penpot");
    let export_report = directory.join("export-report.json");
    let imported = directory.join("imported.nuif");
    let import_report = directory.join("import-report.json");
    let reimported = directory.join("reimported.nuif.json");
    let reimport_report = directory.join("reimport-report.json");
    let synchronized = Path::new("target/penpot-sync-cli-output.penpot");
    let sync_report = Path::new("target/penpot-sync-cli-report.json");
    let edited = Path::new("target/penpot-sync-edited.nuif.json");
    nuif(&["fixture", "penpot-profile", path(&input)?])?;
    nuif(&[
        "export",
        path(&input)?,
        "penpot-v3-0",
        path(&exported)?,
        path(&export_report)?,
    ])?;
    nuif(&[
        "import",
        "penpot-v3-0",
        path(&exported)?,
        path(&imported)?,
        path(&import_report)?,
    ])?;
    if fs::read(&input).map_err(|error| error.to_string())?
        != fs::read(&imported).map_err(|error| error.to_string())?
    {
        return Err("CLI Penpot export/import changed canonical NUIF bytes".to_owned());
    }
    nuif(&[
        "sync",
        "penpot-v3-0",
        path(&exported)?,
        path(edited)?,
        path(synchronized)?,
        path(sync_report)?,
    ])?;
    nuif(&[
        "import",
        "penpot-v3-0",
        path(synchronized)?,
        path(&reimported)?,
        path(&reimport_report)?,
    ])?;
    if fs::read(edited).map_err(|error| error.to_string())?
        != fs::read(&reimported).map_err(|error| error.to_string())?
    {
        return Err(
            "CLI Penpot synchronization changed edited canonical document bytes".to_owned(),
        );
    }
    let report = read_json(sync_report)?;
    if report["status"] != "passed" || report["edits"].as_array().map(Vec::len) != Some(8) {
        return Err(
            "CLI Penpot bridge did not produce the expected eight package edits".to_owned(),
        );
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
    let independent_input = reference.join("input.nuif.json");
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
    nuif(&["migrate", path(&input)?, path(&independent_input)?])?;
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
            path(&independent_input)?,
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

fn adapter_audit() -> Result<(), String> {
    let index = read_json(Path::new("adapters/index.json"))?;
    let targets = index["targets"]
        .as_array()
        .ok_or("adapters/index.json targets must be an array")?;
    let expected = BTreeSet::from([
        "adobe-uxp",
        "dtcg",
        "figma",
        "flutter",
        "html-css",
        "jetpack-compose",
        "penpot",
        "react",
        "svelte",
        "svg",
        "swiftui",
    ]);
    let observed = targets
        .iter()
        .filter_map(|target| target["id"].as_str())
        .collect::<BTreeSet<_>>();
    let mut failures = Vec::new();
    if observed != expected || observed.len() != targets.len() {
        failures
            .push("target inventory is incomplete or contains duplicate identifiers".to_owned());
    }
    for target in targets {
        audit_adapter_target(target, &mut failures);
    }
    let report = serde_json::json!({
        "schema_version": 1,
        "status": if failures.is_empty() { "passed" } else { "failed" },
        "source": {
            "revision": command_text("git", &["rev-parse", "HEAD"]),
            "dirty": command_text("git", &["status", "--porcelain"]).map(|value| !value.is_empty()),
        },
        "summary": {
            "advertised_targets": targets.len(),
            "integrated_targets": targets.iter().filter(|target| target["status"] == "integrated").count(),
            "integrated_profiles": targets.iter().filter_map(|target| target["profiles"].as_array()).map(Vec::len).sum::<usize>(),
            "researched_or_bounded_targets": targets.len().saturating_sub(failures.len()),
            "blocking_failures": failures.len(),
        },
        "targets": targets,
        "failures": failures,
    });
    fs::create_dir_all("target").map_err(|error| error.to_string())?;
    fs::write(
        "target/adapter-coverage-report.json",
        serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    if report["status"] == "passed" {
        Ok(())
    } else {
        Err("adapter coverage audit failed; inspect target/adapter-coverage-report.json".to_owned())
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "the dependency register and observed Cargo graph stay together in one audit path"
)]
fn dependency_audit() -> Result<(), String> {
    let index = read_json(Path::new("dependencies/index.json"))?;
    let registered = index["dependencies"]
        .as_array()
        .ok_or("dependencies/index.json dependencies must be an array")?;
    let mut failures = Vec::new();
    let mut registered_names = BTreeSet::new();
    for dependency in registered {
        let Some(name) = required_json_string(dependency, "name", "dependency", &mut failures)
        else {
            continue;
        };
        if !registered_names.insert(name) {
            failures.push(format!("duplicate dependency registration: {name}"));
        }
        for field in ["role", "decision", "rationale"] {
            required_json_string(dependency, field, name, &mut failures);
        }
        if !matches!(
            dependency["decision"].as_str(),
            Some("retain" | "fork" | "watch" | "replace")
        ) {
            failures.push(format!("{name}: decision is not declared"));
        }
        for field in ["alternatives", "evidence"] {
            let Some(values) = dependency[field].as_array() else {
                failures.push(format!("{name}: {field} must be an array"));
                continue;
            };
            if values.is_empty()
                || values
                    .iter()
                    .any(|value| value.as_str().is_none_or(str::is_empty))
            {
                failures.push(format!("{name}: {field} must contain non-empty strings"));
            }
            if field == "evidence" {
                for evidence in values.iter().filter_map(serde_json::Value::as_str) {
                    if !Path::new(evidence).is_file() {
                        failures.push(format!("{name}: evidence file is absent: {evidence}"));
                    }
                }
            }
        }
    }

    let output = Command::new("cargo")
        .args(["metadata", "--locked", "--format-version", "1"])
        .output()
        .map_err(|error| error.to_string())?;
    check_status(
        output.status,
        "cargo",
        &["metadata", "--locked", "--format-version", "1"],
    )?;
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).map_err(|error| error.to_string())?;
    let workspace_members = metadata["workspace_members"]
        .as_array()
        .ok_or("cargo metadata omitted workspace_members")?
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect::<BTreeSet<_>>();
    let packages = metadata["packages"]
        .as_array()
        .ok_or("cargo metadata omitted packages")?;
    let mut observed_names = BTreeSet::new();
    let mut resolved_versions = std::collections::BTreeMap::<String, BTreeSet<String>>::new();
    for package in packages {
        let Some(name) = package["name"].as_str() else {
            continue;
        };
        if package["source"].is_string()
            && let Some(version) = package["version"].as_str()
        {
            resolved_versions
                .entry(name.to_owned())
                .or_default()
                .insert(version.to_owned());
        }
        if !package["id"]
            .as_str()
            .is_some_and(|id| workspace_members.contains(id))
        {
            continue;
        }
        for dependency in package["dependencies"].as_array().into_iter().flatten() {
            if dependency["path"].is_null()
                && let Some(name) = dependency["name"].as_str()
            {
                observed_names.insert(name);
            }
        }
    }
    let missing = observed_names
        .difference(&registered_names)
        .copied()
        .collect::<Vec<_>>();
    let stale = registered_names
        .difference(&observed_names)
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        failures.push(format!(
            "unregistered direct dependencies: {}",
            missing.join(", ")
        ));
    }
    if !stale.is_empty() {
        failures.push(format!(
            "registered dependencies are not direct dependencies: {}",
            stale.join(", ")
        ));
    }
    let resolved_versions = resolved_versions
        .into_iter()
        .filter(|(name, _)| registered_names.contains(name.as_str()))
        .map(|(name, versions)| (name, versions.into_iter().collect::<Vec<_>>()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let report = serde_json::json!({
        "schema_version": 1,
        "status": if failures.is_empty() { "passed" } else { "failed" },
        "source": {
            "revision": command_text("git", &["rev-parse", "HEAD"]),
            "dirty": command_text("git", &["status", "--porcelain"]).map(|value| !value.is_empty()),
        },
        "summary": {
            "registered_direct_dependencies": registered_names.len(),
            "observed_direct_dependencies": observed_names.len(),
            "blocking_failures": failures.len(),
        },
        "observed": observed_names,
        "resolved_versions": resolved_versions,
        "dependencies": registered,
        "failures": failures,
    });
    fs::create_dir_all("target").map_err(|error| error.to_string())?;
    fs::write(
        "target/dependency-audit-report.json",
        serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    if report["status"] == "passed" {
        Ok(())
    } else {
        Err("dependency audit failed; inspect target/dependency-audit-report.json".to_owned())
    }
}

fn audit_adapter_target(target: &serde_json::Value, failures: &mut Vec<String>) {
    let Some(id) = required_json_string(target, "id", "target", failures) else {
        return;
    };
    for field in ["surface", "research", "next_profile", "boundary"] {
        required_json_string(target, field, id, failures);
    }
    let status = required_json_string(target, "status", id, failures).unwrap_or_default();
    if !matches!(
        status,
        "integrated" | "researched" | "external_blocked" | "external_runtime"
    ) {
        failures.push(format!("{id}: status {status:?} is not declared"));
    }
    if let Some(research) = target["research"].as_str() {
        let record = format!("research/items/{research}.md");
        match fs::read_to_string(&record) {
            Ok(contents) if contents.contains(&format!("id: nuif:research:{research}")) => {}
            Ok(_) => failures.push(format!("{id}: {record} has the wrong research identifier")),
            Err(error) => failures.push(format!("{id}: cannot read {record}: {error}")),
        }
    }
    let profiles = target["profiles"].as_array();
    let directions = target["directions"].as_array();
    if status == "integrated" {
        if profiles.is_none_or(Vec::is_empty) || directions.is_none_or(Vec::is_empty) {
            failures.push(format!(
                "{id}: integrated target lacks profiles or directions"
            ));
        }
        for profile in profiles.into_iter().flatten() {
            audit_adapter_profile(id, profile, failures);
        }
    } else if profiles.is_some_and(|profiles| !profiles.is_empty())
        || directions.is_some_and(|directions| !directions.is_empty())
    {
        failures.push(format!(
            "{id}: non-integrated target claims executable capabilities"
        ));
    }
}

fn audit_adapter_profile(target: &str, profile: &serde_json::Value, failures: &mut Vec<String>) {
    let Some(name) = required_json_string(profile, "name", target, failures) else {
        return;
    };
    for field in ["crate", "profile"] {
        let Some(path) = required_json_string(profile, field, name, failures) else {
            continue;
        };
        let check = if field == "crate" {
            Path::new(path).join("Cargo.toml")
        } else {
            Path::new(path).to_owned()
        };
        if !check.exists() {
            failures.push(format!("{name}: missing {}", check.display()));
        }
    }
    if let Some(gate) = required_json_string(profile, "gate", name, failures) {
        let xtask = fs::read_to_string("xtask/src/main.rs").unwrap_or_default();
        if !xtask.contains(&format!("Some(\"{gate}\")")) {
            failures.push(format!("{name}: xtask command {gate} is not routed"));
        }
    }
}

fn required_json_string<'a>(
    value: &'a serde_json::Value,
    field: &str,
    context: &str,
    failures: &mut Vec<String>,
) -> Option<&'a str> {
    match value[field].as_str() {
        Some(value) if !value.trim().is_empty() => Some(value),
        _ => {
            failures.push(format!("{context}: {field} must be a non-empty string"));
            None
        }
    }
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
        "file_menu_rgba_sha256",
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
        || first["file_menu_routes"].as_array().map(Vec::len) != Some(13)
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

fn editor_install_trial() -> Result<(), String> {
    editor_install::trial(&[])
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
pub(crate) struct EditorPackage {
    pub(crate) package_root: std::path::PathBuf,
    pub(crate) binary: std::path::PathBuf,
    pub(crate) app_bundle: Option<std::path::PathBuf>,
    pub(crate) archive: std::path::PathBuf,
}

fn editor_package() -> Result<(), String> {
    let package = build_editor_package()?;
    println!("packaged native editor: {}", package.package_root.display());
    Ok(())
}

pub(crate) fn build_editor_package() -> Result<EditorPackage, String> {
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
    let version = editor_version()?;
    let package_name = format!(
        "nuif-editor-{version}-{}-{}",
        env::consts::OS,
        env::consts::ARCH
    );
    let dist = target_root.join("dist");
    let package_root = dist.join(&package_name);
    if package_root.exists() {
        fs::remove_dir_all(&package_root).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(&package_root).map_err(|error| error.to_string())?;
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
    let version_output = Command::new(binary)
        .arg("--version")
        .output()
        .map_err(|error| format!("could not inspect packaged editor version: {error}"))?;
    check_status(version_output.status, path(binary)?, &["--version"])?;
    let expected_version = format!("NUIF Editor {version}");
    if String::from_utf8_lossy(&version_output.stdout).trim() != expected_version {
        return Err("packaged editor version does not match its manifest".to_owned());
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
        "version_test": {
            "command": [binary, "--version"],
            "expected": expected_version,
            "status": "passed"
        },
        "signing": {
            "status": "unsigned",
            "note": "unsigned package; release signing requires platform credentials"
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
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?;
    fs::write(dist.join("editor-package-manifest.json"), &manifest_bytes)
        .map_err(|error| error.to_string())?;
    fs::write(
        dist.join(format!("{package_name}.manifest.json")),
        manifest_bytes,
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
        .replace("@SHORT_VERSION@", short_version(version))
        .replace("@BUILD_VERSION@", &build_version(version)?);
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

pub(crate) fn editor_version() -> Result<String, String> {
    let output = Command::new("cargo")
        .args(["metadata", "--locked", "--format-version", "1", "--no-deps"])
        .output()
        .map_err(|error| error.to_string())?;
    check_status(
        output.status,
        "cargo",
        &["metadata", "--locked", "--format-version", "1", "--no-deps"],
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

fn short_version(version: &str) -> &str {
    version.split_once('-').map_or(version, |(base, _)| base)
}

fn build_version(version: &str) -> Result<String, String> {
    let value = env::var("NUIF_BUILD_NUMBER").unwrap_or_else(|_| {
        version
            .rsplit_once('.')
            .and_then(|(_, value)| value.parse::<u64>().ok())
            .unwrap_or(1)
            .to_string()
    });
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("NUIF_BUILD_NUMBER must contain decimal digits".to_owned());
    }
    Ok(value)
}

fn release_check(tag: Option<&str>) -> Result<(), String> {
    let tag = tag.ok_or_else(|| "release-check requires a tag".to_owned())?;
    let version = editor_version()?;
    validate_release_tag(&version, tag)?;
    let dirty =
        command_text("git", &["status", "--porcelain"]).is_some_and(|status| !status.is_empty());
    if dirty {
        return Err("release source tree contains uncommitted changes".to_owned());
    }
    let report = serde_json::json!({
        "schema_version": 1,
        "status": "passed",
        "tag": tag,
        "version": version,
        "source_revision": command_text("git", &["rev-parse", "HEAD"]),
        "source_dirty": false,
        "toolchain": command_text("rustc", &["--version"]),
    });
    fs::create_dir_all("target/dist").map_err(|error| error.to_string())?;
    fs::write(
        "target/dist/release-check.json",
        serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    println!("release tag {tag} matches NUIF Editor {version}");
    Ok(())
}

fn validate_release_tag(version: &str, tag: &str) -> Result<(), String> {
    let expected = format!("v{version}");
    if tag != expected {
        return Err(format!(
            "release tag {tag:?} does not match editor version {version:?}; expected {expected:?}"
        ));
    }
    if version.contains('+') {
        return Err("release versions must not contain SemVer build metadata".to_owned());
    }
    Ok(())
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

pub(crate) fn command_text(program: &str, arguments: &[&str]) -> Option<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_tag_must_match_the_editor_version_exactly() {
        assert!(validate_release_tag("0.1.0-alpha.1", "v0.1.0-alpha.1").is_ok());
        assert!(validate_release_tag("0.1.0-alpha.1", "v0.1.0-alpha.2").is_err());
        assert!(validate_release_tag("0.1.0+local", "v0.1.0+local").is_err());
    }

    #[test]
    fn macos_short_version_excludes_the_prerelease_suffix() {
        assert_eq!(short_version("0.1.0-alpha.1"), "0.1.0");
        assert_eq!(short_version("1.2.3"), "1.2.3");
    }
}
