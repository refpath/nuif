use nuif_codec::canonical_hash;
use nuif_core::{EntityId, Relation, validate};
use nuif_html::accessibility::{
    WEB_ACCESSIBILITY_PROFILE, WebAccessibilityError, WebAccessibilityProjection,
    project_web_accessibility, web_accessibility_fixture,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    if let Err(error) = run() {
        eprintln!("accessibility-mapping: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let arguments = arguments()?;
    let document = web_accessibility_fixture();
    let projection = project_web_accessibility(&document).map_err(|error| error.to_string())?;
    let repeated = project_web_accessibility(&document).map_err(|error| error.to_string())?;
    let checks = evaluate_checks(&document, &projection, &repeated)?;
    let passed = checks
        .as_object()
        .is_some_and(|checks| checks.values().all(|value| value == true));
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:accessibility-mapping",
        "status": if passed { "passed" } else { "failed" },
        "profile": WEB_ACCESSIBILITY_PROFILE,
        "source": {
            "revision": command_text("git", &["rev-parse", "HEAD"]),
            "dirty": command_text("git", &["status", "--porcelain"]).map(|value| !value.is_empty()),
            "toolchain": command_text("rustc", &["--version"]),
            "os": env::consts::OS,
            "architecture": env::consts::ARCH,
        },
        "document": {
            "canonical_hash": canonical_hash(&document).map_err(|error| error.to_string())?,
            "entities": document.entities.len(),
            "relations": document.relations.len(),
        },
        "projection": {
            "html_sha256": format!("{:x}", Sha256::digest(projection.html.as_bytes())),
            "semantic_nodes": projection.nodes.len(),
            "roles": projection.nodes.iter().map(|node| node.role.as_str()).collect::<Vec<_>>(),
            "mapped_relations": ["labelled-by", "described-by", "controls", "owns", "flow-to"],
            "non_claims": [
                "no application behavior or arbitrary script synthesis",
                "no native platform accessibility API equivalence",
                "no role state or relationship outside nuif-web-accessibility-0",
            ],
        },
        "checks": checks,
    });
    write(&arguments.html, projection.html.as_bytes())?;
    write(
        &arguments.expected,
        &serde_json::to_vec_pretty(&projection).map_err(|error| error.to_string())?,
    )?;
    write(
        &arguments.report,
        &serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )?;
    if passed {
        println!(
            "accessibility mapping: {} semantic nodes, status passed",
            projection.nodes.len()
        );
        Ok(())
    } else {
        Err("static accessibility mapping checks failed".to_owned())
    }
}

fn evaluate_checks(
    document: &nuif_core::Document,
    projection: &WebAccessibilityProjection,
    repeated: &WebAccessibilityProjection,
) -> Result<serde_json::Value, String> {
    let button = projection
        .nodes
        .iter()
        .find(|node| node.entity == EntityId::new(0x13))
        .ok_or_else(|| "button projection is absent".to_owned())?;
    let checkbox = projection
        .nodes
        .iter()
        .find(|node| node.entity == EntityId::new(0x12))
        .ok_or_else(|| "checkbox projection is absent".to_owned())?;
    let roles = projection
        .nodes
        .iter()
        .map(|node| node.role.as_str())
        .collect::<BTreeSet<_>>();
    let states = projection
        .nodes
        .iter()
        .flat_map(|node| node.states.keys().map(String::as_str))
        .collect::<BTreeSet<_>>();
    let relationships = projection
        .nodes
        .iter()
        .flat_map(|node| node.relationships.keys().map(String::as_str))
        .collect::<BTreeSet<_>>();
    Ok(json!({
        "fixture_valid": validate(document).iter().all(|diagnostic| diagnostic.severity != nuif_core::Severity::Error),
        "projection_fixpoint": projection == repeated,
        "all_semantic_nodes_projected": projection.nodes.len() == document.entities.len(),
        "all_profile_roles_covered": roles == BTreeSet::from(["button", "checkbox", "group", "img", "main", "navigation", "paragraph", "radio", "region", "switch"]),
        "all_profile_states_covered": states == BTreeSet::from(["checked", "disabled", "expanded", "pressed", "required"]),
        "all_profile_relationships_covered": relationships == BTreeSet::from(["controls", "described-by", "flow-to", "labelled-by", "owns"]),
        "labelled_by_name_computed": checkbox.accessible_name.as_deref() == Some("Receive updates"),
        "button_relationships_retained": button.relationships.get("controls") == Some(&vec![EntityId::new(0x14)])
            && button.relationships.get("described-by") == Some(&vec![EntityId::new(0x17)]),
        "native_and_aria_states_emitted": projection.html.contains(" checked")
            && projection.html.contains(" required")
            && projection.html.contains(" disabled")
            && projection.html.contains("aria-expanded=\"true\"")
            && projection.html.contains("aria-pressed=\"false\"")
            && projection.html.contains("aria-checked=\"true\""),
        "source_is_inert": !projection.html.contains("<script")
            && !projection.html.contains("http://")
            && !projection.html.contains("https://"),
        "unsupported_role_typed": unsupported_role_typed(document),
        "unsupported_state_typed": unsupported_state_typed(document),
        "ambiguous_name_typed": ambiguous_name_typed(document),
        "missing_name_typed": missing_name_typed(document),
        "empty_name_typed": empty_name_typed(document),
        "prohibited_name_typed": prohibited_name_typed(document),
        "unnamed_label_typed": unnamed_label_typed(document),
        "duplicate_relation_typed": duplicate_relation_typed(document),
        "owned_target_conflict_typed": owned_target_conflict_typed(document),
        "owned_tree_cycle_typed": owned_tree_cycle_typed(document),
        "relation_source_without_role_typed": relation_source_without_role_typed(document),
        "invalid_containment_typed": invalid_containment_typed(document),
        "switch_checked_required": switch_checked_required(document),
        "unsupported_relation_typed": unsupported_relation_typed(document),
    }))
}

fn unsupported_role_typed(document: &nuif_core::Document) -> bool {
    let mut document = document.clone();
    document
        .entities
        .get_mut(&EntityId::new(0x13))
        .unwrap()
        .semantics
        .role = Some("dialog".to_owned());
    matches!(
        project_web_accessibility(&document),
        Err(WebAccessibilityError::UnsupportedRole { .. })
    )
}

fn unsupported_state_typed(document: &nuif_core::Document) -> bool {
    let mut document = document.clone();
    document
        .entities
        .get_mut(&EntityId::new(0x13))
        .unwrap()
        .semantics
        .states
        .insert("checked".to_owned(), true);
    matches!(
        project_web_accessibility(&document),
        Err(WebAccessibilityError::UnsupportedState { .. })
    )
}

fn ambiguous_name_typed(document: &nuif_core::Document) -> bool {
    let mut document = document.clone();
    document.relations.push(Relation {
        kind: "labelled-by".to_owned(),
        source: EntityId::new(0x13),
        target: EntityId::new(0x11),
    });
    matches!(
        project_web_accessibility(&document),
        Err(WebAccessibilityError::AmbiguousName { .. })
    )
}

fn unsupported_relation_typed(document: &nuif_core::Document) -> bool {
    let mut document = document.clone();
    document.relations.push(Relation {
        kind: "activates".to_owned(),
        source: EntityId::new(0x13),
        target: EntityId::new(0x14),
    });
    matches!(
        project_web_accessibility(&document),
        Err(WebAccessibilityError::UnsupportedRelation { .. })
    )
}

fn missing_name_typed(document: &nuif_core::Document) -> bool {
    let mut document = document.clone();
    document
        .relations
        .retain(|relation| relation.kind != "labelled-by");
    matches!(
        project_web_accessibility(&document),
        Err(WebAccessibilityError::MissingName { .. })
    )
}

fn empty_name_typed(document: &nuif_core::Document) -> bool {
    let mut document = document.clone();
    document
        .entities
        .get_mut(&EntityId::new(0x13))
        .unwrap()
        .semantics
        .accessible_name = Some(" \n\t ".to_owned());
    matches!(
        project_web_accessibility(&document),
        Err(WebAccessibilityError::EmptyName { .. })
    )
}

fn prohibited_name_typed(document: &nuif_core::Document) -> bool {
    let mut document = document.clone();
    document
        .entities
        .get_mut(&EntityId::new(0x11))
        .unwrap()
        .semantics
        .accessible_name = Some("Paragraph".to_owned());
    matches!(
        project_web_accessibility(&document),
        Err(WebAccessibilityError::ProhibitedName { .. })
    )
}

fn unnamed_label_typed(document: &nuif_core::Document) -> bool {
    let mut document = document.clone();
    document
        .entities
        .get_mut(&EntityId::new(0x11))
        .unwrap()
        .authored
        .text = None;
    matches!(
        project_web_accessibility(&document),
        Err(WebAccessibilityError::UnnamedLabelTarget { .. })
    )
}

fn duplicate_relation_typed(document: &nuif_core::Document) -> bool {
    let mut document = document.clone();
    document.relations.push(document.relations[0].clone());
    matches!(
        project_web_accessibility(&document),
        Err(WebAccessibilityError::DuplicateRelation { .. })
    )
}

fn owned_target_conflict_typed(document: &nuif_core::Document) -> bool {
    let mut document = document.clone();
    document.relations.push(Relation {
        kind: "owns".to_owned(),
        source: EntityId::new(0x10),
        target: EntityId::new(0x1a),
    });
    matches!(
        project_web_accessibility(&document),
        Err(WebAccessibilityError::OwnedTargetConflict { .. })
    )
}

fn owned_tree_cycle_typed(document: &nuif_core::Document) -> bool {
    let mut document = document.clone();
    document.relations.push(Relation {
        kind: "owns".to_owned(),
        source: EntityId::new(0x1a),
        target: EntityId::new(0x19),
    });
    matches!(
        project_web_accessibility(&document),
        Err(WebAccessibilityError::OwnedTreeCycle { .. })
    )
}

fn relation_source_without_role_typed(document: &nuif_core::Document) -> bool {
    let mut document = document.clone();
    document
        .entities
        .get_mut(&EntityId::new(0x18))
        .unwrap()
        .semantics
        .role = None;
    matches!(
        project_web_accessibility(&document),
        Err(WebAccessibilityError::RelationSourceWithoutRole { .. })
    )
}

fn invalid_containment_typed(document: &nuif_core::Document) -> bool {
    let mut document = document.clone();
    document
        .entities
        .get_mut(&EntityId::new(0x10))
        .unwrap()
        .children
        .retain(|child| *child != EntityId::new(0x11));
    document
        .entities
        .get_mut(&EntityId::new(0x12))
        .unwrap()
        .children
        .push(EntityId::new(0x11));
    matches!(
        project_web_accessibility(&document),
        Err(WebAccessibilityError::InvalidContainment { .. })
    )
}

fn switch_checked_required(document: &nuif_core::Document) -> bool {
    let mut document = document.clone();
    document
        .entities
        .get_mut(&EntityId::new(0x15))
        .unwrap()
        .semantics
        .states
        .remove("checked");
    matches!(
        project_web_accessibility(&document),
        Err(WebAccessibilityError::UnsupportedState { state, .. }) if state == "checked-required"
    )
}

struct Arguments {
    report: PathBuf,
    html: PathBuf,
    expected: PathBuf,
}

fn arguments() -> Result<Arguments, String> {
    let mut values = env::args().skip(1);
    let mut report = PathBuf::from("target/accessibility-mapping-static-report.json");
    let mut html = PathBuf::from("target/accessibility-mapping-fixture.html");
    let mut expected = PathBuf::from("target/accessibility-mapping-expected.json");
    while let Some(argument) = values.next() {
        let target = match argument.as_str() {
            "--report" => &mut report,
            "--html" => &mut html,
            "--expected" => &mut expected,
            "--help" | "-h" => {
                return Err("usage: accessibility-mapping [--report <json>] [--html <html>] [--expected <json>]".to_owned());
            }
            unknown => return Err(format!("unknown argument {unknown:?}")),
        };
        *target = values
            .next()
            .ok_or_else(|| format!("{argument} requires a path"))?
            .into();
    }
    Ok(Arguments {
        report,
        html,
        expected,
    })
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut bytes = bytes.to_vec();
    if path
        .extension()
        .is_some_and(|extension| extension == "json")
    {
        bytes.push(b'\n');
    }
    fs::write(path, bytes).map_err(|error| error.to_string())
}

fn command_text(program: &str, arguments: &[&str]) -> Option<String> {
    let output = Command::new(program).args(arguments).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
