use nuif_core::{
    Document, Entity, EntityId, EntityKind, Relation, Semantics, Severity, TextContent, validate,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use thiserror::Error;

pub const WEB_ACCESSIBILITY_PROFILE: &str = "nuif-web-accessibility-0";
pub const MAX_WEB_ACCESSIBILITY_NODES: usize = 4_096;
pub const MAX_WEB_ACCESSIBILITY_RELATIONS: usize = 8_192;

const SUPPORTED_RELATIONS: &[(&str, &str)] = &[
    ("labelled-by", "aria-labelledby"),
    ("described-by", "aria-describedby"),
    ("controls", "aria-controls"),
    ("owns", "aria-owns"),
    ("flow-to", "aria-flowto"),
];

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebAccessibilityProjection {
    pub schema_version: u32,
    pub profile: String,
    pub html: String,
    pub nodes: Vec<WebAccessibilityNode>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebAccessibilityNode {
    pub entity: EntityId,
    pub role: String,
    pub accessible_name: Option<String>,
    pub states: BTreeMap<String, bool>,
    pub relationships: BTreeMap<String, Vec<EntityId>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum WebAccessibilityError {
    #[error("document is structurally invalid")]
    InvalidDocument,
    #[error("web accessibility projection exceeds its node or relationship limit")]
    ResourceLimit,
    #[error("semantic role {role:?} on {entity} is outside {WEB_ACCESSIBILITY_PROFILE}")]
    UnsupportedRole { entity: EntityId, role: String },
    #[error("semantic state {state:?} is invalid for role {role:?} on {entity}")]
    UnsupportedState {
        entity: EntityId,
        role: String,
        state: String,
    },
    #[error("semantic role {role:?} on {entity} requires an accessible name")]
    MissingName { entity: EntityId, role: String },
    #[error("accessible name on {entity} is empty after whitespace normalization")]
    EmptyName { entity: EntityId },
    #[error("semantic role {role:?} on {entity} prohibits an author-provided name")]
    ProhibitedName { entity: EntityId, role: String },
    #[error("entity {entity} supplies both an accessible name and labelled-by relationship")]
    AmbiguousName { entity: EntityId },
    #[error("labelled-by target {target} for {entity} has no bounded textual name")]
    UnnamedLabelTarget { entity: EntityId, target: EntityId },
    #[error("relationship {kind:?} from {entity} is outside {WEB_ACCESSIBILITY_PROFILE}")]
    UnsupportedRelation { entity: EntityId, kind: String },
    #[error("relationship {kind:?} from {entity} repeats target {target}")]
    DuplicateRelation {
        entity: EntityId,
        kind: String,
        target: EntityId,
    },
    #[error("owned target {target} has conflicting owners {first_source} and {second_source}")]
    OwnedTargetConflict {
        target: EntityId,
        first_source: EntityId,
        second_source: EntityId,
    },
    #[error("owned accessibility tree contains a cycle through {entity}")]
    OwnedTreeCycle { entity: EntityId },
    #[error("entity {entity} has accessibility relationships without a supported semantic role")]
    RelationSourceWithoutRole { entity: EntityId },
    #[error("role {role:?} on {entity} cannot contain child elements in this profile")]
    InvalidContainment { entity: EntityId, role: String },
}

#[derive(Clone, Copy)]
struct RoleMapping {
    tag: &'static str,
    explicit_role: Option<&'static str>,
    name: NameRule,
    allowed_states: &'static [&'static str],
    void: bool,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum NameRule {
    Optional,
    Required,
    Prohibited,
}

#[derive(Default)]
struct RelationAttributes {
    by_source: BTreeMap<EntityId, BTreeMap<&'static str, Vec<EntityId>>>,
}

/// Projects the bounded portable semantic subset to inert HTML and ARIA.
///
/// The result preserves document containment and stable entity identifiers,
/// but it does not synthesize application behavior. Unsupported roles, states,
/// relationships, ambiguous names and invalid containment fail closed.
///
/// # Errors
///
/// Returns a typed error for invalid documents, resource overflow, or semantics
/// outside `nuif-web-accessibility-0`.
pub fn project_web_accessibility(
    document: &Document,
) -> Result<WebAccessibilityProjection, WebAccessibilityError> {
    if validate(document)
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        return Err(WebAccessibilityError::InvalidDocument);
    }
    if document.entities.len() > MAX_WEB_ACCESSIBILITY_NODES
        || document.relations.len() > MAX_WEB_ACCESSIBILITY_RELATIONS
    {
        return Err(WebAccessibilityError::ResourceLimit);
    }
    let relations = relation_attributes(document)?;
    let mut mapped = BTreeMap::new();
    let mut nodes = Vec::new();
    for entity in document.entities.values() {
        let Some(role) = entity.semantics.role.as_deref() else {
            if relations.by_source.contains_key(&entity.id) {
                return Err(WebAccessibilityError::RelationSourceWithoutRole { entity: entity.id });
            }
            continue;
        };
        let role_mapping =
            role_mapping(role).ok_or_else(|| WebAccessibilityError::UnsupportedRole {
                entity: entity.id,
                role: role.to_owned(),
            })?;
        if role_mapping.void && !entity.children.is_empty()
            || role_mapping.tag == "p" && !entity.children.is_empty()
        {
            return Err(WebAccessibilityError::InvalidContainment {
                entity: entity.id,
                role: role.to_owned(),
            });
        }
        validate_states(entity, role_mapping)?;
        let relationship_map = relations
            .by_source
            .get(&entity.id)
            .cloned()
            .unwrap_or_default();
        let labelled_by = relationship_map
            .get("aria-labelledby")
            .cloned()
            .unwrap_or_default();
        if role_mapping.name == NameRule::Prohibited && entity.semantics.accessible_name.is_some() {
            return Err(WebAccessibilityError::ProhibitedName {
                entity: entity.id,
                role: role.to_owned(),
            });
        }
        let direct_name = entity
            .semantics
            .accessible_name
            .as_deref()
            .map(normalize_accessible_text);
        if direct_name.as_deref().is_some_and(str::is_empty) {
            return Err(WebAccessibilityError::EmptyName { entity: entity.id });
        }
        if direct_name.is_some() && !labelled_by.is_empty() {
            return Err(WebAccessibilityError::AmbiguousName { entity: entity.id });
        }
        let accessible_name = if labelled_by.is_empty() {
            direct_name
        } else {
            Some(resolve_label(document, entity.id, &labelled_by)?)
        };
        match role_mapping.name {
            NameRule::Required if accessible_name.as_deref().is_none_or(str::is_empty) => {
                return Err(WebAccessibilityError::MissingName {
                    entity: entity.id,
                    role: role.to_owned(),
                });
            }
            _ => {}
        }
        mapped.insert(entity.id, role_mapping);
        nodes.push(WebAccessibilityNode {
            entity: entity.id,
            role: role.to_owned(),
            accessible_name,
            states: entity.semantics.states.clone(),
            relationships: relationship_map
                .iter()
                .map(|(attribute, targets)| (relation_kind(attribute).to_owned(), targets.clone()))
                .collect(),
        });
    }
    let mut html = String::from(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<title>NUIF accessibility oracle</title>\n</head>\n<body>\n",
    );
    for root in &document.roots {
        write_entity(document, *root, &mapped, &relations, 1, &mut html)?;
    }
    html.push_str("</body>\n</html>\n");
    Ok(WebAccessibilityProjection {
        schema_version: 1,
        profile: WEB_ACCESSIBILITY_PROFILE.to_owned(),
        html,
        nodes,
    })
}

/// Returns the exact role, name, state and relationship fixture used by the
/// foreign-browser accessibility oracle.
#[must_use]
pub fn web_accessibility_fixture() -> Document {
    let mut document = Document::empty(EntityId::new(1));
    let mut main = semantic_entity(0x10, EntityKind::Container, "main", Some("NUIF workspace"));
    main.children = vec![
        EntityId::new(0x11),
        EntityId::new(0x12),
        EntityId::new(0x13),
        EntityId::new(0x15),
        EntityId::new(0x16),
        EntityId::new(0x14),
        EntityId::new(0x18),
        EntityId::new(0x19),
        EntityId::new(0x1a),
    ];
    let label = text_entity(0x11, "Receive updates");
    let mut checkbox = semantic_entity(0x12, EntityKind::Container, "checkbox", None);
    checkbox.semantics.states.extend([
        ("checked".to_owned(), true),
        ("disabled".to_owned(), false),
        ("required".to_owned(), true),
    ]);
    let mut button = semantic_entity(0x13, EntityKind::Container, "button", Some("Save draft"));
    button.semantics.states.extend([
        ("disabled".to_owned(), false),
        ("expanded".to_owned(), true),
        ("pressed".to_owned(), false),
    ]);
    let mut region = semantic_entity(0x14, EntityKind::Container, "region", Some("Inspector"));
    region.children.push(EntityId::new(0x17));
    let mut switch = semantic_entity(0x15, EntityKind::Container, "switch", Some("Enable motion"));
    switch
        .semantics
        .states
        .extend([("checked".to_owned(), true), ("disabled".to_owned(), true)]);
    let image = semantic_entity(0x16, EntityKind::Container, "img", Some("Document preview"));
    let details = text_entity(0x17, "Changes are saved locally");
    let navigation = semantic_entity(
        0x18,
        EntityKind::Container,
        "navigation",
        Some("Document tools"),
    );
    let group = semantic_entity(0x19, EntityKind::Container, "group", Some("Export options"));
    let mut radio = semantic_entity(0x1a, EntityKind::Container, "radio", Some("PDF document"));
    radio.semantics.states.extend([
        ("checked".to_owned(), false),
        ("disabled".to_owned(), true),
        ("required".to_owned(), true),
    ]);
    document.roots.push(main.id);
    for entity in [
        main, label, checkbox, button, region, switch, image, details, navigation, group, radio,
    ] {
        document.entities.insert(entity.id, entity);
    }
    document.relations = vec![
        Relation {
            kind: "labelled-by".to_owned(),
            source: EntityId::new(0x12),
            target: EntityId::new(0x11),
        },
        Relation {
            kind: "controls".to_owned(),
            source: EntityId::new(0x13),
            target: EntityId::new(0x14),
        },
        Relation {
            kind: "described-by".to_owned(),
            source: EntityId::new(0x13),
            target: EntityId::new(0x17),
        },
        Relation {
            kind: "flow-to".to_owned(),
            source: EntityId::new(0x18),
            target: EntityId::new(0x14),
        },
        Relation {
            kind: "owns".to_owned(),
            source: EntityId::new(0x19),
            target: EntityId::new(0x1a),
        },
    ];
    document
}

fn semantic_entity(
    id: u128,
    kind: EntityKind,
    role: &str,
    accessible_name: Option<&str>,
) -> Entity {
    let mut entity = Entity::new(EntityId::new(id), kind);
    entity.semantics = Semantics {
        role: Some(role.to_owned()),
        accessible_name: accessible_name.map(str::to_owned),
        states: BTreeMap::new(),
    };
    entity
}

fn text_entity(id: u128, content: &str) -> Entity {
    let mut entity = semantic_entity(id, EntityKind::Text, "paragraph", None);
    entity.authored.text = Some(TextContent {
        content: content.to_owned(),
        font: "fixture-font".to_owned(),
        font_sha256: "0".repeat(64),
        font_asset: None,
        size: 16.0,
        line_height: 20.0,
    });
    entity
}

fn relation_attributes(document: &Document) -> Result<RelationAttributes, WebAccessibilityError> {
    let mut attributes = RelationAttributes::default();
    let mut seen = BTreeSet::new();
    let mut owned_by = BTreeMap::new();
    for relation in &document.relations {
        let Some((_, attribute)) = SUPPORTED_RELATIONS
            .iter()
            .find(|(kind, _)| *kind == relation.kind)
        else {
            return Err(WebAccessibilityError::UnsupportedRelation {
                entity: relation.source,
                kind: relation.kind.clone(),
            });
        };
        if !seen.insert((relation.source, relation.kind.as_str(), relation.target)) {
            return Err(WebAccessibilityError::DuplicateRelation {
                entity: relation.source,
                kind: relation.kind.clone(),
                target: relation.target,
            });
        }
        if relation.kind == "owns"
            && let Some(first_source) = owned_by.insert(relation.target, relation.source)
            && first_source != relation.source
        {
            return Err(WebAccessibilityError::OwnedTargetConflict {
                target: relation.target,
                first_source,
                second_source: relation.source,
            });
        }
        attributes
            .by_source
            .entry(relation.source)
            .or_default()
            .entry(attribute)
            .or_default()
            .push(relation.target);
    }
    validate_owned_tree(&attributes)?;
    Ok(attributes)
}

fn validate_owned_tree(attributes: &RelationAttributes) -> Result<(), WebAccessibilityError> {
    let starts = attributes
        .by_source
        .iter()
        .filter(|(_, relations)| relations.contains_key("aria-owns"))
        .map(|(source, _)| *source)
        .collect::<Vec<_>>();
    let mut state = BTreeMap::<EntityId, u8>::new();
    for start in starts {
        if state.get(&start) == Some(&2) {
            continue;
        }
        let mut stack = vec![(start, false)];
        while let Some((entity, exiting)) = stack.pop() {
            if exiting {
                state.insert(entity, 2);
                continue;
            }
            match state.get(&entity).copied() {
                Some(1) => return Err(WebAccessibilityError::OwnedTreeCycle { entity }),
                Some(2) => continue,
                _ => {}
            }
            state.insert(entity, 1);
            stack.push((entity, true));
            if let Some(targets) = attributes
                .by_source
                .get(&entity)
                .and_then(|relations| relations.get("aria-owns"))
            {
                stack.extend(targets.iter().rev().map(|target| (*target, false)));
            }
        }
    }
    Ok(())
}

fn resolve_label(
    document: &Document,
    source: EntityId,
    targets: &[EntityId],
) -> Result<String, WebAccessibilityError> {
    targets
        .iter()
        .map(|target| {
            let entity = &document.entities[target];
            let label = entity
                .authored
                .text
                .as_ref()
                .map(|text| normalize_accessible_text(&text.content))
                .filter(|text| !text.is_empty())
                .or_else(|| {
                    entity
                        .semantics
                        .accessible_name
                        .as_deref()
                        .map(normalize_accessible_text)
                        .filter(|name| !name.is_empty())
                })
                .ok_or(WebAccessibilityError::UnnamedLabelTarget {
                    entity: source,
                    target: *target,
                })?;
            Ok(label)
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|labels| labels.join(" "))
}

fn normalize_accessible_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn validate_states(
    entity: &Entity,
    role_mapping: RoleMapping,
) -> Result<(), WebAccessibilityError> {
    let role = entity.semantics.role.as_deref().unwrap_or_default();
    for state in entity.semantics.states.keys() {
        if !role_mapping.allowed_states.contains(&state.as_str()) {
            return Err(WebAccessibilityError::UnsupportedState {
                entity: entity.id,
                role: role.to_owned(),
                state: state.clone(),
            });
        }
    }
    if role == "switch" && !entity.semantics.states.contains_key("checked") {
        return Err(WebAccessibilityError::UnsupportedState {
            entity: entity.id,
            role: role.to_owned(),
            state: "checked-required".to_owned(),
        });
    }
    Ok(())
}

fn role_mapping(role: &str) -> Option<RoleMapping> {
    Some(match role {
        "button" => RoleMapping {
            tag: "button",
            explicit_role: None,
            name: NameRule::Required,
            allowed_states: &["disabled", "expanded", "pressed"],
            void: false,
        },
        "checkbox" | "radio" => RoleMapping {
            tag: "input",
            explicit_role: None,
            name: NameRule::Required,
            allowed_states: &["checked", "disabled", "required"],
            void: true,
        },
        "group" => RoleMapping {
            tag: "div",
            explicit_role: Some("group"),
            name: NameRule::Optional,
            allowed_states: &[],
            void: false,
        },
        "img" => RoleMapping {
            tag: "div",
            explicit_role: Some("img"),
            name: NameRule::Required,
            allowed_states: &[],
            void: false,
        },
        "main" => RoleMapping {
            tag: "main",
            explicit_role: None,
            name: NameRule::Optional,
            allowed_states: &[],
            void: false,
        },
        "navigation" => RoleMapping {
            tag: "nav",
            explicit_role: None,
            name: NameRule::Optional,
            allowed_states: &[],
            void: false,
        },
        "paragraph" => RoleMapping {
            tag: "p",
            explicit_role: None,
            name: NameRule::Prohibited,
            allowed_states: &[],
            void: false,
        },
        "region" => RoleMapping {
            tag: "section",
            explicit_role: None,
            name: NameRule::Required,
            allowed_states: &[],
            void: false,
        },
        "switch" => RoleMapping {
            tag: "button",
            explicit_role: Some("switch"),
            name: NameRule::Required,
            allowed_states: &["checked", "disabled"],
            void: false,
        },
        _ => return None,
    })
}

fn write_entity(
    document: &Document,
    id: EntityId,
    mapped: &BTreeMap<EntityId, RoleMapping>,
    relations: &RelationAttributes,
    depth: usize,
    html: &mut String,
) -> Result<(), WebAccessibilityError> {
    let entity = &document.entities[&id];
    let indentation = "  ".repeat(depth);
    let role_mapping = mapped.get(&id).copied();
    let tag = role_mapping.map_or("div", |mapping| mapping.tag);
    write!(
        html,
        "{indentation}<{tag} id=\"{}\" data-nuif-id=\"{id}\"",
        html_id(id)
    )
    .expect("string formatting cannot fail");
    if tag == "button" {
        html.push_str(" type=\"button\"");
    } else if tag == "input" {
        let input_type = if entity.semantics.role.as_deref() == Some("radio") {
            "radio"
        } else {
            "checkbox"
        };
        write!(html, " type=\"{input_type}\"").expect("string formatting cannot fail");
    }
    if let Some(mapping) = role_mapping {
        if let Some(role) = mapping.explicit_role {
            write!(html, " role=\"{role}\"").expect("string formatting cannot fail");
        }
        if let Some(name) = &entity.semantics.accessible_name {
            write!(html, " aria-label=\"{}\"", escape_attribute(name))
                .expect("string formatting cannot fail");
        }
        write_states(entity, html);
    }
    if let Some(attributes) = relations.by_source.get(&id) {
        for (attribute, targets) in attributes {
            let value = targets
                .iter()
                .map(|target| html_id(*target))
                .collect::<Vec<_>>()
                .join(" ");
            write!(html, " {attribute}=\"{value}\"").expect("string formatting cannot fail");
        }
    }
    if role_mapping.is_some_and(|mapping| mapping.void) {
        html.push_str(">\n");
        return Ok(());
    }
    html.push('>');
    if let Some(text) = &entity.authored.text {
        html.push_str(&escape_text(&text.content));
    }
    if entity.children.is_empty() {
        writeln!(html, "</{tag}>").expect("string formatting cannot fail");
        return Ok(());
    }
    html.push('\n');
    for child in &entity.children {
        write_entity(document, *child, mapped, relations, depth + 1, html)?;
    }
    writeln!(html, "{indentation}</{tag}>").expect("string formatting cannot fail");
    Ok(())
}

fn write_states(entity: &Entity, html: &mut String) {
    let role = entity.semantics.role.as_deref().unwrap_or_default();
    for (state, value) in &entity.semantics.states {
        match (role, state.as_str(), *value) {
            ("button" | "switch" | "checkbox" | "radio", "disabled", true) => {
                html.push_str(" disabled");
            }
            ("checkbox" | "radio", "checked", true) => html.push_str(" checked"),
            ("checkbox" | "radio", "required", true) => html.push_str(" required"),
            ("button", "expanded" | "pressed", value) => {
                write!(html, " aria-{state}=\"{value}\"").expect("string formatting cannot fail");
            }
            ("switch", "checked", value) => {
                write!(html, " aria-checked=\"{value}\"").expect("string formatting cannot fail");
            }
            _ => {}
        }
    }
}

fn html_id(id: EntityId) -> String {
    format!("nuif-{id}")
}

fn relation_kind(attribute: &str) -> &'static str {
    SUPPORTED_RELATIONS
        .iter()
        .find_map(|(kind, candidate)| (*candidate == attribute).then_some(*kind))
        .expect("only supported relationship attributes are stored")
}

fn escape_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document_with(role: &str, name: Option<&str>) -> Document {
        let mut document = Document::empty(EntityId::new(1));
        let mut entity = Entity::new(EntityId::new(2), EntityKind::Container);
        entity.semantics = Semantics {
            role: Some(role.to_owned()),
            accessible_name: name.map(str::to_owned),
            states: BTreeMap::new(),
        };
        document.roots.push(entity.id);
        document.entities.insert(entity.id, entity);
        document
    }

    #[test]
    fn native_and_aria_roles_are_projected_without_scripts() {
        let mut document = document_with("main", Some("Workspace & tools"));
        let mut button = Entity::new(EntityId::new(3), EntityKind::Container);
        button.semantics.role = Some("button".to_owned());
        button.semantics.accessible_name = Some("Save \"draft\"".to_owned());
        button.semantics.states.insert("pressed".to_owned(), false);
        document
            .entities
            .get_mut(&EntityId::new(2))
            .unwrap()
            .children
            .push(button.id);
        document.entities.insert(button.id, button);
        let projection = project_web_accessibility(&document).unwrap();
        assert!(projection.html.contains("<main"));
        assert!(projection.html.contains(" type=\"button\""));
        assert!(projection.html.contains("aria-pressed=\"false\""));
        assert!(projection.html.contains("Save &quot;draft&quot;"));
        assert!(!projection.html.contains("<script"));
    }

    #[test]
    fn browser_oracle_fixture_projects_every_declared_semantic_node() {
        let fixture = web_accessibility_fixture();
        let projection = project_web_accessibility(&fixture).unwrap();
        assert_eq!(projection.nodes.len(), fixture.entities.len());
        assert_eq!(projection.nodes.len(), 11);
        assert!(projection.html.contains("aria-controls="));
        assert!(projection.html.contains("aria-describedby="));
        assert!(projection.html.contains("aria-flowto="));
    }

    #[test]
    fn labelled_by_is_resolved_and_retained() {
        let mut document = document_with("paragraph", None);
        document
            .entities
            .get_mut(&EntityId::new(2))
            .unwrap()
            .authored
            .text = Some(nuif_core::TextContent {
            content: "  Receive\n  updates  ".to_owned(),
            font: "fixture-font".to_owned(),
            font_sha256: "0".repeat(64),
            font_asset: None,
            size: 16.0,
            line_height: 20.0,
        });
        let mut checkbox = Entity::new(EntityId::new(3), EntityKind::Container);
        checkbox.semantics.role = Some("checkbox".to_owned());
        document.roots.push(checkbox.id);
        document.entities.insert(checkbox.id, checkbox);
        document.relations.push(Relation {
            kind: "labelled-by".to_owned(),
            source: EntityId::new(3),
            target: EntityId::new(2),
        });
        let projection = project_web_accessibility(&document).unwrap();
        let node = projection
            .nodes
            .iter()
            .find(|node| node.entity == EntityId::new(3))
            .unwrap();
        assert_eq!(node.accessible_name.as_deref(), Some("Receive updates"));
        assert!(projection.html.contains(&format!(
            "aria-labelledby=\"{}\"",
            html_id(EntityId::new(2))
        )));
    }

    #[test]
    fn unsupported_and_ambiguous_semantics_fail_closed() {
        assert!(matches!(
            project_web_accessibility(&document_with("dialog", Some("Dialog"))),
            Err(WebAccessibilityError::UnsupportedRole { .. })
        ));
        let mut invalid_state = document_with("button", Some("Save"));
        invalid_state
            .entities
            .get_mut(&EntityId::new(2))
            .unwrap()
            .semantics
            .states
            .insert("checked".to_owned(), true);
        assert!(matches!(
            project_web_accessibility(&invalid_state),
            Err(WebAccessibilityError::UnsupportedState { .. })
        ));
        let mut ambiguous = document_with("button", Some("Save"));
        let mut label = Entity::new(EntityId::new(3), EntityKind::Text);
        label.authored.text = Some(nuif_core::TextContent {
            content: "Other".to_owned(),
            font: "fixture-font".to_owned(),
            font_sha256: "0".repeat(64),
            font_asset: None,
            size: 16.0,
            line_height: 20.0,
        });
        ambiguous.roots.push(label.id);
        ambiguous.entities.insert(label.id, label);
        ambiguous.relations.push(Relation {
            kind: "labelled-by".to_owned(),
            source: EntityId::new(2),
            target: EntityId::new(3),
        });
        assert!(matches!(
            project_web_accessibility(&ambiguous),
            Err(WebAccessibilityError::AmbiguousName { .. })
        ));
    }

    #[test]
    fn every_remaining_profile_failure_is_typed() {
        let fixture = web_accessibility_fixture();

        let mut missing_name = fixture.clone();
        missing_name
            .relations
            .retain(|relation| relation.kind != "labelled-by");
        assert!(matches!(
            project_web_accessibility(&missing_name),
            Err(WebAccessibilityError::MissingName { .. })
        ));

        let mut prohibited_name = fixture.clone();
        prohibited_name
            .entities
            .get_mut(&EntityId::new(0x11))
            .unwrap()
            .semantics
            .accessible_name = Some("Paragraph".to_owned());
        assert!(matches!(
            project_web_accessibility(&prohibited_name),
            Err(WebAccessibilityError::ProhibitedName { .. })
        ));

        let mut unnamed_label = fixture.clone();
        unnamed_label
            .entities
            .get_mut(&EntityId::new(0x11))
            .unwrap()
            .authored
            .text = None;
        assert!(matches!(
            project_web_accessibility(&unnamed_label),
            Err(WebAccessibilityError::UnnamedLabelTarget { .. })
        ));

        let mut duplicate_relation = fixture.clone();
        duplicate_relation
            .relations
            .push(duplicate_relation.relations[0].clone());
        assert!(matches!(
            project_web_accessibility(&duplicate_relation),
            Err(WebAccessibilityError::DuplicateRelation { .. })
        ));

        let mut relation_without_role = fixture.clone();
        relation_without_role
            .entities
            .get_mut(&EntityId::new(0x18))
            .unwrap()
            .semantics
            .role = None;
        assert!(matches!(
            project_web_accessibility(&relation_without_role),
            Err(WebAccessibilityError::RelationSourceWithoutRole { .. })
        ));

        let mut invalid_containment = fixture.clone();
        invalid_containment
            .entities
            .get_mut(&EntityId::new(0x10))
            .unwrap()
            .children
            .retain(|child| *child != EntityId::new(0x11));
        invalid_containment
            .entities
            .get_mut(&EntityId::new(0x12))
            .unwrap()
            .children
            .push(EntityId::new(0x11));
        assert!(matches!(
            project_web_accessibility(&invalid_containment),
            Err(WebAccessibilityError::InvalidContainment { .. })
        ));

        let mut switch_without_checked = fixture;
        switch_without_checked
            .entities
            .get_mut(&EntityId::new(0x15))
            .unwrap()
            .semantics
            .states
            .remove("checked");
        assert!(matches!(
            project_web_accessibility(&switch_without_checked),
            Err(WebAccessibilityError::UnsupportedState { state, .. }) if state == "checked-required"
        ));
    }

    #[test]
    fn normalized_names_and_owned_trees_fail_closed() {
        let fixture = web_accessibility_fixture();
        let mut empty_name = fixture.clone();
        empty_name
            .entities
            .get_mut(&EntityId::new(0x13))
            .unwrap()
            .semantics
            .accessible_name = Some(" \n\t ".to_owned());
        assert!(matches!(
            project_web_accessibility(&empty_name),
            Err(WebAccessibilityError::EmptyName { .. })
        ));

        let mut owned_target_conflict = fixture.clone();
        owned_target_conflict.relations.push(Relation {
            kind: "owns".to_owned(),
            source: EntityId::new(0x10),
            target: EntityId::new(0x1a),
        });
        assert!(matches!(
            project_web_accessibility(&owned_target_conflict),
            Err(WebAccessibilityError::OwnedTargetConflict { .. })
        ));

        let mut owned_tree_cycle = fixture;
        owned_tree_cycle.relations.push(Relation {
            kind: "owns".to_owned(),
            source: EntityId::new(0x1a),
            target: EntityId::new(0x19),
        });
        assert!(matches!(
            project_web_accessibility(&owned_tree_cycle),
            Err(WebAccessibilityError::OwnedTreeCycle { .. })
        ));
    }
}
