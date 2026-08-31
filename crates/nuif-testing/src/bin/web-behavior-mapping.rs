use nuif_behavior::{
    ANNOUNCEMENT_CAPABILITY, BehaviorAction, BehaviorEffectKind, BehaviorRuntime, BehaviorValue,
    VISIBILITY_CAPABILITY, behavior_fixture,
};
use nuif_codec::canonical_hash;
use nuif_core::EntityId;
use nuif_html::behavior::{
    WEB_BEHAVIOR_PROFILE, WebBehaviorError, WebBehaviorProjection, project_web_behavior,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    if let Err(error) = run() {
        eprintln!("web-behavior-mapping: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let arguments = arguments()?;
    let (document, program, events) = behavior_fixture();
    let capabilities = BTreeSet::from([
        VISIBILITY_CAPABILITY.to_owned(),
        ANNOUNCEMENT_CAPABILITY.to_owned(),
    ]);
    let expected = BehaviorRuntime::new(&program, &document, &capabilities)
        .and_then(|mut runtime| runtime.run(&events))
        .map_err(|error| error.to_string())?;
    let projection =
        project_web_behavior(&document, &program).map_err(|error| error.to_string())?;
    let repeated = project_web_behavior(&document, &program).map_err(|error| error.to_string())?;
    let checks = evaluate_checks(&document, &program, &projection, &repeated)?;
    let passed = all_true(&checks);
    let expected_browser = json!({
        "schema_version": 1,
        "profile": WEB_BEHAVIOR_PROFILE,
        "source_profile": projection.source_profile,
        "event_sources": projection.event_sources,
        "events": events,
        "expected": expected,
    });
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:web-behavior-mapping",
        "status": if passed { "passed" } else { "failed" },
        "profile": WEB_BEHAVIOR_PROFILE,
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
        },
        "projection": {
            "source_profile": projection.source_profile,
            "html_sha256": format!("{:x}", Sha256::digest(projection.html.as_bytes())),
            "script_sha256": projection.script_sha256,
            "csp_script_source": projection.csp_script_source,
            "event_sources": projection.event_sources,
            "effect_capabilities": projection.effect_capabilities,
            "mappings": {
                "activate": "native button pointer click plus Enter and Space keyboard activation",
                "visibility": "HTMLElement.hidden",
                "announcement": "unfocused role=status live region text",
            },
            "non_claims": [
                "no behavior import from HTML and no arbitrary authored JavaScript",
                "no checkbox radio disabled-control or non-activate event lowering",
                "no timers navigation animation network filesystem or host business logic",
                "browser DOM and ARIA observations do not establish assistive-technology speech",
            ],
        },
        "checks": checks,
    });
    write(&arguments.html, projection.html.as_bytes())?;
    write(
        &arguments.expected,
        &serde_json::to_vec_pretty(&expected_browser).map_err(|error| error.to_string())?,
    )?;
    write(
        &arguments.report,
        &serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )?;
    if passed {
        println!(
            "web behavior mapping: {} events, {} native sources, status passed",
            events.len(),
            projection.event_sources.len()
        );
        Ok(())
    } else {
        Err("static web behavior mapping checks failed".to_owned())
    }
}

fn evaluate_checks(
    document: &nuif_core::Document,
    program: &nuif_behavior::BehaviorProgram,
    projection: &WebBehaviorProjection,
    repeated: &WebBehaviorProjection,
) -> Result<Value, String> {
    let script = projection
        .html
        .split_once("<script>")
        .and_then(|(_, rest)| rest.split_once("</script>"))
        .map(|(script, _)| script)
        .ok_or_else(|| "generated runtime script is missing".to_owned())?;
    Ok(json!({
        "projection_fixpoint": projection == repeated,
        "source_profile_retained": projection.source_profile == nuif_behavior::BEHAVIOR_PROFILE,
        "all_native_sources_bound": projection.event_sources == vec![EntityId::new(0x21), EntityId::new(0x23), EntityId::new(0x24)],
        "script_digest_exact": format!("{:x}", Sha256::digest(script.as_bytes())) == projection.script_sha256,
        "csp_is_hash_restricted": projection.html.contains(&format!("script-src '{}'", projection.csp_script_source))
            && !projection.html.contains("'unsafe-inline'")
            && !projection.html.contains("'unsafe-eval'"),
        "runtime_has_no_dynamic_authority": !script.contains("eval(")
            && !script.contains("Function(")
            && !script.contains("fetch(")
            && !script.contains("XMLHttpRequest")
            && !script.contains("WebSocket")
            && !script.contains("import("),
        "effects_have_native_targets": projection.html.contains("target.hidden = !action.value.value")
            && projection.html.contains("role=\"status\"")
            && projection.html.contains("aria-live=\"polite\"")
            && projection.html.contains("aria-atomic=\"true\""),
        "no_inline_handlers_or_external_urls": !projection.html.contains("onclick=")
            && !projection.html.contains("onkeydown=")
            && !projection.html.contains("http://")
            && !projection.html.contains("https://"),
        "host_divergence_is_typed": {
            "unsupported_source": unsupported_source_typed(document, program),
            "disabled_source": disabled_source_typed(document, program),
            "repeated_effect": repeated_effect_typed(document, program),
            "multiple_announcements": multiple_announcements_typed(document, program),
            "empty_announcement": empty_announcement_typed(document, program),
            "hidden_event_source": hidden_event_source_typed(document, program),
        },
        "embedded_data_is_script_safe": embedded_data_is_script_safe(document, program),
    }))
}

fn unsupported_source_typed(
    document: &nuif_core::Document,
    program: &nuif_behavior::BehaviorProgram,
) -> bool {
    let mut document = document.clone();
    document
        .entities
        .get_mut(&EntityId::new(0x21))
        .unwrap()
        .semantics
        .role = Some("checkbox".to_owned());
    matches!(
        project_web_behavior(&document, program),
        Err(WebBehaviorError::UnsupportedEventSource { .. })
    )
}

fn disabled_source_typed(
    document: &nuif_core::Document,
    program: &nuif_behavior::BehaviorProgram,
) -> bool {
    let mut document = document.clone();
    document
        .entities
        .get_mut(&EntityId::new(0x21))
        .unwrap()
        .semantics
        .states
        .insert("disabled".to_owned(), true);
    matches!(
        project_web_behavior(&document, program),
        Err(WebBehaviorError::DisabledEventSource { .. })
    )
}

fn repeated_effect_typed(
    document: &nuif_core::Document,
    program: &nuif_behavior::BehaviorProgram,
) -> bool {
    let mut program = program.clone();
    let transition = &mut program.states.get_mut("closed").unwrap().transitions[2];
    transition.actions.push(transition.actions[1].clone());
    matches!(
        project_web_behavior(document, &program),
        Err(WebBehaviorError::RepeatedEffect { .. })
    )
}

fn multiple_announcements_typed(
    document: &nuif_core::Document,
    program: &nuif_behavior::BehaviorProgram,
) -> bool {
    let mut program = program.clone();
    program.states.get_mut("closed").unwrap().transitions[2]
        .actions
        .push(BehaviorAction::Emit {
            effect: BehaviorEffectKind::Announcement,
            target: EntityId::new(0x20),
            value: BehaviorValue::String("Ready".to_owned()),
        });
    matches!(
        project_web_behavior(document, &program),
        Err(WebBehaviorError::MultipleAnnouncements { .. })
    )
}

fn empty_announcement_typed(
    document: &nuif_core::Document,
    program: &nuif_behavior::BehaviorProgram,
) -> bool {
    let mut program = program.clone();
    let BehaviorAction::Emit { value, .. } =
        &mut program.states.get_mut("closed").unwrap().transitions[1].actions[0]
    else {
        return false;
    };
    *value = BehaviorValue::String(" \n ".to_owned());
    matches!(
        project_web_behavior(document, &program),
        Err(WebBehaviorError::EmptyAnnouncement { .. })
    )
}

fn embedded_data_is_script_safe(
    document: &nuif_core::Document,
    program: &nuif_behavior::BehaviorProgram,
) -> bool {
    let mut program = program.clone();
    let BehaviorAction::Emit { value, .. } =
        &mut program.states.get_mut("closed").unwrap().transitions[1].actions[0]
    else {
        return false;
    };
    *value = BehaviorValue::String(
        "</script><script>globalThis.nuifInjected=true</script>\u{2028}".to_owned(),
    );
    project_web_behavior(document, &program).is_ok_and(|projection| {
        !projection
            .html
            .contains("</script><script>globalThis.nuifInjected")
            && projection.html.contains("\\u003c/script\\u003e")
            && projection.html.contains("\\u2028")
            && projection.html.matches("<script>").count() == 1
    })
}

fn hidden_event_source_typed(
    document: &nuif_core::Document,
    program: &nuif_behavior::BehaviorProgram,
) -> bool {
    let mut program = program.clone();
    program.states.get_mut("open").unwrap().transitions[0]
        .actions
        .push(BehaviorAction::Emit {
            effect: BehaviorEffectKind::Visibility,
            target: EntityId::new(0x20),
            value: BehaviorValue::Boolean(false),
        });
    matches!(
        project_web_behavior(document, &program),
        Err(WebBehaviorError::VisibilityHidesEventSource { .. })
    )
}

fn all_true(value: &Value) -> bool {
    match value {
        Value::Bool(value) => *value,
        Value::Object(values) => values.values().all(all_true),
        _ => false,
    }
}

struct Arguments {
    report: PathBuf,
    html: PathBuf,
    expected: PathBuf,
}

fn arguments() -> Result<Arguments, String> {
    let mut values = env::args().skip(1);
    let mut report = PathBuf::from("target/web-behavior-static-report.json");
    let mut html = PathBuf::from("target/web-behavior-fixture.html");
    let mut expected = PathBuf::from("target/web-behavior-expected.json");
    while let Some(argument) = values.next() {
        let target = match argument.as_str() {
            "--report" => &mut report,
            "--html" => &mut html,
            "--expected" => &mut expected,
            "--help" | "-h" => {
                return Err(
                    "usage: web-behavior-mapping [--report <json>] [--html <html>] [--expected <json>]"
                        .to_owned(),
                );
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
