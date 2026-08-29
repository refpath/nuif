use nuif_testing::layout_differential::{DifferentialConfig, run_and_write};
use std::env;
use std::path::PathBuf;

fn main() {
    if let Err(error) = run() {
        eprintln!("layout-differential: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut config = DifferentialConfig::default();
    let mut args = env::args().skip(1);
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--output" => {
                config.output = PathBuf::from(
                    args.next()
                        .ok_or_else(|| "--output requires a path".to_owned())?,
                );
            }
            "--chrome" => {
                config.chrome = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| "--chrome requires a path".to_owned())?,
                ));
            }
            "--allow-browser-version" => config.enforce_browser_version = false,
            unknown => return Err(format!("unknown argument: {unknown}")),
        }
    }
    let report = run_and_write(&config)?;
    println!(
        "layout differential: {} cases, {} comparisons, {} classified divergences, {} unexplained",
        report.summary.cases,
        report.summary.comparisons,
        report.summary.classified_divergences,
        report.summary.unclassified_divergences,
    );
    if report.passed() {
        Ok(())
    } else {
        Err(format!(
            "report failed; inspect {}",
            config.output.display()
        ))
    }
}
