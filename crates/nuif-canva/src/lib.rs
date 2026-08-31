#![doc = "Pure mapping for the bounded Canva current-page adoption profile."]

use nuif_adapter::{
    CorrespondenceTarget, FidelityEntry, HostAdapterReport, HostCorrespondenceRecord, HostDirection,
};
use nuif_codec::canonical_hash;
use nuif_core::{
    Color, ColorSpace, Document, Entity, EntityId, EntityKind, ExtensionDeclarations, Fidelity,
    LayoutStyle, Point, ShapeKind, SizeIntent, TextContent, validate,
};
use nuif_text::{PINNED_FONT_NAME, PINNED_FONT_SHA256};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use thiserror::Error;

pub const PROFILE_NAME: &str = "nuif-canva-design-editing-0";
pub const HOST_APPLICATION: &str = "Canva Design";
pub const APPS_SDK_VERSION: &str = "2";
pub const MAX_SNAPSHOT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_ELEMENTS: usize = 16_384;
pub const MAX_DEPTH: usize = 64;
pub const MAX_TEXT_UTF16: usize = 4_096;
pub const MAX_STRING_BYTES: usize = 1_048_576;
pub const MIN_ELEMENT_DIMENSION: f64 = 0.01;

#[derive(Clone, Debug, Error)]
pub enum AdapterError {
    #[error("Canva page snapshot exceeds the {MAX_SNAPSHOT_BYTES}-byte profile limit")]
    SnapshotTooLarge,
    #[error("Canva page snapshot JSON is invalid: {0}")]
    Json(String),
    #[error("Canva page profile marker is invalid: {0}")]
    ProfileMarker(String),
    #[error("invalid Canva page value at {pointer}: {reason}")]
    InvalidValue { pointer: String, reason: String },
    #[error("Canva page exceeds the {MAX_ELEMENTS}-element limit")]
    TooManyElements,
    #[error("Canva page exceeds the {MAX_DEPTH}-level depth limit")]
    TooDeep,
    #[error("duplicate Canva host object id: {0}")]
    DuplicateHostId(String),
    #[error("locked Canva content is outside the profile: {0}")]
    LockedContent(String),
    #[error("document is outside {PROFILE_NAME}: {reason}")]
    UnsupportedProfile {
        reason: String,
        report: Box<HostAdapterReport>,
    },
    #[error("canonical processing failed: {0}")]
    Canonical(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanvaPage {
    pub schema_version: u32,
    pub host_application_version: String,
    pub host_api_version: String,
    pub host_document_id: String,
    pub host_document_revision: Option<String>,
    pub page_id: String,
    pub page_name: String,
    pub width: f64,
    pub height: f64,
    #[serde(default)]
    pub elements: Vec<CanvaElement>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanvaElement {
    pub id: String,
    pub kind: CanvaElementKind,
    pub name: Option<String>,
    pub visible: bool,
    pub locked: bool,
    pub opacity: f64,
    pub rotation: f64,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub fill: Option<SolidColor>,
    pub text: Option<CanvaText>,
    #[serde(default)]
    pub unsupported_properties: Vec<String>,
    #[serde(default)]
    pub children: Vec<CanvaElement>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanvaElementKind {
    Group,
    Rectangle,
    Ellipse,
    Text,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolidColor {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub alpha: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanvaText {
    pub characters: String,
    pub font_family: String,
    pub font_sha256: String,
    pub font_size: f64,
    pub line_height: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImportedPage {
    pub document: Document,
    pub report: HostAdapterReport,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportedPage {
    pub page: CanvaPage,
    pub report: HostAdapterReport,
}

/// Imports one normalized, fixed-size Canva current-page snapshot.
///
/// # Errors
///
/// Returns an [`AdapterError`] when the snapshot is malformed, exceeds a profile
/// limit, or contains a property outside the lossless bounded profile.
pub fn import_current_page(bytes: &[u8]) -> Result<ImportedPage, AdapterError> {
    if bytes.len() > MAX_SNAPSHOT_BYTES {
        return Err(AdapterError::SnapshotTooLarge);
    }
    let page: CanvaPage =
        serde_json::from_slice(bytes).map_err(|error| AdapterError::Json(error.to_string()))?;
    import_normalized_page(&page)
}

/// Imports a normalized page value without reparsing transport bytes.
///
/// # Errors
///
/// Returns an [`AdapterError`] when page metadata, geometry, nesting, or mapped
/// content violates the profile.
pub fn import_normalized_page(page: &CanvaPage) -> Result<ImportedPage, AdapterError> {
    validate_page_header(page)?;
    let mut context = ImportContext {
        document: Document::empty(document_entity_id(&page.host_document_id)),
        host_ids: BTreeSet::new(),
        entity_ids: BTreeSet::new(),
        fidelity: Vec::new(),
        correspondences: Vec::new(),
        elements: 0,
        string_bytes: page_string_bytes(page),
    };
    if context.string_bytes > MAX_STRING_BYTES {
        return Err(AdapterError::InvalidValue {
            pointer: "/".to_owned(),
            reason: format!("string data exceeds {MAX_STRING_BYTES} bytes"),
        });
    }
    let surface_id = page_entity_id(&page.page_id);
    if !context.entity_ids.insert(surface_id) {
        return Err(AdapterError::DuplicateHostId(page.page_id.clone()));
    }
    let mut surface = Entity::new(surface_id, EntityKind::Surface);
    surface.name = Some(page.page_name.clone());
    surface.authored.width = SizeIntent::Fixed(page.width);
    surface.authored.height = SizeIntent::Fixed(page.height);
    context.document.roots.push(surface_id);
    context.document.entities.insert(surface_id, surface);
    for (index, element) in page.elements.iter().enumerate() {
        let id = import_element(&mut context, element, surface_id, 0, index)?;
        let Some(surface) = context.document.entities.get_mut(&surface_id) else {
            return Err(AdapterError::Canonical(
                "surface disappeared while importing page".to_owned(),
            ));
        };
        surface.children.push(id);
    }
    let report = host_report(
        page,
        HostDirection::Import,
        context.fidelity,
        context.correspondences,
        &context.document,
    );
    if let Some(reason) = report
        .fidelity
        .iter()
        .find_map(|entry| match &entry.status {
            Fidelity::Unsupported { reason } => Some(reason.clone()),
            _ => None,
        })
    {
        return Err(AdapterError::UnsupportedProfile {
            reason,
            report: Box::new(report),
        });
    }
    let errors = validate(&context.document)
        .into_iter()
        .filter(|item| item.severity == nuif_core::Severity::Error)
        .count();
    if errors > 0 {
        return Err(AdapterError::Canonical(format!(
            "mapped document has {errors} validation errors"
        )));
    }
    Ok(ImportedPage {
        document: context.document,
        report,
    })
}

/// Produces a deterministic current-page plan for a fixed-size document.
///
/// # Errors
///
/// Returns an [`AdapterError`] when the document cannot be represented without
/// loss in the bounded current-page profile.
pub fn export_page(
    document: &Document,
    host_application_version: &str,
) -> Result<ExportedPage, AdapterError> {
    let mut report = empty_report(document, host_application_version, HostDirection::Export);
    if host_application_version.trim().is_empty() {
        return invalid("/host_application_version", "value must not be empty");
    }
    if document.roots.len() != 1 {
        unsupported_document(
            document,
            &mut report,
            "profile requires exactly one page root",
        );
    }
    if !document.tokens.is_empty()
        || !document.relations.is_empty()
        || !document.assets.is_empty()
        || !document.extensions.0.is_empty()
        || document.extension_declarations != ExtensionDeclarations::default()
    {
        unsupported_document(
            document,
            &mut report,
            "tokens, relations, assets and extensions are outside the profile",
        );
    }
    let root = document
        .roots
        .first()
        .and_then(|id| document.entities.get(id));
    let Some(root) = root else {
        return Err(AdapterError::UnsupportedProfile {
            reason: "page root is missing".to_owned(),
            report: Box::new(report),
        });
    };
    if !matches!(root.kind, EntityKind::Surface) {
        unsupported_entity(
            root,
            &mut report,
            "/kind",
            "the page root must be a surface",
        );
    }
    let elements = root
        .children
        .iter()
        .filter_map(|id| document.entities.get(id))
        .map(|entity| export_element(document, entity, 0, &mut report))
        .collect::<Result<Vec<_>, _>>()?;
    if elements.len() != root.children.len() {
        unsupported_entity(
            root,
            &mut report,
            "/children",
            "one or more page children are missing",
        );
    }
    if !report.is_lossless() {
        return Err(AdapterError::UnsupportedProfile {
            reason: "one or more document properties are outside the bounded Canva page profile"
                .to_owned(),
            report: Box::new(report),
        });
    }
    let page = CanvaPage {
        schema_version: 1,
        host_application_version: host_application_version.to_owned(),
        host_api_version: APPS_SDK_VERSION.to_owned(),
        host_document_id: format!("nuif-doc:{}", document.id),
        host_document_revision: None,
        page_id: format!("nuif-page:{}", root.id),
        page_name: root.name.clone().unwrap_or_else(|| "NUIF page".to_owned()),
        width: fixed_dimension(&root.authored.width, root, "/authored/width", &mut report)?,
        height: fixed_dimension(&root.authored.height, root, "/authored/height", &mut report)?,
        elements,
    };
    Ok(ExportedPage { page, report })
}

struct ImportContext {
    document: Document,
    host_ids: BTreeSet<String>,
    entity_ids: BTreeSet<EntityId>,
    fidelity: Vec<FidelityEntry>,
    correspondences: Vec<HostCorrespondenceRecord>,
    elements: usize,
    string_bytes: usize,
}

fn validate_page_header(page: &CanvaPage) -> Result<(), AdapterError> {
    if page.schema_version != 1 {
        return Err(AdapterError::ProfileMarker(format!(
            "schema version {} is not 1",
            page.schema_version
        )));
    }
    for (pointer, value) in [
        ("/host_application_version", &page.host_application_version),
        ("/host_api_version", &page.host_api_version),
        ("/host_document_id", &page.host_document_id),
        ("/page_id", &page.page_id),
        ("/page_name", &page.page_name),
    ] {
        if value.trim().is_empty() {
            return invalid(pointer, "value must not be empty");
        }
    }
    if page.host_api_version != APPS_SDK_VERSION {
        return Err(AdapterError::ProfileMarker(format!(
            "Apps SDK version {:?} is not {:?}",
            page.host_api_version, APPS_SDK_VERSION
        )));
    }
    for (pointer, value) in [("/width", page.width), ("/height", page.height)] {
        if !value.is_finite() || value < MIN_ELEMENT_DIMENSION {
            return invalid(pointer, "page dimensions must be finite and at least 0.01");
        }
    }
    Ok(())
}

fn import_element(
    context: &mut ImportContext,
    element: &CanvaElement,
    _parent: EntityId,
    depth: usize,
    index: usize,
) -> Result<EntityId, AdapterError> {
    if depth >= MAX_DEPTH {
        return Err(AdapterError::TooDeep);
    }
    context.elements = context.elements.saturating_add(1);
    if context.elements > MAX_ELEMENTS {
        return Err(AdapterError::TooManyElements);
    }
    if element.id.trim().is_empty() {
        return invalid(
            &format!("/elements/{index}/id"),
            "host id must not be empty",
        );
    }
    if !context.host_ids.insert(element.id.clone()) {
        return Err(AdapterError::DuplicateHostId(element.id.clone()));
    }
    if element.locked {
        return Err(AdapterError::LockedContent(element.id.clone()));
    }
    let id = host_entity_id(&element.id);
    if !context.entity_ids.insert(id) {
        return Err(AdapterError::DuplicateHostId(element.id.clone()));
    }
    validate_geometry(element, index)?;
    let mut entity = Entity::new(id, imported_kind(element.kind));
    entity.name.clone_from(&element.name);
    entity.authored.position = Point {
        x: element.x,
        y: element.y,
    };
    entity.authored.width = SizeIntent::Fixed(element.width);
    entity.authored.height = SizeIntent::Fixed(element.height);
    entity.authored.fill = element.fill.map(import_fill);
    if element.rotation != 0.0 {
        unsupported_element(
            context,
            id,
            "/rotation",
            "rotation is not represented in profile 0",
        );
    }
    if (element.opacity - 1.0).abs() > f64::EPSILON {
        unsupported_element(
            context,
            id,
            "/opacity",
            "transparency is not represented in profile 0",
        );
    }
    if !element.visible {
        unsupported_element(
            context,
            id,
            "/visible",
            "hidden elements are not represented in profile 0",
        );
    }
    if !element.unsupported_properties.is_empty() {
        unsupported_element(
            context,
            id,
            "/unsupported_properties",
            "host properties were not mapped",
        );
    }
    if matches!(element.kind, CanvaElementKind::Text) {
        let Some(text) = element.text.as_ref() else {
            return invalid(
                &format!("/elements/{index}/text"),
                "text element requires text content",
            );
        };
        entity.authored.text = Some(import_text(text, id, context)?);
    } else if element.text.is_some() {
        unsupported_element(context, id, "/text", "text is only mapped on text elements");
    }
    for (child_index, child) in element.children.iter().enumerate() {
        if !matches!(element.kind, CanvaElementKind::Group) {
            unsupported_element(context, id, "/children", "only groups may contain children");
            break;
        }
        let child_id = import_element(context, child, id, depth + 1, child_index)?;
        entity.children.push(child_id);
    }
    context.document.entities.insert(id, entity);
    context.correspondences.push(HostCorrespondenceRecord {
        target: CorrespondenceTarget::Entity { id },
        host_object_id: element.id.clone(),
        host_property: None,
    });
    context.fidelity.push(FidelityEntry {
        target: CorrespondenceTarget::Entity { id },
        pointer: format!("/elements/{}", element.id),
        status: Fidelity::Lossless,
    });
    if element.kind == CanvaElementKind::Group {
        context.fidelity.last_mut().expect("just inserted").status = Fidelity::Lossless;
    }
    Ok(id)
}

fn validate_geometry(element: &CanvaElement, index: usize) -> Result<(), AdapterError> {
    for (name, value) in [
        ("x", element.x),
        ("y", element.y),
        ("rotation", element.rotation),
        ("opacity", element.opacity),
    ] {
        if !value.is_finite() {
            return invalid(&format!("/elements/{index}/{name}"), "value must be finite");
        }
    }
    for (name, value) in [("width", element.width), ("height", element.height)] {
        if !value.is_finite() || value < MIN_ELEMENT_DIMENSION {
            return invalid(
                &format!("/elements/{index}/{name}"),
                "dimension must be finite and at least 0.01",
            );
        }
    }
    if !(0.0..=1.0).contains(&element.opacity) {
        return invalid(
            &format!("/elements/{index}/opacity"),
            "opacity must be in 0..=1",
        );
    }
    Ok(())
}

fn imported_kind(kind: CanvaElementKind) -> EntityKind {
    match kind {
        CanvaElementKind::Group => EntityKind::Container,
        CanvaElementKind::Rectangle => EntityKind::Shape(ShapeKind::Rectangle),
        CanvaElementKind::Ellipse => EntityKind::Shape(ShapeKind::Ellipse),
        CanvaElementKind::Text => EntityKind::Text,
    }
}

fn import_fill(fill: SolidColor) -> Color {
    Color {
        space: ColorSpace::Srgb,
        red: fill.red,
        green: fill.green,
        blue: fill.blue,
        alpha: fill.alpha,
    }
}

fn import_text(
    text: &CanvaText,
    id: EntityId,
    context: &mut ImportContext,
) -> Result<TextContent, AdapterError> {
    if text.characters.encode_utf16().count() > MAX_TEXT_UTF16 {
        return Err(AdapterError::InvalidValue {
            pointer: format!("/entities/{id}/text/characters"),
            reason: format!("text exceeds {MAX_TEXT_UTF16} UTF-16 code units"),
        });
    }
    if text.font_family != PINNED_FONT_NAME || text.font_sha256 != PINNED_FONT_SHA256 {
        unsupported_element(
            context,
            id,
            "/text/font",
            "font identity is not the pinned profile font",
        );
    }
    if !text.font_size.is_finite()
        || text.font_size <= 0.0
        || !text.line_height.is_finite()
        || text.line_height <= 0.0
    {
        return Err(AdapterError::InvalidValue {
            pointer: format!("/entities/{id}/text"),
            reason: "font size and line height must be finite and positive".to_owned(),
        });
    }
    Ok(TextContent {
        content: text.characters.clone(),
        font: text.font_family.clone(),
        font_sha256: text.font_sha256.clone(),
        font_asset: None,
        size: text.font_size,
        line_height: text.line_height,
    })
}

fn export_element(
    document: &Document,
    entity: &Entity,
    depth: usize,
    report: &mut HostAdapterReport,
) -> Result<CanvaElement, AdapterError> {
    if depth >= MAX_DEPTH {
        return Err(AdapterError::TooDeep);
    }
    let kind = match entity.kind {
        EntityKind::Container => CanvaElementKind::Group,
        EntityKind::Shape(ShapeKind::Rectangle) => CanvaElementKind::Rectangle,
        EntityKind::Shape(ShapeKind::Ellipse) => CanvaElementKind::Ellipse,
        EntityKind::Text => CanvaElementKind::Text,
        _ => {
            unsupported_entity(
                entity,
                report,
                "/kind",
                "entity kind is outside the profile",
            );
            CanvaElementKind::Rectangle
        }
    };
    if entity.authored.layout != LayoutStyle::default() {
        unsupported_entity(
            entity,
            report,
            "/authored/layout",
            "only freeform layout is mapped",
        );
    }
    if !entity.authored.responsive.is_empty()
        || !entity.authored.values.is_empty()
        || !entity.extensions.0.is_empty()
        || entity.semantics != nuif_core::Semantics::default()
    {
        unsupported_entity(
            entity,
            report,
            "",
            "responsive values, semantics, extensions and custom values are outside the profile",
        );
    }
    let (width, height) = (
        fixed_dimension(&entity.authored.width, entity, "/authored/width", report)?,
        fixed_dimension(&entity.authored.height, entity, "/authored/height", report)?,
    );
    let text = export_text(entity, report);
    if !matches!(entity.kind, EntityKind::Container) && !entity.children.is_empty() {
        unsupported_entity(
            entity,
            report,
            "/children",
            "only groups may contain children",
        );
    }
    let children = export_children(document, entity, depth, report)?;
    record_export_mapping(entity, report);
    Ok(CanvaElement {
        id: format!("nuif:{}", entity.id),
        kind,
        name: entity.name.clone(),
        visible: true,
        locked: false,
        opacity: 1.0,
        rotation: 0.0,
        x: entity.authored.position.x,
        y: entity.authored.position.y,
        width,
        height,
        fill: entity.authored.fill.map(export_fill),
        text,
        unsupported_properties: Vec::new(),
        children,
    })
}

fn export_text(entity: &Entity, report: &mut HostAdapterReport) -> Option<CanvaText> {
    match (&entity.kind, &entity.authored.text) {
        (EntityKind::Text, Some(text))
            if text.font == PINNED_FONT_NAME
                && text.font_sha256 == PINNED_FONT_SHA256
                && text.font_asset.is_none()
                && text.size.is_finite()
                && text.size > 0.0
                && text.line_height.is_finite()
                && text.line_height > 0.0
                && text.content.encode_utf16().count() <= MAX_TEXT_UTF16 =>
        {
            Some(CanvaText {
                characters: text.content.clone(),
                font_family: text.font.clone(),
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
                "text requires the exact pinned font and bounded literal content",
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

fn export_children(
    document: &Document,
    entity: &Entity,
    depth: usize,
    report: &mut HostAdapterReport,
) -> Result<Vec<CanvaElement>, AdapterError> {
    let children = entity
        .children
        .iter()
        .filter_map(|id| document.entities.get(id))
        .map(|child| export_element(document, child, depth + 1, report))
        .collect::<Result<Vec<_>, _>>()?;
    if children.len() != entity.children.len() {
        unsupported_entity(
            entity,
            report,
            "/children",
            "one or more children are missing",
        );
    }
    Ok(children)
}

fn record_export_mapping(entity: &Entity, report: &mut HostAdapterReport) {
    report.correspondences.push(HostCorrespondenceRecord {
        target: CorrespondenceTarget::Entity { id: entity.id },
        host_object_id: format!("nuif:{}", entity.id),
        host_property: None,
    });
    report.fidelity.push(FidelityEntry {
        target: CorrespondenceTarget::Entity { id: entity.id },
        pointer: format!("/entities/{}", entity.id),
        status: Fidelity::Lossless,
    });
}

fn export_fill(color: Color) -> SolidColor {
    SolidColor {
        red: color.red,
        green: color.green,
        blue: color.blue,
        alpha: color.alpha,
    }
}

fn fixed_dimension(
    value: &SizeIntent,
    entity: &Entity,
    pointer: &str,
    report: &mut HostAdapterReport,
) -> Result<f64, AdapterError> {
    match value {
        SizeIntent::Fixed(value) if value.is_finite() && *value >= MIN_ELEMENT_DIMENSION => {
            Ok(*value)
        }
        _ => {
            unsupported_entity(
                entity,
                report,
                pointer,
                "dimension must be a finite fixed value of at least 0.01",
            );
            Err(AdapterError::UnsupportedProfile {
                reason: format!("{pointer} is outside the fixed-page profile"),
                report: Box::new(report.clone()),
            })
        }
    }
}

fn host_entity_id(host_id: &str) -> EntityId {
    if let Some(raw) = host_id.strip_prefix("nuif:")
        && let Ok(value) = u128::from_str_radix(raw, 16)
    {
        return EntityId::new(value);
    }
    let digest = Sha256::digest(format!("canva:{host_id}").as_bytes());
    EntityId::new(u128::from_be_bytes(
        digest[..16].try_into().expect("SHA-256 prefix"),
    ))
}

fn page_entity_id(host_id: &str) -> EntityId {
    if let Some(raw) = host_id.strip_prefix("nuif-page:")
        && let Ok(value) = u128::from_str_radix(raw, 16)
    {
        return EntityId::new(value);
    }
    host_entity_id(&format!("page:{host_id}"))
}

fn document_entity_id(host_id: &str) -> EntityId {
    if let Some(raw) = host_id.strip_prefix("nuif-doc:")
        && let Ok(value) = u128::from_str_radix(raw, 16)
    {
        return EntityId::new(value);
    }
    host_entity_id(&format!("document:{host_id}"))
}

fn page_string_bytes(page: &CanvaPage) -> usize {
    let mut total = page.host_application_version.len()
        + page.host_api_version.len()
        + page.host_document_id.len()
        + page.page_id.len()
        + page.page_name.len();
    for element in &page.elements {
        visit_element_string_bytes(element, &mut total);
    }
    total
}

fn visit_element_string_bytes(element: &CanvaElement, total: &mut usize) {
    *total = total
        .saturating_add(element.id.len())
        .saturating_add(element.name.as_ref().map_or(0, String::len));
    *total = total.saturating_add(element.text.as_ref().map_or(0, |text| {
        text.characters.len() + text.font_family.len() + text.font_sha256.len()
    }));
    for child in &element.children {
        visit_element_string_bytes(child, total);
    }
}

fn host_report(
    page: &CanvaPage,
    direction: HostDirection,
    fidelity: Vec<FidelityEntry>,
    correspondences: Vec<HostCorrespondenceRecord>,
    document: &Document,
) -> HostAdapterReport {
    HostAdapterReport {
        schema_version: 1,
        profile: PROFILE_NAME.to_owned(),
        direction,
        host_application: format!("{HOST_APPLICATION} {}", page.host_application_version),
        host_api_version: page.host_api_version.clone(),
        host_document_revision: page.host_document_revision.clone(),
        canonical_hash: canonical_hash(document).ok(),
        fidelity,
        correspondences,
        unmapped_host_data_preserved: false,
    }
}

fn empty_report(
    document: &Document,
    host_version: &str,
    direction: HostDirection,
) -> HostAdapterReport {
    HostAdapterReport {
        schema_version: 1,
        profile: PROFILE_NAME.to_owned(),
        direction,
        host_application: format!("{HOST_APPLICATION} {host_version}"),
        host_api_version: APPS_SDK_VERSION.to_owned(),
        host_document_revision: None,
        canonical_hash: canonical_hash(document).ok(),
        fidelity: Vec::new(),
        correspondences: Vec::new(),
        unmapped_host_data_preserved: false,
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

fn unsupported_element(context: &mut ImportContext, id: EntityId, pointer: &str, reason: &str) {
    context.fidelity.push(FidelityEntry {
        target: CorrespondenceTarget::Entity { id },
        pointer: format!("/entities/{id}{pointer}"),
        status: Fidelity::Unsupported {
            reason: reason.to_owned(),
        },
    });
}

fn invalid<T>(pointer: &str, reason: &str) -> Result<T, AdapterError> {
    Err(AdapterError::InvalidValue {
        pointer: pointer.to_owned(),
        reason: reason.to_owned(),
    })
}

/// A small fixed-page fixture for pure mapping and host-report tests.
#[must_use]
pub fn profile_fixture() -> Document {
    let mut document = Document::empty(EntityId::new(1));
    let surface_id = EntityId::new(0x10);
    let rectangle_id = EntityId::new(0x20);
    let text_id = EntityId::new(0x21);
    let mut surface = Entity::new(surface_id, EntityKind::Surface);
    surface.name = Some("Canva page".to_owned());
    surface.authored.width = SizeIntent::Fixed(320.0);
    surface.authored.height = SizeIntent::Fixed(200.0);
    let mut rectangle = Entity::new(rectangle_id, EntityKind::Shape(ShapeKind::Rectangle));
    rectangle.name = Some("Card".to_owned());
    rectangle.authored.position = Point { x: 16.0, y: 16.0 };
    rectangle.authored.width = SizeIntent::Fixed(160.0);
    rectangle.authored.height = SizeIntent::Fixed(80.0);
    rectangle.authored.fill = Some(Color {
        space: ColorSpace::Srgb,
        red: 0.2,
        green: 0.4,
        blue: 0.8,
        alpha: 1.0,
    });
    let mut text = Entity::new(text_id, EntityKind::Text);
    text.name = Some("Title".to_owned());
    text.authored.position = Point { x: 24.0, y: 28.0 };
    text.authored.width = SizeIntent::Fixed(120.0);
    text.authored.height = SizeIntent::Fixed(24.0);
    text.authored.text = Some(TextContent {
        content: "Canva profile".to_owned(),
        font: PINNED_FONT_NAME.to_owned(),
        font_sha256: PINNED_FONT_SHA256.to_owned(),
        font_asset: None,
        size: 18.0,
        line_height: 24.0,
    });
    surface.children.extend([rectangle_id, text_id]);
    document.roots.push(surface_id);
    document.entities.insert(surface_id, surface);
    document.entities.insert(rectangle_id, rectangle);
    document.entities.insert(text_id, text);
    document
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page() -> CanvaPage {
        let exported = export_page(&profile_fixture(), "2026.1").unwrap();
        exported.page
    }

    #[test]
    fn fixed_page_round_trip_is_lossless_and_deterministic() {
        let page = page();
        let imported = import_normalized_page(&page).unwrap();
        let exported = export_page(&imported.document, "2026.1").unwrap();
        assert!(imported.report.is_lossless());
        assert!(exported.report.is_lossless());
        assert_eq!(
            canonical_hash(&import_normalized_page(&exported.page).unwrap().document).unwrap(),
            canonical_hash(&imported.document).unwrap()
        );
    }

    #[test]
    fn unsupported_transparency_is_attributed_before_mutation() {
        let mut page = page();
        page.elements[0].opacity = 0.5;
        let error = import_normalized_page(&page).unwrap_err();
        assert!(matches!(error, AdapterError::UnsupportedProfile { .. }));
    }

    #[test]
    fn duplicate_host_ids_fail_closed() {
        let mut page = page();
        page.elements[1].id = page.elements[0].id.clone();
        assert!(matches!(
            import_normalized_page(&page),
            Err(AdapterError::DuplicateHostId(_))
        ));
    }
}
