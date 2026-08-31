//! Finite web lowering for the bounded NUIF behavior sidecar.

use crate::accessibility::{WebAccessibilityError, project_web_accessibility};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use nuif_behavior::{
    ANNOUNCEMENT_CAPABILITY, BEHAVIOR_PROFILE, BehaviorAction, BehaviorEffectKind, BehaviorError,
    BehaviorProgram, BehaviorValue, VISIBILITY_CAPABILITY, validate_program,
};
use nuif_core::{Document, EntityId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use thiserror::Error;

pub const WEB_BEHAVIOR_PROFILE: &str = "nuif-web-behavior-0";
const STATUS_ID: &str = "nuif-behavior-status";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebBehaviorProjection {
    pub schema_version: u32,
    pub profile: String,
    pub source_profile: String,
    pub html: String,
    pub script_sha256: String,
    pub csp_script_source: String,
    pub event_sources: Vec<EntityId>,
    pub effect_capabilities: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum WebBehaviorError {
    #[error(transparent)]
    Behavior(#[from] BehaviorError),
    #[error(transparent)]
    Accessibility(#[from] WebAccessibilityError),
    #[error("behavior JSON serialization failed: {0}")]
    Serialization(String),
    #[error("event source {entity} has unsupported web role {role:?}")]
    UnsupportedEventSource { entity: EntityId, role: String },
    #[error("event source {entity} is disabled and cannot receive native activation")]
    DisabledEventSource { entity: EntityId },
    #[error(
        "transition {transition:?} emits repeated {effect:?} effects to {target} in one run-to-completion step"
    )]
    RepeatedEffect {
        transition: String,
        effect: BehaviorEffectKind,
        target: EntityId,
    },
    #[error("transition {transition:?} emits more than one announcement in one task")]
    MultipleAnnouncements { transition: String },
    #[error("transition {transition:?} emits an empty web announcement")]
    EmptyAnnouncement { transition: String },
    #[error(
        "transition {transition:?} can hide event source {event_source} through target {target}"
    )]
    VisibilityHidesEventSource {
        transition: String,
        target: EntityId,
        event_source: EntityId,
    },
    #[error("generated accessibility HTML did not contain the expected insertion point")]
    ProjectionInvariant,
}

/// Lowers a validated behavior program into native controls plus one finite
/// generated interpreter protected by an exact CSP hash.
///
/// This profile supports native `button` and button-backed `switch` activation,
/// the HTML `hidden` property, and a single polite `status` live region. It does
/// not accept authored JavaScript or any effect outside the source profile.
///
/// # Errors
///
/// Returns a typed error when either source profile is invalid, native web
/// activation would diverge, or one task could collapse observable effects.
pub fn project_web_behavior(
    document: &Document,
    program: &BehaviorProgram,
) -> Result<WebBehaviorProjection, WebBehaviorError> {
    validate_program(program, document)?;
    validate_web_envelope(document, program)?;
    let accessibility = project_web_accessibility(document)?;
    let event_sources = web_event_sources(document);
    let program_json = serde_json::to_string(program)
        .map_err(|error| WebBehaviorError::Serialization(error.to_string()))?;
    let script = runtime_script(&escape_script_json(&program_json), &event_sources);
    let digest = Sha256::digest(script.as_bytes());
    let csp_script_source = format!("sha256-{}", BASE64.encode(digest));
    let csp = format!(
        "default-src 'none'; script-src '{csp_script_source}'; style-src 'none'; img-src 'none'; font-src 'none'; connect-src 'none'; base-uri 'none'; form-action 'none'"
    );
    let head_marker = "<head>\n<meta charset=\"utf-8\">\n";
    let head = format!(
        "<head>\n<meta charset=\"utf-8\">\n<meta http-equiv=\"Content-Security-Policy\" content=\"{csp}\">\n"
    );
    let mut html = accessibility.html.replacen(head_marker, &head, 1);
    if html == accessibility.html {
        return Err(WebBehaviorError::ProjectionInvariant);
    }
    html = html.replace(
        "<title>NUIF accessibility oracle</title>",
        "<title>NUIF web behavior oracle</title>",
    );
    let body_marker = "</body>\n</html>\n";
    let body = format!(
        "  <div id=\"{STATUS_ID}\" role=\"status\" aria-live=\"polite\" aria-atomic=\"true\"></div>\n<script>{script}</script>\n</body>\n</html>\n"
    );
    let projected = html.replacen(body_marker, &body, 1);
    if projected == html {
        return Err(WebBehaviorError::ProjectionInvariant);
    }
    Ok(WebBehaviorProjection {
        schema_version: 1,
        profile: WEB_BEHAVIOR_PROFILE.to_owned(),
        source_profile: BEHAVIOR_PROFILE.to_owned(),
        html: projected,
        script_sha256: format!("{digest:x}"),
        csp_script_source,
        event_sources,
        effect_capabilities: vec![
            VISIBILITY_CAPABILITY.to_owned(),
            ANNOUNCEMENT_CAPABILITY.to_owned(),
        ],
    })
}

fn validate_web_envelope(
    document: &Document,
    program: &BehaviorProgram,
) -> Result<(), WebBehaviorError> {
    let program_sources = program_event_sources(program);
    let web_sources = web_event_sources(document);
    for source in program_sources {
        let entity = &document.entities[&source];
        let role = entity.semantics.role.as_deref().unwrap_or_default();
        if !matches!(role, "button" | "switch") {
            return Err(WebBehaviorError::UnsupportedEventSource {
                entity: source,
                role: role.to_owned(),
            });
        }
        if entity.semantics.states.get("disabled") == Some(&true) {
            return Err(WebBehaviorError::DisabledEventSource { entity: source });
        }
    }
    for state in program.states.values() {
        for transition in &state.transitions {
            let mut effects = BTreeSet::new();
            let mut announcement = false;
            for action in &transition.actions {
                let BehaviorAction::Emit {
                    effect,
                    target,
                    value,
                } = action
                else {
                    continue;
                };
                let effect_name = match effect {
                    BehaviorEffectKind::Visibility => "visibility",
                    BehaviorEffectKind::Announcement => "announcement",
                };
                if !effects.insert((effect_name, *target)) {
                    return Err(WebBehaviorError::RepeatedEffect {
                        transition: transition.id.clone(),
                        effect: *effect,
                        target: *target,
                    });
                }
                if *effect == BehaviorEffectKind::Visibility
                    && value == &BehaviorValue::Boolean(false)
                    && let Some(source) = web_sources
                        .iter()
                        .find(|source| contains_entity(document, *target, **source))
                {
                    return Err(WebBehaviorError::VisibilityHidesEventSource {
                        transition: transition.id.clone(),
                        target: *target,
                        event_source: *source,
                    });
                }
                if *effect == BehaviorEffectKind::Announcement {
                    if announcement {
                        return Err(WebBehaviorError::MultipleAnnouncements {
                            transition: transition.id.clone(),
                        });
                    }
                    announcement = true;
                    if matches!(value, BehaviorValue::String(value) if value.trim().is_empty()) {
                        return Err(WebBehaviorError::EmptyAnnouncement {
                            transition: transition.id.clone(),
                        });
                    }
                }
            }
        }
    }
    Ok(())
}

fn contains_entity(document: &Document, ancestor: EntityId, descendant: EntityId) -> bool {
    if ancestor == descendant {
        return true;
    }
    let mut pending = document.entities[&ancestor].children.clone();
    while let Some(candidate) = pending.pop() {
        if candidate == descendant {
            return true;
        }
        pending.extend_from_slice(&document.entities[&candidate].children);
    }
    false
}

fn program_event_sources(program: &BehaviorProgram) -> Vec<EntityId> {
    program
        .states
        .values()
        .flat_map(|state| &state.transitions)
        .map(|transition| transition.event.source)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn web_event_sources(document: &Document) -> Vec<EntityId> {
    document
        .entities
        .values()
        .filter(|entity| {
            matches!(entity.semantics.role.as_deref(), Some("button" | "switch"))
                && entity.semantics.states.get("disabled") != Some(&true)
        })
        .map(|entity| entity.id)
        .collect()
}

fn escape_script_json(json: &str) -> String {
    json.replace('&', "\\u0026")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029")
}

fn runtime_script(program_json: &str, sources: &[EntityId]) -> String {
    let source_json = serde_json::to_string(sources).expect("entity identifiers always serialize");
    format!(
        r#"(() => {{
"use strict";
const program = {program_json};
const sources = {source_json};
const nodes = new Map(Array.from(document.querySelectorAll("[data-nuif-id]"), node => [node.dataset.nuifId, node]));
const status = document.getElementById("{STATUS_ID}");
let state = program.initial_state;
const variables = Object.fromEntries(Object.entries(program.variables));
const equal = (left, right) => left.type === right.type && left.value === right.value;
const matches = guard => guard === null || (guard.type === "equals" && equal(variables[guard.variable], guard.value));
const apply = action => {{
  if (action.type === "set") {{ variables[action.variable] = action.value; return; }}
  if (action.type === "toggle_boolean") {{ variables[action.variable] = {{ type: "boolean", value: !variables[action.variable].value }}; return; }}
  const target = nodes.get(action.target);
  if (action.effect === "visibility") {{ target.hidden = !action.value.value; return; }}
  status.dataset.nuifAnnouncementTarget = action.target;
  status.textContent = action.value.value;
}};
const dispatch = source => {{
  const transition = program.states[state].transitions.find(candidate => candidate.event.kind === "activate" && candidate.event.source === source && matches(candidate.guard));
  if (transition === undefined) {{
    document.documentElement.dataset.nuifBehaviorLastTransition = "";
    return;
  }}
  for (const action of transition.actions) apply(action);
  state = transition.target_state;
  document.documentElement.dataset.nuifBehaviorState = state;
  document.documentElement.dataset.nuifBehaviorLastTransition = transition.id;
}};
document.documentElement.dataset.nuifBehaviorProfile = "{WEB_BEHAVIOR_PROFILE}";
document.documentElement.dataset.nuifBehaviorState = state;
document.documentElement.dataset.nuifBehaviorLastTransition = "";
for (const source of sources) nodes.get(source).addEventListener("click", () => dispatch(source));
}})();
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use nuif_behavior::behavior_fixture;

    #[test]
    fn fixture_projects_to_hash_restricted_runtime() {
        let (document, program, _) = behavior_fixture();
        let projection = project_web_behavior(&document, &program).unwrap();
        assert_eq!(projection.profile, WEB_BEHAVIOR_PROFILE);
        assert!(
            projection
                .html
                .contains(&format!("script-src '{}'", projection.csp_script_source))
        );
        assert!(!projection.html.contains("'unsafe-inline'"));
        assert!(!projection.html.contains("eval("));
    }
}
