#![doc = "Bounded deterministic behavior state-machine research profile."]

use nuif_core::{
    Document, Entity, EntityId, EntityKind, Semantics, Severity, is_identifier, validate,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use thiserror::Error;

pub const BEHAVIOR_PROFILE: &str = "nuif-behavior-state-machine-0";
pub const MAX_STATES: usize = 128;
pub const MAX_TRANSITIONS: usize = 1_024;
pub const MAX_ACTIONS: usize = 4_096;
pub const MAX_ACTIONS_PER_TRANSITION: usize = 64;
pub const MAX_VARIABLES: usize = 128;
pub const MAX_CAPABILITIES: usize = 64;
pub const MAX_EVENTS_PER_RUN: usize = 4_096;
pub const MAX_IDENTIFIER_BYTES: usize = 128;
pub const MAX_STRING_BYTES: usize = 4_096;

const ACTIVATABLE_ROLES: &[&str] = &["button", "checkbox", "radio", "switch"];
const VISIBILITY_CAPABILITY: &str = "effect.visibility";
const ANNOUNCEMENT_CAPABILITY: &str = "effect.announcement";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BehaviorProgram {
    pub schema_version: u32,
    pub profile: String,
    pub capabilities: BTreeMap<String, CapabilityPolicy>,
    pub initial_state: String,
    pub variables: BTreeMap<String, BehaviorValue>,
    pub states: BTreeMap<String, BehaviorState>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityPolicy {
    Required,
    OptionalNoop,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BehaviorState {
    pub transitions: Vec<BehaviorTransition>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BehaviorTransition {
    pub id: String,
    pub event: BehaviorEvent,
    pub guard: Option<BehaviorGuard>,
    pub target_state: String,
    pub actions: Vec<BehaviorAction>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BehaviorEvent {
    pub kind: BehaviorEventKind,
    pub source: EntityId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BehaviorEventKind {
    Activate,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "type")]
pub enum BehaviorGuard {
    Equals {
        variable: String,
        value: BehaviorValue,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "type")]
pub enum BehaviorAction {
    Set {
        variable: String,
        value: BehaviorValue,
    },
    ToggleBoolean {
        variable: String,
    },
    Emit {
        effect: BehaviorEffectKind,
        target: EntityId,
        value: BehaviorValue,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    deny_unknown_fields,
    rename_all = "snake_case",
    tag = "type",
    content = "value"
)]
pub enum BehaviorValue {
    Boolean(bool),
    String(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BehaviorEffectKind {
    Visibility,
    Announcement,
}

impl BehaviorEffectKind {
    const fn capability(self) -> &'static str {
        match self {
            Self::Visibility => VISIBILITY_CAPABILITY,
            Self::Announcement => ANNOUNCEMENT_CAPABILITY,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BehaviorEffect {
    pub kind: BehaviorEffectKind,
    pub target: EntityId,
    pub value: BehaviorValue,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkippedOptionalEffect {
    pub capability: String,
    pub kind: BehaviorEffectKind,
    pub target: EntityId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BehaviorTrace {
    pub event: BehaviorEvent,
    pub from_state: String,
    pub transition: Option<String>,
    pub to_state: String,
    pub variables: BTreeMap<String, BehaviorValue>,
    pub effects: Vec<BehaviorEffect>,
    pub skipped_optional: Vec<SkippedOptionalEffect>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BehaviorRun {
    pub schema_version: u32,
    pub profile: String,
    pub traces: Vec<BehaviorTrace>,
    pub final_state: String,
    pub variables: BTreeMap<String, BehaviorValue>,
}

#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum BehaviorError {
    #[error("NUIF document is structurally invalid")]
    InvalidDocument,
    #[error("behavior schema version {found} is unsupported")]
    UnsupportedSchema { found: u32 },
    #[error("behavior profile {found:?} is unsupported")]
    UnsupportedProfile { found: String },
    #[error("behavior resource limit exceeded: {resource}")]
    ResourceLimit { resource: &'static str },
    #[error("{kind} identifier {value:?} is invalid")]
    InvalidIdentifier { kind: &'static str, value: String },
    #[error("behavior string exceeds the profile limit")]
    StringLimit,
    #[error("behavior program has no states")]
    EmptyStateSet,
    #[error("initial state {state:?} does not exist")]
    MissingInitialState { state: String },
    #[error("transition {transition:?} targets missing state {state:?}")]
    MissingTargetState { transition: String, state: String },
    #[error("transition identifier {transition:?} is duplicated")]
    DuplicateTransition { transition: String },
    #[error("transition {transition:?} refers to unknown variable {variable:?}")]
    UnknownVariable {
        transition: String,
        variable: String,
    },
    #[error("transition {transition:?} uses the wrong value type for {subject:?}")]
    TypeMismatch { transition: String, subject: String },
    #[error("transition {transition:?} uses undeclared capability {capability:?}")]
    UndeclaredCapability {
        transition: String,
        capability: String,
    },
    #[error("capability {capability:?} is outside {BEHAVIOR_PROFILE}")]
    UnsupportedCapability { capability: String },
    #[error("required capability {capability:?} is unavailable")]
    MissingRequiredCapability { capability: String },
    #[error("transition {transition:?} refers to unknown entity {entity}")]
    UnknownEntity {
        transition: String,
        entity: EntityId,
    },
    #[error("transition {transition:?} activates incompatible role on entity {entity}")]
    IncompatibleEventSource {
        transition: String,
        entity: EntityId,
    },
    #[error("state {state:?} is unreachable from the initial state")]
    UnreachableState { state: String },
    #[error("event run exceeds the profile limit")]
    EventLimit,
    #[error("external event source {entity} is unknown or not activatable")]
    InvalidExternalEvent { entity: EntityId },
}

#[derive(Clone)]
pub struct BehaviorRuntime {
    program: BehaviorProgram,
    supported_capabilities: BTreeSet<String>,
    activatable_entities: BTreeSet<EntityId>,
    active_state: String,
    variables: BTreeMap<String, BehaviorValue>,
}

impl BehaviorRuntime {
    /// Creates a validated runtime without executing graph actions.
    ///
    /// # Errors
    ///
    /// Returns a typed error if the document or program is invalid or a
    /// required target capability is unavailable.
    pub fn new(
        program: &BehaviorProgram,
        document: &Document,
        supported_capabilities: &BTreeSet<String>,
    ) -> Result<Self, BehaviorError> {
        validate_program(program, document)?;
        for (capability, policy) in &program.capabilities {
            if *policy == CapabilityPolicy::Required && !supported_capabilities.contains(capability)
            {
                return Err(BehaviorError::MissingRequiredCapability {
                    capability: capability.clone(),
                });
            }
        }
        Ok(Self {
            program: program.clone(),
            supported_capabilities: supported_capabilities.clone(),
            activatable_entities: document
                .entities
                .values()
                .filter(|entity| {
                    entity
                        .semantics
                        .role
                        .as_deref()
                        .is_some_and(|role| ACTIVATABLE_ROLES.contains(&role))
                })
                .map(|entity| entity.id)
                .collect(),
            active_state: program.initial_state.clone(),
            variables: program.variables.clone(),
        })
    }

    /// Executes each external event to completion in source order.
    ///
    /// # Errors
    ///
    /// Returns `EventLimit` before execution when the event sequence is too
    /// large for the bounded profile.
    pub fn run(&mut self, events: &[BehaviorEvent]) -> Result<BehaviorRun, BehaviorError> {
        if events.len() > MAX_EVENTS_PER_RUN {
            return Err(BehaviorError::EventLimit);
        }
        if let Some(event) = events
            .iter()
            .find(|event| !self.activatable_entities.contains(&event.source))
        {
            return Err(BehaviorError::InvalidExternalEvent {
                entity: event.source,
            });
        }
        let traces = events.iter().map(|event| self.dispatch(*event)).collect();
        Ok(BehaviorRun {
            schema_version: 1,
            profile: BEHAVIOR_PROFILE.to_owned(),
            traces,
            final_state: self.active_state.clone(),
            variables: self.variables.clone(),
        })
    }

    fn dispatch(&mut self, event: BehaviorEvent) -> BehaviorTrace {
        let from_state = self.active_state.clone();
        let transition = self.program.states[&from_state]
            .transitions
            .iter()
            .find(|transition| {
                transition.event == event
                    && transition
                        .guard
                        .as_ref()
                        .is_none_or(|guard| self.guard(guard))
            })
            .cloned();
        let mut effects = Vec::new();
        let mut skipped_optional = Vec::new();
        if let Some(transition) = &transition {
            for action in &transition.actions {
                match action {
                    BehaviorAction::Set { variable, value } => {
                        self.variables.insert(variable.clone(), value.clone());
                    }
                    BehaviorAction::ToggleBoolean { variable } => {
                        let BehaviorValue::Boolean(value) = self.variables[variable] else {
                            unreachable!("validated toggle action always refers to a Boolean")
                        };
                        self.variables
                            .insert(variable.clone(), BehaviorValue::Boolean(!value));
                    }
                    BehaviorAction::Emit {
                        effect,
                        target,
                        value,
                    } => {
                        let capability = effect.capability();
                        if self.supported_capabilities.contains(capability) {
                            effects.push(BehaviorEffect {
                                kind: *effect,
                                target: *target,
                                value: value.clone(),
                            });
                        } else {
                            skipped_optional.push(SkippedOptionalEffect {
                                capability: capability.to_owned(),
                                kind: *effect,
                                target: *target,
                            });
                        }
                    }
                }
            }
            self.active_state.clone_from(&transition.target_state);
        }
        BehaviorTrace {
            event,
            from_state,
            transition: transition.map(|transition| transition.id),
            to_state: self.active_state.clone(),
            variables: self.variables.clone(),
            effects,
            skipped_optional,
        }
    }

    fn guard(&self, guard: &BehaviorGuard) -> bool {
        match guard {
            BehaviorGuard::Equals { variable, value } => {
                self.variables.get(variable) == Some(value)
            }
        }
    }
}

/// Validates the complete static behavior envelope before runtime allocation.
///
/// # Errors
///
/// Returns the first deterministic profile, identity, reference, type, graph,
/// capability, or resource error.
pub fn validate_program(
    program: &BehaviorProgram,
    document: &Document,
) -> Result<(), BehaviorError> {
    validate_header_and_limits(program, document)?;
    validate_identifiers_and_values(program)?;
    validate_graph(program, document)
}

fn validate_header_and_limits(
    program: &BehaviorProgram,
    document: &Document,
) -> Result<(), BehaviorError> {
    if validate(document)
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        return Err(BehaviorError::InvalidDocument);
    }
    if program.schema_version != 1 {
        return Err(BehaviorError::UnsupportedSchema {
            found: program.schema_version,
        });
    }
    if program.profile != BEHAVIOR_PROFILE {
        return Err(BehaviorError::UnsupportedProfile {
            found: program.profile.clone(),
        });
    }
    if program.states.is_empty() {
        return Err(BehaviorError::EmptyStateSet);
    }
    let transitions = program
        .states
        .values()
        .map(|state| state.transitions.len())
        .sum::<usize>();
    let actions = program
        .states
        .values()
        .flat_map(|state| &state.transitions)
        .map(|transition| transition.actions.len())
        .sum::<usize>();
    for (resource, used, limit) in [
        ("states", program.states.len(), MAX_STATES),
        ("transitions", transitions, MAX_TRANSITIONS),
        ("actions", actions, MAX_ACTIONS),
        ("variables", program.variables.len(), MAX_VARIABLES),
        ("capabilities", program.capabilities.len(), MAX_CAPABILITIES),
    ] {
        if used > limit {
            return Err(BehaviorError::ResourceLimit { resource });
        }
    }
    if program
        .states
        .values()
        .flat_map(|state| &state.transitions)
        .any(|transition| transition.actions.len() > MAX_ACTIONS_PER_TRANSITION)
    {
        return Err(BehaviorError::ResourceLimit {
            resource: "actions per transition",
        });
    }
    Ok(())
}

fn validate_identifiers_and_values(program: &BehaviorProgram) -> Result<(), BehaviorError> {
    validate_identifier("initial state", &program.initial_state)?;
    for (state, definition) in &program.states {
        validate_identifier("state", state)?;
        for transition in &definition.transitions {
            validate_identifier("transition", &transition.id)?;
            validate_identifier("target state", &transition.target_state)?;
            if let Some(BehaviorGuard::Equals { variable, value }) = &transition.guard {
                validate_identifier("variable", variable)?;
                validate_value(value)?;
            }
            for action in &transition.actions {
                match action {
                    BehaviorAction::Set { variable, value } => {
                        validate_identifier("variable", variable)?;
                        validate_value(value)?;
                    }
                    BehaviorAction::ToggleBoolean { variable } => {
                        validate_identifier("variable", variable)?;
                    }
                    BehaviorAction::Emit { value, .. } => validate_value(value)?,
                }
            }
        }
    }
    for (variable, value) in &program.variables {
        validate_identifier("variable", variable)?;
        validate_value(value)?;
    }
    for capability in program.capabilities.keys() {
        validate_identifier("capability", capability)?;
        if ![VISIBILITY_CAPABILITY, ANNOUNCEMENT_CAPABILITY].contains(&capability.as_str()) {
            return Err(BehaviorError::UnsupportedCapability {
                capability: capability.clone(),
            });
        }
    }
    Ok(())
}

fn validate_graph(program: &BehaviorProgram, document: &Document) -> Result<(), BehaviorError> {
    if !program.states.contains_key(&program.initial_state) {
        return Err(BehaviorError::MissingInitialState {
            state: program.initial_state.clone(),
        });
    }
    let mut transition_ids = BTreeSet::new();
    for state in program.states.values() {
        for transition in &state.transitions {
            if !transition_ids.insert(transition.id.as_str()) {
                return Err(BehaviorError::DuplicateTransition {
                    transition: transition.id.clone(),
                });
            }
            validate_transition(program, document, transition)?;
        }
    }
    validate_reachability(program)
}

fn validate_transition(
    program: &BehaviorProgram,
    document: &Document,
    transition: &BehaviorTransition,
) -> Result<(), BehaviorError> {
    if !program.states.contains_key(&transition.target_state) {
        return Err(BehaviorError::MissingTargetState {
            transition: transition.id.clone(),
            state: transition.target_state.clone(),
        });
    }
    let Some(source) = document.entities.get(&transition.event.source) else {
        return Err(BehaviorError::UnknownEntity {
            transition: transition.id.clone(),
            entity: transition.event.source,
        });
    };
    if !source
        .semantics
        .role
        .as_deref()
        .is_some_and(|role| ACTIVATABLE_ROLES.contains(&role))
    {
        return Err(BehaviorError::IncompatibleEventSource {
            transition: transition.id.clone(),
            entity: transition.event.source,
        });
    }
    if let Some(BehaviorGuard::Equals { variable, value }) = &transition.guard {
        validate_variable_value(program, transition, variable, value)?;
    }
    for action in &transition.actions {
        match action {
            BehaviorAction::Set { variable, value } => {
                validate_variable_value(program, transition, variable, value)?;
            }
            BehaviorAction::ToggleBoolean { variable } => {
                let value = program.variables.get(variable).ok_or_else(|| {
                    BehaviorError::UnknownVariable {
                        transition: transition.id.clone(),
                        variable: variable.clone(),
                    }
                })?;
                if !matches!(value, BehaviorValue::Boolean(_)) {
                    return Err(BehaviorError::TypeMismatch {
                        transition: transition.id.clone(),
                        subject: variable.clone(),
                    });
                }
            }
            BehaviorAction::Emit {
                effect,
                target,
                value,
            } => {
                let capability = effect.capability();
                if !program.capabilities.contains_key(capability) {
                    return Err(BehaviorError::UndeclaredCapability {
                        transition: transition.id.clone(),
                        capability: capability.to_owned(),
                    });
                }
                if !document.entities.contains_key(target) {
                    return Err(BehaviorError::UnknownEntity {
                        transition: transition.id.clone(),
                        entity: *target,
                    });
                }
                let valid_type = matches!(
                    (effect, value),
                    (BehaviorEffectKind::Visibility, BehaviorValue::Boolean(_))
                        | (BehaviorEffectKind::Announcement, BehaviorValue::String(_))
                );
                if !valid_type {
                    return Err(BehaviorError::TypeMismatch {
                        transition: transition.id.clone(),
                        subject: capability.to_owned(),
                    });
                }
            }
        }
    }
    Ok(())
}

fn validate_variable_value(
    program: &BehaviorProgram,
    transition: &BehaviorTransition,
    variable: &str,
    value: &BehaviorValue,
) -> Result<(), BehaviorError> {
    let initial =
        program
            .variables
            .get(variable)
            .ok_or_else(|| BehaviorError::UnknownVariable {
                transition: transition.id.clone(),
                variable: variable.to_owned(),
            })?;
    if !same_value_type(initial, value) {
        return Err(BehaviorError::TypeMismatch {
            transition: transition.id.clone(),
            subject: variable.to_owned(),
        });
    }
    Ok(())
}

fn validate_reachability(program: &BehaviorProgram) -> Result<(), BehaviorError> {
    let mut reachable = BTreeSet::from([program.initial_state.as_str()]);
    let mut queue = VecDeque::from([program.initial_state.as_str()]);
    while let Some(state) = queue.pop_front() {
        for transition in &program.states[state].transitions {
            if reachable.insert(&transition.target_state) {
                queue.push_back(&transition.target_state);
            }
        }
    }
    if let Some(state) = program
        .states
        .keys()
        .find(|state| !reachable.contains(state.as_str()))
    {
        return Err(BehaviorError::UnreachableState {
            state: state.clone(),
        });
    }
    Ok(())
}

fn validate_identifier(kind: &'static str, value: &str) -> Result<(), BehaviorError> {
    if value.len() > MAX_IDENTIFIER_BYTES || !is_identifier(value) {
        return Err(BehaviorError::InvalidIdentifier {
            kind,
            value: value.to_owned(),
        });
    }
    Ok(())
}

fn validate_value(value: &BehaviorValue) -> Result<(), BehaviorError> {
    if matches!(value, BehaviorValue::String(value) if value.len() > MAX_STRING_BYTES) {
        return Err(BehaviorError::StringLimit);
    }
    Ok(())
}

const fn same_value_type(left: &BehaviorValue, right: &BehaviorValue) -> bool {
    matches!(
        (left, right),
        (BehaviorValue::Boolean(_), BehaviorValue::Boolean(_))
            | (BehaviorValue::String(_), BehaviorValue::String(_))
    )
}

/// Returns the complete deterministic fixture used by the foreign-runtime gate.
#[must_use]
pub fn behavior_fixture() -> (Document, BehaviorProgram, Vec<BehaviorEvent>) {
    let mut document = Document::empty(EntityId::new(1));
    let mut root = semantic_entity(0x20, "main", "Behavior fixture");
    root.children = vec![
        EntityId::new(0x21),
        EntityId::new(0x22),
        EntityId::new(0x23),
        EntityId::new(0x24),
    ];
    let trigger = semantic_entity(0x21, "button", "Toggle panel");
    let panel = semantic_entity(0x22, "region", "Panel");
    let disable = semantic_entity(0x23, "button", "Disable panel");
    let ignored = semantic_entity(0x24, "button", "Ignored action");
    document.roots.push(root.id);
    for entity in [root, trigger, panel, disable, ignored] {
        document.entities.insert(entity.id, entity);
    }

    let trigger = activation(0x21);
    let disable = activation(0x23);
    let events = vec![trigger, trigger, disable, trigger, activation(0x24)];
    (document, fixture_program(trigger, disable), events)
}

fn fixture_program(trigger: BehaviorEvent, disable: BehaviorEvent) -> BehaviorProgram {
    BehaviorProgram {
        schema_version: 1,
        profile: BEHAVIOR_PROFILE.to_owned(),
        capabilities: BTreeMap::from([
            (VISIBILITY_CAPABILITY.to_owned(), CapabilityPolicy::Required),
            (
                ANNOUNCEMENT_CAPABILITY.to_owned(),
                CapabilityPolicy::OptionalNoop,
            ),
        ]),
        initial_state: "closed".to_owned(),
        variables: BTreeMap::from([
            ("enabled".to_owned(), BehaviorValue::Boolean(true)),
            ("seen".to_owned(), BehaviorValue::Boolean(false)),
        ]),
        states: BTreeMap::from([
            ("closed".to_owned(), fixture_closed_state(trigger, disable)),
            ("open".to_owned(), fixture_open_state(trigger, disable)),
        ]),
    }
}

fn fixture_closed_state(trigger: BehaviorEvent, disable: BehaviorEvent) -> BehaviorState {
    BehaviorState {
        transitions: vec![
            BehaviorTransition {
                id: "disable-from-closed".to_owned(),
                event: disable,
                guard: None,
                target_state: "closed".to_owned(),
                actions: vec![BehaviorAction::ToggleBoolean {
                    variable: "enabled".to_owned(),
                }],
            },
            BehaviorTransition {
                id: "blocked-open".to_owned(),
                event: trigger,
                guard: Some(BehaviorGuard::Equals {
                    variable: "enabled".to_owned(),
                    value: BehaviorValue::Boolean(false),
                }),
                target_state: "closed".to_owned(),
                actions: vec![BehaviorAction::Emit {
                    effect: BehaviorEffectKind::Announcement,
                    target: EntityId::new(0x22),
                    value: BehaviorValue::String("Unavailable".to_owned()),
                }],
            },
            BehaviorTransition {
                id: "open-panel".to_owned(),
                event: trigger,
                guard: Some(BehaviorGuard::Equals {
                    variable: "enabled".to_owned(),
                    value: BehaviorValue::Boolean(true),
                }),
                target_state: "open".to_owned(),
                actions: vec![
                    BehaviorAction::Set {
                        variable: "seen".to_owned(),
                        value: BehaviorValue::Boolean(true),
                    },
                    BehaviorAction::Emit {
                        effect: BehaviorEffectKind::Visibility,
                        target: EntityId::new(0x22),
                        value: BehaviorValue::Boolean(true),
                    },
                    BehaviorAction::Emit {
                        effect: BehaviorEffectKind::Announcement,
                        target: EntityId::new(0x22),
                        value: BehaviorValue::String("Opened".to_owned()),
                    },
                ],
            },
        ],
    }
}

fn fixture_open_state(trigger: BehaviorEvent, disable: BehaviorEvent) -> BehaviorState {
    BehaviorState {
        transitions: vec![
            BehaviorTransition {
                id: "close-panel".to_owned(),
                event: trigger,
                guard: None,
                target_state: "closed".to_owned(),
                actions: vec![
                    BehaviorAction::Emit {
                        effect: BehaviorEffectKind::Visibility,
                        target: EntityId::new(0x22),
                        value: BehaviorValue::Boolean(false),
                    },
                    BehaviorAction::Emit {
                        effect: BehaviorEffectKind::Announcement,
                        target: EntityId::new(0x22),
                        value: BehaviorValue::String("Closed".to_owned()),
                    },
                ],
            },
            BehaviorTransition {
                id: "disable-from-open".to_owned(),
                event: disable,
                guard: None,
                target_state: "open".to_owned(),
                actions: vec![BehaviorAction::ToggleBoolean {
                    variable: "enabled".to_owned(),
                }],
            },
        ],
    }
}

const fn activation(source: u128) -> BehaviorEvent {
    BehaviorEvent {
        kind: BehaviorEventKind::Activate,
        source: EntityId::new(source),
    }
}

fn semantic_entity(id: u128, role: &str, name: &str) -> Entity {
    let mut entity = Entity::new(EntityId::new(id), EntityKind::Container);
    entity.semantics = Semantics {
        role: Some(role.to_owned()),
        accessible_name: Some(name.to_owned()),
        states: BTreeMap::new(),
    };
    entity
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_capabilities() -> BTreeSet<String> {
        BTreeSet::from([
            VISIBILITY_CAPABILITY.to_owned(),
            ANNOUNCEMENT_CAPABILITY.to_owned(),
        ])
    }

    #[test]
    fn fixture_is_deterministic_and_run_to_completion() {
        let (document, program, events) = behavior_fixture();
        let mut first = BehaviorRuntime::new(&program, &document, &all_capabilities()).unwrap();
        let mut second = first.clone();
        let first = first.run(&events).unwrap();
        let second = second.run(&events).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.final_state, "closed");
        assert_eq!(first.traces[0].transition.as_deref(), Some("open-panel"));
        assert_eq!(first.traces[1].transition.as_deref(), Some("close-panel"));
        assert_eq!(first.traces[3].transition.as_deref(), Some("blocked-open"));
        assert_eq!(first.traces[4].transition, None);
        assert_eq!(
            first.variables.get("seen"),
            Some(&BehaviorValue::Boolean(true))
        );
    }

    #[test]
    fn required_capabilities_fail_and_optional_capabilities_noop() {
        let (document, program, events) = behavior_fixture();
        assert!(matches!(
            BehaviorRuntime::new(&program, &document, &BTreeSet::new()),
            Err(BehaviorError::MissingRequiredCapability { .. })
        ));
        let required_only = BTreeSet::from([VISIBILITY_CAPABILITY.to_owned()]);
        let mut runtime = BehaviorRuntime::new(&program, &document, &required_only).unwrap();
        let result = runtime.run(&events).unwrap();
        assert_eq!(
            result
                .traces
                .iter()
                .map(|trace| trace.skipped_optional.len())
                .sum::<usize>(),
            3
        );
        assert_eq!(
            result
                .traces
                .iter()
                .flat_map(|trace| &trace.effects)
                .count(),
            2
        );
    }

    #[test]
    fn invalid_references_types_and_reachability_fail_closed() {
        let (document, program, _) = behavior_fixture();
        let mut invalid = program.clone();
        invalid.states.get_mut("closed").unwrap().transitions[0].actions =
            vec![BehaviorAction::Set {
                variable: "enabled".to_owned(),
                value: BehaviorValue::String("yes".to_owned()),
            }];
        assert!(matches!(
            validate_program(&invalid, &document),
            Err(BehaviorError::TypeMismatch { .. })
        ));

        let mut unreachable = program;
        unreachable.states.insert(
            "orphan".to_owned(),
            BehaviorState {
                transitions: Vec::new(),
            },
        );
        assert!(matches!(
            validate_program(&unreachable, &document),
            Err(BehaviorError::UnreachableState { .. })
        ));
    }

    #[test]
    fn serialized_program_rejects_unknown_fields() {
        let (_, program, _) = behavior_fixture();
        let mut value = serde_json::to_value(program).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("script".to_owned(), serde_json::json!("alert(1)"));
        assert!(serde_json::from_value::<BehaviorProgram>(value).is_err());

        let mut nested = serde_json::to_value(behavior_fixture().1).unwrap();
        nested["states"]["closed"]["transitions"][0]["actions"][0]["script"] =
            serde_json::json!("alert(1)");
        assert!(serde_json::from_value::<BehaviorProgram>(nested).is_err());
    }

    #[test]
    fn invalid_external_events_fail_before_state_changes() {
        let (document, program, events) = behavior_fixture();
        let mut events = events;
        events.push(activation(0xff));
        let mut runtime = BehaviorRuntime::new(&program, &document, &all_capabilities()).unwrap();
        assert!(matches!(
            runtime.run(&events),
            Err(BehaviorError::InvalidExternalEvent { .. })
        ));
        assert_eq!(runtime.active_state, "closed");
        assert_eq!(runtime.variables, program.variables);
    }
}
