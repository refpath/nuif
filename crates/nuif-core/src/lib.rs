#![doc = "Canonical in-memory model and structural validation for NUIF."]

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Resource limits for the executable profile-0 model.
///
/// These limits bound work after syntax parsing. Encoded byte and syntax-depth
/// limits live in `nuif-codec`, where they can be enforced before allocation.
pub const PROFILE0_RESOURCE_LIMITS: ResourceLimits = ResourceLimits {
    entities: 8_192,
    roots: 4_096,
    tokens: 8_192,
    relations: 32_768,
    child_references: 8_191,
    responsive_overrides: 16_384,
    property_values: 65_536,
    property_depth: 24,
    containment_depth: 128,
    binary_bytes: 8 * 1024 * 1024,
    string_bytes: 8 * 1024 * 1024,
    single_string_bytes: 1024 * 1024,
};

/// Cardinality and retained-data bounds for one decoded profile-0 document.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceLimits {
    pub entities: usize,
    pub roots: usize,
    pub tokens: usize,
    pub relations: usize,
    pub child_references: usize,
    pub responsive_overrides: usize,
    pub property_values: usize,
    pub property_depth: usize,
    pub containment_depth: usize,
    pub binary_bytes: usize,
    pub string_bytes: usize,
    pub single_string_bytes: usize,
}

/// Measured semantic size of a decoded document.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub struct ResourceUsage {
    pub entities: usize,
    pub roots: usize,
    pub tokens: usize,
    pub relations: usize,
    pub child_references: usize,
    pub responsive_overrides: usize,
    pub property_values: usize,
    pub property_depth: usize,
    pub containment_depth: usize,
    pub binary_bytes: usize,
    pub string_bytes: usize,
}

/// A semantic-model resource bound exceeded by an untrusted document.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceLimitExceeded {
    pub resource: &'static str,
    pub limit: usize,
    pub observed: usize,
}

impl fmt::Display for ResourceLimitExceeded {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} limit {} exceeded by observed value {}",
            self.resource, self.limit, self.observed
        )
    }
}

impl std::error::Error for ResourceLimitExceeded {}

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

const MAX_VALIDATION_DIAGNOSTICS: usize = 1024;

trait DiagnosticSink {
    fn push_capped(&mut self, diagnostic: Diagnostic);
}

impl DiagnosticSink for Vec<Diagnostic> {
    fn push_capped(&mut self, diagnostic: Diagnostic) {
        if self.len() < MAX_VALIDATION_DIAGNOSTICS {
            self.push(diagnostic);
        } else if self.len() == MAX_VALIDATION_DIAGNOSTICS {
            self.push(Diagnostic::error(
                "VALIDATION_DIAGNOSTICS_TRUNCATED",
                format!(
                    "validation stopped retaining issues after {MAX_VALIDATION_DIAGNOSTICS} diagnostics"
                ),
                None,
            ));
        }
    }
}

/// Measures and enforces the executable profile-0 semantic resource limits.
///
/// The walk is iterative for recursively nestable property values. Containment
/// depth is derived from the first observed parent of each entity; documents
/// with multiple parents are rejected by structural validation before layout.
///
/// # Errors
///
/// Returns the first exceeded bound together with its limit and observed size.
pub fn resource_usage(document: &Document) -> Result<ResourceUsage, ResourceLimitExceeded> {
    let limits = PROFILE0_RESOURCE_LIMITS;
    let mut usage = ResourceUsage {
        entities: document.entities.len(),
        roots: document.roots.len(),
        tokens: document.tokens.len(),
        relations: document.relations.len(),
        ..ResourceUsage::default()
    };
    check_limit("entities", usage.entities, limits.entities)?;
    check_limit("roots", usage.roots, limits.roots)?;
    check_limit("tokens", usage.tokens, limits.tokens)?;
    check_limit("relations", usage.relations, limits.relations)?;

    let mut parents = BTreeMap::new();
    for entity in document.entities.values() {
        add_optional_string(&mut usage, entity.name.as_deref(), limits)?;
        add_count(
            &mut usage.child_references,
            entity.children.len(),
            "child references",
            limits.child_references,
        )?;
        for child in &entity.children {
            parents.entry(*child).or_insert(entity.id);
        }
        add_count(
            &mut usage.responsive_overrides,
            entity.authored.responsive.len(),
            "responsive overrides",
            limits.responsive_overrides,
        )?;
        if let Some(text) = &entity.authored.text {
            add_string(&mut usage, &text.content, limits)?;
            add_string(&mut usage, &text.font, limits)?;
            add_string(&mut usage, &text.font_sha256, limits)?;
        }
        for override_value in &entity.authored.responsive {
            add_optional_string(&mut usage, override_value.when.theme.as_deref(), limits)?;
        }
        for (key, value) in &entity.authored.values {
            add_string(&mut usage, key, limits)?;
            inspect_property_value(&mut usage, value, limits)?;
        }
        add_optional_string(&mut usage, entity.semantics.role.as_deref(), limits)?;
        add_optional_string(
            &mut usage,
            entity.semantics.accessible_name.as_deref(),
            limits,
        )?;
        for key in entity.semantics.states.keys() {
            add_string(&mut usage, key, limits)?;
        }
        if let EntityKind::Unknown(unknown) = &entity.kind {
            add_string(&mut usage, &unknown.namespace, limits)?;
            add_string(&mut usage, &unknown.kind, limits)?;
            add_binary(&mut usage, unknown.payload.bytes.len(), limits)?;
        }
        inspect_extensions(&mut usage, &entity.extensions, limits)?;
    }

    for token in document.tokens.values() {
        add_string(&mut usage, &token.name, limits)?;
        inspect_property_value(&mut usage, &token.value, limits)?;
    }
    for relation in &document.relations {
        add_string(&mut usage, &relation.kind, limits)?;
    }
    for namespace in document
        .extension_declarations
        .used
        .iter()
        .chain(document.extension_declarations.required.iter())
    {
        add_string(&mut usage, namespace, limits)?;
    }
    for (namespace, fallback) in &document.extension_declarations.fallback_kind {
        add_string(&mut usage, namespace, limits)?;
        add_string(&mut usage, fallback, limits)?;
    }
    inspect_extensions(&mut usage, &document.extensions, limits)?;

    for id in document.entities.keys() {
        let mut current = Some(*id);
        let mut path = BTreeSet::new();
        let mut depth = 0_usize;
        while let Some(item) = current {
            if !path.insert(item) {
                break;
            }
            depth = depth.saturating_add(1);
            usage.containment_depth = usage.containment_depth.max(depth);
            check_limit(
                "containment depth",
                usage.containment_depth,
                limits.containment_depth,
            )?;
            current = parents.get(&item).copied();
        }
    }
    Ok(usage)
}

fn inspect_property_value(
    usage: &mut ResourceUsage,
    root: &PropertyValue,
    limits: ResourceLimits,
) -> Result<(), ResourceLimitExceeded> {
    let mut pending = vec![(root, 1_usize)];
    while let Some((value, depth)) = pending.pop() {
        usage.property_values = usage.property_values.saturating_add(1);
        check_limit(
            "property values",
            usage.property_values,
            limits.property_values,
        )?;
        usage.property_depth = usage.property_depth.max(depth);
        check_limit(
            "property depth",
            usage.property_depth,
            limits.property_depth,
        )?;
        match value {
            PropertyValue::String(value) => add_string(usage, value, limits)?,
            PropertyValue::Bytes(value) => add_binary(usage, value.len(), limits)?,
            PropertyValue::Array(values) => {
                pending.extend(values.iter().map(|value| (value, depth.saturating_add(1))));
            }
            PropertyValue::Object(values) => {
                for (key, value) in values {
                    add_string(usage, key, limits)?;
                    pending.push((value, depth.saturating_add(1)));
                }
            }
            PropertyValue::Null
            | PropertyValue::Boolean(_)
            | PropertyValue::Integer(_)
            | PropertyValue::Real(_)
            | PropertyValue::Token(_) => {}
        }
    }
    Ok(())
}

fn inspect_extensions(
    usage: &mut ResourceUsage,
    extensions: &Extensions,
    limits: ResourceLimits,
) -> Result<(), ResourceLimitExceeded> {
    for (namespace, payload) in &extensions.0 {
        add_string(usage, namespace, limits)?;
        add_binary(usage, payload.bytes.len(), limits)?;
    }
    Ok(())
}

fn add_optional_string(
    usage: &mut ResourceUsage,
    value: Option<&str>,
    limits: ResourceLimits,
) -> Result<(), ResourceLimitExceeded> {
    value.map_or(Ok(()), |value| add_string(usage, value, limits))
}

fn add_string(
    usage: &mut ResourceUsage,
    value: &str,
    limits: ResourceLimits,
) -> Result<(), ResourceLimitExceeded> {
    check_limit(
        "single string bytes",
        value.len(),
        limits.single_string_bytes,
    )?;
    add_count(
        &mut usage.string_bytes,
        value.len(),
        "string bytes",
        limits.string_bytes,
    )
}

fn add_binary(
    usage: &mut ResourceUsage,
    bytes: usize,
    limits: ResourceLimits,
) -> Result<(), ResourceLimitExceeded> {
    add_count(
        &mut usage.binary_bytes,
        bytes,
        "binary bytes",
        limits.binary_bytes,
    )
}

fn add_count(
    value: &mut usize,
    increment: usize,
    resource: &'static str,
    limit: usize,
) -> Result<(), ResourceLimitExceeded> {
    *value = value.saturating_add(increment);
    check_limit(resource, *value, limit)
}

fn check_limit(
    resource: &'static str,
    observed: usize,
    limit: usize,
) -> Result<(), ResourceLimitExceeded> {
    if observed > limit {
        Err(ResourceLimitExceeded {
            resource,
            limit,
            observed,
        })
    } else {
        Ok(())
    }
}

#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "one ordered validation pass keeps cross-entity invariant state explicit"
)]
pub fn validate(document: &Document) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    if let Err(error) = resource_usage(document) {
        diagnostics.push_capped(Diagnostic::error(
            "MODEL_RESOURCE_LIMIT_EXCEEDED",
            error.to_string(),
            None,
        ));
        return diagnostics;
    }
    if document.schema_version > CURRENT_SCHEMA_VERSION {
        diagnostics.push_capped(Diagnostic::error(
                "MODEL_DOCUMENT_VERSION_UNSUPPORTED",
                format!(
                    "document schema version {} is newer than supported version {CURRENT_SCHEMA_VERSION}",
                    document.schema_version
                ),
                None,
            ),
        );
    }

    let mut parents = BTreeMap::new();
    let mut root_set = BTreeSet::new();
    for root in &document.roots {
        if !root_set.insert(*root) {
            diagnostics.push_capped(Diagnostic::error(
                "MODEL_DUPLICATE_ROOT",
                format!("root {root} appears more than once"),
                Some(*root),
            ));
        }
        if !document.entities.contains_key(root) {
            diagnostics.push_capped(Diagnostic::error(
                "MODEL_ROOT_MISSING",
                format!("root {root} does not exist"),
                Some(*root),
            ));
        }
    }

    for (key, entity) in &document.entities {
        if *key != entity.id {
            diagnostics.push_capped(Diagnostic::error(
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
            diagnostics.push_capped(Diagnostic::error(
                "MODEL_ENTITY_VERSION_NOT_OPAQUE",
                "a newer entity schema version must be represented as unknown",
                Some(entity.id),
            ));
        }
        let mut local = BTreeSet::new();
        for child in &entity.children {
            if !local.insert(*child) {
                diagnostics.push_capped(Diagnostic::error(
                    "MODEL_DUPLICATE_CHILD",
                    format!("child {child} appears more than once"),
                    Some(entity.id),
                ));
            }
            if !document.entities.contains_key(child) {
                diagnostics.push_capped(Diagnostic::error(
                    "MODEL_CHILD_MISSING",
                    format!("child {child} does not exist"),
                    Some(entity.id),
                ));
            }
            if let Some(previous) = parents.insert(*child, entity.id) {
                diagnostics.push_capped(Diagnostic::error(
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
            diagnostics.push_capped(Diagnostic::error(
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
            diagnostics.push_capped(Diagnostic::error(
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
            diagnostics.push_capped(Diagnostic::error(
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
            diagnostics.push_capped(Diagnostic::error(
                "MODEL_RELATION_TARGET_MISSING",
                format!("relation {} has a missing endpoint", relation.kind),
                Some(relation.source),
            ));
        }
    }

    for (key, token) in &document.tokens {
        if *key != token.id {
            diagnostics.push_capped(Diagnostic::error(
                "MODEL_TOKEN_KEY_MISMATCH",
                format!("token map key {key} differs from embedded id {}", token.id),
                None,
            ));
        }
        validate_property_value(document, None, &token.value, &mut diagnostics);
    }

    for namespace in &document.extension_declarations.required {
        if !document.extension_declarations.used.contains(namespace) {
            diagnostics.push_capped(Diagnostic::error(
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
            diagnostics.push_capped(Diagnostic::error(
                "EXTENSION_NAMESPACE_INVALID",
                format!("extension namespace {namespace:?} is not a lowercase NUIF identifier"),
                None,
            ));
        }
    }
    for namespace in &document.extension_declarations.used {
        diagnostics.push_capped(Diagnostic {
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
        diagnostics.push_capped(Diagnostic {
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
            },
        );
    }
    for (namespace, fallback_kind) in &document.extension_declarations.fallback_kind {
        if !document.extension_declarations.used.contains(namespace) {
            diagnostics.push_capped(Diagnostic::error(
                "EXTENSION_FALLBACK_NOT_USED",
                format!("fallback namespace {namespace} is absent from extensions_used"),
                None,
            ));
        }
        if !is_identifier(fallback_kind) {
            diagnostics.push_capped(Diagnostic::error(
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
        diagnostics.push_capped(Diagnostic::error(
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
            diagnostics.push_capped(Diagnostic::error(
                "MODEL_RESPONSIVE_RANGE_INVALID",
                "responsive min_width must not exceed max_width",
                Some(entity.id),
            ));
        }
    }
    for value in authored_numbers {
        if !value.is_finite() {
            diagnostics.push_capped(Diagnostic::error(
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
            diagnostics.push_capped(Diagnostic::error(
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
            diagnostics.push_capped(Diagnostic::error(
                "UNKNOWN_NAMESPACE_UNDECLARED",
                format!(
                    "unknown-kind namespace {} is absent from extensions_used",
                    unknown.namespace
                ),
                Some(entity.id),
            ));
        }
        if !is_identifier(&unknown.kind) {
            diagnostics.push_capped(Diagnostic::error(
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
            diagnostics.push_capped(Diagnostic::error(
                "MODEL_IDENTIFIER_INVALID",
                format!("property key {key:?} is not a lowercase NUIF identifier"),
                Some(entity.id),
            ));
        }
    }
    if let Some(role) = &entity.semantics.role
        && !is_identifier(role)
    {
        diagnostics.push_capped(Diagnostic::error(
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
        PropertyValue::Real(real) if !real.is_finite() => {
            diagnostics.push_capped(Diagnostic::error(
                "MODEL_NON_FINITE_NUMBER",
                "real property values must be finite",
                entity,
            ));
        }
        PropertyValue::Token(id) if !document.tokens.contains_key(id) => {
            diagnostics.push_capped(Diagnostic::error(
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
                    diagnostics.push_capped(Diagnostic::error(
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
            diagnostics.push_capped(Diagnostic::error(
                "EXTENSION_NAMESPACE_INVALID",
                format!("extension namespace {namespace:?} is not a lowercase NUIF identifier"),
                entity,
            ));
        }
        if !used.contains(namespace) {
            diagnostics.push_capped(Diagnostic::error(
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
    enum Frame {
        Enter(EntityId),
        Exit(EntityId),
    }

    let mut stack = vec![Frame::Enter(id)];
    while let Some(frame) = stack.pop() {
        match frame {
            Frame::Enter(current) => {
                if visiting.contains(&current) {
                    diagnostics.push_capped(Diagnostic::error(
                        "MODEL_CONTAINMENT_CYCLE",
                        format!("containment cycle includes {current}"),
                        Some(current),
                    ));
                    continue;
                }
                if !reached.insert(current) {
                    continue;
                }
                visiting.insert(current);
                stack.push(Frame::Exit(current));
                if let Some(entity) = document.entities.get(&current) {
                    stack.extend(entity.children.iter().rev().copied().map(Frame::Enter));
                }
            }
            Frame::Exit(current) => {
                visiting.remove(&current);
            }
        }
    }
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

    #[test]
    fn resource_usage_rejects_recursive_property_and_containment_depth() {
        let mut nested = PropertyValue::Null;
        for _ in 0..PROFILE0_RESOURCE_LIMITS.property_depth {
            nested = PropertyValue::Array(vec![nested]);
        }
        let mut property_document = Document::empty(EntityId::new(1));
        let mut property_root = Entity::new(EntityId::new(2), EntityKind::Container);
        property_root
            .authored
            .values
            .insert("nested".to_owned(), nested);
        property_document.roots.push(property_root.id);
        property_document
            .entities
            .insert(property_root.id, property_root);
        assert_eq!(
            resource_usage(&property_document).unwrap_err().resource,
            "property depth"
        );

        let mut tree_document = Document::empty(EntityId::new(1));
        for index in 0..=PROFILE0_RESOURCE_LIMITS.containment_depth {
            let id = EntityId::new(u128::try_from(index + 2).unwrap());
            let mut entity = Entity::new(id, EntityKind::Container);
            if index < PROFILE0_RESOURCE_LIMITS.containment_depth {
                entity
                    .children
                    .push(EntityId::new(u128::try_from(index + 3).unwrap()));
            }
            if index == 0 {
                tree_document.roots.push(id);
            }
            tree_document.entities.insert(id, entity);
        }
        assert_eq!(
            resource_usage(&tree_document).unwrap_err().resource,
            "containment depth"
        );
    }

    #[test]
    fn validation_caps_retained_diagnostics() {
        let mut document = Document::empty(EntityId::new(1));
        document.roots = vec![EntityId::new(2); MAX_VALIDATION_DIAGNOSTICS * 2];
        let diagnostics = validate(&document);
        assert_eq!(diagnostics.len(), MAX_VALIDATION_DIAGNOSTICS + 1);
        assert_eq!(
            diagnostics.last().map(|item| item.code.as_str()),
            Some("VALIDATION_DIAGNOSTICS_TRUNCATED")
        );
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one regression enumerates every public profile resource field"
    )]
    fn resource_usage_enforces_every_cardinality_and_retained_data_limit() {
        let mut document = Document::empty(EntityId::new(1));
        for index in 0..=PROFILE0_RESOURCE_LIMITS.entities {
            let id = EntityId::new(u128::try_from(index + 2).unwrap());
            document
                .entities
                .insert(id, Entity::new(id, EntityKind::Container));
        }
        assert_eq!(resource_usage(&document).unwrap_err().resource, "entities");

        let mut document = Document::empty(EntityId::new(1));
        document.roots = vec![EntityId::new(2); PROFILE0_RESOURCE_LIMITS.roots + 1];
        assert_eq!(resource_usage(&document).unwrap_err().resource, "roots");

        let mut document = Document::empty(EntityId::new(1));
        for index in 0..=PROFILE0_RESOURCE_LIMITS.tokens {
            let id = EntityId::new(u128::try_from(index + 2).unwrap());
            document.tokens.insert(
                id,
                Token {
                    id,
                    name: String::new(),
                    value: PropertyValue::Null,
                },
            );
        }
        assert_eq!(resource_usage(&document).unwrap_err().resource, "tokens");

        let mut document = Document::empty(EntityId::new(1));
        document.relations = vec![
            Relation {
                kind: "probe".to_owned(),
                source: EntityId::new(2),
                target: EntityId::new(2),
            };
            PROFILE0_RESOURCE_LIMITS.relations + 1
        ];
        assert_eq!(resource_usage(&document).unwrap_err().resource, "relations");

        let mut document = Document::empty(EntityId::new(1));
        let mut root = Entity::new(EntityId::new(2), EntityKind::Container);
        root.children = vec![EntityId::new(3); PROFILE0_RESOURCE_LIMITS.child_references + 1];
        document.entities.insert(root.id, root);
        assert_eq!(
            resource_usage(&document).unwrap_err().resource,
            "child references"
        );

        let mut document = Document::empty(EntityId::new(1));
        let mut root = Entity::new(EntityId::new(2), EntityKind::Container);
        root.authored.responsive = vec![
            ResponsiveOverride {
                when: ContextPredicate {
                    min_width: None,
                    max_width: None,
                    theme: None,
                },
                direction: None,
                gap: None,
                width: None,
                height: None,
            };
            PROFILE0_RESOURCE_LIMITS.responsive_overrides + 1
        ];
        document.entities.insert(root.id, root);
        assert_eq!(
            resource_usage(&document).unwrap_err().resource,
            "responsive overrides"
        );

        let mut document = Document::empty(EntityId::new(1));
        let mut root = Entity::new(EntityId::new(2), EntityKind::Container);
        root.authored.values.insert(
            "values".to_owned(),
            PropertyValue::Array(vec![
                PropertyValue::Null;
                PROFILE0_RESOURCE_LIMITS.property_values
            ]),
        );
        document.entities.insert(root.id, root);
        assert_eq!(
            resource_usage(&document).unwrap_err().resource,
            "property values"
        );

        let mut document = Document::empty(EntityId::new(1));
        let token_count = PROFILE0_RESOURCE_LIMITS.string_bytes
            / PROFILE0_RESOURCE_LIMITS.single_string_bytes
            + 1;
        for index in 0..token_count {
            let id = EntityId::new(u128::try_from(index + 2).unwrap());
            document.tokens.insert(
                id,
                Token {
                    id,
                    name: "x".repeat(PROFILE0_RESOURCE_LIMITS.single_string_bytes),
                    value: PropertyValue::Null,
                },
            );
        }
        assert_eq!(
            resource_usage(&document).unwrap_err().resource,
            "string bytes"
        );

        let mut document = Document::empty(EntityId::new(1));
        let mut root = Entity::new(EntityId::new(2), EntityKind::Container);
        root.authored.values.insert(
            "bytes".to_owned(),
            PropertyValue::Bytes(vec![0; PROFILE0_RESOURCE_LIMITS.binary_bytes + 1]),
        );
        document.entities.insert(root.id, root);
        assert_eq!(
            resource_usage(&document).unwrap_err().resource,
            "binary bytes"
        );
    }
}
