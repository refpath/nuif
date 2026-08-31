use std::collections::BTreeSet;
use std::fs;

const REGISTRY: &str = "docs/DIAGNOSTICS.md";
const SOURCES: &[&str] = &[
    "crates/nuif-core/src/lib.rs",
    "crates/nuif-layout/src/lib.rs",
    "crates/nuif-testing/src/lib.rs",
];
const PREFIXES: &[&str] = &[
    "ASSET_",
    "COLOR_",
    "EXTENSION_",
    "GRID_",
    "IMAGE_",
    "LAYOUT_",
    "MODEL_",
    "RESOURCE_",
    "SNAPSHOT_",
    "TEXT_",
    "TRIAL_",
    "UNKNOWN_",
    "VALIDATION_",
];

pub fn audit() -> Result<(), String> {
    let registry = fs::read_to_string(REGISTRY).map_err(|error| error.to_string())?;
    let (registered, mut failures) = parse_registry(&registry);
    let emitted = emitted_codes(&mut failures);
    let missing = emitted.difference(&registered).cloned().collect::<Vec<_>>();
    let stale = registered.difference(&emitted).cloned().collect::<Vec<_>>();
    if !missing.is_empty() {
        failures.push(format!(
            "emitted diagnostic codes absent from the registry: {}",
            missing.join(", ")
        ));
    }
    if !stale.is_empty() {
        failures.push(format!(
            "registered diagnostic codes absent from their owning sources: {}",
            stale.join(", ")
        ));
    }
    let report = serde_json::json!({
        "schema_version": 1,
        "status": if failures.is_empty() { "passed" } else { "failed" },
        "source": {
            "revision": super::command_text("git", &["rev-parse", "HEAD"]),
            "dirty": super::command_text("git", &["status", "--porcelain"])
                .map(|value| !value.is_empty()),
        },
        "registry": REGISTRY,
        "sources": SOURCES,
        "registered_codes": registered,
        "emitted_codes": emitted,
        "summary": {
            "registered": registered.len(),
            "emitted": emitted.len(),
            "blocking_failures": failures.len(),
        },
        "failures": failures,
    });
    fs::create_dir_all("target").map_err(|error| error.to_string())?;
    fs::write(
        "target/diagnostic-registry-report.json",
        serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    if report["status"] == "passed" {
        Ok(())
    } else {
        Err(
            "diagnostic registry audit failed; inspect target/diagnostic-registry-report.json"
                .to_owned(),
        )
    }
}

fn parse_registry(markdown: &str) -> (BTreeSet<String>, Vec<String>) {
    let mut codes = Vec::new();
    let mut failures = Vec::new();
    for (line_number, line) in markdown.lines().enumerate() {
        let Some(rest) = line.strip_prefix("| `") else {
            continue;
        };
        let Some((code, fields)) = rest.split_once("` |") else {
            failures.push(format!("registry line {} is malformed", line_number + 1));
            continue;
        };
        let fields = fields
            .split('|')
            .map(str::trim)
            .filter(|field| !field.is_empty())
            .collect::<Vec<_>>();
        if !is_code(code) || fields.len() != 4 {
            failures.push(format!(
                "registry line {} requires a code plus four fields",
                line_number + 1
            ));
            continue;
        }
        if !matches!(fields[0], "error" | "warning" | "information" | "hint") {
            failures.push(format!(
                "registry line {} has unknown severity {:?}",
                line_number + 1,
                fields[0]
            ));
        }
        codes.push(code.to_owned());
    }
    if codes.is_empty() {
        failures.push("diagnostic registry contains no code rows".to_owned());
    }
    if !codes.windows(2).all(|pair| pair[0] < pair[1]) {
        failures.push("diagnostic registry codes must be unique and bytewise sorted".to_owned());
    }
    (codes.into_iter().collect(), failures)
}

fn emitted_codes(failures: &mut Vec<String>) -> BTreeSet<String> {
    let mut codes = BTreeSet::new();
    for source in SOURCES {
        let contents = match fs::read_to_string(source) {
            Ok(contents) => contents,
            Err(error) => {
                failures.push(format!("cannot read diagnostic source {source}: {error}"));
                continue;
            }
        };
        for literal in quoted_literals(&contents) {
            if is_code(literal) && PREFIXES.iter().any(|prefix| literal.starts_with(prefix)) {
                codes.insert(literal.to_owned());
            }
        }
    }
    codes
}

fn quoted_literals(source: &str) -> Vec<&str> {
    let bytes = source.as_bytes();
    let mut literals = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'"' {
            index += 1;
            continue;
        }
        let start = index + 1;
        index = start;
        let mut escaped = false;
        while index < bytes.len() {
            let byte = bytes[index];
            if byte == b'"' && !escaped {
                literals.push(&source[start..index]);
                index += 1;
                break;
            }
            escaped = byte == b'\\' && !escaped;
            if byte != b'\\' {
                escaped = false;
            }
            index += 1;
        }
    }
    literals
}

fn is_code(value: &str) -> bool {
    value.contains('_')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_rows_are_strict_and_sorted() {
        let markdown = "| `ALPHA_CODE` | error | model | core | first |\n| `BETA_CODE` | warning | layout | layout | second |\n";
        let (codes, failures) = parse_registry(markdown);
        assert!(failures.is_empty());
        assert_eq!(codes.len(), 2);
    }

    #[test]
    fn literal_scanner_ignores_lowercase_and_keeps_public_codes() {
        assert_eq!(
            quoted_literals(r#"let a = "MODEL_FAILED"; let b = "not-a-code";"#),
            ["MODEL_FAILED", "not-a-code"]
        );
        assert!(is_code("MODEL_FAILED"));
        assert!(!is_code("not-a-code"));
    }
}
