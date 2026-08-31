use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use sha2::{Digest, Sha256};

mod diagnostics;
mod documentation;
mod editor_install;

type Step = (&'static str, fn() -> Result<(), String>);

const ALL_STEPS: &[Step] = &[
    ("workflow-audit", workflow_audit),
    ("research", research),
    ("adapter-audit", adapter_audit),
    ("dependency-audit", dependency_audit),
    ("diagnostic-audit", diagnostics::audit),
    ("docs-check", documentation::check),
    ("verify", verify),
    ("gate-wasm", gate_wasm),
    ("gate-mcp", gate_mcp),
    ("gate-ffi", gate_ffi),
    ("gate-b", gate_b),
    ("hostile-inputs", hostile_inputs),
    ("reduction-profile", reduction_profile),
    ("editor-hostile-inputs", editor_hostile_inputs),
    ("codec-benchmark", codec_benchmark),
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
    ("gate-react", gate_react),
    ("gate-svelte", gate_svelte),
    ("gate-figma", gate_figma),
    ("gate-canva", gate_canva),
    ("gate-behavior", gate_behavior),
    ("gate-g", gate_g),
    ("gate-h", gate_h),
    ("gate-i-package", gate_i_package),
    ("gate-i-image", gate_i_image),
    ("gate-i-font", gate_i_font),
    ("gate-i-font-metadata", gate_i_font_metadata),
    ("gate-i-font-shaping", gate_i_font_shaping),
    ("gate-i-font-metrics", gate_i_font_metrics),
    ("gate-i-font-global-metrics", gate_i_font_global_metrics),
    ("gate-i-font-security", gate_i_font_security),
    ("gate-i-font-package", gate_i_font_package),
    ("gate-i-font-corpus", gate_i_font_corpus),
    ("gate-i-font-gvar-generated", gate_i_font_gvar_generated),
    ("gate-i-font-runtime", gate_i_font_runtime),
    ("capture-baselines", capture_baselines),
    (
        "reconstruction-provider-manifest",
        reconstruction_provider_manifest,
    ),
    ("reconstruction-corpus-audit", reconstruction_corpus_audit),
    ("reconstruction-evaluation", reconstruction_evaluation),
    ("confidence-calibration", confidence_calibration),
    ("gate-j-live", gate_j_live),
];

const VERIFICATION_ARTIFACTS: &[&str] = &[
    "target/workflow-audit-report.json",
    "target/research-readiness-report.json",
    "target/adapter-coverage-report.json",
    "target/dependency-audit-report.json",
    "target/diagnostic-registry-report.json",
    "target/documentation-catalog.json",
    "target/documentation-report.json",
    "target/wasm-conformance-report.json",
    "target/ffi-header-report.json",
    "target/nuif-wasm-web",
    "target/mcp-conformance-report.json",
    "target/gate-b-report.json",
    "target/hostile-input-report.json",
    "target/reduction-profile-report.json",
    "target/reduction-profile-fixture",
    "target/editor-hostile-input-report.json",
    "target/codec-benchmark-report.json",
    "target/performance-profile-report.json",
    "target/criterion-smoke-report.json",
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
    "target/react-sync-report.json",
    "target/react-sync-output.jsx",
    "target/react-sync-edited.nuif.json",
    "target/react-sync-cli-report.json",
    "target/react-sync-cli-output.jsx",
    "target/svelte-sync-report.json",
    "target/svelte-sync-output.svelte",
    "target/svelte-sync-edited.nuif.json",
    "target/svelte-sync-cli-report.json",
    "target/svelte-sync-cli-output.svelte",
    "target/svelte-compiler-oracle-report.json",
    "target/figma-snapshot-report.json",
    "target/canva-current-page-report.json",
    "target/figma-plugin-shell-report.json",
    "target/figma-plugin-fixture-snapshot.json",
    "target/figma-plugin-fixture.nuif.json",
    "target/figma-plugin-fixture-report.json",
    "target/figma-plugin-plan-validation.json",
    "target/nuif-figma-plugin-review-shell",
    "target/canva-app-shell-report.json",
    "target/canva-app-fixture.nuif.json",
    "target/canva-app-fixture-page.json",
    "target/canva-app-fixture-imported.nuif.json",
    "target/canva-app-fixture-report.json",
    "target/canva-app-plan.json",
    "target/canva-app-plan-report.json",
    "target/canva-app-plan-validation.json",
    "target/canva-app-benchmark-report.json",
    "target/nuif-canva-review-app",
    "target/behavior-portability-fixture.json",
    "target/behavior-portability-static-report.json",
    "target/behavior-portability-report.json",
    "target/behavior-package-fixture.nuif",
    "target/behavior-package-expected.json",
    "target/behavior-package-static-report.json",
    "target/behavior-package-report.json",
    "target/behavior-package-cli-report.json",
    "target/gate-g-report.json",
    "target/gate-g-independent",
    "target/collaboration-report.json",
    "target/collaboration-structure-report.json",
    "target/collaboration-tree-foreign-report.json",
    "target/collaboration-creation-report.json",
    "target/collaboration-nested-creation-report.json",
    "target/collaboration-nested-creation-v1-report.json",
    "target/collaboration-mixed-report.json",
    "target/collaboration-gc-report.json",
    "target/collaboration-gc-prefix-report.json",
    "target/collaboration-automerge-input.json",
    "target/collaboration-automerge-report.json",
    "target/package-resources-report.json",
    "target/package-resources-fixture.nuif",
    "target/package-resources-foreign.nuif",
    "target/package-foreign-oracle-report.json",
    "target/image-resources-report.json",
    "target/font-resources-report.json",
    "target/variable-font-metadata-report.json",
    "target/variable-font-shaping-report.json",
    "target/variable-font-metrics-report.json",
    "target/variable-font-global-metrics-report.json",
    "target/variable-font-security-report.json",
    "target/variable-font-package-report.json",
    "target/variable-font-corpus-report.json",
    "target/variable-font-gvar-generated-report.json",
    "target/variable-font-runtime-report.json",
    "target/capture-reconstruction-report.json",
    "target/reconstruction-provider-manifest-report.json",
    "target/reconstruction-corpus-audit-report.json",
    "target/reconstruction-evaluation-report.json",
    "target/confidence-calibration-report.json",
    "target/live-browser-capture-report.json",
    "target/layout-inference-report.json",
];

fn main() {
    if let Err(error) = run() {
        eprintln!("xtask: {error}");
        std::process::exit(1);
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "the top-level command dispatcher keeps every supported automation entry point explicit"
)]
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
        Some("gate-react") => gate_react(),
        Some("gate-svelte") => gate_svelte(),
        Some("gate-figma") => gate_figma(),
        Some("gate-canva") => gate_canva(),
        Some("gate-behavior") => gate_behavior(),
        Some("gate-behavior-package") => gate_behavior_package(),
        Some("gate-web-behavior") => gate_web_behavior(),
        Some("gate-web-hosts") => gate_web_hosts(),
        Some("gate-wasm") => gate_wasm(),
        Some("gate-mcp") => gate_mcp(),
        Some("gate-ffi") => gate_ffi(),
        Some("wasm-install") => wasm_install(),
        Some("wasm-package") => wasm_package(),
        Some("mcp-package") => mcp_package(),
        Some("cli-package") => cli_package(),
        Some("ffi-package") => ffi_package(),
        Some("conformance-kit") => conformance_kit(),
        Some("gate-g") => gate_g(),
        Some("gate-h") => gate_h(),
        Some("gate-i-package") => gate_i_package(),
        Some("gate-i-image") => gate_i_image(),
        Some("gate-i-font") => gate_i_font(),
        Some("gate-i-font-metadata") => gate_i_font_metadata(),
        Some("gate-i-font-shaping") => gate_i_font_shaping(),
        Some("gate-i-font-metrics") => gate_i_font_metrics(),
        Some("gate-i-font-global-metrics") => gate_i_font_global_metrics(),
        Some("gate-i-font-security") => gate_i_font_security(),
        Some("gate-i-font-package") => gate_i_font_package(),
        Some("gate-i-font-corpus") => gate_i_font_corpus(),
        Some("gate-i-font-gvar-generated") => gate_i_font_gvar_generated(),
        Some("gate-i-font-runtime") => gate_i_font_runtime(),
        Some("capture-baselines") => capture_baselines(),
        Some("reconstruction-provider-manifest") => reconstruction_provider_manifest(),
        Some("reconstruction-corpus-audit") => reconstruction_corpus_audit(),
        Some("reconstruction-evaluation") => reconstruction_evaluation(),
        Some("confidence-calibration") => confidence_calibration(),
        Some("gate-j-live") => gate_j_live(),
        Some("gate-accessibility") => gate_accessibility(),
        Some("browser-install") => browser_install(),
        Some("hostile-inputs") => hostile_inputs(),
        Some("reduction-profile") => reduction_profile(),
        Some("editor-hostile-inputs") => editor_hostile_inputs(),
        Some("fuzz-smoke") => fuzz_smoke(),
        Some("codec-benchmark") => codec_benchmark(),
        Some("performance") => performance(),
        Some("research") => research(),
        Some("workflow-audit") => workflow_audit(),
        Some("adapter-audit") => adapter_audit(),
        Some("dependency-audit") => dependency_audit(),
        Some("diagnostic-audit") => diagnostics::audit(),
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
            "usage: cargo xtask <research|workflow-audit|adapter-audit|dependency-audit|diagnostic-audit|docs-check|docs-build|docs-paper|docs-serve|docs-setup|verify|trial [seed iterations snapshot-interval report-path]|gate-b|gate-c|gate-d|gate-d-text|gate-d-render|gate-f|gate-f-v0|gate-svg|gate-dtcg|gate-penpot|gate-react|gate-svelte|gate-figma|gate-canva|gate-behavior|gate-behavior-package|gate-web-behavior|gate-web-hosts|gate-wasm|gate-mcp|gate-ffi|gate-g|gate-h|gate-i-package|gate-i-image|gate-i-font|gate-i-font-metadata|gate-i-font-shaping|gate-i-font-metrics|gate-i-font-global-metrics|gate-i-font-security|gate-i-font-package|gate-i-font-corpus|gate-i-font-gvar-generated|gate-i-font-runtime|gate-j-live|gate-accessibility|capture-baselines|reconstruction-provider-manifest|reconstruction-corpus-audit|reconstruction-evaluation|confidence-calibration|browser-install|wasm-install|wasm-package|mcp-package|cli-package|ffi-package|conformance-kit|hostile-inputs|reduction-profile|editor-hostile-inputs|fuzz-smoke|codec-benchmark|performance|editor-trial|editor-gui-trial|editor-install-trial|editor-package|editor-launch|editor-install|editor-doctor|editor-rollback|editor-uninstall|editor-update|release-check <tag>|manifest|all>"
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

fn gate_i_package() -> Result<(), String> {
    cargo(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "nuif-testing",
        "--bin",
        "package-resources",
        "--",
        "--output",
        "target/package-resources-report.json",
        "--package-output",
        "target/package-resources-fixture.nuif",
    ])?;
    command(
        "python3",
        &[
            "implementations/python/nuif_package_oracle.py",
            "--input",
            "target/package-resources-fixture.nuif",
            "--output",
            "target/package-resources-foreign.nuif",
            "--report",
            "target/package-foreign-oracle-report.json",
        ],
    )
}

fn gate_i_image() -> Result<(), String> {
    cargo(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "nuif-testing",
        "--bin",
        "image-resources",
        "--",
        "--output",
        "target/image-resources-report.json",
    ])
}

fn gate_i_font() -> Result<(), String> {
    cargo(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "nuif-testing",
        "--bin",
        "font-resources",
        "--",
        "--output",
        "target/font-resources-report.json",
    ])
}

fn gate_i_font_metadata() -> Result<(), String> {
    cargo(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "nuif-testing",
        "--bin",
        "variable-font-metadata",
        "--",
        "--output",
        "target/variable-font-metadata-report.json",
    ])
}

fn gate_i_font_shaping() -> Result<(), String> {
    cargo(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "nuif-testing",
        "--bin",
        "variable-font-shaping",
        "--",
        "--output",
        "target/variable-font-shaping-report.json",
    ])
}

fn gate_i_font_metrics() -> Result<(), String> {
    cargo(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "nuif-testing",
        "--bin",
        "variable-font-metrics",
        "--",
        "--output",
        "target/variable-font-metrics-report.json",
    ])
}

fn gate_i_font_global_metrics() -> Result<(), String> {
    cargo(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "nuif-testing",
        "--bin",
        "variable-font-global-metrics",
        "--",
        "--output",
        "target/variable-font-global-metrics-report.json",
    ])
}

fn gate_i_font_security() -> Result<(), String> {
    cargo(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "nuif-testing",
        "--bin",
        "variable-font-security",
        "--",
        "--output",
        "target/variable-font-security-report.json",
    ])
}

fn gate_i_font_package() -> Result<(), String> {
    cargo(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "nuif-testing",
        "--bin",
        "variable-font-package",
        "--",
        "--output",
        "target/variable-font-package-report.json",
    ])
}

fn gate_i_font_corpus() -> Result<(), String> {
    cargo(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "nuif-testing",
        "--bin",
        "variable-font-corpus",
        "--",
        "--output",
        "target/variable-font-corpus-report.json",
    ])
}

fn gate_i_font_gvar_generated() -> Result<(), String> {
    cargo(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "nuif-testing",
        "--bin",
        "variable-font-gvar-generated",
        "--",
        "--output",
        "target/variable-font-gvar-generated-report.json",
    ])
}

fn gate_i_font_runtime() -> Result<(), String> {
    cargo(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "nuif-testing",
        "--bin",
        "variable-font-runtime",
        "--",
        "--output",
        "target/variable-font-runtime-report.json",
    ])
}

fn capture_baselines() -> Result<(), String> {
    cargo(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "nuif-testing",
        "--bin",
        "capture-reconstruction",
        "--",
        "--output",
        "target/capture-reconstruction-report.json",
    ])
}

fn reconstruction_evaluation() -> Result<(), String> {
    cargo(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "nuif-testing",
        "--bin",
        "reconstruction-evaluation",
        "--",
        "--output",
        "target/reconstruction-evaluation-report.json",
    ])
}

fn confidence_calibration() -> Result<(), String> {
    cargo(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "nuif-testing",
        "--bin",
        "confidence-calibration",
        "--",
        "--output",
        "target/confidence-calibration-report.json",
    ])
}

fn reconstruction_corpus_audit() -> Result<(), String> {
    cargo(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "nuif-testing",
        "--bin",
        "reconstruction-corpus-audit",
        "--",
        "--output",
        "target/reconstruction-corpus-audit-report.json",
    ])
}

fn reconstruction_provider_manifest() -> Result<(), String> {
    cargo(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "nuif-testing",
        "--bin",
        "reconstruction-provider-manifest",
        "--",
        "--output",
        "target/reconstruction-provider-manifest-report.json",
    ])
}

fn gate_j_live() -> Result<(), String> {
    browser_install()?;
    let chrome = wasm_browser_binary()?;
    let browser_lock = read_json(Path::new("conformance/browser-oracle.lock.json"))?;
    let browser_version = browser_lock["version"]
        .as_str()
        .ok_or_else(|| "browser oracle lock has no version".to_owned())?;
    cargo(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "nuif-testing",
        "--bin",
        "live-browser-capture",
        "--",
        "--chrome",
        path(&chrome)?,
        "--browser-version",
        browser_version,
        "--output",
        "target/live-browser-capture-report.json",
        "--layout-output",
        "target/layout-inference-report.json",
    ])
}

fn gate_accessibility() -> Result<(), String> {
    generate_accessibility_mapping()?;
    prepare_web_host_oracle()?;
    run_web_host_oracle(
        "accessibility",
        &[
            "tools/accessibility-oracle/check.mjs",
            "target/accessibility-mapping-fixture.html",
            "target/accessibility-mapping-expected.json",
            "target/accessibility-mapping-static-report.json",
            "target/accessibility-mapping-report.json",
        ],
    )
}

fn generate_accessibility_mapping() -> Result<(), String> {
    cargo(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "nuif-testing",
        "--bin",
        "accessibility-mapping",
    ])
}

fn prepare_web_host_oracle() -> Result<(), String> {
    const ORACLE: &str = "tools/accessibility-oracle";
    const BROWSER_PATH: &str = "target/playwright-browsers";
    command(
        "npm",
        &[
            "ci",
            "--ignore-scripts",
            "--no-audit",
            "--no-fund",
            "--prefix",
            ORACLE,
        ],
    )?;
    let suffix = if cfg!(windows) { ".cmd" } else { "" };
    let playwright = format!("{ORACLE}/node_modules/.bin/playwright{suffix}");
    let install_arguments = if cfg!(target_os = "linux") && env::var_os("CI").is_some() {
        vec!["install", "--with-deps", "chromium", "firefox", "webkit"]
    } else {
        vec!["install", "chromium", "firefox", "webkit"]
    };
    let install_status = Command::new(&playwright)
        .args(&install_arguments)
        .env("PLAYWRIGHT_BROWSERS_PATH", BROWSER_PATH)
        .status()
        .map_err(|error| format!("could not start Playwright browser install: {error}"))?;
    check_status(install_status, &playwright, &install_arguments)
}

fn run_web_host_oracle(name: &str, oracle_arguments: &[&str]) -> Result<(), String> {
    const BROWSER_PATH: &str = "target/playwright-browsers";
    let oracle_status = Command::new("node")
        .args(oracle_arguments)
        .env("PLAYWRIGHT_BROWSERS_PATH", BROWSER_PATH)
        .status()
        .map_err(|error| format!("could not start {name} oracle: {error}"))?;
    check_status(oracle_status, "node", oracle_arguments)
}

fn gate_behavior() -> Result<(), String> {
    gate_behavior_package()?;
    cargo(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "nuif-testing",
        "--bin",
        "behavior-portability",
    ])?;
    command(
        "node",
        &[
            "tools/behavior-oracle/check.mjs",
            "target/behavior-portability-fixture.json",
            "target/behavior-portability-static-report.json",
            "target/behavior-portability-report.json",
        ],
    )
}

fn gate_behavior_package() -> Result<(), String> {
    cargo(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "nuif-testing",
        "--bin",
        "behavior-package",
    ])?;
    command(
        "python3",
        &[
            "tools/behavior-oracle/package_check.py",
            "target/behavior-package-fixture.nuif",
            "target/behavior-package-expected.json",
            "target/behavior-package-static-report.json",
            "target/behavior-package-report.json",
        ],
    )?;
    gate_behavior_package_cli()
}

fn gate_behavior_package_cli() -> Result<(), String> {
    cargo(&["build", "--release", "--locked", "-p", "nuif-cli"])?;
    let executable_suffix = if cfg!(windows) { ".exe" } else { "" };
    let binary = PathBuf::from(format!("target/release/nuif{executable_suffix}"));
    let fixture = Path::new("target/behavior-package-fixture.nuif");
    let copied = Path::new("target/behavior-package-cli-copy.nuif");

    let inspection = Command::new(&binary)
        .args(["inspect", path(fixture)?])
        .output()
        .map_err(|error| format!("could not run behavior-package CLI inspection: {error}"))?;
    check_status(
        inspection.status,
        path(&binary)?,
        &["inspect", path(fixture)?],
    )?;
    let inspection: serde_json::Value = serde_json::from_slice(&inspection.stdout)
        .map_err(|error| format!("behavior-package CLI inspection is not JSON: {error}"))?;
    let missing = &inspection["package"]["capabilities"]["missing_required"];
    if missing != &serde_json::json!(["nuif-behavior-state-machine-0"]) {
        return Err(format!(
            "behavior-package CLI reported the wrong missing capabilities: {missing}"
        ));
    }

    let exported = Command::new(&binary)
        .args(["export", path(fixture)?, "nuif-package-0", path(copied)?])
        .output()
        .map_err(|error| format!("could not run behavior-package CLI copy: {error}"))?;
    check_status(
        exported.status,
        path(&binary)?,
        &["export", path(fixture)?, "nuif-package-0", path(copied)?],
    )?;
    if fs::read(fixture).map_err(|error| error.to_string())?
        != fs::read(copied).map_err(|error| error.to_string())?
    {
        return Err("behavior-package CLI no-op copy changed package bytes".to_owned());
    }

    let render_output = Path::new("target/behavior-package-cli.png");
    let pack_output = Path::new("target/behavior-package-cli-authoring.nuif");
    for output in [render_output, pack_output] {
        if output.is_file() {
            fs::remove_file(output).map_err(|error| error.to_string())?;
        }
    }
    expect_cli_capability_failure(&binary, &["render", path(fixture)?, path(render_output)?])?;
    expect_cli_capability_failure(
        &binary,
        &["pack", path(fixture)?, path(pack_output)?, "--authoring"],
    )?;
    if render_output.exists() || pack_output.exists() {
        return Err("behavior-package CLI wrote output after capability rejection".to_owned());
    }
    let report = serde_json::json!({
        "schema_version": 1,
        "status": "passed",
        "profile": "nuif-package-0",
        "missing_required": missing,
        "checks": [
            "structural_inspection",
            "exact_noop_copy",
            "render_rejected",
            "mode_conversion_rejected",
            "rejection_writes_no_output"
        ]
    });
    fs::write(
        "target/behavior-package-cli-report.json",
        serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    println!("behavior package CLI: 5 checks, status passed");
    Ok(())
}

fn expect_cli_capability_failure(binary: &Path, arguments: &[&str]) -> Result<(), String> {
    let output = Command::new(binary)
        .args(arguments)
        .output()
        .map_err(|error| format!("could not run behavior-package CLI rejection: {error}"))?;
    if output.status.success() {
        return Err(format!(
            "{} {} unexpectedly accepted an unsupported capability package",
            binary.display(),
            arguments.join(" ")
        ));
    }
    let error: serde_json::Value = serde_json::from_slice(&output.stderr)
        .map_err(|decode| format!("behavior-package CLI error is not JSON: {decode}"))?;
    if error["code"] != "PACKAGE_CAPABILITIES_REQUIRED"
        || !error["message"]
            .as_str()
            .is_some_and(|message| message.contains("nuif-behavior-state-machine-0"))
    {
        return Err(format!(
            "behavior-package CLI returned the wrong capability error: {error}"
        ));
    }
    Ok(())
}

fn gate_web_behavior() -> Result<(), String> {
    generate_web_behavior_mapping()?;
    prepare_web_host_oracle()?;
    run_web_behavior_oracle()
}

fn generate_web_behavior_mapping() -> Result<(), String> {
    cargo(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "nuif-testing",
        "--bin",
        "web-behavior-mapping",
    ])
}

fn run_web_behavior_oracle() -> Result<(), String> {
    run_web_host_oracle(
        "web behavior",
        &[
            "tools/accessibility-oracle/behavior-check.mjs",
            "target/web-behavior-fixture.html",
            "target/web-behavior-expected.json",
            "target/web-behavior-static-report.json",
            "target/web-behavior-report.json",
        ],
    )
}

fn gate_web_hosts() -> Result<(), String> {
    generate_accessibility_mapping()?;
    generate_web_behavior_mapping()?;
    prepare_web_host_oracle()?;
    run_web_host_oracle(
        "accessibility",
        &[
            "tools/accessibility-oracle/check.mjs",
            "target/accessibility-mapping-fixture.html",
            "target/accessibility-mapping-expected.json",
            "target/accessibility-mapping-static-report.json",
            "target/accessibility-mapping-report.json",
        ],
    )?;
    run_web_behavior_oracle()
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

fn reduction_profile() -> Result<(), String> {
    let fixture = Path::new("target/reduction-profile-fixture");
    if fixture.exists() {
        fs::remove_dir_all(fixture).map_err(|error| {
            format!(
                "failed to remove generated reducer fixture {}: {error}",
                fixture.display()
            )
        })?;
    }
    cargo(&[
        "run",
        "--locked",
        "-p",
        "nuif-testing",
        "--bin",
        "reduction-profile",
        "--",
        "--output",
        "target/reduction-profile-report.json",
        "--fixture",
        "target/reduction-profile-fixture",
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

#[expect(
    clippy::too_many_lines,
    reason = "the campaign setup, exact target limits and result recording stay reviewable together"
)]
fn fuzz_smoke() -> Result<(), String> {
    const NIGHTLY: &str = "nightly-2026-08-28";
    const DEFAULT_RUNS: u32 = 256;
    let runs = env::var("NUIF_FUZZ_RUNS")
        .ok()
        .map(|value| {
            value
                .parse::<u32>()
                .map_err(|error| format!("invalid NUIF_FUZZ_RUNS={value}: {error}"))
                .and_then(|runs| {
                    (1..=1_000_000)
                        .contains(&runs)
                        .then_some(runs)
                        .ok_or_else(|| "NUIF_FUZZ_RUNS must be between 1 and 1000000".to_owned())
                })
        })
        .transpose()?
        .unwrap_or(DEFAULT_RUNS);
    command(
        "cargo",
        &[
            "+nightly-2026-08-28",
            "run",
            "--locked",
            "--manifest-path",
            "fuzz/Cargo.toml",
            "--bin",
            "seed-corpus",
            "--",
            "target/fuzz-corpus",
        ],
    )?;

    let local = PathBuf::from("target/tools/cargo-fuzz/bin/cargo-fuzz");
    let binary = env::var_os("NUIF_CARGO_FUZZ").map_or_else(
        || {
            if local.is_file() {
                local
            } else {
                PathBuf::from("cargo-fuzz")
            }
        },
        PathBuf::from,
    );
    let version = Command::new(&binary)
        .arg("--version")
        .output()
        .map_err(|error| {
            format!(
                "cannot execute {}: {error}; install cargo-fuzz 0.13.2 or set NUIF_CARGO_FUZZ",
                binary.display()
            )
        })?;
    check_status(version.status, path(&binary)?, &["--version"])?;
    let cargo_fuzz_version = String::from_utf8_lossy(&version.stdout).trim().to_owned();
    if cargo_fuzz_version != "cargo-fuzz 0.13.2" {
        return Err(format!(
            "expected cargo-fuzz 0.13.2, observed {cargo_fuzz_version}"
        ));
    }

    let targets = [
        ("codec_roundtrip", 1_048_576_u32),
        ("package_decode", 1_048_576),
        ("resource_decoders", 1_048_576),
        ("adapter_import", 262_144),
        ("operation_sequence", 4_096),
    ];
    let root = env::current_dir().map_err(|error| error.to_string())?;
    let mut outcomes = Vec::new();
    for (target, max_len) in targets {
        let corpus = format!("target/fuzz-corpus/{target}");
        let artifact_dir = root.join("fuzz/artifacts").join(target);
        fs::create_dir_all(&artifact_dir).map_err(|error| error.to_string())?;
        let arguments = vec![
            "run".to_owned(),
            "--fuzz-dir".to_owned(),
            "fuzz".to_owned(),
            target.to_owned(),
            corpus.clone(),
            "--".to_owned(),
            format!("-runs={runs}"),
            format!("-max_len={max_len}"),
            "-timeout=10".to_owned(),
            "-rss_limit_mb=2048".to_owned(),
            "-malloc_limit_mb=512".to_owned(),
            "-use_value_profile=1".to_owned(),
            "-print_final_stats=1".to_owned(),
            format!("-artifact_prefix={}/", artifact_dir.display()),
        ];
        let status = Command::new(&binary)
            .args(&arguments)
            .env("RUSTUP_TOOLCHAIN", NIGHTLY)
            .status()
            .map_err(|error| format!("failed to execute {}: {error}", binary.display()))?;
        outcomes.push(serde_json::json!({
            "target": target,
            "status": if status.success() { "passed" } else { "failed" },
            "runs": runs,
            "max_input_bytes": max_len,
            "corpus": corpus,
            "artifact_directory": format!("fuzz/artifacts/{target}"),
        }));
        write_fuzz_report(&cargo_fuzz_version, NIGHTLY, &outcomes)?;
        if !status.success() {
            return Err(format!("fuzz target {target} failed with {status}"));
        }
    }
    Ok(())
}

fn write_fuzz_report(
    cargo_fuzz_version: &str,
    nightly: &str,
    outcomes: &[serde_json::Value],
) -> Result<(), String> {
    let passed = outcomes.iter().all(|outcome| outcome["status"] == "passed");
    let report = serde_json::json!({
        "schema_version": 1,
        "status": if passed { "passed" } else { "failed" },
        "source": {
            "revision": command_text("git", &["rev-parse", "HEAD"]),
            "dirty": command_text("git", &["status", "--porcelain"]).map(|value| !value.is_empty()),
        },
        "engine": {
            "cargo_fuzz": cargo_fuzz_version,
            "toolchain": nightly,
            "sanitizer": "address",
            "build_standard_library": true,
        },
        "limits": {
            "timeout_seconds_per_input": 10,
            "rss_megabytes": 2048,
            "allocation_megabytes": 512,
        },
        "targets": outcomes,
    });
    fs::create_dir_all("target").map_err(|error| error.to_string())?;
    fs::write(
        "target/fuzz-smoke-report.json",
        serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
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
        "--",
        "--test",
    ])?;
    let report = serde_json::json!({
        "schema_version": 1,
        "status": "passed",
        "mode": "criterion-test-execution",
        "suites": ["profile_zero", "system_surfaces"],
        "source": {
            "revision": command_text("git", &["rev-parse", "HEAD"]),
            "dirty": command_text("git", &["status", "--porcelain"]).map(|value| !value.is_empty()),
        }
    });
    fs::write(
        "target/criterion-smoke-report.json",
        serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn codec_benchmark() -> Result<(), String> {
    cargo(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "nuif-testing",
        "--bin",
        "codec-benchmark",
        "--",
        "--output",
        "target/codec-benchmark-report.json",
    ])
}

fn browser_install() -> Result<(), String> {
    command("sh", &["tools/browser/install-chrome-for-testing.sh"])
}

fn wasm_install() -> Result<(), String> {
    command("rustup", &["target", "add", "wasm32-unknown-unknown"])?;
    let binary = wasm_bindgen_binary();
    if !binary.is_file() {
        command(
            "cargo",
            &[
                "install",
                "--root",
                "target/wasm-tools",
                "--version",
                "=0.2.127",
                "--locked",
                "wasm-bindgen-cli",
            ],
        )?;
    }
    let observed = command_text(path(&binary)?, &["--version"])
        .ok_or("could not inspect the pinned wasm-bindgen CLI")?;
    if observed != "wasm-bindgen 0.2.127" {
        return Err(format!(
            "wasm-bindgen CLI version mismatch: expected 0.2.127, observed {observed:?}"
        ));
    }
    Ok(())
}

fn gate_wasm() -> Result<(), String> {
    wasm_install()?;
    let node_output = Path::new("target/nuif-wasm-node");
    let web_output = Path::new("target/nuif-wasm-web");
    let smoke_output = Path::new("target/wasm-smoke-output.nuif.json");
    let native_output = Path::new("target/wasm-native-output.nuif.json");
    let fixture = Path::new("target/wasm-smoke-input.nuif.json");
    let package_fixture = Path::new("target/wasm-smoke-input.nuif");
    let package_output = Path::new("target/wasm-smoke-output.nuif");
    let native_package_output = Path::new("target/wasm-native-output.nuif");
    let capability_package = Path::new("target/behavior-package-fixture.nuif");
    let variable_font_package = Path::new("target/wasm-variable-font.nuif");
    let variable_font_snapshot = Path::new("target/wasm-variable-font-snapshot");
    let variable_font_report = variable_font_snapshot.join("expected.report.json");
    let patch = Path::new("target/wasm-smoke-patch.json");
    let report = Path::new("target/wasm-conformance-report.json");
    build_wasm_bindings(node_output, web_output)?;
    generate_wasm_fixtures(fixture, package_fixture)?;
    generate_variable_font_snapshot(variable_font_package, variable_font_snapshot)?;
    cargo(&[
        "run",
        "--quiet",
        "--release",
        "--locked",
        "-p",
        "nuif-testing",
        "--bin",
        "behavior-package",
    ])?;
    command(
        "node",
        &[
            "tools/wasm/smoke.cjs",
            path(&node_output.join("nuif.js"))?,
            path(fixture)?,
            path(smoke_output)?,
            path(patch)?,
            path(report)?,
            path(package_fixture)?,
            path(package_output)?,
            path(capability_package)?,
            path(variable_font_package)?,
            path(variable_font_report.as_path())?,
        ],
    )?;
    compare_wasm_patch(
        fixture,
        patch,
        smoke_output,
        native_output,
        "canonical bytes",
    )?;
    compare_wasm_patch(
        package_fixture,
        patch,
        package_output,
        native_package_output,
        "deterministic package bytes",
    )?;
    let browser_version = run_wasm_browser_smoke(
        web_output,
        variable_font_package,
        variable_font_report.as_path(),
    )?;
    let mut report_json = read_json(report)?;
    report_json["checks"]["browser_web_target_initializes"] = serde_json::Value::Bool(true);
    report_json["browser"] = serde_json::json!({
        "name": "Chrome for Testing",
        "version": browser_version,
        "target": "web",
        "status": "passed",
    });
    let mut report_bytes =
        serde_json::to_vec_pretty(&report_json).map_err(|error| error.to_string())?;
    report_bytes.push(b'\n');
    fs::write(report, report_bytes).map_err(|error| error.to_string())?;
    if report_json["status"] != "passed"
        || report_json["api_profile"] != "nuif-wasm-api-0"
        || report_json["checks"]
            .as_object()
            .is_none_or(|checks| checks.values().any(|value| value != true))
    {
        return Err("WebAssembly conformance report failed its assertions".to_owned());
    }
    Ok(())
}

fn generate_wasm_fixtures(fixture: &Path, package_fixture: &Path) -> Result<(), String> {
    cargo(&[
        "run",
        "--quiet",
        "--locked",
        "-p",
        "nuif-cli",
        "--",
        "fixture",
        "v0-responsive-card",
        path(fixture)?,
    ])?;
    cargo(&[
        "run",
        "--quiet",
        "--locked",
        "-p",
        "nuif-cli",
        "--",
        "pack",
        path(fixture)?,
        path(package_fixture)?,
        "--portable",
    ])
}

fn generate_variable_font_snapshot(package: &Path, snapshot: &Path) -> Result<(), String> {
    cargo(&[
        "run",
        "--quiet",
        "--locked",
        "-p",
        "nuif-cli",
        "--",
        "fixture",
        "variable-font-interior",
        path(package)?,
    ])?;
    cargo(&[
        "run",
        "--quiet",
        "--locked",
        "-p",
        "nuif-cli",
        "--",
        "snapshot",
        path(package)?,
        path(snapshot)?,
        "640",
        "96",
    ])
}

fn gate_ffi() -> Result<(), String> {
    cargo(&["test", "--locked", "-p", "nuif-ffi"])?;
    cargo(&["build", "--release", "--locked", "-p", "nuif-ffi"])?;
    command(
        "cc",
        &[
            "-std=c11",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-fsyntax-only",
            "-I.",
            "tools/ffi/header-smoke.c",
        ],
    )?;
    let runtime_smoke = Path::new("target/ffi-runtime-smoke");
    let runtime_status = if cfg!(windows) {
        "not-run-on-windows"
    } else {
        let rpath = if cfg!(target_os = "macos") {
            "-Wl,-rpath,@loader_path/../release"
        } else {
            "-Wl,-rpath,$ORIGIN/release"
        };
        command(
            "cc",
            &[
                "-std=c11",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-I.",
                "tools/ffi/runtime-smoke.c",
                "-Ltarget/release",
                "-lnuif_ffi",
                rpath,
                "-o",
                "target/ffi-runtime-smoke",
            ],
        )?;
        command(path(runtime_smoke)?, &[])?;
        "passed"
    };
    fs::create_dir_all("target").map_err(|error| error.to_string())?;
    let report = serde_json::json!({
        "schema_version": 1,
        "status": "passed",
        "profile": "nuif-ffi-0",
        "header": "bindings/nuif_ffi.h",
        "consumer": "tools/ffi/header-smoke.c",
        "runtime_consumer": "tools/ffi/runtime-smoke.c",
        "runtime_status": runtime_status,
        "mode": "header syntax and POSIX release-library smoke; no stable ABI claim",
        "source": {
            "revision": command_text("git", &["rev-parse", "HEAD"]),
            "dirty": command_text("git", &["status", "--porcelain"]).map(|value| !value.is_empty()),
            "toolchain": command_text("rustc", &["--version"]),
        },
    });
    fs::write(
        "target/ffi-header-report.json",
        serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn compare_wasm_patch(
    input: &Path,
    patch: &Path,
    wasm_output: &Path,
    native_output: &Path,
    output_kind: &str,
) -> Result<(), String> {
    cargo(&[
        "run",
        "--quiet",
        "--locked",
        "-p",
        "nuif-cli",
        "--",
        "patch",
        path(input)?,
        path(patch)?,
        path(native_output)?,
    ])?;
    if fs::read(wasm_output).map_err(|error| error.to_string())?
        != fs::read(native_output).map_err(|error| error.to_string())?
    {
        return Err(format!(
            "WebAssembly and native APIs produced different {output_kind}"
        ));
    }
    Ok(())
}

fn gate_mcp() -> Result<(), String> {
    let executable_suffix = if cfg!(windows) { ".exe" } else { "" };
    let server = Path::new("target")
        .join("debug")
        .join(format!("nuif-mcp{executable_suffix}"));
    let cli = Path::new("target")
        .join("debug")
        .join(format!("nuif{executable_suffix}"));
    let fixture = Path::new("target/mcp-smoke-input.nuif.json");
    let variable_font_package = Path::new("target/mcp-variable-font.nuif");
    let report = Path::new("target/mcp-conformance-report.json");
    cargo(&["build", "--locked", "-p", "nuif-mcp", "-p", "nuif-cli"])?;
    cargo(&[
        "run",
        "--quiet",
        "--locked",
        "-p",
        "nuif-cli",
        "--",
        "fixture",
        "v0-responsive-card",
        path(fixture)?,
    ])?;
    cargo(&[
        "run",
        "--quiet",
        "--locked",
        "-p",
        "nuif-cli",
        "--",
        "fixture",
        "variable-font-interior",
        path(variable_font_package)?,
    ])?;
    command(
        "python3",
        &[
            "tools/mcp/smoke.py",
            "--server",
            path(&server)?,
            "--cli",
            path(&cli)?,
            "--fixture",
            path(fixture)?,
            "--variable-font-package",
            path(variable_font_package)?,
            "--report",
            path(report)?,
        ],
    )
}

fn run_wasm_browser_smoke(
    web_output: &Path,
    variable_font_package: &Path,
    variable_font_report: &Path,
) -> Result<String, String> {
    browser_install()?;
    let page = web_output.join("browser-smoke.html");
    let browser_fixture = web_output.join("browser-fixture.js");
    fs::copy("tools/wasm/browser-smoke.html", &page).map_err(|error| error.to_string())?;
    let package_bytes = fs::read(variable_font_package).map_err(|error| error.to_string())?;
    let expected = read_json(variable_font_report)?;
    fs::write(
        &browser_fixture,
        format!(
            "export const packageBytes = new Uint8Array({});\nexport const expected = {};\n",
            serde_json::to_string(&package_bytes).map_err(|error| error.to_string())?,
            serde_json::to_string(&expected).map_err(|error| error.to_string())?,
        ),
    )
    .map_err(|error| error.to_string())?;
    let url = local_file_url(&page)?;
    let chrome = wasm_browser_binary()?;
    let output = Command::new(&chrome)
        .args([
            "--headless=new",
            "--disable-gpu",
            "--no-sandbox",
            "--allow-file-access-from-files",
            "--virtual-time-budget=5000",
            "--dump-dom",
        ])
        .arg(url)
        .output()
        .map_err(|error| format!("could not start pinned Chrome: {error}"));
    let cleanup = [page.as_path(), browser_fixture.as_path()]
        .into_iter()
        .map(fs::remove_file)
        .collect::<Result<Vec<_>, _>>();
    let output = output?;
    cleanup.map_err(|error| format!("could not remove browser smoke fixture: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "pinned Chrome failed WebAssembly browser smoke with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let dom = String::from_utf8(output.stdout).map_err(|error| error.to_string())?;
    if !dom.contains("data-status=\"passed\"") {
        return Err(format!(
            "generated browser WebAssembly package did not initialize: {}",
            dom.trim()
        ));
    }
    command_text(path(&chrome)?, &["--version"])
        .ok_or_else(|| "could not inspect the pinned Chrome version".to_owned())
}

fn wasm_browser_binary() -> Result<PathBuf, String> {
    [
        PathBuf::from(
            "target/chrome-for-testing/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
        ),
        PathBuf::from(
            "target/chrome-for-testing/chrome-mac-x64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
        ),
        PathBuf::from("target/chrome-for-testing/chrome-linux64/chrome"),
    ]
    .into_iter()
    .find(|candidate| candidate.is_file())
    .ok_or_else(|| "the pinned Chrome for Testing binary is absent".to_owned())
}

fn local_file_url(file: &Path) -> Result<String, String> {
    let canonical = file.canonicalize().map_err(|error| error.to_string())?;
    let value = canonical
        .to_str()
        .ok_or_else(|| format!("path is not valid UTF-8: {}", canonical.display()))?;
    Ok(format!(
        "file://{}",
        value
            .replace('%', "%25")
            .replace(' ', "%20")
            .replace('#', "%23")
    ))
}

fn build_wasm_bindings(node_output: &Path, web_output: &Path) -> Result<(), String> {
    for directory in [node_output, web_output] {
        if directory.exists() {
            fs::remove_dir_all(directory).map_err(|error| error.to_string())?;
        }
        fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    }
    cargo(&[
        "build",
        "--release",
        "--locked",
        "--target",
        "wasm32-unknown-unknown",
        "-p",
        "nuif-wasm",
    ])?;
    let raw = Path::new("target/wasm32-unknown-unknown/release/nuif_wasm.wasm");
    if !raw.is_file() {
        return Err(format!(
            "WebAssembly build output is absent: {}",
            raw.display()
        ));
    }
    let binary = wasm_bindgen_binary();
    for (target, output) in [("nodejs", node_output), ("web", web_output)] {
        command(
            path(&binary)?,
            &[
                path(raw)?,
                "--target",
                target,
                "--out-dir",
                path(output)?,
                "--out-name",
                "nuif",
                "--typescript",
            ],
        )?;
    }
    for file in ["README.md", "LICENSE-APACHE", "LICENSE-MIT"] {
        fs::copy(
            if file == "README.md" {
                "crates/nuif-wasm/README.md"
            } else {
                file
            },
            web_output.join(file),
        )
        .map_err(|error| error.to_string())?;
    }
    fs::copy(
        "crates/nuif-wasm/package.json",
        web_output.join("package.json"),
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn wasm_bindgen_binary() -> PathBuf {
    Path::new("target")
        .join("wasm-tools")
        .join("bin")
        .join(if cfg!(windows) {
            "wasm-bindgen.exe"
        } else {
            "wasm-bindgen"
        })
}

fn wasm_package() -> Result<(), String> {
    gate_wasm()?;
    let package = read_json(Path::new("crates/nuif-wasm/package.json"))?;
    let version = package["version"]
        .as_str()
        .ok_or("nuif-wasm package version is absent")?;
    let manifest =
        fs::read_to_string("crates/nuif-wasm/Cargo.toml").map_err(|error| error.to_string())?;
    if !manifest.contains(&format!("version = \"{version}\"")) {
        return Err("nuif-wasm Cargo and JavaScript package versions differ".to_owned());
    }
    let package_name = format!("nuif-wasm-{version}-web");
    let dist = Path::new("target/dist");
    let package_root = dist.join(&package_name);
    if package_root.exists() {
        fs::remove_dir_all(&package_root).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(&package_root).map_err(|error| error.to_string())?;
    let source = Path::new("target/nuif-wasm-web");
    let files = copy_wasm_package_files(source, &package_root)?;
    let source_revision = command_text("git", &["rev-parse", "HEAD"])
        .ok_or("could not read the source revision for the WASM package")?;
    let source_dirty = command_text("git", &["status", "--porcelain"])
        .map(|value| !value.is_empty())
        .ok_or("could not inspect the source tree for the WASM package")?;
    let mut binding = serde_json::json!({
        "schema_version": 1,
        "status": "passed",
        "name": "@refpath/nuif-wasm",
        "version": version,
        "api_profile": "nuif-wasm-api-0",
        "target": "web",
        "source_revision": source_revision,
        "source_dirty": source_dirty,
        "files": files,
        "publication": {
            "npm": "not-published",
            "github_release": "downloadable-developer-package"
        }
    });
    fs::write(
        package_root.join("manifest.json"),
        serde_json::to_vec_pretty(&binding).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let archive = create_editor_archive(dist, &package_root, &package_name)?;
    let archive_bytes = fs::read(&archive).map_err(|error| error.to_string())?;
    let archive_name = archive
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("archive name is not valid UTF-8: {}", archive.display()))?;
    binding["archive"] = serde_json::json!({
        "name": archive_name,
        "bytes": archive_bytes.len(),
        "sha256": format!("{:x}", Sha256::digest(&archive_bytes)),
    });
    fs::write(
        dist.join(format!("{package_name}.binding.json")),
        serde_json::to_vec_pretty(&binding).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn mcp_package() -> Result<(), String> {
    gate_mcp()?;
    cargo(&[
        "build",
        "--release",
        "--locked",
        "-p",
        "nuif-mcp",
        "--bin",
        "nuif-mcp",
    ])?;
    let target_root =
        env::var_os("CARGO_TARGET_DIR").map_or_else(|| PathBuf::from("target"), PathBuf::from);
    let executable_suffix = if cfg!(windows) { ".exe" } else { "" };
    let source_binary = target_root
        .join("release")
        .join(format!("nuif-mcp{executable_suffix}"));
    if !source_binary.is_file() {
        return Err(format!(
            "release MCP binary is absent: {}",
            source_binary.display()
        ));
    }
    let fixture = Path::new("target/mcp-smoke-input.nuif.json");
    let report = Path::new("target/mcp-package-conformance-report.json");
    let cli = Path::new("target")
        .join("debug")
        .join(format!("nuif{executable_suffix}"));
    command(
        if cfg!(windows) { "python" } else { "python3" },
        &[
            "tools/mcp/smoke.py",
            "--server",
            path(&source_binary)?,
            "--cli",
            path(&cli)?,
            "--fixture",
            path(fixture)?,
            "--report",
            path(report)?,
        ],
    )?;
    let report_json = read_json(report)?;
    if report_json["status"] != "passed"
        || report_json["api_profile"] != "nuif-mcp-tools-0"
        || report_json["protocol_version"] != "2026-07-28"
        || report_json["authorities"] != serde_json::json!([])
    {
        return Err("release MCP binary failed its declared profile".to_owned());
    }

    let version = workspace_package_version("nuif-mcp")?;
    let package_name = format!(
        "nuif-mcp-{version}-{}-{}",
        env::consts::OS,
        env::consts::ARCH
    );
    let dist = target_root.join("dist");
    let package_root = dist.join(&package_name);
    if package_root.exists() {
        fs::remove_dir_all(&package_root).map_err(|error| error.to_string())?;
    }
    let binary_directory = package_root.join("bin");
    fs::create_dir_all(&binary_directory).map_err(|error| error.to_string())?;
    let binary = binary_directory.join(format!("nuif-mcp{executable_suffix}"));
    fs::copy(&source_binary, &binary).map_err(|error| error.to_string())?;
    for license in ["LICENSE-APACHE", "LICENSE-MIT"] {
        fs::copy(license, package_root.join(license)).map_err(|error| error.to_string())?;
    }
    fs::copy("crates/nuif-mcp/README.md", package_root.join("README.md"))
        .map_err(|error| error.to_string())?;
    fs::copy(report, package_root.join("conformance-report.json"))
        .map_err(|error| error.to_string())?;

    let archive = write_mcp_package_manifest(
        &dist,
        &package_root,
        &binary,
        report,
        &package_name,
        &version,
    )?;
    println!("packaged MCP developer tool: {}", archive.display());
    Ok(())
}

fn cli_package() -> Result<(), String> {
    cargo(&[
        "build",
        "--release",
        "--locked",
        "-p",
        "nuif-cli",
        "--bin",
        "nuif",
    ])?;
    let target_root =
        env::var_os("CARGO_TARGET_DIR").map_or_else(|| PathBuf::from("target"), PathBuf::from);
    let executable_suffix = if cfg!(windows) { ".exe" } else { "" };
    let source_binary = target_root
        .join("release")
        .join(format!("nuif{executable_suffix}"));
    if !source_binary.is_file() {
        return Err(format!(
            "release CLI binary is absent: {}",
            source_binary.display()
        ));
    }

    let version = workspace_package_version("nuif-cli")?;
    let (version_report, capabilities, command_names) =
        inspect_cli_package_identity(&source_binary, &version)?;
    let (validation, inspection) = exercise_cli_package(&source_binary, &target_root)?;

    let package_name = format!(
        "nuif-cli-{version}-{}-{}",
        env::consts::OS,
        env::consts::ARCH
    );
    let dist = target_root.join("dist");
    let package_root = dist.join(&package_name);
    if package_root.exists() {
        fs::remove_dir_all(&package_root).map_err(|error| error.to_string())?;
    }
    let binary_directory = package_root.join("bin");
    fs::create_dir_all(&binary_directory).map_err(|error| error.to_string())?;
    let binary = binary_directory.join(format!("nuif{executable_suffix}"));
    fs::copy(&source_binary, &binary).map_err(|error| error.to_string())?;
    for license in ["LICENSE-APACHE", "LICENSE-MIT"] {
        fs::copy(license, package_root.join(license)).map_err(|error| error.to_string())?;
    }
    fs::copy("crates/nuif-cli/README.md", package_root.join("README.md"))
        .map_err(|error| error.to_string())?;
    let smoke = serde_json::json!({
        "schema_version": 1,
        "status": "passed",
        "version": version_report,
        "capabilities": capabilities,
        "fixture": "v0-responsive-card",
        "validation": validation,
        "inspection": inspection,
    });
    let archive = write_cli_package_manifest(
        &dist,
        &package_root,
        &binary,
        &package_name,
        &version,
        &smoke,
        &command_names,
    )?;
    println!("packaged NUIF CLI: {}", archive.display());
    Ok(())
}

fn ffi_package() -> Result<(), String> {
    gate_ffi()?;
    cargo(&["build", "--release", "--locked", "-p", "nuif-ffi"])?;
    let target_root =
        env::var_os("CARGO_TARGET_DIR").map_or_else(|| PathBuf::from("target"), PathBuf::from);
    let version = workspace_package_version("nuif-ffi")?;
    let package_name = format!(
        "nuif-ffi-{version}-{}-{}",
        env::consts::OS,
        env::consts::ARCH
    );
    let dist = target_root.join("dist");
    let package_root = dist.join(&package_name);
    if package_root.exists() {
        fs::remove_dir_all(&package_root).map_err(|error| error.to_string())?;
    }
    let include = package_root.join("include");
    let libraries = package_root.join("lib");
    fs::create_dir_all(&include).map_err(|error| error.to_string())?;
    fs::create_dir_all(&libraries).map_err(|error| error.to_string())?;
    fs::copy("bindings/nuif_ffi.h", include.join("nuif_ffi.h"))
        .map_err(|error| error.to_string())?;
    fs::copy("bindings/README.md", package_root.join("README.md"))
        .map_err(|error| error.to_string())?;
    fs::copy(
        "target/ffi-header-report.json",
        package_root.join("conformance-report.json"),
    )
    .map_err(|error| error.to_string())?;
    for license in ["LICENSE-APACHE", "LICENSE-MIT"] {
        fs::copy(license, package_root.join(license)).map_err(|error| error.to_string())?;
    }

    let release = target_root.join("release");
    let copied = copy_ffi_libraries(&release, &libraries)?;
    let files = copied
        .iter()
        .map(|path| {
            let bytes = fs::read(path).map_err(|error| error.to_string())?;
            Ok(serde_json::json!({
                "name": path.strip_prefix(&package_root).unwrap_or(path),
                "bytes": bytes.len(),
                "sha256": format!("{:x}", Sha256::digest(&bytes))
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let manifest = serde_json::json!({
        "schema_version": 1,
        "status": "passed",
        "name": "nuif-ffi",
        "version": version,
        "api_profile": "nuif-ffi-0",
        "platform": env::consts::OS,
        "architecture": env::consts::ARCH,
        "source_revision": command_text("git", &["rev-parse", "HEAD"]),
        "source_dirty": command_text("git", &["status", "--porcelain"])
            .map(|value| !value.is_empty()),
        "header": {"path": "include/nuif_ffi.h"},
        "files": files,
        "stability": "experimental; not ABI-stable"
    });
    fs::write(
        package_root.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let archive = create_editor_archive(&dist, &package_root, &package_name)?;
    let archive_bytes = fs::read(&archive).map_err(|error| error.to_string())?;
    let mut release_manifest = manifest;
    release_manifest["archive"] = serde_json::json!({
        "name": archive.file_name().and_then(|name| name.to_str()),
        "bytes": archive_bytes.len(),
        "sha256": format!("{:x}", Sha256::digest(&archive_bytes))
    });
    fs::write(
        dist.join(format!("{package_name}.manifest.json")),
        serde_json::to_vec_pretty(&release_manifest).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    println!("packaged experimental C ABI: {}", archive.display());
    Ok(())
}

fn conformance_kit() -> Result<(), String> {
    let version = editor_version()?;
    verify_kit_reports()?;
    let package_name = format!("nuif-conformance-kit-{version}");
    let dist = Path::new("target/dist");
    let package_root = dist.join(&package_name);
    if package_root.exists() {
        fs::remove_dir_all(&package_root).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(&package_root).map_err(|error| error.to_string())?;
    copy_kit_sources(&package_root)?;
    copy_kit_reports(&package_root)?;

    let source_revision = command_text("git", &["rev-parse", "HEAD"])
        .ok_or("could not read the conformance-kit source revision")?;
    let source_dirty = command_text("git", &["status", "--porcelain"])
        .ok_or("could not inspect the conformance-kit source tree")?;
    if !source_dirty.is_empty() {
        return Err("conformance kit requires a clean source tree".to_owned());
    }
    let files = kit_file_manifest(&package_root)?;
    let manifest = serde_json::json!({
        "schema_version": 1,
        "status": "passed",
        "name": "nuif-conformance-kit",
        "version": version,
        "profile": "nuif-conformance-kit-0",
        "source_revision": source_revision,
        "source_dirty": false,
        "files": files,
        "scope": {
            "specification": "nuif-v0-profile-0",
            "independent_reproduction": "python-standard-library",
            "certification": "not-claimed"
        },
        "publication": "github-release-developer-artifact"
    });
    write_kit_manifest(&package_root, &manifest)?;
    let archive = create_editor_archive(dist, &package_root, &package_name)?;
    let archive_bytes = fs::read(&archive).map_err(|error| error.to_string())?;
    let mut release_manifest = manifest;
    release_manifest["archive"] = serde_json::json!({
        "name": archive.file_name().and_then(|name| name.to_str()),
        "bytes": archive_bytes.len(),
        "sha256": format!("{:x}", Sha256::digest(&archive_bytes))
    });
    fs::write(
        dist.join(format!("{package_name}.manifest.json")),
        serde_json::to_vec_pretty(&release_manifest).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    println!("packaged conformance kit: {}", archive.display());
    Ok(())
}

fn verify_kit_reports() -> Result<(), String> {
    for path in [
        "target/gate-g-report.json",
        "target/package-resources-report.json",
        "target/adapter-coverage-report.json",
    ] {
        let report = read_json(Path::new(path))?;
        if report["status"] != "passed" {
            return Err(format!("conformance evidence report is not passed: {path}"));
        }
    }
    Ok(())
}

fn copy_kit_sources(package_root: &Path) -> Result<(), String> {
    for (source, relative) in [
        ("spec", "spec"),
        ("adapters", "adapters"),
        ("conformance", "conformance"),
        ("implementations/python", "implementations/python"),
        ("docs/schema", "docs/schema"),
    ] {
        copy_kit_tree(Path::new(source), &package_root.join(relative))?;
    }
    for source in ["README.md", "LICENSE-APACHE", "LICENSE-MIT", "CITATION.cff"] {
        fs::copy(source, package_root.join(source)).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn copy_kit_reports(package_root: &Path) -> Result<(), String> {
    let reports = package_root.join("reports");
    fs::create_dir_all(&reports).map_err(|error| error.to_string())?;
    for source in [
        "gate-g-report.json",
        "package-resources-report.json",
        "adapter-coverage-report.json",
    ] {
        fs::copy(Path::new("target").join(source), reports.join(source))
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn copy_kit_tree(source: &Path, destination: &Path) -> Result<(), String> {
    let file_type = fs::symlink_metadata(source)
        .map_err(|error| format!("could not inspect kit source {}: {error}", source.display()))?
        .file_type();
    if file_type.is_symlink() {
        return Err(format!(
            "symlink is not allowed in conformance kit: {}",
            source.display()
        ));
    }
    if file_type.is_file() {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::copy(source, destination).map_err(|error| error.to_string())?;
        return Ok(());
    }
    if !file_type.is_dir() {
        return Err(format!(
            "unsupported conformance kit source: {}",
            source.display()
        ));
    }
    if source
        .file_name()
        .is_some_and(|name| matches!(name.to_str(), Some("node_modules" | "target")))
    {
        return Ok(());
    }
    fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    let mut entries = fs::read_dir(source)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        copy_kit_tree(&entry.path(), &destination.join(entry.file_name()))?;
    }
    Ok(())
}

fn kit_file_manifest(root: &Path) -> Result<Vec<serde_json::Value>, String> {
    let mut paths = Vec::new();
    collect_kit_files(root, root, &mut paths)?;
    paths.sort();
    paths
        .into_iter()
        .map(|relative| {
            let path = root.join(&relative);
            let bytes = fs::read(&path).map_err(|error| error.to_string())?;
            Ok(serde_json::json!({
                "name": relative,
                "bytes": bytes.len(),
                "sha256": format!("{:x}", Sha256::digest(bytes))
            }))
        })
        .collect()
}

fn collect_kit_files(root: &Path, directory: &Path, paths: &mut Vec<String>) -> Result<(), String> {
    let mut entries = fs::read_dir(directory)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_dir() {
            collect_kit_files(root, &path, paths)?;
        } else if file_type.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|error| error.to_string())?
                .to_str()
                .ok_or_else(|| format!("kit path is not UTF-8: {}", path.display()))?;
            paths.push(relative.replace(std::path::MAIN_SEPARATOR, "/"));
        } else {
            return Err(format!(
                "unsupported generated kit entry: {}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn write_kit_manifest(package_root: &Path, manifest: &serde_json::Value) -> Result<(), String> {
    fs::write(
        package_root.join("manifest.json"),
        serde_json::to_vec_pretty(manifest).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn copy_ffi_libraries(release: &Path, destination_root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut copied = Vec::new();
    for entry in fs::read_dir(release).map_err(|error| error.to_string())? {
        let path = entry.map_err(|error| error.to_string())?.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let is_library = name.starts_with("libnuif_ffi") || name.starts_with("nuif_ffi");
        let extension = path.extension().and_then(|extension| extension.to_str());
        let is_supported_extension = matches!(
            (env::consts::OS, extension),
            ("linux", Some("so" | "a"))
                | ("macos", Some("dylib" | "a"))
                | ("windows", Some("dll" | "lib"))
        );
        if is_library && is_supported_extension {
            let destination = destination_root.join(name);
            fs::copy(&path, &destination).map_err(|error| error.to_string())?;
            copied.push(destination);
        }
    }
    if copied.is_empty() {
        return Err(format!(
            "release C ABI library artifacts are absent in {}",
            release.display()
        ));
    }
    copied.sort();
    Ok(copied)
}

fn write_cli_package_manifest(
    dist: &Path,
    package_root: &Path,
    binary: &Path,
    package_name: &str,
    version: &str,
    smoke: &serde_json::Value,
    command_names: &BTreeSet<String>,
) -> Result<PathBuf, String> {
    let smoke_bytes = serde_json::to_vec_pretty(&smoke).map_err(|error| error.to_string())?;
    fs::write(package_root.join("smoke-report.json"), &smoke_bytes)
        .map_err(|error| error.to_string())?;

    let binary_bytes = fs::read(binary).map_err(|error| error.to_string())?;
    let mut manifest = serde_json::json!({
        "schema_version": 1,
        "status": "passed",
        "name": "nuif-cli",
        "version": version,
        "api_profile": "nuif-cli-tools-0",
        "protocol_version": smoke["version"]["protocol"],
        "platform": env::consts::OS,
        "architecture": env::consts::ARCH,
        "source_revision": command_text("git", &["rev-parse", "HEAD"]),
        "source_dirty": command_text("git", &["status", "--porcelain"])
            .map(|value| !value.is_empty()),
        "binary": {
            "path": binary.strip_prefix(package_root).unwrap_or(binary),
            "bytes": binary_bytes.len(),
            "sha256": format!("{:x}", Sha256::digest(&binary_bytes))
        },
        "smoke": {
            "path": "smoke-report.json",
            "sha256": format!("{:x}", Sha256::digest(&smoke_bytes)),
            "status": "passed"
        },
        "commands": command_names,
        "authorities": ["caller-selected-filesystem-paths", "stdin-stdout-stderr"],
        "publication": {
            "crates_io": "not-published",
            "github_release": "downloadable-developer-package"
        },
        "signing": {
            "status": "unsigned",
            "note": "checksums and GitHub attestations do not provide an operating-system publisher identity"
        }
    });
    fs::write(
        package_root.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let archive = create_editor_archive(dist, package_root, package_name)?;
    let archive_bytes = fs::read(&archive).map_err(|error| error.to_string())?;
    manifest["archive"] = serde_json::json!({
        "name": archive.file_name().and_then(|name| name.to_str()),
        "bytes": archive_bytes.len(),
        "sha256": format!("{:x}", Sha256::digest(&archive_bytes))
    });
    fs::write(
        dist.join(format!("{package_name}.manifest.json")),
        serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    Ok(archive)
}

fn cli_json(binary: &Path, arguments: &[&str], purpose: &str) -> Result<serde_json::Value, String> {
    let output = Command::new(binary)
        .args(arguments)
        .output()
        .map_err(|error| format!("could not execute packaged CLI {purpose}: {error}"))?;
    check_status(output.status, path(binary)?, arguments)?;
    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("packaged CLI {purpose} output is not JSON: {error}"))
}

fn inspect_cli_package_identity(
    binary: &Path,
    expected_version: &str,
) -> Result<(serde_json::Value, serde_json::Value, BTreeSet<String>), String> {
    let version = cli_json(binary, &["version"], "version command")?;
    if version["name"] != "nuif" || version["version"] != expected_version {
        return Err("packaged CLI version output does not match its Cargo package".to_owned());
    }
    let capabilities = cli_json(binary, &["capabilities"], "capabilities command")?;
    let command_names = capabilities["commands"]
        .as_array()
        .ok_or("packaged CLI capabilities omit the command inventory")?
        .iter()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let required_commands = [
        "capabilities",
        "canonicalize",
        "export",
        "import",
        "inspect",
        "layout",
        "pack",
        "patch",
        "render",
        "snapshot",
        "unpack",
        "validate",
        "version",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<BTreeSet<_>>();
    if capabilities["status"] != "executable"
        || capabilities["protocol"] != version["protocol"]
        || version["protocol"].as_str().is_none()
        || !required_commands.is_subset(&command_names)
    {
        return Err("packaged CLI failed its declared capability profile".to_owned());
    }
    Ok((version, capabilities, command_names))
}

fn exercise_cli_package(
    binary: &Path,
    target_root: &Path,
) -> Result<(serde_json::Value, serde_json::Value), String> {
    let smoke_input = target_root.join("cli-package-smoke-input.nuif.json");
    let smoke_canonical = target_root.join("cli-package-smoke-canonical.nuif.json");
    command(
        path(binary)?,
        &["fixture", "v0-responsive-card", path(&smoke_input)?],
    )?;
    let validation = cli_json(binary, &["validate", path(&smoke_input)?], "validation")?;
    if validation["status"] != "passed" || validation["issues"]["errors"] != 0 {
        return Err("packaged CLI could not validate its own reference fixture".to_owned());
    }
    command(
        path(binary)?,
        &["canonicalize", path(&smoke_input)?, path(&smoke_canonical)?],
    )?;
    let inspection = cli_json(binary, &["inspect", path(&smoke_canonical)?], "inspection")?;
    if inspection["status"] != "passed"
        || inspection["errors"] != 0
        || inspection["entities"].as_u64().unwrap_or_default() == 0
        || inspection["canonical_hash"].as_str().is_none()
    {
        return Err("packaged CLI fixture inspection did not prove a valid document".to_owned());
    }
    Ok((validation, inspection))
}

fn write_mcp_package_manifest(
    dist: &Path,
    package_root: &Path,
    binary: &Path,
    report: &Path,
    package_name: &str,
    version: &str,
) -> Result<PathBuf, String> {
    let binary_bytes = fs::read(binary).map_err(|error| error.to_string())?;
    let report_bytes = fs::read(report).map_err(|error| error.to_string())?;
    let mut manifest = serde_json::json!({
        "schema_version": 1,
        "status": "passed",
        "name": "nuif-mcp",
        "version": version,
        "api_profile": "nuif-mcp-tools-0",
        "protocol_version": "2026-07-28",
        "platform": env::consts::OS,
        "architecture": env::consts::ARCH,
        "source_revision": command_text("git", &["rev-parse", "HEAD"]),
        "source_dirty": command_text("git", &["status", "--porcelain"])
            .map(|value| !value.is_empty()),
        "binary": {
            "path": binary.strip_prefix(package_root).unwrap_or(binary),
            "bytes": binary_bytes.len(),
            "sha256": format!("{:x}", Sha256::digest(&binary_bytes))
        },
        "conformance": {
            "path": "conformance-report.json",
            "sha256": format!("{:x}", Sha256::digest(&report_bytes)),
            "status": "passed"
        },
        "limits": {
            "message_bytes": 4 * 1024 * 1024,
            "document_bytes": 1024 * 1024,
            "patch_bytes": 1024 * 1024,
            "patch_transactions": 1024,
            "patch_operations": 16_384
        },
        "authorities": [],
        "publication": {
            "crates_io": "not-published",
            "github_release": "downloadable-developer-package"
        },
        "signing": {
            "status": "unsigned",
            "note": "checksums and GitHub attestations do not provide an operating-system publisher identity"
        }
    });
    let internal = serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?;
    fs::write(package_root.join("manifest.json"), internal).map_err(|error| error.to_string())?;
    let archive = create_editor_archive(dist, package_root, package_name)?;
    let archive_bytes = fs::read(&archive).map_err(|error| error.to_string())?;
    manifest["archive"] = serde_json::json!({
        "name": archive.file_name().and_then(|name| name.to_str()),
        "bytes": archive_bytes.len(),
        "sha256": format!("{:x}", Sha256::digest(&archive_bytes))
    });
    let external = serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?;
    fs::write(dist.join(format!("{package_name}.manifest.json")), external)
        .map_err(|error| error.to_string())?;
    Ok(archive)
}

fn copy_wasm_package_files(
    source: &Path,
    package_root: &Path,
) -> Result<Vec<serde_json::Value>, String> {
    let mut files = Vec::new();
    let mut observed_names = BTreeSet::new();
    for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if !file_type.is_file() {
            return Err(format!(
                "unexpected non-file in generated WASM package: {}",
                entry.path().display()
            ));
        }
        let name = entry.file_name().into_string().map_err(|name| {
            format!(
                "generated WASM package filename is not valid UTF-8: {}",
                name.display()
            )
        })?;
        observed_names.insert(name.clone());
        let destination = package_root.join(&name);
        fs::copy(entry.path(), &destination).map_err(|error| error.to_string())?;
        let bytes = fs::read(&destination).map_err(|error| error.to_string())?;
        files.push(serde_json::json!({
            "name": name,
            "bytes": bytes.len(),
            "sha256": format!("{:x}", Sha256::digest(&bytes)),
        }));
    }
    let expected_names = [
        "LICENSE-APACHE",
        "LICENSE-MIT",
        "README.md",
        "nuif.d.ts",
        "nuif.js",
        "nuif_bg.wasm",
        "nuif_bg.wasm.d.ts",
        "package.json",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<BTreeSet<_>>();
    if observed_names != expected_names {
        return Err(format!(
            "generated WASM package file set changed: expected {expected_names:?}, observed {observed_names:?}"
        ));
    }
    files.sort_by(|left, right| left["name"].as_str().cmp(&right["name"].as_str()));
    Ok(files)
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

fn gate_react() -> Result<(), String> {
    cargo(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "nuif-react",
        "--bin",
        "react-sync-profile",
        "--",
        "target/react-sync-report.json",
        "target/react-sync-output.jsx",
        "target/react-sync-edited.nuif.json",
    ])?;
    gate_react_cli_bridge()
}

fn gate_svelte() -> Result<(), String> {
    cargo(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "nuif-svelte",
        "--bin",
        "svelte-sync-profile",
        "--",
        "target/svelte-sync-report.json",
        "target/svelte-sync-output.svelte",
        "target/svelte-sync-edited.nuif.json",
    ])?;
    gate_svelte_cli_bridge()?;
    command(
        "npm",
        &[
            "ci",
            "--ignore-scripts",
            "--no-audit",
            "--no-fund",
            "--prefix",
            "tools/svelte-oracle",
        ],
    )?;
    command(
        "node",
        &[
            "tools/svelte-oracle/check.mjs",
            "target/svelte-compiler-oracle-report.json",
            "target/svelte-sync-output.svelte",
            "target/svelte-sync-cli-output.svelte",
        ],
    )
}

fn gate_figma() -> Result<(), String> {
    cargo(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "nuif-figma",
        "--bin",
        "figma-snapshot-profile",
        "--",
        "target/figma-snapshot-report.json",
    ])?;
    let directory = env::temp_dir().join(format!("nuif-figma-trial-{}", std::process::id()));
    if directory.exists() {
        return Err(format!(
            "temporary path already exists: {}",
            directory.display()
        ));
    }
    fs::create_dir(&directory).map_err(|error| error.to_string())?;
    let input = directory.join("input.nuif.json");
    let plan = directory.join("plan.json");
    let snapshot = directory.join("snapshot.json");
    let imported = directory.join("imported.nuif.json");
    let export_report = directory.join("export-report.json");
    let import_report = directory.join("import-report.json");
    nuif(&["fixture", "figma-profile", path(&input)?])?;
    nuif(&[
        "export",
        path(&input)?,
        "figma-plugin-snapshot-0",
        path(&plan)?,
        path(&export_report)?,
    ])?;
    let plan_json = read_json(&plan)?;
    fs::write(
        &snapshot,
        serde_json::to_vec_pretty(&plan_json["snapshot"]).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    nuif(&[
        "import",
        "figma-plugin-snapshot-0",
        path(&snapshot)?,
        path(&imported)?,
        path(&import_report)?,
    ])?;
    if fs::read(&input).map_err(|error| error.to_string())?
        != fs::read(&imported).map_err(|error| error.to_string())?
    {
        return Err("Figma CLI mapping did not reproduce canonical NUIF bytes".to_owned());
    }
    let export_report_json = read_json(&export_report)?;
    let import_report_json = read_json(&import_report)?;
    if export_report_json["direction"] != "import"
        || import_report_json["direction"] != "export"
        || export_report_json["profile"] != "nuif-figma-plugin-snapshot-0"
        || import_report_json["profile"] != "nuif-figma-plugin-snapshot-0"
    {
        return Err("Figma CLI reports do not declare both mapping directions".to_owned());
    }
    gate_figma_plugin_shell(&plan)?;
    fs::remove_dir_all(&directory).map_err(|error| {
        format!(
            "trial passed but temporary directory {} could not be removed: {error}",
            directory.display()
        )
    })
}

fn gate_canva() -> Result<(), String> {
    cargo(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "nuif-canva",
        "--bin",
        "canva-current-page-profile",
        "--",
        "target/canva-current-page-report.json",
    ])?;
    let report = read_json(Path::new("target/canva-current-page-report.json"))?;
    if report["status"] != "passed"
        || report["profile"] != "nuif-canva-design-editing-0"
        || report["live_host"]["required_before_vendor_integration_claim"] != true
    {
        return Err("Canva current-page profile report failed its assertions".to_owned());
    }
    let input = Path::new("target/canva-app-fixture.nuif.json");
    let plan = Path::new("target/canva-app-plan.json");
    let page = Path::new("target/canva-app-fixture-page.json");
    let imported = Path::new("target/canva-app-fixture-imported.nuif.json");
    let adapter_report = Path::new("target/canva-app-fixture-report.json");
    nuif(&["fixture", "canva-profile", path(input)?])?;
    nuif(&[
        "export",
        path(input)?,
        "canva-design-editing-plan-0",
        path(plan)?,
        "target/canva-app-plan-report.json",
    ])?;
    let plan_json = read_json(plan)?;
    fs::write(
        page,
        serde_json::to_vec_pretty(&plan_json["page"]).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    nuif(&[
        "import",
        "canva-design-editing-0",
        path(page)?,
        path(imported)?,
        path(adapter_report)?,
    ])?;
    if fs::read(input).map_err(|error| error.to_string())?
        != fs::read(imported).map_err(|error| error.to_string())?
    {
        return Err("Canva CLI mapping did not reproduce canonical NUIF bytes".to_owned());
    }
    gate_canva_app_shell(plan)
}

fn gate_canva_app_shell(rust_plan: &Path) -> Result<(), String> {
    const APP: &str = "adapters/canva/app";
    command(
        "npm",
        &[
            "--prefix",
            APP,
            "ci",
            "--ignore-scripts",
            "--no-audit",
            "--no-fund",
        ],
    )?;
    command("npm", &["--prefix", APP, "run", "check"])?;
    let first_build = read_json(Path::new(APP).join("dist/build-report.json").as_path())?;
    command("npm", &["--prefix", APP, "run", "build"])?;
    let second_build = read_json(Path::new(APP).join("dist/build-report.json").as_path())?;
    if first_build != second_build {
        return Err("Canva review app did not rebuild deterministically".to_owned());
    }
    let root = env::current_dir().map_err(|error| error.to_string())?;
    let rust_plan = if rust_plan.is_absolute() {
        rust_plan.to_path_buf()
    } else {
        root.join(rust_plan)
    };
    let validation = root.join("target/canva-app-plan-validation.json");
    command(
        "npm",
        &[
            "--prefix",
            APP,
            "run",
            "validate-plan",
            "--",
            path(&rust_plan)?,
            path(&validation)?,
        ],
    )?;
    let validated = read_json(&validation)?;
    if validated["status"] != "passed" || validated["profile"] != "nuif-canva-design-editing-0" {
        return Err("compiled Canva shell rejected the Rust mutation plan".to_owned());
    }
    let benchmark = root.join("target/canva-app-benchmark-report.json");
    command(
        "npm",
        &["--prefix", APP, "run", "benchmark", "--", path(&benchmark)?],
    )?;
    let benchmark_report = read_json(&benchmark)?;
    if benchmark_report["status"] != "passed"
        || benchmark_report["live_host"]["status"] != "not_run"
        || benchmark_report["hostile_rejection"]["rejected"]
            != benchmark_report["hostile_rejection"]["expected"]
    {
        return Err("Canva review app benchmark contract failed".to_owned());
    }
    package_canva_app_shell(APP, &second_build)?;
    write_canva_app_report(&second_build, &validated, &benchmark_report)
}

fn package_canva_app_shell(app: &str, build: &serde_json::Value) -> Result<(), String> {
    if build["status"] != "passed"
        || build["review_bundle"] != true
        || build["live_ready"] != false
        || build["network_domains"] != serde_json::json!([])
        || build["license_scope"] != "Canva Platform permitted apps only"
    {
        return Err("Canva review build report exceeds its credential-free scope".to_owned());
    }
    let source = Path::new(app).join("dist");
    let package = Path::new("target/nuif-canva-review-app");
    if package.exists() {
        fs::remove_dir_all(package).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(package).map_err(|error| error.to_string())?;
    for name in ["app.js", "CANVA-SDK-LICENSE.md", "build-report.json"] {
        fs::copy(source.join(name), package.join(name)).map_err(|error| error.to_string())?;
    }
    fs::copy(Path::new(app).join("README.md"), package.join("README.md"))
        .map_err(|error| error.to_string())?;
    fs::copy(
        "target/canva-app-types/report.json",
        package.join("type-declaration-report.json"),
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn write_canva_app_report(
    build: &serde_json::Value,
    validation: &serde_json::Value,
    benchmark: &serde_json::Value,
) -> Result<(), String> {
    let lock = read_json(Path::new("adapters/canva/app/package-lock.json"))?;
    let adapter = read_json(Path::new("target/canva-app-fixture-report.json"))?;
    let report = serde_json::json!({
        "schema_version": 1,
        "status": "passed",
        "profile": "nuif-canva-design-editing-0",
        "scope": "static compiled Canva-only review app and normalized mapping; no live Canva execution",
        "toolchain": {
            "node": command_text("node", &["--version"]).unwrap_or_else(|| "unreported".to_owned()),
            "canva_design": lock["packages"]["node_modules/@canva/design"]["version"],
            "typescript": lock["packages"]["node_modules/typescript"]["version"],
            "esbuild": lock["packages"]["node_modules/esbuild"]["version"]
        },
        "build": build,
        "rust_plan_validation": validation,
        "benchmark": benchmark,
        "fixture": {
            "canonical_hash": adapter["canonical_hash"],
            "fidelity_entries": adapter["fidelity"].as_array().map_or(0, Vec::len),
            "correspondences": adapter["correspondences"].as_array().map_or(0, Vec::len),
            "canonical_bytes_equal": true
        },
        "safety": {
            "network_domains": [],
            "explicit_apply_confirmation": true,
            "empty_page_preflight": true,
            "single_sync_mock_test": true,
            "sdk_license_included": true,
            "marketplace_credentials": false
        },
        "live_host": {
            "status": "not_run",
            "required_before_vendor_integration_claim": true
        }
    });
    fs::write(
        "target/canva-app-shell-report.json",
        serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn gate_figma_plugin_shell(rust_plan: &Path) -> Result<(), String> {
    const PLUGIN: &str = "adapters/figma/plugin";
    command(
        "npm",
        &[
            "--prefix",
            PLUGIN,
            "ci",
            "--ignore-scripts",
            "--no-audit",
            "--no-fund",
        ],
    )?;
    command("npm", &["--prefix", PLUGIN, "run", "check"])?;
    let root = env::current_dir().map_err(|error| error.to_string())?;
    let plan_validation = root.join("target/figma-plugin-plan-validation.json");
    command(
        "npm",
        &[
            "--prefix",
            PLUGIN,
            "run",
            "validate-plan",
            "--",
            path(rust_plan)?,
            path(&plan_validation)?,
        ],
    )?;
    let snapshot = root.join("target/figma-plugin-fixture-snapshot.json");
    command(
        "npm",
        &["--prefix", PLUGIN, "run", "fixture", "--", path(&snapshot)?],
    )?;
    let document = Path::new("target/figma-plugin-fixture.nuif.json");
    let adapter_report = Path::new("target/figma-plugin-fixture-report.json");
    nuif(&[
        "import",
        "figma-plugin-snapshot-0",
        path(&snapshot)?,
        path(document)?,
        path(adapter_report)?,
    ])?;
    nuif(&["validate", path(document)?])?;

    let build = package_figma_plugin_shell(PLUGIN)?;

    let fixture = read_json(&snapshot)?;
    let imported = read_json(adapter_report)?;
    let validated_plan = read_json(&plan_validation)?;
    if fixture["schema_version"] != 1
        || fixture["root"]["kind"] != "FRAME"
        || imported["profile"] != "nuif-figma-plugin-snapshot-0"
        || imported["direction"] != "export"
        || validated_plan["status"] != "passed"
        || validated_plan["profile"] != "nuif-figma-plugin-snapshot-0"
    {
        return Err(
            "compiled Figma shell fixture did not cross the Rust adapter boundary".to_owned(),
        );
    }
    let fixture_nodes = count_snapshot_json_nodes(&fixture["root"])?;
    let lock = read_json(Path::new(PLUGIN).join("package-lock.json").as_path())?;
    let report = serde_json::json!({
        "schema_version": 1,
        "status": "passed",
        "profile": "nuif-figma-plugin-snapshot-0",
        "scope": "static compiled no-network review shell plus TypeScript-to-Rust fixture; no live Figma execution",
        "toolchain": {
            "node": command_text("node", &["--version"]).unwrap_or_else(|| "unreported".to_owned()),
            "figma_plugin_typings": lock["packages"]["node_modules/@figma/plugin-typings"]["version"],
            "typescript": lock["packages"]["node_modules/typescript"]["version"],
            "esbuild": lock["packages"]["node_modules/esbuild"]["version"]
        },
        "build": build,
        "rust_plan_validation": validated_plan,
        "fixture": {
            "snapshot_bytes": fs::metadata(&snapshot).map_err(|error| error.to_string())?.len(),
            "nodes": fixture_nodes,
            "canonical_hash": imported["canonical_hash"],
            "fidelity_entries": imported["fidelity"].as_array().map_or(0, Vec::len),
            "correspondences": imported["correspondences"].as_array().map_or(0, Vec::len)
        },
        "safety": {
            "network_domains": [],
            "manifest_id": "reviewer-assigned-required",
            "explicit_apply_confirmation": true,
            "host_mutation_cleanup_compiled": true
        },
        "live_host": {
            "status": "not_run",
            "required_before_vendor_integration_claim": true
        }
    });
    fs::write(
        "target/figma-plugin-shell-report.json",
        serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn package_figma_plugin_shell(plugin: &str) -> Result<serde_json::Value, String> {
    let source_dist = Path::new(plugin).join("dist");
    let build = read_json(source_dist.join("build-report.json").as_path())?;
    if build["status"] != "passed"
        || build["live_ready"] != false
        || build["network_domains"] != serde_json::json!([])
    {
        return Err("Figma review shell build report overclaims its static evidence".to_owned());
    }
    if source_dist.join("manifest.json").exists() {
        return Err("credential-free Figma build unexpectedly emitted a live manifest".to_owned());
    }
    let template = fs::read_to_string(source_dist.join("manifest.template.json"))
        .map_err(|error| error.to_string())?;
    if !template.contains("REPLACE_WITH_FIGMA_PLUGIN_ID")
        || !template.contains("\"allowedDomains\": [\"none\"]")
    {
        return Err(
            "Figma manifest template lost its assigned-ID or no-network boundary".to_owned(),
        );
    }
    let package = Path::new("target/nuif-figma-plugin-review-shell");
    if package.exists() {
        fs::remove_dir_all(package).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(package).map_err(|error| error.to_string())?;
    for name in [
        "main.js",
        "ui.html",
        "manifest.template.json",
        "build-report.json",
    ] {
        fs::copy(source_dist.join(name), package.join(name)).map_err(|error| error.to_string())?;
    }
    fs::copy(
        Path::new(plugin).join("README.md"),
        package.join("README.md"),
    )
    .map_err(|error| error.to_string())?;
    Ok(build)
}

fn count_snapshot_json_nodes(node: &serde_json::Value) -> Result<usize, String> {
    let children = node["children"]
        .as_array()
        .ok_or("Figma fixture node is missing its children array")?;
    children.iter().try_fold(1_usize, |count, child| {
        count_snapshot_json_nodes(child).map(|child_count| count + child_count)
    })
}

fn gate_svelte_cli_bridge() -> Result<(), String> {
    let directory = env::temp_dir().join(format!("nuif-svelte-trial-{}", std::process::id()));
    if directory.exists() {
        return Err(format!(
            "temporary path already exists: {}",
            directory.display()
        ));
    }
    fs::create_dir(&directory).map_err(|error| error.to_string())?;
    let input = directory.join("input.nuif.json");
    let exported = directory.join("exported.svelte");
    let export_report = directory.join("export-report.json");
    let imported = directory.join("imported.nuif.json");
    let import_report = directory.join("import-report.json");
    let reimported = directory.join("reimported.nuif.json");
    let reimport_report = directory.join("reimport-report.json");
    let synchronized = Path::new("target/svelte-sync-cli-output.svelte");
    let sync_report = Path::new("target/svelte-sync-cli-report.json");
    let edited = Path::new("target/svelte-sync-edited.nuif.json");
    nuif(&["fixture", "svelte-profile", path(&input)?])?;
    nuif(&[
        "export",
        path(&input)?,
        "svelte-static-0",
        path(&exported)?,
        path(&export_report)?,
    ])?;
    nuif(&[
        "import",
        "svelte-static-0",
        path(&exported)?,
        path(&imported)?,
        path(&import_report)?,
    ])?;
    if fs::read(&input).map_err(|error| error.to_string())?
        != fs::read(&imported).map_err(|error| error.to_string())?
    {
        return Err("CLI Svelte export/import changed canonical NUIF bytes".to_owned());
    }
    nuif(&[
        "sync",
        "svelte-static-0",
        path(&exported)?,
        path(edited)?,
        path(synchronized)?,
        path(sync_report)?,
    ])?;
    nuif(&[
        "import",
        "svelte-static-0",
        path(synchronized)?,
        path(&reimported)?,
        path(&reimport_report)?,
    ])?;
    if fs::read(edited).map_err(|error| error.to_string())?
        != fs::read(&reimported).map_err(|error| error.to_string())?
    {
        return Err(
            "CLI Svelte synchronization changed edited canonical document bytes".to_owned(),
        );
    }
    let report = read_json(sync_report)?;
    if report["status"] != "passed" || report["edits"].as_array().map(Vec::len) != Some(11) {
        return Err("CLI Svelte bridge did not produce the expected 11 source edits".to_owned());
    }
    fs::remove_dir_all(&directory).map_err(|error| {
        format!(
            "trial passed but temporary directory {} could not be removed: {error}",
            directory.display()
        )
    })
}

fn gate_react_cli_bridge() -> Result<(), String> {
    let directory = env::temp_dir().join(format!("nuif-react-trial-{}", std::process::id()));
    if directory.exists() {
        return Err(format!(
            "temporary path already exists: {}",
            directory.display()
        ));
    }
    fs::create_dir(&directory).map_err(|error| error.to_string())?;
    let input = directory.join("input.nuif.json");
    let exported = directory.join("exported.jsx");
    let export_report = directory.join("export-report.json");
    let imported = directory.join("imported.nuif.json");
    let import_report = directory.join("import-report.json");
    let reimported = directory.join("reimported.nuif.json");
    let reimport_report = directory.join("reimport-report.json");
    let synchronized = Path::new("target/react-sync-cli-output.jsx");
    let sync_report = Path::new("target/react-sync-cli-report.json");
    let edited = Path::new("target/react-sync-edited.nuif.json");
    nuif(&["fixture", "react-profile", path(&input)?])?;
    nuif(&[
        "export",
        path(&input)?,
        "react-jsx-0",
        path(&exported)?,
        path(&export_report)?,
    ])?;
    nuif(&[
        "import",
        "react-jsx-0",
        path(&exported)?,
        path(&imported)?,
        path(&import_report)?,
    ])?;
    if fs::read(&input).map_err(|error| error.to_string())?
        != fs::read(&imported).map_err(|error| error.to_string())?
    {
        return Err("CLI React JSX export/import changed canonical NUIF bytes".to_owned());
    }
    nuif(&[
        "sync",
        "react-jsx-0",
        path(&exported)?,
        path(edited)?,
        path(synchronized)?,
        path(sync_report)?,
    ])?;
    nuif(&[
        "import",
        "react-jsx-0",
        path(synchronized)?,
        path(&reimported)?,
        path(&reimport_report)?,
    ])?;
    if fs::read(edited).map_err(|error| error.to_string())?
        != fs::read(&reimported).map_err(|error| error.to_string())?
    {
        return Err(
            "CLI React JSX synchronization changed edited canonical document bytes".to_owned(),
        );
    }
    let report = read_json(sync_report)?;
    if report["status"] != "passed" || report["edits"].as_array().map(Vec::len) != Some(11) {
        return Err("CLI React JSX bridge did not produce the expected 11 source edits".to_owned());
    }
    fs::remove_dir_all(&directory).map_err(|error| {
        format!(
            "trial passed but temporary directory {} could not be removed: {error}",
            directory.display()
        )
    })
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

fn gate_h_conformance(
    binary: &str,
    output: &str,
    oracle_input: Option<&str>,
) -> Result<(), String> {
    let mut args = vec![
        "run",
        "--release",
        "--locked",
        "-p",
        "nuif-conformance",
        "--bin",
        binary,
        "--",
        "--output",
        output,
    ];
    if let Some(input) = oracle_input {
        args.extend(["--oracle-input", input]);
    }
    cargo(&args)
}

fn gate_h() -> Result<(), String> {
    gate_h_conformance(
        "collaboration-registers",
        "target/collaboration-report.json",
        None,
    )?;
    gate_h_conformance(
        "collaboration-structure",
        "target/collaboration-structure-report.json",
        Some("target/collaboration-automerge-input.json"),
    )?;
    command(
        "python3",
        &[
            "implementations/python/nuif_tree_materializer.py",
            "target/collaboration-automerge-input.json",
            "target/collaboration-tree-foreign-report.json",
        ],
    )?;
    gate_h_conformance(
        "collaboration-creation",
        "target/collaboration-creation-report.json",
        None,
    )?;
    gate_h_conformance(
        "collaboration-nested-creation",
        "target/collaboration-nested-creation-report.json",
        None,
    )?;
    gate_h_conformance(
        "collaboration-nested-creation-v1",
        "target/collaboration-nested-creation-v1-report.json",
        None,
    )?;
    gate_h_conformance(
        "collaboration-mixed",
        "target/collaboration-mixed-report.json",
        None,
    )?;
    gate_h_conformance(
        "collaboration-gc",
        "target/collaboration-gc-report.json",
        None,
    )?;
    gate_h_conformance(
        "collaboration-gc-prefix",
        "target/collaboration-gc-prefix-report.json",
        None,
    )?;
    command(
        "npm",
        &[
            "ci",
            "--ignore-scripts",
            "--no-audit",
            "--no-fund",
            "--prefix",
            "tools/automerge-oracle",
        ],
    )?;
    command(
        "node",
        &[
            "tools/automerge-oracle/check.mjs",
            "target/collaboration-automerge-input.json",
            "target/collaboration-automerge-report.json",
        ],
    )
}

const MAX_WORKFLOW_BYTES: u64 = 1_048_576;
const MAX_WORKFLOW_DEPTH: usize = 64;

fn workflow_audit() -> Result<(), String> {
    let directory = Path::new(".github/workflows");
    let mut entries = fs::read_dir(directory)
        .map_err(|error| format!("cannot read {}: {error}", directory.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("cannot enumerate {}: {error}", directory.display()))?;
    entries.sort_by_key(std::fs::DirEntry::file_name);

    let mut failures = Vec::new();
    let mut workflows = Vec::new();
    let mut action_references = Vec::new();
    let mut artifact_paths = 0usize;
    for entry in entries {
        let path = entry.path();
        let extension = path.extension().and_then(|value| value.to_str());
        if !matches!(extension, Some("yml" | "yaml")) {
            continue;
        }
        let display = path.display().to_string();
        workflows.push(display.clone());
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("cannot inspect {display}: {error}"))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            failures.push(format!("{display}: workflow must be a regular file"));
            continue;
        }
        if metadata.len() > MAX_WORKFLOW_BYTES {
            failures.push(format!(
                "{display}: {} bytes exceeds the {MAX_WORKFLOW_BYTES} byte limit",
                metadata.len()
            ));
            continue;
        }
        let bytes = fs::read(&path).map_err(|error| format!("cannot read {display}: {error}"))?;
        let source = match std::str::from_utf8(&bytes) {
            Ok(source) => source,
            Err(error) => {
                failures.push(format!("{display}: workflow is not UTF-8: {error}"));
                continue;
            }
        };
        let value = match serde_saphyr::from_str::<serde_json::Value>(source) {
            Ok(serde_json::Value::Object(value)) => serde_json::Value::Object(value),
            Ok(_) => {
                failures.push(format!("{display}: workflow root must be a mapping"));
                continue;
            }
            Err(error) => {
                failures.push(format!("{display}: strict YAML parse failed: {error}"));
                continue;
            }
        };
        audit_workflow_value(
            &value,
            &display,
            0,
            &mut action_references,
            &mut artifact_paths,
            &mut failures,
        );
    }
    if workflows.is_empty() {
        failures.push(".github/workflows contains no YAML workflows".to_owned());
    }

    let report = serde_json::json!({
        "schema_version": 1,
        "status": if failures.is_empty() { "passed" } else { "failed" },
        "limits": {
            "workflow_bytes": MAX_WORKFLOW_BYTES,
            "workflow_depth": MAX_WORKFLOW_DEPTH,
        },
        "summary": {
            "workflows": workflows.len(),
            "external_action_references": action_references.len(),
            "artifact_paths": artifact_paths,
            "blocking_failures": failures.len(),
        },
        "workflows": workflows,
        "action_references": action_references,
        "failures": failures,
    });
    fs::create_dir_all("target").map_err(|error| error.to_string())?;
    fs::write(
        "target/workflow-audit-report.json",
        serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    if report["status"] == "passed" {
        Ok(())
    } else {
        Err("workflow audit failed; inspect target/workflow-audit-report.json".to_owned())
    }
}

fn audit_workflow_value(
    value: &serde_json::Value,
    location: &str,
    depth: usize,
    action_references: &mut Vec<serde_json::Value>,
    artifact_paths: &mut usize,
    failures: &mut Vec<String>,
) {
    if depth > MAX_WORKFLOW_DEPTH {
        failures.push(format!(
            "{location}: workflow nesting exceeds {MAX_WORKFLOW_DEPTH}"
        ));
        return;
    }
    match value {
        serde_json::Value::Object(object) => {
            if let Some(uses) = object.get("uses").and_then(serde_json::Value::as_str) {
                if !uses.starts_with("./") {
                    action_references.push(serde_json::json!({
                        "location": location,
                        "uses": uses,
                    }));
                    if !immutable_action_reference(uses) {
                        failures.push(format!(
                            "{location}/uses: external action must use a full commit SHA: {uses}"
                        ));
                    }
                }
                if uses.starts_with("actions/upload-artifact@")
                    && let Some(path) = object
                        .get("with")
                        .and_then(serde_json::Value::as_object)
                        .and_then(|with| with.get("path"))
                        .and_then(serde_json::Value::as_str)
                {
                    let mut seen = BTreeSet::new();
                    for path in path.lines().map(str::trim).filter(|path| !path.is_empty()) {
                        *artifact_paths += 1;
                        if !seen.insert(path) {
                            failures.push(format!(
                                "{location}/with/path: duplicate artifact path {path:?}"
                            ));
                        }
                    }
                }
            }
            for (key, child) in object {
                audit_workflow_value(
                    child,
                    &format!("{location}/{key}"),
                    depth + 1,
                    action_references,
                    artifact_paths,
                    failures,
                );
            }
        }
        serde_json::Value::Array(array) => {
            for (index, child) in array.iter().enumerate() {
                audit_workflow_value(
                    child,
                    &format!("{location}/{index}"),
                    depth + 1,
                    action_references,
                    artifact_paths,
                    failures,
                );
            }
        }
        _ => {}
    }
}

fn immutable_action_reference(reference: &str) -> bool {
    if let Some(image) = reference.strip_prefix("docker://") {
        return image.rsplit_once("@sha256:").is_some_and(|(name, digest)| {
            !name.is_empty()
                && digest.len() == 64
                && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        });
    }
    reference
        .rsplit_once('@')
        .is_some_and(|(action, revision)| {
            action.contains('/')
                && !action.contains('@')
                && revision.len() == 40
                && revision.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
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
        "affinity",
        "canva",
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
            audit_adapter_profile(id, profile, directions, failures);
        }
        let target_direction_list = directions.into_iter().flatten().collect::<Vec<_>>();
        let target_direction_set = target_direction_list
            .iter()
            .filter_map(|direction| direction.as_str())
            .collect::<BTreeSet<_>>();
        if target_direction_set.len() != target_direction_list.len() {
            failures.push(format!(
                "{id}: target directions contain a duplicate or non-string value"
            ));
        }
        if target_direction_set
            .iter()
            .any(|direction| !matches!(*direction, "import" | "export" | "synchronize"))
        {
            failures.push(format!("{id}: target directions contain an unknown value"));
        }
        let profile_direction_set = profiles
            .into_iter()
            .flatten()
            .flat_map(|profile| profile["directions"].as_array().into_iter().flatten())
            .filter_map(serde_json::Value::as_str)
            .collect::<BTreeSet<_>>();
        if target_direction_set != profile_direction_set {
            failures.push(format!(
                "{id}: target directions must equal the union of profile directions"
            ));
        }
    } else if profiles.is_some_and(|profiles| !profiles.is_empty())
        || directions.is_some_and(|directions| !directions.is_empty())
    {
        failures.push(format!(
            "{id}: non-integrated target claims executable capabilities"
        ));
    }
}

fn audit_adapter_profile(
    target: &str,
    profile: &serde_json::Value,
    target_directions: Option<&Vec<serde_json::Value>>,
    failures: &mut Vec<String>,
) {
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
    let Some(directions) = profile["directions"].as_array() else {
        failures.push(format!("{name}: directions must be an array"));
        return;
    };
    if directions.is_empty() {
        failures.push(format!("{name}: directions must not be empty"));
    }
    let target_directions = target_directions
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .collect::<BTreeSet<_>>();
    let mut observed = BTreeSet::new();
    for direction in directions {
        let Some(direction) = direction.as_str() else {
            failures.push(format!("{name}: direction must be a string"));
            continue;
        };
        if !matches!(direction, "import" | "export" | "synchronize") {
            failures.push(format!("{name}: direction {direction:?} is not declared"));
        }
        if !target_directions.contains(direction) {
            failures.push(format!(
                "{name}: direction {direction:?} is absent from target {target}"
            ));
        }
        if !observed.insert(direction) {
            failures.push(format!("{name}: direction {direction:?} is duplicated"));
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
        || first["semantic_nodes"] != 14
        || first["file_menu_routes"].as_array().map(Vec::len) != Some(17)
        || first["operations"] != 10
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
            "package archive is absent or empty: {}",
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
    workspace_package_version("nuif-editor")
}

fn workspace_package_version(name: &str) -> Result<String, String> {
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
        .and_then(|packages| packages.iter().find(|package| package["name"] == name))
        .and_then(|package| package["version"].as_str())
        .map(str::to_owned)
        .ok_or_else(|| format!("cargo metadata did not contain the {name} version"))
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

    #[test]
    fn workflow_action_references_require_immutable_digests() {
        assert!(immutable_action_reference(
            "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1"
        ));
        assert!(immutable_action_reference(
            "docker://example.invalid/tool@sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        ));
        assert!(!immutable_action_reference("actions/checkout@v7"));
        assert!(!immutable_action_reference(
            "actions/checkout@owner@3d3c42e5aac5ba805825da76410c181273ba90b1"
        ));
    }

    #[test]
    fn workflow_artifact_paths_must_be_unique_per_upload() {
        let value = serde_json::json!({
            "uses": "actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a",
            "with": { "path": "target/report.json\ntarget/report.json\n" }
        });
        let mut references = Vec::new();
        let mut paths = 0;
        let mut failures = Vec::new();
        audit_workflow_value(
            &value,
            "fixture",
            0,
            &mut references,
            &mut paths,
            &mut failures,
        );
        assert_eq!(paths, 2);
        assert_eq!(failures.len(), 1);
    }

    #[test]
    fn adapter_profile_directions_must_refine_the_target_union() {
        let target_directions = vec![serde_json::json!("export")];
        let profile = serde_json::json!({
            "name": "nuif-web-accessibility-0",
            "directions": ["export"],
            "crate": "crates/nuif-html",
            "profile": "adapters/html-css/ACCESSIBILITY-PROFILE.md",
            "gate": "gate-accessibility"
        });
        let mut failures = Vec::new();
        audit_adapter_profile(
            "html-css",
            &profile,
            Some(&target_directions),
            &mut failures,
        );
        assert!(
            failures
                .iter()
                .all(|failure| !failure.contains("direction")),
            "{failures:?}"
        );

        let mut invalid = profile.clone();
        invalid["directions"] = serde_json::json!(["synchronize"]);
        audit_adapter_profile(
            "html-css",
            &invalid,
            Some(&target_directions),
            &mut failures,
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("absent from target"))
        );

        let target = serde_json::json!({
            "id": "html-css",
            "surface": "test",
            "status": "integrated",
            "research": "tree-sitter",
            "directions": ["export", "synchronize"],
            "profiles": [profile],
            "next_profile": "test",
            "boundary": "test"
        });
        let mut target_failures = Vec::new();
        audit_adapter_target(&target, &mut target_failures);
        assert!(
            target_failures
                .iter()
                .any(|failure| failure.contains("must equal the union"))
        );
    }
}
