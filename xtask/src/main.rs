use std::env;
use std::fs;
use std::path::Path;
use std::process::{Command, ExitStatus};

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
            cargo(&[
                "run",
                "--locked",
                "-p",
                "nuif-cli",
                "--",
                "trial",
                &seed,
                &iterations,
                &snapshot_interval,
            ])
        }
        Some("gate-b") => gate_b(),
        Some("gate-c") => gate_c(),
        Some("gate-d-text") => gate_d_text(),
        Some("browser-install") => browser_install(),
        Some("hostile-inputs") => hostile_inputs(),
        Some("research") => research(),
        Some("editor-trial") => editor_trial(),
        Some("all") => {
            research()?;
            verify()?;
            gate_b()?;
            hostile_inputs()?;
            gate_c()?;
            gate_d_text()?;
            editor_trial()
        }
        _ => Err(
            "usage: cargo xtask <research|verify|trial [seed iterations snapshot-interval]|gate-b|gate-c|gate-d-text|browser-install|hostile-inputs|editor-trial|all>"
                .to_owned(),
        ),
    }
}

fn gate_b() -> Result<(), String> {
    cargo(&[
        "run", "--locked", "-p", "nuif-cli", "--", "trial", "24301", "10000", "100",
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
    let output = directory.join("edited.nuif");
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
        path(&output)?,
    ])?;
    cargo(&[
        "run",
        "--locked",
        "-p",
        "nuif-cli",
        "--",
        "validate",
        path(&output)?,
    ])?;
    fs::remove_dir_all(&directory).map_err(|error| {
        format!(
            "trial passed but temporary directory {} could not be removed: {error}",
            directory.display()
        )
    })
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
