#![doc = "Canonical in-memory model and structural validation for NUIF."]

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

pub const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EntityId(#[serde(with = "entity_id_serde")] pub u128);

impl EntityId {
    #[must_use]
    pub const fn new(value: u128) -> Self {
        Self(value)
    }
}

impl fmt::Display for EntityId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:032x}", self.0)
    }
}

impl FromStr for EntityId {
    type Err = ParseEntityIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() != 32 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(ParseEntityIdError);
        }
        u128::from_str_radix(value, 16)
            .map(Self)
            .map_err(|_| ParseEntityIdError)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParseEntityIdError;

impl fmt::Display for ParseEntityIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("entity identifier must contain exactly 32 hexadecimal digits")
    }
}

impl std::error::Error for ParseEntityIdError {}

mod entity_id_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::str::FromStr;

    use super::EntityId;

    pub fn serialize<S: Serializer>(value: &u128, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{value:032x}"))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u128, D::Error> {
        let value = String::deserialize(deserializer)?;
        EntityId::from_str(&value)
            .map(|id| id.0)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub schema_version: u32,
    pub id: EntityId,
    pub entities: BTreeMap<EntityId, Entity>,
    pub roots: Vec<EntityId>,
    #[serde(default)]
    pub tokens: BTreeMap<EntityId, Token>,
    #[serde(default)]
    pub relations: Vec<Relation>,
    #[serde(default)]
    pub extension_declarations: ExtensionDeclarations,
    #[serde(default)]
    pub extensions: Extensions,
}

impl Document {
    #[must_use]
    pub fn empty(id: EntityId) -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            id,
            entities: BTreeMap::new(),
            roots: Vec::new(),
            tokens: BTreeMap::new(),
            relations: Vec::new(),
            extension_declarations: ExtensionDeclarations::default(),
            extensions: Extensions::default(),
        }
    }

    #[must_use]
    pub fn parent_of(&self, child: EntityId) -> Option<EntityId> {
        self.entities
            .iter()
            .find_map(|(id, entity)| entity.children.contains(&child).then_some(*id))
    }

    #[must_use]
    pub fn contains_descendant(&self, ancestor: EntityId, candidate: EntityId) -> bool {
        let mut pending = vec![ancestor];
        let mut visited = BTreeSet::new();
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }
            if id == candidate && id != ancestor {
                return true;
            }
            if let Some(entity) = self.entities.get(&id) {
                pending.extend(entity.children.iter().copied());
            }
        }
        false
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entity {
    pub schema_version: u32,
    pub id: EntityId,
    pub name: Option<String>,
    pub kind: EntityKind,
    #[serde(default)]
    pub children: Vec<EntityId>,
    #[serde(default)]
    pub authored: AuthoredProperties,
    #[serde(default)]
    pub semantics: Semantics,
    #[serde(default)]
    pub extensions: Extensions,
}

impl Entity {
    #[must_use]
    pub fn new(id: EntityId, kind: EntityKind) -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            id,
            name: None,
            kind,
            children: Vec::new(),
            authored: AuthoredProperties::default(),
            semantics: Semantics::default(),
            extensions: Extensions::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "data")]
pub enum EntityKind {
    Surface,
    Container,
    Shape(ShapeKind),
    Text,
    Image,
    Component,
    Instance { component: EntityId },
    Unknown(UnknownKind),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShapeKind {
    Rectangle,
    Ellipse,
    Path,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnknownKind {
    pub namespace: String,
    pub kind: String,
    pub schema_version: u32,
    pub payload: OpaquePayload,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredProperties {
    pub width: SizeIntent,
    pub height: SizeIntent,
    pub position: Point,
    pub layout: LayoutStyle,
    pub fill: Option<Color>,
    pub text: Option<TextContent>,
    #[serde(default)]
    pub responsive: Vec<ResponsiveOverride>,
    #[serde(default)]
    pub values: BTreeMap<String, PropertyValue>,
}

impl Default for AuthoredProperties {
    fn default() -> Self {
        Self {
            width: SizeIntent::Auto,
            height: SizeIntent::Auto,
            position: Point::default(),
            layout: LayoutStyle::default(),
            fill: None,
            text: None,
            responsive: Vec::new(),
            values: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum SizeIntent {
    #[default]
    Auto,
    Fixed(f64),
    Fill,
    Intrinsic,
    Percentage(f64),
    MinContent,
    MaxContent,
    FitContent(f64),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutStyle {
    pub family: LayoutFamily,
    pub direction: FlowDirection,
    pub gap: f64,
    pub padding: Edges,
    pub align: Align,
}

impl Default for LayoutStyle {
    fn default() -> Self {
        Self {
            family: LayoutFamily::Freeform,
            direction: FlowDirection::Row,
            gap: 0.0,
            padding: Edges::default(),
            align: Align::Start,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutFamily {
    #[default]
    Freeform,
    Stack,
    Flex,
    Grid,
    Constraint,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowDirection {
    #[default]
    Row,
    Column,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Align {
    #[default]
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Edges {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResponsiveOverride {
    pub when: ContextPredicate,
    pub direction: Option<FlowDirection>,
    pub gap: Option<f64>,
    pub width: Option<SizeIntent>,
    pub height: Option<SizeIntent>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextPredicate {
    pub min_width: Option<f64>,
    pub max_width: Option<f64>,
    pub theme: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Color {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub alpha: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextContent {
    pub content: String,
    pub font: String,
    pub font_sha256: String,
    pub size: f64,
    pub line_height: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum PropertyValue {
    Null,
    Boolean(bool),
    Integer(i64),
    Real(f64),
    String(String),
    Bytes(#[serde(with = "serde_bytes")] Vec<u8>),
    Array(Vec<Self>),
    Object(BTreeMap<String, Self>),
    Token(EntityId),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Token {
    pub id: EntityId,
    pub name: String,
    pub value: PropertyValue,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Relation {
    pub kind: String,
    pub source: EntityId,
    pub target: EntityId,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Semantics {
    pub role: Option<String>,
    pub accessible_name: Option<String>,
    #[serde(default)]
    pub states: BTreeMap<String, bool>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpaqueEncoding {
    Cbor,
    Octets,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpaquePayload {
    pub encoding: OpaqueEncoding,
    #[serde(with = "serde_bytes")]
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Extensions(pub BTreeMap<String, OpaquePayload>);

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionDeclarations {
    pub used: BTreeSet<String>,
    pub required: BTreeSet<String>,
    pub fallback_kind: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "class")]
pub enum Fidelity {
    Lossless,
    Representable,
    Approximated { reason: String },
    PreservedUnrenderable { namespace: String },
    Unsupported { reason: String },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
    Information,
    Hint,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    pub entity: Option<EntityId>,
    pub pointer: Option<String>,
    pub fidelity: Option<Fidelity>,
}

impl Diagnostic {
    fn error(code: &str, message: impl Into<String>, entity: Option<EntityId>) -> Self {
        Self {
            code: code.to_owned(),
            severity: Severity::Error,
            message: message.into(),
            entity,
            pointer: entity.map(|id| format!("/entities/{id}")),
            fidelity: None,
        }
    }
}

#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "one ordered validation pass keeps cross-entity invariant state explicit"
)]
pub fn validate(document: &Document) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    if document.schema_version > CURRENT_SCHEMA_VERSION {
        diagnostics.push(Diagnostic::error(
            "MODEL_DOCUMENT_VERSION_UNSUPPORTED",
            format!(
                "document schema version {} is newer than supported version {CURRENT_SCHEMA_VERSION}",
                document.schema_version
            ),
            None,
        ));
    }

    let mut parents = BTreeMap::new();
    let mut root_set = BTreeSet::new();
    for root in &document.roots {
        if !root_set.insert(*root) {
            diagnostics.push(Diagnostic::error(
                "MODEL_DUPLICATE_ROOT",
                format!("root {root} appears more than once"),
                Some(*root),
            ));
        }
        if !document.entities.contains_key(root) {
            diagnostics.push(Diagnostic::error(
                "MODEL_ROOT_MISSING",
                format!("root {root} does not exist"),
                Some(*root),
            ));
        }
    }

    for (key, entity) in &document.entities {
        if *key != entity.id {
            diagnostics.push(Diagnostic::error(
                "MODEL_ENTITY_KEY_MISMATCH",
                format!(
                    "entity map key {key} differs from embedded id {}",
                    entity.id
                ),
                Some(*key),
            ));
        }
        if entity.schema_version > CURRENT_SCHEMA_VERSION
            && !matches!(entity.kind, EntityKind::Unknown(_))
        {
            diagnostics.push(Diagnostic::error(
                "MODEL_ENTITY_VERSION_NOT_OPAQUE",
                "a newer entity schema version must be represented as unknown",
                Some(entity.id),
            ));
        }
        let mut local = BTreeSet::new();
        for child in &entity.children {
            if !local.insert(*child) {
                diagnostics.push(Diagnostic::error(
                    "MODEL_DUPLICATE_CHILD",
                    format!("child {child} appears more than once"),
                    Some(entity.id),
                ));
            }
            if !document.entities.contains_key(child) {
                diagnostics.push(Diagnostic::error(
                    "MODEL_CHILD_MISSING",
                    format!("child {child} does not exist"),
                    Some(entity.id),
                ));
            }
            if let Some(previous) = parents.insert(*child, entity.id) {
                diagnostics.push(Diagnostic::error(
                    "MODEL_MULTIPLE_PARENTS",
                    format!("child {child} belongs to both {previous} and {}", entity.id),
                    Some(*child),
                ));
            }
        }
        validate_entity(document, entity, &mut diagnostics);
    }

    for root in &document.roots {
        if parents.contains_key(root) {
            diagnostics.push(Diagnostic::error(
                "MODEL_ROOT_HAS_PARENT",
                format!("root {root} also has a parent"),
                Some(*root),
            ));
        }
    }

    let mut reached = BTreeSet::new();
    let mut visiting = BTreeSet::new();
    for root in &document.roots {
        visit(
            document,
            *root,
            &mut reached,
            &mut visiting,
            &mut diagnostics,
        );
    }
    for id in document.entities.keys() {
        if !reached.contains(id) {
            diagnostics.push(Diagnostic::error(
                "MODEL_ENTITY_UNREACHABLE",
                format!("entity {id} is not reachable from a root"),
                Some(*id),
            ));
        }
    }
    // A root-only traversal cannot observe cycles in detached subgraphs. Seed
    // the completed set with reachable entities, then inspect every remaining
    // component so malformed unreachable content cannot evade cycle checks.
    let mut cycle_checked = reached;
    let mut cycle_visiting = BTreeSet::new();
    for id in document.entities.keys() {
        visit(
            document,
            *id,
            &mut cycle_checked,
            &mut cycle_visiting,
            &mut diagnostics,
        );
    }

    for relation in &document.relations {
        if !is_identifier(&relation.kind) {
            diagnostics.push(Diagnostic::error(
                "MODEL_IDENTIFIER_INVALID",
                format!(
                    "relation kind {:?} is not a lowercase NUIF identifier",
                    relation.kind
                ),
                Some(relation.source),
            ));
        }
        if !document.entities.contains_key(&relation.source)
            || !document.entities.contains_key(&relation.target)
        {
            diagnostics.push(Diagnostic::error(
                "MODEL_RELATION_TARGET_MISSING",
                format!("relation {} has a missing endpoint", relation.kind),
                Some(relation.source),
            ));
        }
    }

    for (key, token) in &document.tokens {
        if *key != token.id {
            diagnostics.push(Diagnostic::error(
                "MODEL_TOKEN_KEY_MISMATCH",
                format!("token map key {key} differs from embedded id {}", token.id),
                None,
            ));
        }
        validate_property_value(document, None, &token.value, &mut diagnostics);
    }

    for namespace in &document.extension_declarations.required {
        if !document.extension_declarations.used.contains(namespace) {
            diagnostics.push(Diagnostic::error(
                "EXTENSION_REQUIRED_NOT_USED",
                format!("required namespace {namespace} is absent from extensions_used"),
                None,
            ));
        }
    }
    for namespace in document
        .extension_declarations
        .used
        .iter()
        .chain(document.extension_declarations.required.iter())
        .chain(document.extension_declarations.fallback_kind.keys())
    {
        if !is_identifier(namespace) {
            diagnostics.push(Diagnostic::error(
                "EXTENSION_NAMESPACE_INVALID",
                format!("extension namespace {namespace:?} is not a lowercase NUIF identifier"),
                None,
            ));
        }
    }
    for namespace in &document.extension_declarations.used {
        diagnostics.push(Diagnostic {
            code: "EXTENSION_UNSUPPORTED".to_owned(),
            severity: Severity::Information,
            message: format!(
                "reference profile 0 preserves namespace {namespace} but does not interpret it"
            ),
            entity: None,
            pointer: Some("/extension_declarations/used".to_owned()),
            fidelity: Some(Fidelity::PreservedUnrenderable {
                namespace: namespace.clone(),
            }),
        });
    }
    for namespace in &document.extension_declarations.required {
        diagnostics.push(Diagnostic {
            code: "EXTENSION_REQUIRED_UNSUPPORTED".to_owned(),
            severity: Severity::Warning,
            message: format!(
                "required namespace {namespace} is not interpreted; structural editing remains available"
            ),
            entity: None,
            pointer: Some("/extension_declarations/required".to_owned()),
            fidelity: Some(Fidelity::PreservedUnrenderable {
                namespace: namespace.clone(),
            }),
        });
    }
    for (namespace, fallback_kind) in &document.extension_declarations.fallback_kind {
        if !document.extension_declarations.used.contains(namespace) {
            diagnostics.push(Diagnostic::error(
                "EXTENSION_FALLBACK_NOT_USED",
                format!("fallback namespace {namespace} is absent from extensions_used"),
                None,
            ));
        }
        if !is_identifier(fallback_kind) {
            diagnostics.push(Diagnostic::error(
                "MODEL_IDENTIFIER_INVALID",
                format!("fallback kind {fallback_kind:?} is not a lowercase NUIF identifier"),
                None,
            ));
        }
    }
    validate_extensions(
        &document.extension_declarations.used,
        &document.extensions,
        None,
        &mut diagnostics,
    );
    diagnostics
}

fn validate_entity(document: &Document, entity: &Entity, diagnostics: &mut Vec<Diagnostic>) {
    validate_authored_numbers(entity, diagnostics);
    validate_entity_identifiers(document, entity, diagnostics);
    if let EntityKind::Instance { component } = entity.kind
        && !matches!(
            document.entities.get(&component).map(|item| &item.kind),
            Some(EntityKind::Component)
        )
    {
        diagnostics.push(Diagnostic::error(
            "MODEL_COMPONENT_MISSING",
            format!("instance references missing or non-component entity {component}"),
            Some(entity.id),
        ));
    }
    validate_property_values(document, entity, diagnostics);
    validate_extensions(
        &document.extension_declarations.used,
        &entity.extensions,
        Some(entity.id),
        diagnostics,
    );
}

fn validate_authored_numbers(entity: &Entity, diagnostics: &mut Vec<Diagnostic>) {
    let mut authored_numbers = vec![
        entity.authored.position.x,
        entity.authored.position.y,
        entity.authored.layout.gap,
        entity.authored.layout.padding.top,
        entity.authored.layout.padding.right,
        entity.authored.layout.padding.bottom,
        entity.authored.layout.padding.left,
    ];
    authored_numbers.extend(size_number(&entity.authored.width));
    authored_numbers.extend(size_number(&entity.authored.height));
    if let Some(fill) = entity.authored.fill {
        authored_numbers.extend([
            f64::from(fill.red),
            f64::from(fill.green),
            f64::from(fill.blue),
            f64::from(fill.alpha),
        ]);
    }
    if let Some(text) = &entity.authored.text {
        authored_numbers.extend([text.size, text.line_height]);
    }
    for responsive in &entity.authored.responsive {
        authored_numbers.extend(responsive.when.min_width);
        authored_numbers.extend(responsive.when.max_width);
        authored_numbers.extend(responsive.gap);
        authored_numbers.extend(responsive.width.as_ref().and_then(size_number));
        authored_numbers.extend(responsive.height.as_ref().and_then(size_number));
        if let (Some(minimum), Some(maximum)) =
            (responsive.when.min_width, responsive.when.max_width)
            && minimum > maximum
        {
            diagnostics.push(Diagnostic::error(
                "MODEL_RESPONSIVE_RANGE_INVALID",
                "responsive min_width must not exceed max_width",
                Some(entity.id),
            ));
        }
    }
    for value in authored_numbers {
        if !value.is_finite() {
            diagnostics.push(Diagnostic::error(
                "MODEL_NON_FINITE_NUMBER",
                "authored numeric values must be finite",
                Some(entity.id),
            ));
        }
    }
}

fn validate_entity_identifiers(
    document: &Document,
    entity: &Entity,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let EntityKind::Unknown(unknown) = &entity.kind {
        if !is_identifier(&unknown.namespace) {
            diagnostics.push(Diagnostic::error(
                "UNKNOWN_NAMESPACE_INVALID",
                format!(
                    "unknown-kind namespace {:?} is not a lowercase NUIF identifier",
                    unknown.namespace
                ),
                Some(entity.id),
            ));
        }
        if !document
            .extension_declarations
            .used
            .contains(&unknown.namespace)
        {
            diagnostics.push(Diagnostic::error(
                "UNKNOWN_NAMESPACE_UNDECLARED",
                format!(
                    "unknown-kind namespace {} is absent from extensions_used",
                    unknown.namespace
                ),
                Some(entity.id),
            ));
        }
        if !is_identifier(&unknown.kind) {
            diagnostics.push(Diagnostic::error(
                "MODEL_IDENTIFIER_INVALID",
                format!(
                    "unknown kind {:?} is not a lowercase NUIF identifier",
                    unknown.kind
                ),
                Some(entity.id),
            ));
        }
    }
    for key in entity
        .authored
        .values
        .keys()
        .chain(entity.semantics.states.keys())
    {
        if !is_identifier(key) {
            diagnostics.push(Diagnostic::error(
                "MODEL_IDENTIFIER_INVALID",
                format!("property key {key:?} is not a lowercase NUIF identifier"),
                Some(entity.id),
            ));
        }
    }
    if let Some(role) = &entity.semantics.role
        && !is_identifier(role)
    {
        diagnostics.push(Diagnostic::error(
            "MODEL_IDENTIFIER_INVALID",
            format!("semantic role {role:?} is not a lowercase NUIF identifier"),
            Some(entity.id),
        ));
    }
}

fn size_number(intent: &SizeIntent) -> Option<f64> {
    match intent {
        SizeIntent::Fixed(value)
        | SizeIntent::Percentage(value)
        | SizeIntent::FitContent(value) => Some(*value),
        SizeIntent::Auto
        | SizeIntent::Fill
        | SizeIntent::Intrinsic
        | SizeIntent::MinContent
        | SizeIntent::MaxContent => None,
    }
}

fn validate_property_values(
    document: &Document,
    entity: &Entity,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for value in entity.authored.values.values() {
        validate_property_value(document, Some(entity.id), value, diagnostics);
    }
}

fn validate_property_value(
    document: &Document,
    entity: Option<EntityId>,
    value: &PropertyValue,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match value {
        PropertyValue::Real(real) if !real.is_finite() => diagnostics.push(Diagnostic::error(
            "MODEL_NON_FINITE_NUMBER",
            "real property values must be finite",
            entity,
        )),
        PropertyValue::Token(id) if !document.tokens.contains_key(id) => {
            diagnostics.push(Diagnostic::error(
                "MODEL_TOKEN_MISSING",
                format!("token reference {id} does not exist"),
                entity,
            ));
        }
        PropertyValue::Array(values) => {
            for value in values {
                validate_property_value(document, entity, value, diagnostics);
            }
        }
        PropertyValue::Object(values) => {
            for (key, value) in values {
                if !is_identifier(key) {
                    diagnostics.push(Diagnostic::error(
                        "MODEL_IDENTIFIER_INVALID",
                        format!("property key {key:?} is not a lowercase NUIF identifier"),
                        entity,
                    ));
                }
                validate_property_value(document, entity, value, diagnostics);
            }
        }
        _ => {}
    }
}

fn validate_extensions(
    used: &BTreeSet<String>,
    extensions: &Extensions,
    entity: Option<EntityId>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for namespace in extensions.0.keys() {
        if !is_identifier(namespace) {
            diagnostics.push(Diagnostic::error(
                "EXTENSION_NAMESPACE_INVALID",
                format!("extension namespace {namespace:?} is not a lowercase NUIF identifier"),
                entity,
            ));
        }
        if !used.contains(namespace) {
            diagnostics.push(Diagnostic::error(
                "EXTENSION_UNDECLARED",
                format!("extension namespace {namespace} is absent from extensions_used"),
                entity,
            ));
        }
    }
}

fn visit(
    document: &Document,
    id: EntityId,
    reached: &mut BTreeSet<EntityId>,
    visiting: &mut BTreeSet<EntityId>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !visiting.insert(id) {
        diagnostics.push(Diagnostic::error(
            "MODEL_CONTAINMENT_CYCLE",
            format!("containment cycle includes {id}"),
            Some(id),
        ));
        return;
    }
    if reached.insert(id)
        && let Some(entity) = document.entities.get(&id)
    {
        for child in &entity.children {
            visit(document, *child, reached, visiting, diagnostics);
        }
    }
    visiting.remove(&id);
}

#[must_use]
pub fn is_identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    (first.is_ascii_lowercase() || first.is_ascii_digit())
        && bytes.all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'_' | b'.' | b':' | b'-')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_have_stable_text_form() {
        let id = EntityId::new(0x1234);
        assert_eq!(id.to_string(), "00000000000000000000000000001234");
        assert_eq!(id.to_string().parse(), Ok(id));
    }

    #[test]
    fn validation_finds_unreachable_entities_and_undeclared_extensions() {
        let mut document = Document::empty(EntityId::new(1));
        let mut entity = Entity::new(EntityId::new(2), EntityKind::Container);
        entity.extensions.0.insert(
            "vendor.probe".to_owned(),
            OpaquePayload {
                encoding: OpaqueEncoding::Octets,
                bytes: vec![1, 2, 3],
            },
        );
        document.entities.insert(entity.id, entity);
        let codes = validate(&document)
            .into_iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<BTreeSet<_>>();
        assert!(codes.contains("MODEL_ENTITY_UNREACHABLE"));
        assert!(codes.contains("EXTENSION_UNDECLARED"));
    }

    #[test]
    fn identifier_grammar_is_lowercase() {
        assert!(is_identifier("vendor.probe-1"));
        assert!(!is_identifier("VENDOR_probe"));
        assert!(!is_identifier("-bad"));
    }

    #[test]
    fn validation_finds_cycles_in_unreachable_subgraphs() {
        let mut document = Document::empty(EntityId::new(1));
        let root = Entity::new(EntityId::new(2), EntityKind::Container);
        document.roots.push(root.id);
        document.entities.insert(root.id, root);
        let mut first = Entity::new(EntityId::new(3), EntityKind::Container);
        let mut second = Entity::new(EntityId::new(4), EntityKind::Container);
        first.children.push(second.id);
        second.children.push(first.id);
        document.entities.insert(first.id, first);
        document.entities.insert(second.id, second);

        let codes = validate(&document)
            .into_iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<BTreeSet<_>>();
        assert!(codes.contains("MODEL_ENTITY_UNREACHABLE"));
        assert!(codes.contains("MODEL_CONTAINMENT_CYCLE"));
    }

    #[test]
    fn validation_covers_nested_numbers_tokens_and_identifiers() {
        let mut document = Document::empty(EntityId::new(1));
        let mut root = Entity::new(EntityId::new(2), EntityKind::Container);
        root.authored.width = SizeIntent::Fixed(f64::INFINITY);
        root.authored.values.insert(
            "BadKey".to_owned(),
            PropertyValue::Object(BTreeMap::from([(
                "alsoBad".to_owned(),
                PropertyValue::Real(1.0),
            )])),
        );
        document.roots.push(root.id);
        document.entities.insert(root.id, root);
        let token_id = EntityId::new(3);
        document.tokens.insert(
            token_id,
            Token {
                id: token_id,
                name: "bad numeric token".to_owned(),
                value: PropertyValue::Real(f64::NAN),
            },
        );

        let codes = validate(&document)
            .into_iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>();
        assert!(
            codes
                .iter()
                .filter(|code| *code == "MODEL_NON_FINITE_NUMBER")
                .count()
                >= 2
        );
        assert!(
            codes
                .iter()
                .filter(|code| *code == "MODEL_IDENTIFIER_INVALID")
                .count()
                >= 2
        );
    }
}
