//! Generated source-synchronization corpus; not an external interoperability study.

use nuif_codec::canonical_hash;
use nuif_core::{Document, EntityId, FlowDirection};
use nuif_html::{AdapterError, RetentiveSource, SourceEdit};
use serde_json::{Value, json};
use std::{env, fs, path::Path, process::Command};

fn main() {
    if let Err(error) = run() {
        eprintln!("source-workflow: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut cases = Vec::new();
    for full_v0 in [false, true] {
        for grouped in [false, true] {
            for count in [1, 8, 64, 256] {
                for variant in 0..3 {
                    cases.push(evaluate(full_v0, grouped, count, variant).map_err(|error| format!("full_v0={full_v0}, grouped={grouped}, count={count}, variant={variant}: {error}"))?);
                }
            }
        }
    }
    let failures = cases
        .iter()
        .filter(|case| case["status"] != "passed")
        .count();
    let report = json!({
        "schema_version": 1,
        "status": if failures == 0 { "passed" } else { "failed" },
        "method": "deterministic generated corpus with metamorphic checks",
        "source": {
            "revision": command_text("git", &["rev-parse", "HEAD"]),
            "dirty": command_text("git", &["status", "--porcelain"]).map(|value| !value.is_empty()),
            "toolchain": command_text("rustc", &["--version"]),
            "os": env::consts::OS, "architecture": env::consts::ARCH,
        },
        "limitations": [
            "fixtures are generated from the reference model and are not independently authored",
            "round-trip agreement does not establish independent semantic correctness",
            "retained CSS and HTML may affect browser rendering; visual equivalence is not measured",
            "no native usability, live design-host or arbitrary-source claim"
        ],
        "summary": { "cases": cases.len(), "blocking_failures": failures },
        "cases": cases,
    });
    let path = Path::new("target/source-workflow-report.json");
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(path, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "generated source workflow: {} cases, {failures} failures",
        report["summary"]["cases"]
    );
    if failures != 0 {
        return Err("source workflow checks failed".into());
    }
    Ok(())
}

fn evaluate(
    full_v0: bool,
    grouped: bool,
    count: usize,
    variant: usize,
) -> Result<Value, Box<dyn std::error::Error>> {
    let before = fixture(grouped, count);
    let export = |document: &Document| {
        if full_v0 {
            nuif_html::export_v0_document(document)
        } else {
            nuif_html::export_document(document)
        }
    };
    let import = |source: &str| {
        if full_v0 {
            nuif_html::import_v0_source(source)
        } else {
            nuif_html::import_source(source)
        }
    };
    let sync = |source: &RetentiveSource, document: &Document| {
        if full_v0 {
            nuif_html::synchronize_v0(source, document)
        } else {
            nuif_html::synchronize(source, document)
        }
    };
    let exported = export(&before)?;
    let source = retained_variant(&exported.source, variant);
    let imported = import(&source)?;
    let noop = sync(&imported.retentive, &before)?;
    let after = edited(&before, 1);
    let synchronized = sync(&imported.retentive, &after)?;
    let repeated = sync(&imported.retentive, &after)?;
    let reimported = import(&synchronized.source)?;
    let fixpoint = sync(&reimported.retentive, &after)?;
    let second_document = edited(&after, 2);
    let second = sync(&reimported.retentive, &second_document)?;
    let mut structural = after.clone();
    let mut added = structural.entities[&EntityId::new(0x1000)].clone();
    added.id = EntityId::new(0x90000);
    structural
        .entities
        .get_mut(&structural.roots[0])
        .unwrap()
        .children
        .push(added.id);
    structural.entities.insert(added.id, added);
    let mut stale: RetentiveSource = imported.retentive.clone();
    stale.report.correspondences[0].span.start = usize::MAX;
    let checks = json!({
        "import_exact": imported.document == before,
        "noop_source_exact": noop.source == source && noop.edits.is_empty(),
        "synchronized_import_exact": reimported.document == after,
        "repeat_exact": repeated.source == synchronized.source && repeated.edits == synchronized.edits,
        "fixpoint_exact": fixpoint.source == synchronized.source && fixpoint.edits.is_empty(),
        "unmapped_bytes_exact": unchanged_outside_edits(&source, &synchronized.source, &synchronized.edits),
        "second_edit_import_exact": import(&second.source)?.document == second_document,
        "second_edit_locality": unchanged_outside_edits(&synchronized.source, &second.source, &second.edits),
        "mapped_edits_present": synchronized.edits.len() > count,
        "structural_edit_refused": matches!(sync(&imported.retentive, &structural), Err(AdapterError::UnmappedChanges { .. })),
        "stale_span_refused": matches!(sync(&stale, &after), Err(AdapterError::StaleSpan { .. })),
    });
    let passed = checks
        .as_object()
        .unwrap()
        .values()
        .all(|value| value == true);
    Ok(json!({
        "profile": if full_v0 { nuif_html::V0_PROFILE_NAME } else { nuif_html::PROFILE_NAME },
        "family": if grouped { "groups-of-eight" } else { "siblings" },
        "text_entities": count,
        "source_variant": (["canonical", "foreign-regions-lf", "foreign-regions-crlf"][variant]),
        "source_bytes": source.len(), "entities": before.entities.len(),
        "mapped_edits": synchronized.edits.len(),
        "before_hash": canonical_hash(&before)?, "after_hash": canonical_hash(&after)?,
        "status": if passed { "passed" } else { "failed" }, "checks": checks,
    }))
}

fn fixture(grouped: bool, count: usize) -> Document {
    let mut document = nuif_html::profile_fixture();
    let root_id = document.roots[0];
    let template = document.entities.remove(&EntityId::new(0x11)).unwrap();
    document
        .entities
        .get_mut(&root_id)
        .unwrap()
        .children
        .clear();
    let container = document.entities[&root_id].clone();
    let samples = [
        "",
        "Ångström Ελληνικά 漢字",
        "<&>\"'",
        "🙂 e\u{301}",
        "אבג العربية",
        "  spaced  ",
    ];
    for index in 0..count {
        let parent = if grouped {
            let group_id = EntityId::new(0x10000 + (index / 8) as u128);
            if index % 8 == 0 {
                let mut group = container.clone();
                group.id = group_id;
                group.name = Some(format!("group {}", index / 8));
                group.authored.layout.direction = FlowDirection::Row;
                document.entities.insert(group_id, group);
                document
                    .entities
                    .get_mut(&root_id)
                    .unwrap()
                    .children
                    .push(group_id);
            }
            group_id
        } else {
            root_id
        };
        let mut text = template.clone();
        text.id = EntityId::new(0x1000 + index as u128);
        text.name = Some(format!("text {index}"));
        samples[index % samples.len()]
            .clone_into(&mut text.authored.text.as_mut().unwrap().content);
        document
            .entities
            .get_mut(&parent)
            .unwrap()
            .children
            .push(text.id);
        document.entities.insert(text.id, text);
    }
    document
}

fn edited(before: &Document, round: usize) -> Document {
    let mut after = before.clone();
    after
        .entities
        .get_mut(&after.roots[0])
        .unwrap()
        .authored
        .layout
        .gap += 0.25;
    for entity in after.entities.values_mut() {
        if let Some(text) = &mut entity.authored.text {
            text.content = if round == 1 {
                format!("{} <&> β", text.content)
            } else {
                "終".to_owned()
            };
        }
    }
    after
}

fn retained_variant(source: &str, variant: usize) -> String {
    if variant == 0 {
        return source.to_owned();
    }
    let source = source
        .replace(
            "</style>",
            "/* retained CSS: α🙂 */\naside[data-user] { color: teal; }\n</style>",
        )
        .replace(
            "</body>",
            "<!-- retained HTML: & untouched -->\n<aside data-user hidden title='retained'>foreign</aside>\n</body>",
        );
    if variant == 2 {
        source.replace('\n', "\r\n")
    } else {
        source
    }
}

fn unchanged_outside_edits(before: &str, after: &str, edits: &[SourceEdit]) -> bool {
    let (mut old, mut new) = (0, 0);
    for edit in edits {
        let unchanged = &before[old..edit.span.start];
        if after.get(new..new + unchanged.len()) != Some(unchanged) {
            return false;
        }
        new += unchanged.len();
        if after.get(new..new + edit.replacement.len()) != Some(edit.replacement.as_str()) {
            return false;
        }
        new += edit.replacement.len();
        old = edit.span.end;
    }
    after.get(new..) == Some(&before[old..])
}

fn command_text(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
