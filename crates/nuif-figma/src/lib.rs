#![doc = "Pure mapping for the bounded Figma plug-in snapshot profile."]

use nuif_adapter::{
    CorrespondenceTarget, FidelityEntry, HostAdapterReport, HostCorrespondenceRecord, HostDirection,
};
use nuif_codec::canonical_hash;
use nuif_core::{
    Align, Color, ColorSpace, Document, Edges, Entity, EntityId, EntityKind, ExtensionDeclarations,
    Fidelity, FlowDirection, GridAutoFlow, GridPlacement, LayoutFamily, LayoutStyle, Point,
    Semantics, ShapeKind, SizeIntent, TextContent, validate,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::str::FromStr;
use thiserror::Error;

pub const PROFILE_NAME: &str = "nuif-figma-plugin-snapshot-0";
pub const HOST_APPLICATION: &str = "Figma Design";
pub const PLUGIN_API_VERSION: &str = "1.0.0";
pub const MAX_SNAPSHOT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_NODES: usize = 16_384;
pub const MAX_DEPTH: usize = 64;
pub const MAX_TEXT_UTF16: usize = 4_096;
pub const MAX_STRING_BYTES: usize = 256 * 1024;

#[derive(Clone, Debug, Error)]
pub enum AdapterError {
    #[error("Figma snapshot exceeds the {MAX_SNAPSHOT_BYTES}-byte profile limit")]
    SnapshotTooLarge,
    #[error("Figma snapshot JSON is invalid: {0}")]
    Json(String),
    #[error("Figma snapshot profile marker is invalid: {0}")]
    ProfileMarker(String),
    #[error("invalid Figma snapshot value at {pointer}: {reason}")]
    InvalidValue { pointer: String, reason: String },
    #[error("Figma snapshot exceeds the {MAX_NODES}-node limit")]
    TooManyNodes,
    #[error("Figma snapshot exceeds the {MAX_DEPTH}-level depth limit")]
    TooDeep,
    #[error("duplicate Figma host object id: {0}")]
    DuplicateHostId(String),
    #[error("document is outside {PROFILE_NAME}: {reason}")]
    UnsupportedProfile {
        reason: String,
        report: Box<HostAdapterReport>,
    },
    #[error("canonical processing failed: {0}")]
    Canonical(String),
    #[error("generated Figma mutation plan did not map back to the requested document")]
    PlanMismatch,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginSnapshot {
    pub schema_version: u32,
    pub host_application_version: String,
    pub host_api_version: String,
    pub host_document_id: String,
    pub host_document_revision: Option<String>,
    pub page_id: String,
    pub page_name: String,
    pub nuif_document_id: Option<String>,
    pub root: SnapshotNode,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotNode {
    pub id: String,
    pub name: String,
    pub kind: HostNodeKind,
    pub visible: bool,
    pub opacity: f64,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub fill: Option<SolidPaint>,
    #[serde(default)]
    pub layout: HostLayout,
    pub text: Option<HostText>,
    pub nuif_entity_id: Option<String>,
    #[serde(default)]
    pub unsupported_properties: Vec<String>,
    #[serde(default)]
    pub children: Vec<SnapshotNode>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HostNodeKind {
    Frame,
    Group,
    Rectangle,
    Ellipse,
    Text,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HostLayoutMode {
    #[default]
    None,
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HostAxisAlign {
    #[default]
    Min,
    Center,
    Max,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostLayout {
    pub mode: HostLayoutMode,
    pub item_spacing: f64,
    pub padding_top: f64,
    pub padding_right: f64,
    pub padding_bottom: f64,
    pub padding_left: f64,
    pub primary_axis_align: HostAxisAlign,
    pub counter_axis_align: HostAxisAlign,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolidPaint {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub alpha: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostText {
    pub characters: String,
    pub font_family: String,
    pub font_style: String,
    pub font_sha256: String,
    pub font_size: f64,
    pub line_height: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImportedSnapshot {
    pub document: Document,
    pub report: HostAdapterReport,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginMutationPlan {
    pub schema_version: u32,
    pub profile: String,
    pub snapshot: PluginSnapshot,
    pub report: HostAdapterReport,
}

/// Imports a bounded, normalized snapshot produced by the plug-in main thread.
///
/// # Errors
///
/// Rejects malformed, duplicate, unbounded or structurally invalid snapshots.
pub fn import_snapshot(bytes: &[u8]) -> Result<ImportedSnapshot, AdapterError> {
    if bytes.len() > MAX_SNAPSHOT_BYTES {
        return Err(AdapterError::SnapshotTooLarge);
    }
    let snapshot: PluginSnapshot =
        serde_json::from_slice(bytes).map_err(|error| AdapterError::Json(error.to_string()))?;
    import_normalized_snapshot(&snapshot)
}

/// Maps a canonical document to a deterministic host mutation-plan tree.
///
/// # Errors
///
/// Returns property-attributed fidelity when the document exceeds the profile.
pub fn plan_import(
    document: &Document,
    host_application_version: &str,
) -> Result<PluginMutationPlan, AdapterError> {
    let snapshot = snapshot_from_document(document, host_application_version)?;
    let imported = import_normalized_snapshot(&snapshot)?;
    if imported.document != *document {
        return Err(AdapterError::PlanMismatch);
    }
    let mut report = imported.report;
    report.direction = HostDirection::Import;
    Ok(PluginMutationPlan {
        schema_version: 1,
        profile: PROFILE_NAME.to_owned(),
        snapshot,
        report,
    })
}

struct ImportContext {
    document: Document,
    host_ids: BTreeSet<String>,
    entity_ids: BTreeSet<EntityId>,
    fidelity: Vec<FidelityEntry>,
    correspondences: Vec<HostCorrespondenceRecord>,
    nodes: usize,
    string_bytes: usize,
    identity_scope: String,
    unmapped_host_data_preserved: bool,
}

fn import_normalized_snapshot(snapshot: &PluginSnapshot) -> Result<ImportedSnapshot, AdapterError> {
    validate_snapshot_header(snapshot)?;
    let (document_id, exact_document_identity) = allocated_id(
        &format!("{}\0{}", snapshot.host_document_id, snapshot.page_id),
        snapshot.nuif_document_id.as_deref(),
        &BTreeSet::new(),
    );
    let mut context = ImportContext {
        document: Document::empty(document_id),
        host_ids: BTreeSet::new(),
        entity_ids: BTreeSet::from([document_id]),
        fidelity: Vec::new(),
        correspondences: Vec::new(),
        nodes: 0,
        string_bytes: header_string_bytes(snapshot),
        identity_scope: format!("{}\0{}", snapshot.host_document_id, snapshot.page_id),
        unmapped_host_data_preserved: exact_document_identity,
    };
    context.fidelity.push(FidelityEntry {
        target: CorrespondenceTarget::Document { id: document_id },
        pointer: "/identity".to_owned(),
        status: if exact_document_identity {
            Fidelity::Lossless
        } else {
            Fidelity::Representable
        },
    });
    context.correspondences.push(HostCorrespondenceRecord {
        target: CorrespondenceTarget::Document { id: document_id },
        host_object_id: snapshot.host_document_id.clone(),
        host_property: Some("document_id".to_owned()),
    });
    let root = map_snapshot_node(&snapshot.root, true, 0, &mut context)?;
    context.document.roots.push(root);
    let diagnostics = validate(&context.document);
    if let Some(error) = diagnostics
        .iter()
        .find(|item| item.severity == nuif_core::Severity::Error)
    {
        return Err(AdapterError::InvalidValue {
            pointer: error.pointer.clone().unwrap_or_default(),
            reason: error.message.clone(),
        });
    }
    let hash = canonical_hash(&context.document)
        .map_err(|error| AdapterError::Canonical(error.to_string()))?;
    let report = HostAdapterReport {
        schema_version: 1,
        profile: PROFILE_NAME.to_owned(),
        direction: HostDirection::Export,
        host_application: format!("{HOST_APPLICATION} {}", snapshot.host_application_version),
        host_api_version: snapshot.host_api_version.clone(),
        host_document_revision: snapshot.host_document_revision.clone(),
        canonical_hash: Some(hash),
        fidelity: context.fidelity,
        correspondences: context.correspondences,
        unmapped_host_data_preserved: context.unmapped_host_data_preserved,
    };
    if !report.validation_errors().is_empty() {
        return Err(AdapterError::ProfileMarker(
            "generated host report is invalid".to_owned(),
        ));
    }
    Ok(ImportedSnapshot {
        document: context.document,
        report,
    })
}

fn validate_snapshot_header(snapshot: &PluginSnapshot) -> Result<(), AdapterError> {
    if snapshot.schema_version != 1 {
        return Err(AdapterError::ProfileMarker(
            "schema_version must equal 1".to_owned(),
        ));
    }
    for (pointer, value) in [
        (
            "/host_application_version",
            snapshot.host_application_version.as_str(),
        ),
        ("/host_api_version", snapshot.host_api_version.as_str()),
        ("/host_document_id", snapshot.host_document_id.as_str()),
        ("/page_id", snapshot.page_id.as_str()),
        ("/page_name", snapshot.page_name.as_str()),
    ] {
        if value.trim().is_empty() {
            return invalid(pointer, "value must not be empty");
        }
    }
    if snapshot.root.kind != HostNodeKind::Frame {
        return invalid("/root/kind", "the selected root must be a Figma FRAME");
    }
    Ok(())
}

fn map_snapshot_node(
    node: &SnapshotNode,
    root: bool,
    depth: usize,
    context: &mut ImportContext,
) -> Result<EntityId, AdapterError> {
    if depth >= MAX_DEPTH {
        return Err(AdapterError::TooDeep);
    }
    context.nodes += 1;
    if context.nodes > MAX_NODES {
        return Err(AdapterError::TooManyNodes);
    }
    if node.id.trim().is_empty() {
        return invalid("/root/id", "host object id must not be empty");
    }
    if !context.host_ids.insert(node.id.clone()) {
        return Err(AdapterError::DuplicateHostId(node.id.clone()));
    }
    context.string_bytes = context
        .string_bytes
        .saturating_add(node.id.len())
        .saturating_add(node.name.len())
        .saturating_add(
            node.unsupported_properties
                .iter()
                .map(String::len)
                .sum::<usize>(),
        );
    if let Some(text) = &node.text {
        context.string_bytes = context
            .string_bytes
            .saturating_add(text.characters.len())
            .saturating_add(text.font_family.len())
            .saturating_add(text.font_style.len())
            .saturating_add(text.font_sha256.len());
    }
    if context.string_bytes > MAX_STRING_BYTES {
        return invalid(
            "/root",
            "combined snapshot strings exceed the profile limit",
        );
    }
    for property in &node.unsupported_properties {
        if property.is_empty()
            || property.len() > 128
            || !property
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
        {
            return invalid(
                &format!("/nodes/{}/unsupported_properties", node.id),
                "property names must be 1..=128 ASCII identifier characters",
            );
        }
    }
    let scoped_host_id = format!("{}\0{}", context.identity_scope, node.id);
    let (id, exact_identity) = allocated_id(
        &scoped_host_id,
        node.nuif_entity_id.as_deref(),
        &context.entity_ids,
    );
    if !exact_identity {
        context.unmapped_host_data_preserved = false;
    }
    context.entity_ids.insert(id);
    let kind = map_kind(node.kind, root)?;
    let mut entity = Entity::new(id, kind);
    entity.name = Some(node.name.clone());
    map_geometry(node, &mut entity)?;
    map_layout(node, &mut entity, root)?;
    map_text(node, &mut entity)?;
    for child in &node.children {
        entity
            .children
            .push(map_snapshot_node(child, false, depth + 1, context)?);
    }
    if !matches!(node.kind, HostNodeKind::Frame | HostNodeKind::Group) && !node.children.is_empty()
    {
        return invalid(
            &format!("/nodes/{}/children", node.id),
            "leaf Figma nodes cannot contain children in this profile",
        );
    }
    record_node_evidence(node, id, root, exact_identity, context);
    context.document.entities.insert(id, entity);
    Ok(id)
}

fn map_kind(kind: HostNodeKind, root: bool) -> Result<EntityKind, AdapterError> {
    match (kind, root) {
        (HostNodeKind::Frame, true) => Ok(EntityKind::Surface),
        (HostNodeKind::Frame | HostNodeKind::Group, false) => Ok(EntityKind::Container),
        (HostNodeKind::Rectangle, false) => Ok(EntityKind::Shape(ShapeKind::Rectangle)),
        (HostNodeKind::Ellipse, false) => Ok(EntityKind::Shape(ShapeKind::Ellipse)),
        (HostNodeKind::Text, false) => Ok(EntityKind::Text),
        (_, true) => invalid("/root/kind", "the document root must be a FRAME"),
    }
}

fn map_geometry(node: &SnapshotNode, entity: &mut Entity) -> Result<(), AdapterError> {
    let numbers = [node.opacity, node.x, node.y, node.width, node.height];
    if numbers.into_iter().any(|value| !value.is_finite()) {
        return invalid(
            &format!("/nodes/{}/geometry", node.id),
            "values must be finite",
        );
    }
    if node.width < 0.0 || node.height < 0.0 || !(0.0..=1.0).contains(&node.opacity) {
        return invalid(
            &format!("/nodes/{}/geometry", node.id),
            "dimensions must be non-negative and opacity must be in 0..=1",
        );
    }
    if entity.kind == EntityKind::Surface && (node.width <= 0.0 || node.height <= 0.0) {
        return invalid(
            &format!("/nodes/{}/geometry", node.id),
            "the selected root frame must have positive dimensions",
        );
    }
    if node.kind == HostNodeKind::Group && node.fill.is_some() {
        return invalid(
            &format!("/nodes/{}/fill", node.id),
            "GROUP does not expose a mapped fill in this profile",
        );
    }
    entity.authored.position = Point {
        x: node.x,
        y: node.y,
    };
    entity.authored.width = SizeIntent::Fixed(node.width);
    entity.authored.height = SizeIntent::Fixed(node.height);
    entity.authored.fill = node.fill.map(color_from_paint).transpose()?;
    Ok(())
}

fn map_layout(node: &SnapshotNode, entity: &mut Entity, root: bool) -> Result<(), AdapterError> {
    let layout = &node.layout;
    let metrics = [
        layout.item_spacing,
        layout.padding_top,
        layout.padding_right,
        layout.padding_bottom,
        layout.padding_left,
    ];
    if metrics
        .into_iter()
        .any(|value| !value.is_finite() || value < 0.0)
    {
        return invalid(
            &format!("/nodes/{}/layout", node.id),
            "spacing and padding must be finite and non-negative",
        );
    }
    if node.kind == HostNodeKind::Group && layout != &HostLayout::default() {
        return invalid(
            &format!("/nodes/{}/layout", node.id),
            "GROUP layout must be NONE with default metrics",
        );
    }
    if layout.mode == HostLayoutMode::None && layout != &HostLayout::default() {
        return invalid(
            &format!("/nodes/{}/layout", node.id),
            "layout NONE must carry default, non-applicable auto-layout metrics",
        );
    }
    if !matches!(node.kind, HostNodeKind::Frame | HostNodeKind::Group)
        && layout != &HostLayout::default()
    {
        return invalid(
            &format!("/nodes/{}/layout", node.id),
            "leaf layout must be default",
        );
    }
    if layout.mode != HostLayoutMode::None && layout.primary_axis_align != HostAxisAlign::Min {
        return invalid(
            &format!("/nodes/{}/layout/primary_axis_align", node.id),
            "only packed MIN primary-axis alignment is representable",
        );
    }
    entity.authored.layout = LayoutStyle {
        family: if layout.mode == HostLayoutMode::None {
            LayoutFamily::Freeform
        } else {
            LayoutFamily::Stack
        },
        direction: if layout.mode == HostLayoutMode::Vertical {
            FlowDirection::Column
        } else {
            FlowDirection::Row
        },
        gap: layout.item_spacing,
        padding: Edges {
            top: layout.padding_top,
            right: layout.padding_right,
            bottom: layout.padding_bottom,
            left: layout.padding_left,
        },
        align: match layout.counter_axis_align {
            HostAxisAlign::Min => Align::Start,
            HostAxisAlign::Center => Align::Center,
            HostAxisAlign::Max => Align::End,
        },
        ..LayoutStyle::default()
    };
    if root {
        entity.authored.position = Point::default();
    }
    Ok(())
}

fn map_text(node: &SnapshotNode, entity: &mut Entity) -> Result<(), AdapterError> {
    match (node.kind, &node.text) {
        (HostNodeKind::Text, Some(text)) => {
            if text.characters.encode_utf16().count() > MAX_TEXT_UTF16 {
                return invalid(
                    &format!("/nodes/{}/text/characters", node.id),
                    "text exceeds the UTF-16 code-unit limit",
                );
            }
            if text.font_family != nuif_text::PINNED_FONT_NAME
                || text.font_style != "Regular"
                || text.font_sha256 != nuif_text::PINNED_FONT_SHA256
                || !text.font_size.is_finite()
                || text.font_size <= 0.0
                || !text.line_height.is_finite()
                || text.line_height <= 0.0
            {
                return invalid(
                    &format!("/nodes/{}/text", node.id),
                    "text requires the exact pinned Ahem Regular identity and positive metrics",
                );
            }
            entity.authored.text = Some(TextContent {
                content: text.characters.clone(),
                font: text.font_family.clone(),
                font_sha256: text.font_sha256.clone(),
                font_asset: None,
                size: text.font_size,
                line_height: text.line_height,
            });
        }
        (HostNodeKind::Text, None) => {
            return invalid(
                &format!("/nodes/{}/text", node.id),
                "TEXT metadata is required",
            );
        }
        (_, Some(_)) => {
            return invalid(
                &format!("/nodes/{}/text", node.id),
                "non-text nodes cannot carry text metadata",
            );
        }
        (_, None) => {}
    }
    Ok(())
}

fn record_node_evidence(
    node: &SnapshotNode,
    id: EntityId,
    root: bool,
    exact_identity: bool,
    context: &mut ImportContext,
) {
    let target = CorrespondenceTarget::Entity { id };
    for pointer in [
        "/kind",
        "/name",
        "/children",
        "/authored/position",
        "/authored/width",
        "/authored/height",
        "/authored/fill",
        "/authored/layout",
        "/authored/text",
    ] {
        context.fidelity.push(FidelityEntry {
            target: target.clone(),
            pointer: format!("/entities/{id}{pointer}"),
            status: if (node.kind == HostNodeKind::Group && pointer == "/kind")
                || (root && pointer == "/authored/position" && (node.x != 0.0 || node.y != 0.0))
            {
                context.unmapped_host_data_preserved = false;
                Fidelity::Representable
            } else {
                Fidelity::Lossless
            },
        });
        context.correspondences.push(HostCorrespondenceRecord {
            target: target.clone(),
            host_object_id: node.id.clone(),
            host_property: Some(pointer.trim_start_matches('/').to_owned()),
        });
    }
    context.fidelity.push(FidelityEntry {
        target: target.clone(),
        pointer: format!("/entities/{id}/identity"),
        status: if exact_identity {
            Fidelity::Lossless
        } else {
            Fidelity::Representable
        },
    });
    if !node.visible || (node.opacity - 1.0).abs() > f64::EPSILON {
        context.unmapped_host_data_preserved = false;
        context.fidelity.push(FidelityEntry {
            target: target.clone(),
            pointer: format!("/entities/{id}/appearance"),
            status: Fidelity::Unsupported {
                reason: "node visibility and node opacity are not first-class NUIF model fields"
                    .to_owned(),
            },
        });
    }
    for property in &node.unsupported_properties {
        context.unmapped_host_data_preserved = false;
        context.fidelity.push(FidelityEntry {
            target: target.clone(),
            pointer: format!("/host/{}/{property}", node.id),
            status: Fidelity::Unsupported {
                reason: format!("Figma property {property} is outside {PROFILE_NAME}"),
            },
        });
    }
}

fn color_from_paint(paint: SolidPaint) -> Result<Color, AdapterError> {
    let channels = [paint.red, paint.green, paint.blue, paint.alpha];
    if channels
        .into_iter()
        .any(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
    {
        return invalid(
            "/fill",
            "solid paint channels must be finite values in 0..=1",
        );
    }
    Ok(Color {
        space: ColorSpace::Srgb,
        red: paint.red,
        green: paint.green,
        blue: paint.blue,
        alpha: paint.alpha,
    })
}

fn allocated_id(
    host_id: &str,
    claimed: Option<&str>,
    used: &BTreeSet<EntityId>,
) -> (EntityId, bool) {
    if let Some(claimed) = claimed
        && let Ok(id) = EntityId::from_str(claimed)
        && !used.contains(&id)
    {
        return (id, true);
    }
    for nonce in 0_u64.. {
        let mut hash = Sha256::new();
        hash.update(PROFILE_NAME.as_bytes());
        hash.update([0]);
        hash.update(host_id.as_bytes());
        hash.update(nonce.to_be_bytes());
        let digest: [u8; 32] = hash.finalize().into();
        let id = EntityId::new(u128::from_be_bytes(
            digest[..16]
                .try_into()
                .expect("SHA-256 prefix has 16 bytes"),
        ));
        if !used.contains(&id) {
            return (id, false);
        }
    }
    unreachable!("u64 identity nonce space is finite but practically exhaustive")
}

fn header_string_bytes(snapshot: &PluginSnapshot) -> usize {
    snapshot.host_application_version.len()
        + snapshot.host_api_version.len()
        + snapshot.host_document_id.len()
        + snapshot
            .host_document_revision
            .as_ref()
            .map_or(0, String::len)
        + snapshot.page_id.len()
        + snapshot.page_name.len()
        + snapshot.nuif_document_id.as_ref().map_or(0, String::len)
}

fn invalid<T>(pointer: &str, reason: &str) -> Result<T, AdapterError> {
    Err(AdapterError::InvalidValue {
        pointer: pointer.to_owned(),
        reason: reason.to_owned(),
    })
}

fn snapshot_from_document(
    document: &Document,
    host_application_version: &str,
) -> Result<PluginSnapshot, AdapterError> {
    let mut report = empty_import_report(document, host_application_version);
    if host_application_version.trim().is_empty() {
        return invalid("/host_application_version", "value must not be empty");
    }
    if document.roots.len() != 1 {
        unsupported_document(
            document,
            &mut report,
            "profile requires exactly one surface root",
        );
    }
    let root = document
        .roots
        .first()
        .and_then(|id| document.entities.get(id))
        .map(|entity| export_node(document, entity, true, 0, &mut report))
        .transpose()?;
    if !document.tokens.is_empty()
        || !document.relations.is_empty()
        || !document.assets.is_empty()
        || !document.extensions.0.is_empty()
        || document.extension_declarations != ExtensionDeclarations::default()
    {
        unsupported_document(
            document,
            &mut report,
            "tokens, relations, assets and extensions are outside the snapshot profile",
        );
    }
    if !report.is_lossless() || root.is_none() {
        return Err(AdapterError::UnsupportedProfile {
            reason: "one or more document properties are outside the bounded profile".to_owned(),
            report: Box::new(report),
        });
    }
    Ok(PluginSnapshot {
        schema_version: 1,
        host_application_version: host_application_version.to_owned(),
        host_api_version: PLUGIN_API_VERSION.to_owned(),
        host_document_id: format!("plan:{}", document.id),
        host_document_revision: None,
        page_id: "current-page".to_owned(),
        page_name: "NUIF import".to_owned(),
        nuif_document_id: Some(document.id.to_string()),
        root: root.expect("root presence checked"),
    })
}

fn export_node(
    document: &Document,
    entity: &Entity,
    root: bool,
    depth: usize,
    report: &mut HostAdapterReport,
) -> Result<SnapshotNode, AdapterError> {
    if depth >= MAX_DEPTH {
        return Err(AdapterError::TooDeep);
    }
    let (kind, width, height) = export_node_header(entity, root, report);
    let leaf = !matches!(entity.kind, EntityKind::Surface | EntityKind::Container);
    let layout = if leaf && entity.authored.layout != LayoutStyle::default() {
        unsupported_entity(
            entity,
            report,
            "/authored/layout",
            "leaf nodes require default layout metadata",
        );
        HostLayout::default()
    } else {
        export_layout(entity, report)
    };
    let text = export_text(entity, report);
    let children = if leaf && !entity.children.is_empty() {
        unsupported_entity(
            entity,
            report,
            "/children",
            "leaf nodes cannot contain children",
        );
        Vec::new()
    } else {
        entity
            .children
            .iter()
            .filter_map(|id| document.entities.get(id))
            .map(|child| export_node(document, child, false, depth + 1, report))
            .collect::<Result<Vec<_>, _>>()?
    };
    if children.len() != entity.children.len() {
        unsupported_entity(
            entity,
            report,
            "/children",
            "one or more children are missing",
        );
    }
    record_export_correspondence(entity, report);
    Ok(SnapshotNode {
        id: format!("pending:{}", entity.id),
        name: entity.name.clone().unwrap_or_default(),
        kind,
        visible: true,
        opacity: 1.0,
        x: if root {
            0.0
        } else {
            entity.authored.position.x
        },
        y: if root {
            0.0
        } else {
            entity.authored.position.y
        },
        width,
        height,
        fill: export_fill(entity, report),
        layout,
        text,
        nuif_entity_id: Some(entity.id.to_string()),
        unsupported_properties: Vec::new(),
        children,
    })
}

fn export_node_header(
    entity: &Entity,
    root: bool,
    report: &mut HostAdapterReport,
) -> (HostNodeKind, f64, f64) {
    let kind = match (&entity.kind, root) {
        (EntityKind::Surface, true) | (EntityKind::Container, false) => HostNodeKind::Frame,
        (EntityKind::Shape(ShapeKind::Rectangle), false) => HostNodeKind::Rectangle,
        (EntityKind::Shape(ShapeKind::Ellipse), false) => HostNodeKind::Ellipse,
        (EntityKind::Text, false) => HostNodeKind::Text,
        _ => {
            unsupported_entity(
                entity,
                report,
                "/kind",
                "entity kind is outside the profile",
            );
            HostNodeKind::Rectangle
        }
    };
    if !entity.extensions.0.is_empty()
        || entity.semantics != Semantics::default()
        || !entity.authored.responsive.is_empty()
        || !entity.authored.values.is_empty()
        || entity.authored.grid_placement != GridPlacement::default()
        || entity.authored.image.is_some()
    {
        unsupported_entity(
            entity,
            report,
            "",
            "extensions, semantics, responsive values, grid placement and images are outside the profile",
        );
    }
    if !entity.authored.position.x.is_finite() || !entity.authored.position.y.is_finite() {
        unsupported_entity(
            entity,
            report,
            "/authored/position",
            "position must be finite",
        );
    }
    if root && entity.authored.position != Point::default() {
        unsupported_entity(
            entity,
            report,
            "/authored/position",
            "surface position must be the normalized origin",
        );
    }
    let dimensions = match (&entity.authored.width, &entity.authored.height) {
        (SizeIntent::Fixed(width), SizeIntent::Fixed(height))
            if width.is_finite() && *width >= 0.0 && height.is_finite() && *height >= 0.0 =>
        {
            (*width, *height)
        }
        _ => {
            unsupported_entity(
                entity,
                report,
                "/authored",
                "width and height must be finite fixed values",
            );
            (0.0, 0.0)
        }
    };
    if root && (dimensions.0 <= 0.0 || dimensions.1 <= 0.0) {
        unsupported_entity(
            entity,
            report,
            "/authored",
            "the surface requires positive dimensions",
        );
    }
    (kind, dimensions.0, dimensions.1)
}

fn export_layout(entity: &Entity, report: &mut HostAdapterReport) -> HostLayout {
    let layout = &entity.authored.layout;
    let metrics = [
        layout.gap,
        layout.padding.top,
        layout.padding.right,
        layout.padding.bottom,
        layout.padding.left,
    ];
    if metrics
        .into_iter()
        .any(|value| !value.is_finite() || value < 0.0)
    {
        unsupported_entity(
            entity,
            report,
            "/authored/layout",
            "gap and padding must be finite and non-negative",
        );
        return HostLayout::default();
    }
    if layout.family == LayoutFamily::Freeform && layout != &LayoutStyle::default() {
        unsupported_entity(
            entity,
            report,
            "/authored/layout",
            "freeform layout requires default non-applicable metrics",
        );
        return HostLayout::default();
    }
    let mode = match (layout.family, layout.direction) {
        (LayoutFamily::Freeform, _) => HostLayoutMode::None,
        (LayoutFamily::Stack, FlowDirection::Row) => HostLayoutMode::Horizontal,
        (LayoutFamily::Stack, FlowDirection::Column) => HostLayoutMode::Vertical,
        _ => {
            unsupported_entity(
                entity,
                report,
                "/authored/layout/family",
                "only freeform and stack layout are mapped",
            );
            HostLayoutMode::None
        }
    };
    if !layout.grid.columns.is_empty()
        || !layout.grid.rows.is_empty()
        || layout.grid.auto_flow != GridAutoFlow::default()
    {
        unsupported_entity(
            entity,
            report,
            "/authored/layout/grid",
            "grid is outside the profile",
        );
    }
    HostLayout {
        mode,
        item_spacing: layout.gap,
        padding_top: layout.padding.top,
        padding_right: layout.padding.right,
        padding_bottom: layout.padding.bottom,
        padding_left: layout.padding.left,
        primary_axis_align: HostAxisAlign::Min,
        counter_axis_align: match layout.align {
            Align::Start => HostAxisAlign::Min,
            Align::Center => HostAxisAlign::Center,
            Align::End => HostAxisAlign::Max,
            Align::Stretch => {
                unsupported_entity(
                    entity,
                    report,
                    "/authored/layout/align",
                    "stretch has no exact counter-axis setting in this profile",
                );
                HostAxisAlign::Min
            }
        },
    }
}

fn export_fill(entity: &Entity, report: &mut HostAdapterReport) -> Option<SolidPaint> {
    entity.authored.fill.and_then(|color| {
        let channels = [color.red, color.green, color.blue, color.alpha];
        if color.space != ColorSpace::Srgb
            || channels
                .into_iter()
                .any(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
        {
            unsupported_entity(
                entity,
                report,
                "/authored/fill",
                "fill must be finite sRGB with channels in 0..=1",
            );
            None
        } else {
            Some(SolidPaint {
                red: color.red,
                green: color.green,
                blue: color.blue,
                alpha: color.alpha,
            })
        }
    })
}

fn export_text(entity: &Entity, report: &mut HostAdapterReport) -> Option<HostText> {
    match (&entity.kind, &entity.authored.text) {
        (EntityKind::Text, Some(text))
            if text.font == nuif_text::PINNED_FONT_NAME
                && text.font_sha256 == nuif_text::PINNED_FONT_SHA256
                && text.font_asset.is_none()
                && text.size.is_finite()
                && text.size > 0.0
                && text.line_height.is_finite()
                && text.line_height > 0.0
                && text.content.encode_utf16().count() <= MAX_TEXT_UTF16 =>
        {
            Some(HostText {
                characters: text.content.clone(),
                font_family: text.font.clone(),
                font_style: "Regular".to_owned(),
                font_sha256: text.font_sha256.clone(),
                font_size: text.size,
                line_height: text.line_height,
            })
        }
        (EntityKind::Text, _) => {
            unsupported_entity(
                entity,
                report,
                "/authored/text",
                "text requires the exact pinned Ahem identity and bounded literal content",
            );
            None
        }
        (_, None) => None,
        (_, Some(_)) => {
            unsupported_entity(
                entity,
                report,
                "/authored/text",
                "non-text entity carries text",
            );
            None
        }
    }
}

fn empty_import_report(document: &Document, host_version: &str) -> HostAdapterReport {
    HostAdapterReport {
        schema_version: 1,
        profile: PROFILE_NAME.to_owned(),
        direction: HostDirection::Import,
        host_application: format!("{HOST_APPLICATION} {host_version}"),
        host_api_version: PLUGIN_API_VERSION.to_owned(),
        host_document_revision: None,
        canonical_hash: canonical_hash(document).ok(),
        fidelity: Vec::new(),
        correspondences: Vec::new(),
        unmapped_host_data_preserved: true,
    }
}

fn unsupported_document(document: &Document, report: &mut HostAdapterReport, reason: &str) {
    report.fidelity.push(FidelityEntry {
        target: CorrespondenceTarget::Document { id: document.id },
        pointer: String::new(),
        status: Fidelity::Unsupported {
            reason: reason.to_owned(),
        },
    });
}

fn unsupported_entity(entity: &Entity, report: &mut HostAdapterReport, suffix: &str, reason: &str) {
    report.fidelity.push(FidelityEntry {
        target: CorrespondenceTarget::Entity { id: entity.id },
        pointer: format!("/entities/{}{suffix}", entity.id),
        status: Fidelity::Unsupported {
            reason: reason.to_owned(),
        },
    });
}

fn record_export_correspondence(entity: &Entity, report: &mut HostAdapterReport) {
    let target = CorrespondenceTarget::Entity { id: entity.id };
    report.correspondences.push(HostCorrespondenceRecord {
        target: target.clone(),
        host_object_id: format!("pending:{}", entity.id),
        host_property: None,
    });
    report.fidelity.push(FidelityEntry {
        target,
        pointer: format!("/entities/{}", entity.id),
        status: Fidelity::Lossless,
    });
}

/// Reference document for the exact snapshot/mutation-plan subset.
#[must_use]
pub fn profile_fixture() -> Document {
    let mut document = Document::empty(EntityId::new(1));
    let surface_id = EntityId::new(0x10);
    let rectangle_id = EntityId::new(0x20);
    let text_id = EntityId::new(0x21);
    let mut surface = Entity::new(surface_id, EntityKind::Surface);
    surface.name = Some("Profile card".to_owned());
    surface.authored.width = SizeIntent::Fixed(320.0);
    surface.authored.height = SizeIntent::Fixed(200.0);
    surface.authored.fill = Some(Color {
        space: ColorSpace::Srgb,
        red: 1.0,
        green: 1.0,
        blue: 1.0,
        alpha: 1.0,
    });
    surface.authored.layout.family = LayoutFamily::Stack;
    surface.authored.layout.direction = FlowDirection::Column;
    surface.authored.layout.gap = 16.0;
    surface.authored.layout.padding = Edges {
        top: 24.0,
        right: 24.0,
        bottom: 24.0,
        left: 24.0,
    };
    surface.children.extend([rectangle_id, text_id]);
    let mut rectangle = Entity::new(rectangle_id, EntityKind::Shape(ShapeKind::Rectangle));
    rectangle.name = Some("Card".to_owned());
    rectangle.authored.width = SizeIntent::Fixed(272.0);
    rectangle.authored.height = SizeIntent::Fixed(96.0);
    rectangle.authored.fill = Some(Color {
        space: ColorSpace::Srgb,
        red: 0.2,
        green: 0.4,
        blue: 0.8,
        alpha: 1.0,
    });
    let mut text = Entity::new(text_id, EntityKind::Text);
    text.name = Some("Label".to_owned());
    text.authored.width = SizeIntent::Fixed(272.0);
    text.authored.height = SizeIntent::Fixed(24.0);
    text.authored.fill = Some(Color {
        space: ColorSpace::Srgb,
        red: 0.1,
        green: 0.1,
        blue: 0.1,
        alpha: 1.0,
    });
    text.authored.text = Some(TextContent {
        content: "NUIF Figma snapshot".to_owned(),
        font: nuif_text::PINNED_FONT_NAME.to_owned(),
        font_sha256: nuif_text::PINNED_FONT_SHA256.to_owned(),
        font_asset: None,
        size: 16.0,
        line_height: 24.0,
    });
    document.roots.push(surface_id);
    for entity in [surface, rectangle, text] {
        document.entities.insert(entity.id, entity);
    }
    document
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_round_trip_is_exact_and_deterministic() {
        let document = profile_fixture();
        let first = plan_import(&document, "desktop-test").unwrap();
        let second = plan_import(&document, "desktop-test").unwrap();
        assert_eq!(first, second);
        let bytes = serde_json::to_vec(&first.snapshot).unwrap();
        let imported = import_snapshot(&bytes).unwrap();
        assert_eq!(imported.document, document);
        assert!(imported.report.is_lossless());
        assert!(imported.report.validation_errors().is_empty());
    }

    #[test]
    fn missing_and_duplicate_portable_ids_are_repaired_deterministically() {
        let document = profile_fixture();
        let mut plan = plan_import(&document, "desktop-test").unwrap();
        let duplicate = plan.snapshot.root.nuif_entity_id.clone();
        plan.snapshot.root.nuif_entity_id = None;
        plan.snapshot.root.children[0].nuif_entity_id = duplicate;
        let bytes = serde_json::to_vec(&plan.snapshot).unwrap();
        let first = import_snapshot(&bytes).unwrap();
        let second = import_snapshot(&bytes).unwrap();
        assert_eq!(first.document, second.document);
        assert!(!first.report.is_lossless());
        assert!(!first.report.unmapped_host_data_preserved);
        assert!(first.report.fidelity.iter().any(|entry| {
            entry.pointer.ends_with("/identity") && entry.status == Fidelity::Representable
        }));
    }

    #[test]
    fn unsupported_host_appearance_is_never_silent() {
        let document = profile_fixture();
        let mut plan = plan_import(&document, "desktop-test").unwrap();
        plan.snapshot.root.children[0].opacity = 0.5;
        plan.snapshot.root.children[0]
            .unsupported_properties
            .push("effects".to_owned());
        let imported = import_snapshot(&serde_json::to_vec(&plan.snapshot).unwrap()).unwrap();
        assert!(imported.report.fidelity.iter().any(|entry| {
            matches!(entry.status, Fidelity::Unsupported { .. })
                && entry.pointer.ends_with("/appearance")
        }));
        assert!(imported.report.fidelity.iter().any(|entry| {
            matches!(entry.status, Fidelity::Unsupported { .. })
                && entry.pointer.ends_with("/effects")
        }));
    }

    #[test]
    fn hostile_snapshot_bounds_and_structure_fail_atomically() {
        assert!(matches!(
            import_snapshot(&vec![b' '; MAX_SNAPSHOT_BYTES + 1]),
            Err(AdapterError::SnapshotTooLarge)
        ));
        let mut plan = plan_import(&profile_fixture(), "desktop-test").unwrap();
        plan.snapshot.root.children[0].id = plan.snapshot.root.id.clone();
        assert!(matches!(
            import_snapshot(&serde_json::to_vec(&plan.snapshot).unwrap()),
            Err(AdapterError::DuplicateHostId(_))
        ));
        let mut property = plan.snapshot;
        property
            .root
            .unsupported_properties
            .push("../effects".to_owned());
        assert!(matches!(
            import_snapshot(&serde_json::to_vec(&property).unwrap()),
            Err(AdapterError::InvalidValue { .. })
        ));
    }

    #[test]
    fn snapshot_node_depth_count_string_and_text_limits_are_exact() {
        let plan = plan_import(&profile_fixture(), "desktop-test").unwrap();

        let mut deep = plan.snapshot.clone();
        deep.root.children.clear();
        let mut cursor = &mut deep.root;
        for index in 0..MAX_DEPTH {
            cursor.children.push(empty_frame(format!("depth-{index}")));
            cursor = cursor.children.last_mut().unwrap();
        }
        let deep_error = import_normalized_snapshot(&deep).unwrap_err();
        assert!(
            matches!(deep_error, AdapterError::TooDeep),
            "unexpected depth error: {deep_error:?}"
        );
        assert!(import_snapshot(&serde_json::to_vec(&deep).unwrap()).is_err());

        let mut numerous = plan.snapshot.clone();
        let leaf = numerous.root.children[0].clone();
        numerous.root.children.clear();
        for index in 0..MAX_NODES {
            let mut node = leaf.clone();
            node.id = format!("node-{index}");
            node.nuif_entity_id = None;
            numerous.root.children.push(node);
        }
        assert!(matches!(
            import_snapshot(&serde_json::to_vec(&numerous).unwrap()),
            Err(AdapterError::TooManyNodes)
        ));

        let mut strings = plan.snapshot.clone();
        strings.root.name = "x".repeat(MAX_STRING_BYTES + 1);
        assert!(matches!(
            import_snapshot(&serde_json::to_vec(&strings).unwrap()),
            Err(AdapterError::InvalidValue { .. })
        ));

        let mut text = plan.snapshot;
        text.root.children[1].text.as_mut().unwrap().characters = "x".repeat(MAX_TEXT_UTF16 + 1);
        assert!(matches!(
            import_snapshot(&serde_json::to_vec(&text).unwrap()),
            Err(AdapterError::InvalidValue { .. })
        ));
    }

    #[test]
    fn unsupported_document_returns_property_fidelity() {
        let mut document = profile_fixture();
        let id = document.roots[0];
        document
            .entities
            .get_mut(&id)
            .unwrap()
            .authored
            .layout
            .family = LayoutFamily::Grid;
        let AdapterError::UnsupportedProfile { report, .. } =
            plan_import(&document, "desktop-test").unwrap_err()
        else {
            panic!("unsupported layout returned the wrong error");
        };
        assert!(report.fidelity.iter().any(|entry| {
            entry.pointer.ends_with("/authored/layout/family")
                && matches!(entry.status, Fidelity::Unsupported { .. })
        }));
    }

    fn empty_frame(id: String) -> SnapshotNode {
        SnapshotNode {
            id,
            name: "Frame".to_owned(),
            kind: HostNodeKind::Frame,
            visible: true,
            opacity: 1.0,
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
            fill: None,
            layout: HostLayout::default(),
            text: None,
            nuif_entity_id: None,
            unsupported_properties: Vec::new(),
            children: Vec::new(),
        }
    }
}
