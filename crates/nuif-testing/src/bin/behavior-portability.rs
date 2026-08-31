use nuif_behavior::{
    BEHAVIOR_PROFILE, BehaviorAction, BehaviorError, BehaviorRuntime, BehaviorState, BehaviorValue,
    CapabilityPolicy, MAX_ACTIONS_PER_TRANSITION, MAX_EVENTS_PER_RUN, MAX_STRING_BYTES,
    behavior_fixture, validate_program,
};
use nuif_codec::canonical_hash;
use nuif_core::EntityId;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const VISIBILITY: &str = "effect.visibility";
const ANNOUNCEMENT: &str = "effect.announcement";

fn main() {
    if let Err(error) = run() {
        eprintln!("behavior-portability: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let arguments = arguments()?;
    let (document, program, events) = behavior_fixture();
    let all_capabilities = BTreeSet::from([VISIBILITY.to_owned(), ANNOUNCEMENT.to_owned()]);
    let required_only = BTreeSet::from([VISIBILITY.to_owned()]);
    let full = execute(&program, &document, &events, &all_capabilities)?;
    let repeated = execute(&program, &document, &events, &all_capabilities)?;
    let degraded = execute(&program, &document, &events, &required_only)?;
    let checks = evaluate_checks(&document, &program, &events, &full, &repeated, &degraded);
    let passed = all_true(&checks);
    let fixture = json!({
        "schema_version": 1,
        "profile": BEHAVIOR_PROFILE,
        "document": {
            "entities": document.entities.values().map(|entity| json!({
                "id": entity.id,
                "role": entity.semantics.role,
            })).collect::<Vec<_>>(),
        },
        "program": program,
        "events": events,
        "runs": [
            {
                "name": "all-capabilities",
                "capabilities": all_capabilities,
                "expected": full,
            },
            {
                "name": "required-only",
                "capabilities": required_only,
                "expected": degraded,
            },
        ],
        "missing_required": {
            "capabilities": [],
            "expected_error": "missing-required-capability",
        },
    });
    let fixture_bytes = serde_json::to_vec_pretty(&fixture).map_err(|error| error.to_string())?;
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:behavior-portability",
        "status": if passed { "passed" } else { "failed" },
        "profile": BEHAVIOR_PROFILE,
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
        "program": {
            "states": program.states.len(),
            "transitions": program.states.values().map(|state| state.transitions.len()).sum::<usize>(),
            "variables": program.variables.len(),
            "capabilities": program.capabilities,
            "fixture_sha256": format!("{:x}", Sha256::digest(&fixture_bytes)),
            "non_claims": [
                "the behavior sidecar is not yet part of the canonical NUIF wire model",
                "no timers internal event queue networking navigation animation or arbitrary script",
                "foreign trace agreement does not prove browser DOM or native UI behavior",
            ],
        },
        "checks": checks,
    });
    write_json(&arguments.fixture, &fixture_bytes)?;
    write_json(
        &arguments.report,
        &serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )?;
    if passed {
        println!(
            "behavior portability: {} events, {} states, status passed",
            events.len(),
            program.states.len()
        );
        Ok(())
    } else {
        Err("static behavior portability checks failed".to_owned())
    }
}

fn execute(
    program: &nuif_behavior::BehaviorProgram,
    document: &nuif_core::Document,
    events: &[nuif_behavior::BehaviorEvent],
    capabilities: &BTreeSet<String>,
) -> Result<nuif_behavior::BehaviorRun, String> {
    BehaviorRuntime::new(program, document, capabilities)
        .and_then(|mut runtime| runtime.run(events))
        .map_err(|error| error.to_string())
}

fn evaluate_checks(
    document: &nuif_core::Document,
    program: &nuif_behavior::BehaviorProgram,
    events: &[nuif_behavior::BehaviorEvent],
    full: &nuif_behavior::BehaviorRun,
    repeated: &nuif_behavior::BehaviorRun,
    degraded: &nuif_behavior::BehaviorRun,
) -> Value {
    let full_effects = full.traces.iter().flat_map(|trace| &trace.effects).count();
    let degraded_effects = degraded
        .traces
        .iter()
        .flat_map(|trace| &trace.effects)
        .count();
    let skipped = degraded
        .traces
        .iter()
        .flat_map(|trace| &trace.skipped_optional)
        .count();
    json!({
        "program_valid": validate_program(program, document).is_ok(),
        "reference_fixpoint": full == repeated,
        "all_events_traced": full.traces.len() == events.len(),
        "ordered_guards_select_expected_branch": full.traces[0].transition.as_deref() == Some("open-panel")
            && full.traces[3].transition.as_deref() == Some("blocked-open"),
        "unmatched_event_is_noop": full.traces[4].transition.is_none()
            && full.traces[4].from_state == full.traces[4].to_state,
        "actions_run_in_order": full.variables.get("seen") == Some(&BehaviorValue::Boolean(true))
            && full.final_state == "closed",
        "optional_capability_is_explicit_noop": skipped == 3 && full_effects == 5 && degraded_effects == 2,
        "required_capability_fails_before_execution": missing_required_typed(program, document),
        "profile_failures_are_typed": typed_failures(program, document),
    })
}

fn missing_required_typed(
    program: &nuif_behavior::BehaviorProgram,
    document: &nuif_core::Document,
) -> bool {
    matches!(
        BehaviorRuntime::new(program, document, &BTreeSet::new()),
        Err(BehaviorError::MissingRequiredCapability { capability }) if capability == VISIBILITY
    )
}

fn typed_failures(
    program: &nuif_behavior::BehaviorProgram,
    document: &nuif_core::Document,
) -> Value {
    json!({
        "unsupported_schema": invalid_program(program, document, |program| program.schema_version = 2, |error| matches!(error, BehaviorError::UnsupportedSchema { .. })),
        "unsupported_profile": invalid_program(program, document, |program| "vendor.behavior".clone_into(&mut program.profile), |error| matches!(error, BehaviorError::UnsupportedProfile { .. })),
        "invalid_identifier": invalid_program(program, document, |program| "Bad State".clone_into(&mut program.initial_state), |error| matches!(error, BehaviorError::InvalidIdentifier { .. })),
        "empty_state_set": invalid_program(program, document, |program| program.states.clear(), |error| matches!(error, BehaviorError::EmptyStateSet)),
        "missing_initial": invalid_program(program, document, |program| "missing".clone_into(&mut program.initial_state), |error| matches!(error, BehaviorError::MissingInitialState { .. })),
        "missing_target": invalid_program(program, document, |program| "missing".clone_into(&mut program.states.get_mut("closed").unwrap().transitions[0].target_state), |error| matches!(error, BehaviorError::MissingTargetState { .. })),
        "duplicate_transition": duplicate_transition_typed(program, document),
        "unknown_variable": invalid_first_action(program, document, BehaviorAction::ToggleBoolean { variable: "missing".to_owned() }, |error| matches!(error, BehaviorError::UnknownVariable { .. })),
        "type_mismatch": invalid_first_action(program, document, BehaviorAction::Set { variable: "enabled".to_owned(), value: BehaviorValue::String("yes".to_owned()) }, |error| matches!(error, BehaviorError::TypeMismatch { .. })),
        "undeclared_capability": undeclared_capability_typed(program, document),
        "unsupported_capability": invalid_program(program, document, |program| { program.capabilities.insert("effect.network".to_owned(), CapabilityPolicy::OptionalNoop); }, |error| matches!(error, BehaviorError::UnsupportedCapability { .. })),
        "string_limit": invalid_program(program, document, |program| { program.variables.insert("payload".to_owned(), BehaviorValue::String("x".repeat(MAX_STRING_BYTES + 1))); }, |error| matches!(error, BehaviorError::StringLimit)),
        "invalid_document": invalid_document_typed(program, document),
        "unknown_entity": unknown_entity_typed(program, document),
        "incompatible_event_source": incompatible_event_source_typed(program, document),
        "unreachable_state": invalid_program(program, document, |program| { program.states.insert("orphan".to_owned(), BehaviorState { transitions: Vec::new() }); }, |error| matches!(error, BehaviorError::UnreachableState { .. })),
        "action_budget": action_budget_typed(program, document),
        "event_budget": event_budget_typed(program, document),
        "invalid_external_event": invalid_external_event_typed(program, document),
    })
}

fn invalid_document_typed(
    program: &nuif_behavior::BehaviorProgram,
    document: &nuif_core::Document,
) -> bool {
    let mut invalid = document.clone();
    invalid.roots.push(EntityId::new(0xff));
    matches!(
        validate_program(program, &invalid),
        Err(BehaviorError::InvalidDocument)
    )
}

fn invalid_program<M, P>(
    program: &nuif_behavior::BehaviorProgram,
    document: &nuif_core::Document,
    mutate: M,
    predicate: P,
) -> bool
where
    M: FnOnce(&mut nuif_behavior::BehaviorProgram),
    P: FnOnce(BehaviorError) -> bool,
{
    let mut invalid = program.clone();
    mutate(&mut invalid);
    validate_program(&invalid, document).is_err_and(predicate)
}

fn invalid_first_action<P>(
    program: &nuif_behavior::BehaviorProgram,
    document: &nuif_core::Document,
    action: BehaviorAction,
    predicate: P,
) -> bool
where
    P: FnOnce(BehaviorError) -> bool,
{
    invalid_program(
        program,
        document,
        |program| program.states.get_mut("closed").unwrap().transitions[0].actions = vec![action],
        predicate,
    )
}

fn duplicate_transition_typed(
    program: &nuif_behavior::BehaviorProgram,
    document: &nuif_core::Document,
) -> bool {
    invalid_program(
        program,
        document,
        |program| {
            let duplicate = program.states["closed"].transitions[0].clone();
            program
                .states
                .get_mut("open")
                .unwrap()
                .transitions
                .push(duplicate);
        },
        |error| matches!(error, BehaviorError::DuplicateTransition { .. }),
    )
}

fn undeclared_capability_typed(
    program: &nuif_behavior::BehaviorProgram,
    document: &nuif_core::Document,
) -> bool {
    invalid_program(
        program,
        document,
        |program| {
            program.capabilities.remove(ANNOUNCEMENT);
        },
        |error| matches!(error, BehaviorError::UndeclaredCapability { .. }),
    )
}

fn unknown_entity_typed(
    program: &nuif_behavior::BehaviorProgram,
    document: &nuif_core::Document,
) -> bool {
    invalid_program(
        program,
        document,
        |program| {
            program.states.get_mut("closed").unwrap().transitions[0]
                .event
                .source = EntityId::new(0xff);
        },
        |error| matches!(error, BehaviorError::UnknownEntity { .. }),
    )
}

fn incompatible_event_source_typed(
    program: &nuif_behavior::BehaviorProgram,
    document: &nuif_core::Document,
) -> bool {
    invalid_program(
        program,
        document,
        |program| {
            program.states.get_mut("closed").unwrap().transitions[0]
                .event
                .source = EntityId::new(0x22);
        },
        |error| matches!(error, BehaviorError::IncompatibleEventSource { .. }),
    )
}

fn action_budget_typed(
    program: &nuif_behavior::BehaviorProgram,
    document: &nuif_core::Document,
) -> bool {
    invalid_program(
        program,
        document,
        |program| {
            program.states.get_mut("closed").unwrap().transitions[0].actions = vec![
                BehaviorAction::ToggleBoolean {
                    variable: "enabled".to_owned(),
                };
                MAX_ACTIONS_PER_TRANSITION + 1
            ];
        },
        |error| matches!(error, BehaviorError::ResourceLimit { .. }),
    )
}

fn event_budget_typed(
    program: &nuif_behavior::BehaviorProgram,
    document: &nuif_core::Document,
) -> bool {
    let event = program.states["closed"].transitions[0].event;
    let events = vec![event; MAX_EVENTS_PER_RUN + 1];
    BehaviorRuntime::new(program, document, &BTreeSet::from([VISIBILITY.to_owned()]))
        .and_then(|mut runtime| runtime.run(&events))
        .is_err_and(|error| matches!(error, BehaviorError::EventLimit))
}

fn invalid_external_event_typed(
    program: &nuif_behavior::BehaviorProgram,
    document: &nuif_core::Document,
) -> bool {
    let events = [nuif_behavior::BehaviorEvent {
        kind: nuif_behavior::BehaviorEventKind::Activate,
        source: EntityId::new(0xff),
    }];
    BehaviorRuntime::new(program, document, &BTreeSet::from([VISIBILITY.to_owned()]))
        .and_then(|mut runtime| runtime.run(&events))
        .is_err_and(|error| matches!(error, BehaviorError::InvalidExternalEvent { .. }))
}

fn all_true(value: &Value) -> bool {
    match value {
        Value::Bool(value) => *value,
        Value::Object(values) => values.values().all(all_true),
        _ => false,
    }
}

struct Arguments {
    fixture: PathBuf,
    report: PathBuf,
}

fn arguments() -> Result<Arguments, String> {
    let mut values = env::args().skip(1);
    let mut fixture = PathBuf::from("target/behavior-portability-fixture.json");
    let mut report = PathBuf::from("target/behavior-portability-static-report.json");
    while let Some(argument) = values.next() {
        let target = match argument.as_str() {
            "--fixture" => &mut fixture,
            "--report" => &mut report,
            "--help" | "-h" => {
                return Err(
                    "usage: behavior-portability [--fixture <json>] [--report <json>]".to_owned(),
                );
            }
            unknown => return Err(format!("unknown argument {unknown:?}")),
        };
        *target = values
            .next()
            .ok_or_else(|| format!("{argument} requires a path"))?
            .into();
    }
    Ok(Arguments { fixture, report })
}

fn write_json(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut bytes = bytes.to_vec();
    bytes.push(b'\n');
    fs::write(path, bytes).map_err(|error| error.to_string())
}

fn command_text(program: &str, arguments: &[&str]) -> Option<String> {
    let output = Command::new(program).args(arguments).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
