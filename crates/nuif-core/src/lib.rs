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
    assets: 8_192,
    grid_tracks: 4_096,
};

/// Maximum explicit tracks on either axis of a profile-0 grid.
pub const PROFILE0_MAX_GRID_AXIS_TRACKS: usize = 256;

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
    pub assets: usize,
    pub grid_tracks: usize,
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
    pub assets: usize,
    pub grid_tracks: usize,
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

/// Stable semantic identity of an editable asset.
///
/// Unlike [`ResourceDigest`], this identity survives replacement of the
/// asset's encoded bytes.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AssetId(#[serde(with = "asset_id_serde")] pub u128);

impl AssetId {
    #[must_use]
    pub const fn new(value: u128) -> Self {
        Self(value)
    }
}

impl fmt::Display for AssetId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:032x}", self.0)
    }
}

impl FromStr for AssetId {
    type Err = ParseAssetIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() != 32 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(ParseAssetIdError);
        }
        u128::from_str_radix(value, 16)
            .map(Self)
            .map_err(|_| ParseAssetIdError)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParseAssetIdError;

impl fmt::Display for ParseAssetIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("asset identifier must contain exactly 32 hexadecimal digits")
    }
}

impl std::error::Error for ParseAssetIdError {}

mod asset_id_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::str::FromStr;

    use super::AssetId;

    pub fn serialize<S: Serializer>(value: &u128, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{value:032x}"))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u128, D::Error> {
        let value = String::deserialize(deserializer)?;
        AssetId::from_str(&value)
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
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub assets: BTreeMap<AssetId, Asset>,
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
            assets: BTreeMap::new(),
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
    #[serde(default, skip_serializing_if = "GridPlacement::is_default")]
    pub grid_placement: GridPlacement,
    pub fill: Option<Color>,
    pub text: Option<TextContent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<ImagePaint>,
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
            grid_placement: GridPlacement::default(),
            fill: None,
            text: None,
            image: None,
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
    #[serde(default, skip_serializing_if = "GridStyle::is_default")]
    pub grid: GridStyle,
}

impl Default for LayoutStyle {
    fn default() -> Self {
        Self {
            family: LayoutFamily::Freeform,
            direction: FlowDirection::Row,
            gap: 0.0,
            padding: Edges::default(),
            align: Align::Start,
            grid: GridStyle::default(),
        }
    }
}

/// Bounded explicit-grid properties for a grid container.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridStyle {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub columns: Vec<GridTrack>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rows: Vec<GridTrack>,
    #[serde(default, skip_serializing_if = "GridAutoFlow::is_default")]
    pub auto_flow: GridAutoFlow,
}

impl GridStyle {
    fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

/// A profile-0 explicit grid track.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum GridTrack {
    Fixed(f64),
    Fraction(f64),
}

/// Deterministic traversal order for auto-placed grid items.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GridAutoFlow {
    #[default]
    Row,
    Column,
}

impl GridAutoFlow {
    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "serde skip_serializing_if requires a shared-reference predicate"
    )]
    fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

/// Explicit or auto grid placement authored on an item.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridPlacement {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row: Option<u32>,
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub column_span: u32,
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub row_span: u32,
}

impl Default for GridPlacement {
    fn default() -> Self {
        Self {
            column: None,
            row: None,
            column_span: 1,
            row_span: 1,
        }
    }
}

impl GridPlacement {
    fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

const fn one() -> u32 {
    1
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde skip_serializing_if requires a shared-reference predicate"
)]
const fn is_one(value: &u32) -> bool {
    *value == 1
}

/// Resolved zero-based grid area occupied by one item.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GridArea {
    pub column: u32,
    pub row: u32,
    pub column_span: u32,
    pub row_span: u32,
}

/// A deterministic explicit-grid placement failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GridPlacementError {
    MissingTracks,
    TooManyTracks,
    PartialPosition { entity: EntityId },
    ZeroSpan { entity: EntityId },
    OutOfBounds { entity: EntityId },
    Overlap { entity: EntityId },
    Exhausted { entity: EntityId },
}

impl fmt::Display for GridPlacementError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingTracks => formatter.write_str("grid columns and rows must be non-empty"),
            Self::TooManyTracks => write!(
                formatter,
                "grid axes may contain at most {PROFILE0_MAX_GRID_AXIS_TRACKS} tracks"
            ),
            Self::PartialPosition { entity } => write!(
                formatter,
                "grid item {entity} must specify both row and column or neither"
            ),
            Self::ZeroSpan { entity } => {
                write!(formatter, "grid item {entity} spans must be positive")
            }
            Self::OutOfBounds { entity } => {
                write!(
                    formatter,
                    "grid item {entity} lies outside the explicit grid"
                )
            }
            Self::Overlap { entity } => {
                write!(
                    formatter,
                    "grid item {entity} overlaps an occupied grid cell"
                )
            }
            Self::Exhausted { entity } => write!(
                formatter,
                "grid item {entity} cannot fit without creating an implicit track"
            ),
        }
    }
}

impl std::error::Error for GridPlacementError {}

/// Resolves explicit and auto placements for one bounded profile-0 grid.
///
/// Explicitly positioned items reserve cells first. Remaining items are then
/// placed in child order using the container's row-major or column-major flow.
/// Profile 0 never creates implicit tracks.
///
/// # Errors
///
/// Returns a typed error when tracks or item placements are outside the
/// bounded profile, overlap, or require an implicit track.
pub fn resolve_grid_placements(
    document: &Document,
    container: &Entity,
) -> Result<BTreeMap<EntityId, GridArea>, GridPlacementError> {
    let grid = &container.authored.layout.grid;
    let column_count = grid.columns.len();
    let row_count = grid.rows.len();
    if column_count == 0 || row_count == 0 {
        return Err(GridPlacementError::MissingTracks);
    }
    if column_count > PROFILE0_MAX_GRID_AXIS_TRACKS || row_count > PROFILE0_MAX_GRID_AXIS_TRACKS {
        return Err(GridPlacementError::TooManyTracks);
    }
    let mut occupied = vec![false; column_count.saturating_mul(row_count)];
    let mut areas = BTreeMap::new();
    let mut auto_cursor = 0_usize;

    for child_id in &container.children {
        let Some(child) = document.entities.get(child_id) else {
            continue;
        };
        let placement = child.authored.grid_placement;
        match (placement.column, placement.row) {
            (Some(column), Some(row)) => {
                let area = GridArea {
                    column,
                    row,
                    column_span: placement.column_span,
                    row_span: placement.row_span,
                };
                reserve_grid_area(&mut occupied, column_count, row_count, child.id, area)?;
                areas.insert(child.id, area);
            }
            (None, None) => {
                if placement.column_span == 0 || placement.row_span == 0 {
                    return Err(GridPlacementError::ZeroSpan { entity: child.id });
                }
            }
            _ => return Err(GridPlacementError::PartialPosition { entity: child.id }),
        }
    }

    for child_id in &container.children {
        if areas.contains_key(child_id) {
            continue;
        }
        let Some(child) = document.entities.get(child_id) else {
            continue;
        };
        let placement = child.authored.grid_placement;
        let Some((area, next_cursor)) = find_grid_area(
            &occupied,
            column_count,
            row_count,
            placement.column_span,
            placement.row_span,
            grid.auto_flow,
            auto_cursor,
        ) else {
            return Err(GridPlacementError::Exhausted { entity: child.id });
        };
        reserve_grid_area(&mut occupied, column_count, row_count, child.id, area)?;
        areas.insert(child.id, area);
        auto_cursor = next_cursor;
    }
    Ok(areas)
}

fn reserve_grid_area(
    occupied: &mut [bool],
    column_count: usize,
    row_count: usize,
    entity: EntityId,
    area: GridArea,
) -> Result<(), GridPlacementError> {
    if area.column_span == 0 || area.row_span == 0 {
        return Err(GridPlacementError::ZeroSpan { entity });
    }
    let column =
        usize::try_from(area.column).map_err(|_| GridPlacementError::OutOfBounds { entity })?;
    let row = usize::try_from(area.row).map_err(|_| GridPlacementError::OutOfBounds { entity })?;
    let column_span = usize::try_from(area.column_span)
        .map_err(|_| GridPlacementError::OutOfBounds { entity })?;
    let row_span =
        usize::try_from(area.row_span).map_err(|_| GridPlacementError::OutOfBounds { entity })?;
    let column_end = column
        .checked_add(column_span)
        .ok_or(GridPlacementError::OutOfBounds { entity })?;
    let row_end = row
        .checked_add(row_span)
        .ok_or(GridPlacementError::OutOfBounds { entity })?;
    if column_end > column_count || row_end > row_count {
        return Err(GridPlacementError::OutOfBounds { entity });
    }
    for current_row in row..row_end {
        for current_column in column..column_end {
            let index = current_row * column_count + current_column;
            if occupied[index] {
                return Err(GridPlacementError::Overlap { entity });
            }
        }
    }
    for current_row in row..row_end {
        for current_column in column..column_end {
            occupied[current_row * column_count + current_column] = true;
        }
    }
    Ok(())
}

fn find_grid_area(
    occupied: &[bool],
    column_count: usize,
    row_count: usize,
    column_span: u32,
    row_span: u32,
    auto_flow: GridAutoFlow,
    cursor: usize,
) -> Option<(GridArea, usize)> {
    let column_span = usize::try_from(column_span).ok()?;
    let row_span = usize::try_from(row_span).ok()?;
    if column_span == 0 || row_span == 0 || column_span > column_count || row_span > row_count {
        return None;
    }
    let fits = |column: usize, row: usize| {
        (row..row + row_span).all(|current_row| {
            (column..column + column_span)
                .all(|current_column| !occupied[current_row * column_count + current_column])
        })
    };
    let cell_count = column_count.checked_mul(row_count)?;
    let coordinate = match auto_flow {
        GridAutoFlow::Row => (cursor..cell_count).find_map(|index| {
            let column = index % column_count;
            let row = index / column_count;
            (column + column_span <= column_count
                && row + row_span <= row_count
                && fits(column, row))
            .then_some((column, row, index + column_span))
        }),
        GridAutoFlow::Column => (cursor..cell_count).find_map(|index| {
            let column = index / row_count;
            let row = index % row_count;
            (column + column_span <= column_count
                && row + row_span <= row_count
                && fits(column, row))
            .then_some((column, row, index + row_span))
        }),
    }?;
    Some((
        GridArea {
            column: u32::try_from(coordinate.0).ok()?,
            row: u32::try_from(coordinate.1).ok()?,
            column_span: u32::try_from(column_span).ok()?,
            row_span: u32::try_from(row_span).ok()?,
        },
        coordinate.2,
    ))
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
    #[serde(default)]
    pub space: ColorSpace,
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub alpha: f32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColorSpace {
    #[default]
    Srgb,
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

/// SHA-256 identity of exact immutable resource bytes.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ResourceDigest(pub String);

impl ResourceDigest {
    #[must_use]
    pub fn from_sha256_hex(hex: impl Into<String>) -> Self {
        Self(format!("sha256:{}", hex.into()))
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.0.strip_prefix("sha256:").is_some_and(|hex| {
            hex.len() == 64
                && hex
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
    }

    #[must_use]
    pub fn sha256_hex(&self) -> Option<&str> {
        self.is_valid().then(|| &self.0[7..])
    }
}

impl fmt::Display for ResourceDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceRole {
    Source,
    Authoring,
    Derived,
    Cache,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ResourceLocator {
    Embedded { path: String },
    Linked { uri: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceDerivation {
    pub profile: String,
    pub inputs: Vec<ResourceDigest>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceDescriptor {
    pub digest: ResourceDigest,
    pub size: u64,
    pub media_type: String,
    pub role: ResourceRole,
    pub locator: ResourceLocator,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derivation: Option<ResourceDerivation>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub schema_version: u32,
    pub id: AssetId,
    pub name: Option<String>,
    pub resource: Option<ResourceDigest>,
    pub portability: AssetPortability,
    pub kind: AssetKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetPortability {
    Portable,
    PrivateAuthoring,
    Linked,
    Substituted,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "data")]
pub enum AssetKind {
    Image(ImageAsset),
    Font(FontAsset),
    Unknown(UnknownKind),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageAsset {
    pub width: u32,
    pub height: u32,
    pub decoder_profile: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FontAsset {
    pub face_index: u32,
    #[serde(default)]
    pub names: Vec<String>,
    #[serde(default)]
    pub axes: BTreeMap<String, f64>,
    #[serde(default)]
    pub features: BTreeMap<String, u32>,
    #[serde(default)]
    pub coverage: Vec<CodepointRange>,
    #[serde(default)]
    pub policy_evidence: BTreeMap<String, String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodepointRange {
    pub start: u32,
    pub end: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImagePaint {
    pub asset: AssetId,
    pub fit: ImageFit,
    pub crop: ImageCrop,
    pub transform: AffineTransform,
    pub sampling: ImageSampling,
    pub opacity: f32,
    pub color_conversion: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageFit {
    Fill,
    Contain,
    Cover,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageCrop {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AffineTransform {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub tx: f64,
    pub ty: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageSampling {
    Nearest,
    Linear,
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
        assets: document.assets.len(),
        ..ResourceUsage::default()
    };
    check_limit("entities", usage.entities, limits.entities)?;
    check_limit("roots", usage.roots, limits.roots)?;
    check_limit("tokens", usage.tokens, limits.tokens)?;
    check_limit("relations", usage.relations, limits.relations)?;
    check_limit("assets", usage.assets, limits.assets)?;

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
        add_count(
            &mut usage.grid_tracks,
            entity
                .authored
                .layout
                .grid
                .columns
                .len()
                .saturating_add(entity.authored.layout.grid.rows.len()),
            "grid tracks",
            limits.grid_tracks,
        )?;
        if let Some(text) = &entity.authored.text {
            add_string(&mut usage, &text.content, limits)?;
            add_string(&mut usage, &text.font, limits)?;
            add_string(&mut usage, &text.font_sha256, limits)?;
        }
        if let Some(image) = &entity.authored.image {
            add_string(&mut usage, &image.color_conversion, limits)?;
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
    inspect_assets(&mut usage, document, limits)?;
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

    inspect_containment_depth(&mut usage, document, &parents, limits)?;
    Ok(usage)
}

fn inspect_containment_depth(
    usage: &mut ResourceUsage,
    document: &Document,
    parents: &BTreeMap<EntityId, EntityId>,
    limits: ResourceLimits,
) -> Result<(), ResourceLimitExceeded> {
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
    Ok(())
}

fn inspect_assets(
    usage: &mut ResourceUsage,
    document: &Document,
    limits: ResourceLimits,
) -> Result<(), ResourceLimitExceeded> {
    for asset in document.assets.values() {
        add_optional_string(usage, asset.name.as_deref(), limits)?;
        if let Some(digest) = &asset.resource {
            add_string(usage, &digest.0, limits)?;
        }
        match &asset.kind {
            AssetKind::Image(image) => add_string(usage, &image.decoder_profile, limits)?,
            AssetKind::Font(font) => {
                for name in &font.names {
                    add_string(usage, name, limits)?;
                }
                for tag in font.axes.keys().chain(font.features.keys()) {
                    add_string(usage, tag, limits)?;
                }
                for (key, value) in &font.policy_evidence {
                    add_string(usage, key, limits)?;
                    add_string(usage, value, limits)?;
                }
            }
            AssetKind::Unknown(unknown) => {
                add_string(usage, &unknown.namespace, limits)?;
                add_string(usage, &unknown.kind, limits)?;
                add_binary(usage, unknown.payload.bytes.len(), limits)?;
            }
        }
    }
    Ok(())
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

    validate_grid_structure(document, &parents, &mut diagnostics);

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

    for (key, asset) in &document.assets {
        validate_asset(*key, asset, &mut diagnostics);
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
    validate_grid_style(entity, diagnostics);
    validate_entity_identifiers(document, entity, diagnostics);
    if let Some(text) = &entity.authored.text {
        if !is_sha256(&text.font_sha256) {
            diagnostics.push_capped(Diagnostic::error(
                "TEXT_FONT_HASH_INVALID",
                "text font_sha256 must contain exactly 64 lowercase hexadecimal digits",
                Some(entity.id),
            ));
        }
        if text.size.is_finite()
            && text.line_height.is_finite()
            && (text.size <= 0.0 || text.line_height <= 0.0)
        {
            diagnostics.push_capped(Diagnostic::error(
                "TEXT_METRICS_INVALID",
                "text size and line_height must be positive",
                Some(entity.id),
            ));
        }
    }
    if let Some(fill) = entity.authored.fill {
        let channels = [fill.red, fill.green, fill.blue, fill.alpha];
        if channels.iter().all(|channel| channel.is_finite())
            && channels
                .iter()
                .any(|channel| !(0.0..=1.0).contains(channel))
        {
            diagnostics.push_capped(Diagnostic::error(
                "COLOR_CHANNEL_OUT_OF_RANGE",
                "sRGB fill channels must be between 0 and 1 inclusive",
                Some(entity.id),
            ));
        }
    }
    if let Some(image) = &entity.authored.image {
        validate_image_paint(document, entity, image, diagnostics);
    }
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

fn validate_image_paint(
    document: &Document,
    entity: &Entity,
    image: &ImagePaint,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !matches!(entity.kind, EntityKind::Image) {
        diagnostics.push_capped(Diagnostic::error(
            "IMAGE_PAINT_KIND_INVALID",
            "an image paint may only be authored on an image entity",
            Some(entity.id),
        ));
    }
    if !matches!(
        document.assets.get(&image.asset).map(|asset| &asset.kind),
        Some(AssetKind::Image(_))
    ) {
        diagnostics.push_capped(Diagnostic::error(
            "IMAGE_ASSET_MISSING",
            format!(
                "image paint references missing or non-image asset {}",
                image.asset
            ),
            Some(entity.id),
        ));
    }
    let crop = [
        image.crop.x,
        image.crop.y,
        image.crop.width,
        image.crop.height,
    ];
    if crop.iter().any(|value| !value.is_finite())
        || image.crop.x < 0.0
        || image.crop.y < 0.0
        || image.crop.width <= 0.0
        || image.crop.height <= 0.0
        || image.crop.x + image.crop.width > 1.0
        || image.crop.y + image.crop.height > 1.0
    {
        diagnostics.push_capped(Diagnostic::error(
            "IMAGE_CROP_INVALID",
            "image crop must be a finite, positive normalized rectangle within 0..=1",
            Some(entity.id),
        ));
    }
    let transform = [
        image.transform.a,
        image.transform.b,
        image.transform.c,
        image.transform.d,
        image.transform.tx,
        image.transform.ty,
    ];
    if transform.iter().any(|value| !value.is_finite())
        || !image.opacity.is_finite()
        || !(0.0..=1.0).contains(&image.opacity)
        || !is_identifier(&image.color_conversion)
    {
        diagnostics.push_capped(Diagnostic::error(
            "IMAGE_PAINT_INVALID",
            "image transform and opacity must be finite, opacity must be within 0..=1, and color conversion must be an identifier",
            Some(entity.id),
        ));
    }
}

fn validate_asset(key: AssetId, asset: &Asset, diagnostics: &mut Vec<Diagnostic>) {
    let pointer = Some(format!("/assets/{key}"));
    let mut push = |code: &str, message: String| {
        diagnostics.push_capped(Diagnostic {
            code: code.to_owned(),
            severity: Severity::Error,
            message,
            entity: None,
            pointer: pointer.clone(),
            fidelity: None,
        });
    };
    if key != asset.id {
        push(
            "MODEL_ASSET_KEY_MISMATCH",
            format!("asset map key {key} differs from embedded id {}", asset.id),
        );
    }
    if asset.schema_version > CURRENT_SCHEMA_VERSION && !matches!(asset.kind, AssetKind::Unknown(_))
    {
        push(
            "MODEL_ASSET_VERSION_NOT_OPAQUE",
            "a newer asset schema version must be represented as unknown".to_owned(),
        );
    }
    if let Some(digest) = &asset.resource
        && !digest.is_valid()
    {
        push(
            "RESOURCE_DIGEST_INVALID",
            "resource digest must be sha256 followed by 64 lowercase hexadecimal digits".to_owned(),
        );
    }
    if matches!(
        asset.portability,
        AssetPortability::Portable | AssetPortability::Substituted
    ) && asset.resource.is_none()
    {
        push(
            "ASSET_RESOURCE_REQUIRED",
            "portable and substituted assets require an exact resource digest".to_owned(),
        );
    }
    if matches!(asset.portability, AssetPortability::Unavailable) && asset.resource.is_some() {
        push(
            "ASSET_UNAVAILABLE_HAS_RESOURCE",
            "an unavailable asset cannot bind resource bytes".to_owned(),
        );
    }
    match &asset.kind {
        AssetKind::Image(image) => {
            if image.width == 0 || image.height == 0 || !is_identifier(&image.decoder_profile) {
                push(
                    "IMAGE_ASSET_INVALID",
                    "image dimensions must be positive and decoder profile must be an identifier"
                        .to_owned(),
                );
            }
        }
        AssetKind::Font(font) => {
            if font.axes.values().any(|value| !value.is_finite()) {
                push(
                    "FONT_AXIS_INVALID",
                    "font variation axis values must be finite".to_owned(),
                );
            }
            if font
                .coverage
                .iter()
                .any(|range| range.start > range.end || range.end > 0x10_ffff)
            {
                push(
                    "FONT_COVERAGE_INVALID",
                    "font coverage ranges must be ordered Unicode scalar bounds".to_owned(),
                );
            }
        }
        AssetKind::Unknown(unknown) => {
            if !is_identifier(&unknown.namespace) || !is_identifier(&unknown.kind) {
                push(
                    "MODEL_IDENTIFIER_INVALID",
                    "unknown asset namespace and kind must be lowercase identifiers".to_owned(),
                );
            }
        }
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
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
    authored_numbers.extend(
        entity
            .authored
            .layout
            .grid
            .columns
            .iter()
            .chain(&entity.authored.layout.grid.rows)
            .map(|track| match track {
                GridTrack::Fixed(value) | GridTrack::Fraction(value) => *value,
            }),
    );
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

fn validate_grid_style(entity: &Entity, diagnostics: &mut Vec<Diagnostic>) {
    let layout = &entity.authored.layout;
    if layout.family == LayoutFamily::Grid {
        if layout.grid.columns.is_empty() || layout.grid.rows.is_empty() {
            diagnostics.push_capped(Diagnostic::error(
                "GRID_TRACKS_REQUIRED",
                "grid containers require at least one explicit column and row",
                Some(entity.id),
            ));
        }
        if layout.grid.columns.len() > PROFILE0_MAX_GRID_AXIS_TRACKS
            || layout.grid.rows.len() > PROFILE0_MAX_GRID_AXIS_TRACKS
        {
            diagnostics.push_capped(Diagnostic::error(
                "GRID_TRACK_LIMIT_EXCEEDED",
                format!("grid axes may contain at most {PROFILE0_MAX_GRID_AXIS_TRACKS} tracks"),
                Some(entity.id),
            ));
        }
        if layout
            .grid
            .columns
            .iter()
            .chain(&layout.grid.rows)
            .any(|track| match track {
                GridTrack::Fixed(value) | GridTrack::Fraction(value) => {
                    value.is_finite() && *value <= 0.0
                }
            })
        {
            diagnostics.push_capped(Diagnostic::error(
                "GRID_TRACK_INVALID",
                "fixed sizes and fractional weights must be positive",
                Some(entity.id),
            ));
        }
    } else if !layout.grid.is_default() {
        diagnostics.push_capped(Diagnostic::error(
            "GRID_STYLE_WITHOUT_GRID_FAMILY",
            "grid tracks and auto flow are only valid on a grid container",
            Some(entity.id),
        ));
    }
}

fn validate_grid_structure(
    document: &Document,
    parents: &BTreeMap<EntityId, EntityId>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for entity in document.entities.values() {
        if !entity.authored.grid_placement.is_default()
            && !parents.get(&entity.id).is_some_and(|parent| {
                document
                    .entities
                    .get(parent)
                    .is_some_and(|item| item.authored.layout.family == LayoutFamily::Grid)
            })
        {
            diagnostics.push_capped(Diagnostic::error(
                "GRID_PLACEMENT_WITHOUT_GRID_PARENT",
                "grid placement is only valid on a direct child of a grid container",
                Some(entity.id),
            ));
        }
        if entity.authored.layout.family != LayoutFamily::Grid {
            continue;
        }
        if let Err(error) = resolve_grid_placements(document, entity) {
            let (code, target) = match error {
                GridPlacementError::MissingTracks => ("GRID_TRACKS_REQUIRED", entity.id),
                GridPlacementError::TooManyTracks => ("GRID_TRACK_LIMIT_EXCEEDED", entity.id),
                GridPlacementError::PartialPosition { entity } => {
                    ("GRID_PLACEMENT_PARTIAL", entity)
                }
                GridPlacementError::ZeroSpan { entity } => ("GRID_SPAN_INVALID", entity),
                GridPlacementError::OutOfBounds { entity } => {
                    ("GRID_PLACEMENT_OUT_OF_BOUNDS", entity)
                }
                GridPlacementError::Overlap { entity } => ("GRID_PLACEMENT_OVERLAP", entity),
                GridPlacementError::Exhausted { entity } => {
                    ("GRID_EXPLICIT_AREA_EXHAUSTED", entity)
                }
            };
            if !diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == code && diagnostic.entity == Some(target))
            {
                diagnostics.push_capped(Diagnostic::error(code, error.to_string(), Some(target)));
            }
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
    fn validation_rejects_unpinned_text_identity_and_metrics() {
        let mut document = Document::empty(EntityId::new(1));
        let mut text = Entity::new(EntityId::new(2), EntityKind::Text);
        text.authored.text = Some(TextContent {
            content: "probe".to_owned(),
            font: "unresolved".to_owned(),
            font_sha256: "NOT-A-SHA256".to_owned(),
            size: 0.0,
            line_height: -1.0,
        });
        document.roots.push(text.id);
        document.entities.insert(text.id, text);
        let codes = validate(&document)
            .into_iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<BTreeSet<_>>();
        assert!(codes.contains("TEXT_FONT_HASH_INVALID"));
        assert!(codes.contains("TEXT_METRICS_INVALID"));
    }

    #[test]
    fn validation_rejects_out_of_range_srgb_channels() {
        let mut document = Document::empty(EntityId::new(1));
        let mut shape = Entity::new(EntityId::new(2), EntityKind::Shape(ShapeKind::Rectangle));
        shape.authored.fill = Some(Color {
            space: ColorSpace::Srgb,
            red: 1.01,
            green: 0.0,
            blue: 0.0,
            alpha: 1.0,
        });
        document.roots.push(shape.id);
        document.entities.insert(shape.id, shape);
        let codes = validate(&document)
            .into_iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<BTreeSet<_>>();
        assert!(codes.contains("COLOR_CHANNEL_OUT_OF_RANGE"));
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
        root.authored.layout.grid.columns =
            vec![GridTrack::Fixed(1.0); PROFILE0_RESOURCE_LIMITS.grid_tracks + 1];
        document.entities.insert(root.id, root);
        assert_eq!(
            resource_usage(&document).unwrap_err().resource,
            "grid tracks"
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

    #[test]
    fn asset_identity_and_image_references_are_validated() {
        let asset_id = AssetId::new(0xa0);
        let mut document = Document::empty(EntityId::new(1));
        document.assets.insert(
            asset_id,
            Asset {
                schema_version: CURRENT_SCHEMA_VERSION,
                id: asset_id,
                name: Some("hero".to_owned()),
                resource: Some(ResourceDigest::from_sha256_hex("a".repeat(64))),
                portability: AssetPortability::Portable,
                kind: AssetKind::Image(ImageAsset {
                    width: 32,
                    height: 24,
                    decoder_profile: "nuif-png-0".to_owned(),
                }),
            },
        );
        let mut image = Entity::new(EntityId::new(2), EntityKind::Image);
        image.authored.image = Some(ImagePaint {
            asset: asset_id,
            fit: ImageFit::Contain,
            crop: ImageCrop {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            },
            transform: AffineTransform {
                a: 1.0,
                b: 0.0,
                c: 0.0,
                d: 1.0,
                tx: 0.0,
                ty: 0.0,
            },
            sampling: ImageSampling::Linear,
            opacity: 1.0,
            color_conversion: "srgb".to_owned(),
        });
        document.roots.push(image.id);
        document.entities.insert(image.id, image);
        assert!(validate(&document).is_empty());

        document.assets.get_mut(&asset_id).unwrap().resource =
            Some(ResourceDigest("sha256:ABC".to_owned()));
        assert!(
            validate(&document)
                .iter()
                .any(|diagnostic| diagnostic.code == "RESOURCE_DIGEST_INVALID")
        );
    }

    #[test]
    fn explicit_grid_reserves_positioned_items_before_auto_placement() {
        let mut document = Document::empty(EntityId::new(1));
        let mut root = Entity::new(EntityId::new(2), EntityKind::Container);
        root.authored.layout.family = LayoutFamily::Grid;
        root.authored.layout.grid = GridStyle {
            columns: vec![GridTrack::Fixed(40.0), GridTrack::Fraction(1.0)],
            rows: vec![GridTrack::Fraction(1.0), GridTrack::Fraction(1.0)],
            auto_flow: GridAutoFlow::Row,
        };
        let auto_first = Entity::new(EntityId::new(3), EntityKind::Container);
        let mut explicit_second = Entity::new(EntityId::new(4), EntityKind::Container);
        explicit_second.authored.grid_placement.column = Some(0);
        explicit_second.authored.grid_placement.row = Some(0);
        let auto_third = Entity::new(EntityId::new(5), EntityKind::Container);
        root.children = vec![auto_first.id, explicit_second.id, auto_third.id];
        document.roots.push(root.id);
        document.entities.insert(root.id, root.clone());
        document.entities.insert(auto_first.id, auto_first);
        document
            .entities
            .insert(explicit_second.id, explicit_second);
        document.entities.insert(auto_third.id, auto_third);

        assert!(validate(&document).is_empty());
        let areas = resolve_grid_placements(&document, &root).unwrap();
        assert_eq!(
            areas[&EntityId::new(3)],
            GridArea {
                column: 1,
                row: 0,
                column_span: 1,
                row_span: 1,
            }
        );
        assert_eq!(
            areas[&EntityId::new(4)],
            GridArea {
                column: 0,
                row: 0,
                column_span: 1,
                row_span: 1,
            }
        );
        assert_eq!(areas[&EntityId::new(5)].row, 1);
    }

    #[test]
    fn grid_validation_rejects_partial_overlap_and_implicit_tracks() {
        let mut document = Document::empty(EntityId::new(1));
        let mut root = Entity::new(EntityId::new(2), EntityKind::Container);
        root.authored.layout.family = LayoutFamily::Grid;
        root.authored.layout.grid.columns = vec![GridTrack::Fraction(1.0)];
        root.authored.layout.grid.rows = vec![GridTrack::Fixed(20.0)];
        let mut first = Entity::new(EntityId::new(3), EntityKind::Container);
        first.authored.grid_placement.column = Some(0);
        let second = Entity::new(EntityId::new(4), EntityKind::Container);
        root.children = vec![first.id, second.id];
        document.roots.push(root.id);
        document.entities.insert(root.id, root);
        document.entities.insert(first.id, first);
        document.entities.insert(second.id, second);
        assert!(
            validate(&document)
                .iter()
                .any(|diagnostic| diagnostic.code == "GRID_PLACEMENT_PARTIAL")
        );

        let first = document.entities.get_mut(&EntityId::new(3)).unwrap();
        first.authored.grid_placement.row = Some(0);
        assert!(
            validate(&document)
                .iter()
                .any(|diagnostic| diagnostic.code == "GRID_EXPLICIT_AREA_EXHAUSTED")
        );

        let second = document.entities.get_mut(&EntityId::new(4)).unwrap();
        second.authored.grid_placement.column = Some(0);
        second.authored.grid_placement.row = Some(0);
        assert!(
            validate(&document)
                .iter()
                .any(|diagnostic| diagnostic.code == "GRID_PLACEMENT_OVERLAP")
        );
    }

    #[test]
    fn grid_auto_flow_is_sparse_and_does_not_backfill() {
        let mut document = Document::empty(EntityId::new(1));
        let mut root = Entity::new(EntityId::new(2), EntityKind::Container);
        root.authored.layout.family = LayoutFamily::Grid;
        root.authored.layout.grid.columns = vec![GridTrack::Fraction(1.0); 3];
        root.authored.layout.grid.rows = vec![GridTrack::Fraction(1.0); 2];
        let mut wide_first = Entity::new(EntityId::new(3), EntityKind::Container);
        wide_first.authored.grid_placement.column_span = 2;
        let mut wide_second = Entity::new(EntityId::new(4), EntityKind::Container);
        wide_second.authored.grid_placement.column_span = 2;
        let final_item = Entity::new(EntityId::new(5), EntityKind::Container);
        root.children = vec![wide_first.id, wide_second.id, final_item.id];
        document.roots.push(root.id);
        document.entities.insert(root.id, root.clone());
        document.entities.insert(wide_first.id, wide_first);
        document.entities.insert(wide_second.id, wide_second);
        document.entities.insert(final_item.id, final_item);

        let areas = resolve_grid_placements(&document, &root).unwrap();
        assert_eq!(
            (
                areas[&EntityId::new(3)].column,
                areas[&EntityId::new(3)].row
            ),
            (0, 0)
        );
        assert_eq!(
            (
                areas[&EntityId::new(4)].column,
                areas[&EntityId::new(4)].row
            ),
            (0, 1)
        );
        assert_eq!(
            (
                areas[&EntityId::new(5)].column,
                areas[&EntityId::new(5)].row
            ),
            (2, 1)
        );
    }
}
