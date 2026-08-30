#![doc = "Bounded retentive Penpot v3 package adapter."]

use nuif_adapter::{
    CorrespondenceTarget, FidelityEntry, PackageCorrespondenceRecord, PackageEdit, PackageReport,
    SourceSpan,
};
use nuif_codec::canonical_hash;
use nuif_core::{
    Color, ColorSpace, Document, Entity, EntityId, EntityKind, Fidelity, Point, ShapeKind,
    SizeIntent, TextContent,
};
use serde::Deserialize;
use serde_json::{Map, Value, value::RawValue};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read, Write};
use std::str::FromStr;
use thiserror::Error;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

mod profile;

pub use profile::profile_fixture;

pub const PROFILE_NAME: &str = "nuif-penpot-v3-0";
pub const MAX_PACKAGE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_MEMBER_COUNT: usize = 4_096;
pub const MAX_MEMBER_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_EXPANDED_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_JSON_DEPTH: usize = 64;
pub const MAX_JSON_VALUES: usize = 131_072;
pub const MAX_COMPRESSION_RATIO: u64 = 1_000;
const MIN_DEFLATE_BYTES: usize = 4 * 1024;

const SOURCE_FORMAT: &str = "Penpot v3 manifest version 1 / data version 67";
const ROOT_UUID: &str = "00000000-0000-0000-0000-000000000000";
const FEATURES: &[&str] = &[
    "plugins/runtime",
    "design-tokens/v1",
    "variants/v1",
    "layout/grid",
    "styles/v2",
    "components/v2",
    "fdata/shape-data-type",
];
const MIGRATIONS: &[&str] = &[
    "legacy-2",
    "legacy-3",
    "legacy-5",
    "legacy-6",
    "legacy-7",
    "legacy-8",
    "legacy-9",
    "legacy-10",
    "legacy-11",
    "legacy-12",
    "legacy-13",
    "legacy-14",
    "legacy-16",
    "legacy-17",
    "legacy-18",
    "legacy-19",
    "legacy-25",
    "legacy-26",
    "legacy-27",
    "legacy-28",
    "legacy-29",
    "legacy-31",
    "legacy-32",
    "legacy-33",
    "legacy-34",
    "legacy-36",
    "legacy-37",
    "legacy-38",
    "legacy-39",
    "legacy-40",
    "legacy-41",
    "legacy-42",
    "legacy-43",
    "legacy-44",
    "legacy-45",
    "legacy-46",
    "legacy-47",
    "legacy-48",
    "legacy-49",
    "legacy-50",
    "legacy-51",
    "legacy-52",
    "legacy-53",
    "legacy-54",
    "legacy-55",
    "legacy-56",
    "legacy-57",
    "legacy-59",
    "legacy-62",
    "legacy-65",
    "legacy-66",
    "legacy-67",
    "0001-remove-tokens-from-groups",
    "0002-normalize-bool-content-v2",
    "0003-convert-path-content-v2",
    "0005-deprecate-image-type",
    "0006-fix-old-texts-fills",
    "0008-fix-library-colors-v4",
    "0009-clean-library-colors",
    "0009-add-partial-text-touched-flags",
    "0010-fix-swap-slots-pointing-non-existent-shapes",
    "0011-fix-invalid-text-touched-flags",
    "0012-fix-position-data",
    "0013-fix-component-path",
    "0013-clear-invalid-strokes-and-fills",
    "0014-fix-tokens-lib-duplicate-ids",
    "0014-clear-components-nil-objects",
    "0015-fix-text-attrs-blank-strings",
    "0015-clean-shadow-color",
    "0016-copy-fills-from-position-data-to-text-node",
];

#[derive(Debug, Error)]
pub enum AdapterError {
    #[error("Penpot package exceeds the {MAX_PACKAGE_BYTES}-byte input limit")]
    PackageTooLarge,
    #[error("Penpot package exceeds the {MAX_MEMBER_COUNT}-member limit")]
    TooManyMembers,
    #[error("Penpot package member {member} exceeds the {MAX_MEMBER_BYTES}-byte limit")]
    MemberTooLarge { member: String },
    #[error("Penpot package exceeds the {MAX_EXPANDED_BYTES}-byte expanded limit")]
    ExpandedPackageTooLarge,
    #[error("unsafe or non-portable Penpot member name: {0}")]
    UnsafeMemberName(String),
    #[error("duplicate Penpot member name: {0}")]
    DuplicateMember(String),
    #[error("unsupported compression method for Penpot member {member}: {method}")]
    UnsupportedCompression { member: String, method: String },
    #[error("encrypted Penpot member is unsupported: {0}")]
    EncryptedMember(String),
    #[error("invalid ZIP package: {0}")]
    Zip(String),
    #[error("Penpot JSON member {member} is invalid: {reason}")]
    Json { member: String, reason: String },
    #[error("Penpot profile marker is invalid: {0}")]
    ProfileMarker(String),
    #[error("invalid Penpot value at {pointer}: {reason}")]
    InvalidValue { pointer: String, reason: String },
    #[error("document is outside {PROFILE_NAME}: {reason}")]
    UnsupportedProfile {
        reason: String,
        report: Box<PackageReport>,
    },
    #[error("edited document contains changes outside {PROFILE_NAME}: {reason}")]
    UnmappedChanges {
        reason: String,
        report: Box<PackageReport>,
    },
    #[error("synchronized Penpot package did not reimport to the requested document")]
    SynchronizationMismatch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PackageMember {
    name: String,
    payload: Vec<u8>,
    compression: CompressionMethod,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FieldKind {
    Name,
    X,
    Y,
    Width,
    Height,
    X2,
    Y2,
    Fill,
    TextContent,
    TextFont,
    TextFontHash,
    TextSize,
    TextLineHeight,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FieldRecord {
    target: CorrespondenceTarget,
    member: String,
    pointer: String,
    span: SourceSpan,
    kind: FieldKind,
}

#[derive(Clone, Debug)]
pub struct RetentivePackage {
    archive: Vec<u8>,
    members: Vec<PackageMember>,
    document: Document,
    fields: Vec<FieldRecord>,
    report: PackageReport,
}

impl RetentivePackage {
    #[must_use]
    pub fn original_bytes(&self) -> &[u8] {
        &self.archive
    }

    #[must_use]
    pub fn report(&self) -> &PackageReport {
        &self.report
    }
}

#[derive(Clone, Debug)]
pub struct ImportedPackage {
    pub document: Document,
    pub retentive: RetentivePackage,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExportedPackage {
    pub bytes: Vec<u8>,
    pub report: PackageReport,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SynchronizedPackage {
    pub bytes: Vec<u8>,
    pub edits: Vec<PackageEdit>,
    pub report: PackageReport,
}

/// Exports one document inside the bounded Penpot package profile.
///
/// # Errors
///
/// Returns a typed profile, serialization or round-trip error before exposing
/// partial package bytes.
pub fn export_document(document: &Document) -> Result<ExportedPackage, AdapterError> {
    let issues = profile::profile_issues(document);
    if !issues.is_empty() {
        return Err(AdapterError::UnsupportedProfile {
            reason: "model projection rejected one or more properties".to_owned(),
            report: Box::new(package_report(document, issues, Vec::new(), false)?),
        });
    }
    let page_id = derived_page_id(document);
    if document.entities.contains_key(&page_id) || document.id == page_id {
        return Err(AdapterError::UnsupportedProfile {
            reason: "derived Penpot page identity collides with a mapped identity".to_owned(),
            report: Box::new(package_report(document, Vec::new(), Vec::new(), false)?),
        });
    }
    let members = render_members(document, page_id)?;
    let bytes = write_archive(&members)?;
    let imported = import_package(&bytes)?;
    if imported.document != *document {
        return Err(AdapterError::SynchronizationMismatch);
    }
    Ok(ExportedPackage {
        bytes,
        report: imported.retentive.report,
    })
}

/// Imports and retains one bounded Penpot v3 package.
///
/// # Errors
///
/// Returns a typed package, resource, JSON, profile or model error. No archive
/// member is extracted to the filesystem.
#[expect(
    clippy::too_many_lines,
    clippy::case_sensitive_file_extension_comparisons,
    reason = "the bounded package walk keeps its ordering and exact lowercase wire paths visible in one audit surface"
)]
pub fn import_package(bytes: &[u8]) -> Result<ImportedPackage, AdapterError> {
    let members = read_archive(bytes)?;
    let by_name = members
        .iter()
        .map(|member| (member.name.as_str(), member))
        .collect::<BTreeMap<_, _>>();
    let manifest_member = by_name
        .get("manifest.json")
        .ok_or_else(|| AdapterError::ProfileMarker("manifest.json is missing".to_owned()))?;
    let manifest = json_value(manifest_member)?;
    require_integer(&manifest, "version", 1, "/manifest/version")?;
    require_string(&manifest, "type", "penpot/export-files", "/manifest/type")?;
    let files = manifest
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| AdapterError::ProfileMarker("manifest files must be an array".to_owned()))?;
    if files.len() != 1 {
        return Err(AdapterError::ProfileMarker(
            "profile requires exactly one file".to_owned(),
        ));
    }
    if manifest.get("relations").is_some_and(|value| {
        value
            .as_array()
            .is_none_or(|relations| !relations.is_empty())
    }) {
        return Err(AdapterError::ProfileMarker(
            "profile does not admit library relations".to_owned(),
        ));
    }
    let file_uuid = files[0]
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| AdapterError::ProfileMarker("manifest file id is missing".to_owned()))?;
    let document_id = parse_uuid(file_uuid, "/manifest/files/0/id")?;
    let file_member_name = format!("files/{file_uuid}.json");
    let file_member = by_name
        .get(file_member_name.as_str())
        .ok_or_else(|| AdapterError::ProfileMarker(format!("{file_member_name} is missing")))?;
    let file_metadata = json_value(file_member)?;
    if parse_uuid(
        file_metadata
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| AdapterError::ProfileMarker("file metadata id is missing".to_owned()))?,
        "/file/id",
    )? != document_id
    {
        return Err(AdapterError::ProfileMarker(
            "manifest and file metadata identities differ".to_owned(),
        ));
    }
    require_integer(&file_metadata, "version", 67, "/file/version")?;

    let pages_prefix = format!("files/{file_uuid}/pages/");
    let page_members = members
        .iter()
        .filter(|member| {
            member.name.starts_with(&pages_prefix)
                && member.name.ends_with(".json")
                && !member.name[pages_prefix.len()..].contains('/')
        })
        .collect::<Vec<_>>();
    if page_members.len() != 1 {
        return Err(AdapterError::ProfileMarker(
            "profile requires exactly one page metadata member".to_owned(),
        ));
    }
    let page = json_value(page_members[0])?;
    let page_uuid = page
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| AdapterError::ProfileMarker("page id is missing".to_owned()))?;
    parse_uuid(page_uuid, "/page/id")?;
    let expected_page_name = format!("{pages_prefix}{page_uuid}.json");
    if page_members[0].name != expected_page_name {
        return Err(AdapterError::ProfileMarker(
            "page member path and identity differ".to_owned(),
        ));
    }

    let shapes_prefix = format!("{pages_prefix}{page_uuid}/");
    let root_name = format!("{shapes_prefix}{ROOT_UUID}.json");
    let root_member = by_name
        .get(root_name.as_str())
        .ok_or_else(|| AdapterError::ProfileMarker("Penpot root frame is missing".to_owned()))?;
    let root = json_value(root_member)?;
    require_string(&root, "id", ROOT_UUID, "/root/id")?;
    require_string(&root, "type", "frame", "/root/type")?;
    let root_children = string_array(&root, "shapes", "/root/shapes")?;
    if root_children.len() != 1 {
        return Err(AdapterError::ProfileMarker(
            "Penpot root frame must contain exactly one board".to_owned(),
        ));
    }
    let surface_uuid = &root_children[0];
    let surface_id = parse_uuid(surface_uuid, "/root/shapes/0")?;
    let surface_name = format!("{shapes_prefix}{surface_uuid}.json");
    let surface_member = by_name.get(surface_name.as_str()).ok_or_else(|| {
        AdapterError::ProfileMarker("mapped Penpot board member is missing".to_owned())
    })?;

    let mut document = Document::empty(document_id);
    let mut fields = Vec::new();
    let mut fidelity = Vec::new();
    let (mut surface, children) = parse_shape(
        surface_member,
        page_uuid,
        ROOT_UUID,
        ROOT_UUID,
        Some(EntityKind::Surface),
        &mut fields,
        &mut fidelity,
    )?;
    if surface.id != surface_id {
        return Err(AdapterError::ProfileMarker(
            "root child and board identities differ".to_owned(),
        ));
    }
    surface.children.clear();
    for child_uuid in children {
        let child_id = parse_uuid(&child_uuid, "/surface/shapes")?;
        let member_name = format!("{shapes_prefix}{child_uuid}.json");
        let member = by_name.get(member_name.as_str()).ok_or_else(|| {
            AdapterError::ProfileMarker(format!("mapped child member is missing: {member_name}"))
        })?;
        let (child, grandchildren) = parse_shape(
            member,
            page_uuid,
            surface_uuid,
            surface_uuid,
            None,
            &mut fields,
            &mut fidelity,
        )?;
        if child.id != child_id || !grandchildren.is_empty() {
            return Err(AdapterError::ProfileMarker(
                "mapped leaf identity or containment is inconsistent".to_owned(),
            ));
        }
        surface.children.push(child.id);
        if document.entities.insert(child.id, child).is_some() {
            return Err(AdapterError::ProfileMarker(
                "mapped shape identity is duplicated".to_owned(),
            ));
        }
    }
    document.roots.push(surface.id);
    document.entities.insert(surface.id, surface);
    let issues = profile::profile_issues(&document);
    fidelity.extend(issues);
    let mapped_member_names = [
        "manifest.json".to_owned(),
        file_member_name,
        expected_page_name,
        root_name,
    ]
    .into_iter()
    .chain(
        document
            .entities
            .keys()
            .map(|id| format!("{shapes_prefix}{}.json", uuid(*id))),
    )
    .collect::<BTreeSet<_>>();
    for member in members
        .iter()
        .filter(|member| !mapped_member_names.contains(&member.name))
    {
        fidelity.push(FidelityEntry {
            target: CorrespondenceTarget::Document { id: document.id },
            pointer: format!("/package/members/{}", member.name),
            status: Fidelity::PreservedUnrenderable {
                namespace: "org.penpot.package".to_owned(),
            },
        });
    }
    for id in document.entities.keys().copied() {
        let target = CorrespondenceTarget::Entity { id };
        if !fidelity.iter().any(|entry| entry.target == target) {
            fidelity.push(FidelityEntry {
                target,
                pointer: format!("/entities/{id}"),
                status: Fidelity::Lossless,
            });
        }
    }
    if !fidelity
        .iter()
        .all(|entry| entry.status == Fidelity::Lossless)
    {
        fidelity.sort_by(|left, right| {
            left.pointer
                .cmp(&right.pointer)
                .then_with(|| left.target.cmp(&right.target))
        });
    }
    let correspondences = fields
        .iter()
        .map(|field| PackageCorrespondenceRecord {
            target: field.target.clone(),
            member: field.member.clone(),
            pointer: field.pointer.clone(),
            span: field.span,
        })
        .collect::<Vec<_>>();
    let report = package_report(&document, fidelity, correspondences, true)?;
    Ok(ImportedPackage {
        document: document.clone(),
        retentive: RetentivePackage {
            archive: bytes.to_vec(),
            members,
            document,
            fields,
            report,
        },
    })
}

/// Applies mapped scalar changes while retaining every other member payload.
///
/// # Errors
///
/// Returns a typed profile, mapping, ZIP or exact-reimport error without
/// returning a partially rewritten package.
pub fn synchronize(
    retained: &RetentivePackage,
    edited: &Document,
) -> Result<SynchronizedPackage, AdapterError> {
    let issues = profile::profile_issues(edited);
    if !issues.is_empty() {
        return Err(AdapterError::UnmappedChanges {
            reason: "edited model is outside the package profile".to_owned(),
            report: Box::new(package_report(edited, issues, Vec::new(), false)?),
        });
    }
    if !mapped_structure_equal(&retained.document, edited) {
        return Err(AdapterError::UnmappedChanges {
            reason: "containment, identity, kind, optional name/fill inventory or text metadata shape changed"
                .to_owned(),
            report: Box::new(package_report(edited, Vec::new(), Vec::new(), false)?),
        });
    }
    if retained.document == *edited {
        return Ok(SynchronizedPackage {
            bytes: retained.archive.clone(),
            edits: Vec::new(),
            report: retained.report.clone(),
        });
    }

    let mut patches = BTreeMap::<String, Vec<(SourceSpan, Vec<u8>, PackageEdit)>>::new();
    for field in &retained.fields {
        let before = field_value(&retained.document, field)?;
        let after = field_value(edited, field)?;
        if before == after {
            continue;
        }
        let edit = PackageEdit {
            target: field.target.clone(),
            member: field.member.clone(),
            pointer: field.pointer.clone(),
            span: field.span,
        };
        patches
            .entry(field.member.clone())
            .or_default()
            .push((field.span, after, edit));
    }
    let mut members = retained.members.clone();
    let mut edits = Vec::new();
    for member in &mut members {
        let Some(member_patches) = patches.get_mut(&member.name) else {
            continue;
        };
        member_patches.sort_by_key(|left| std::cmp::Reverse(left.0.start));
        let mut previous_start = member.payload.len();
        for (span, replacement, edit) in member_patches.iter() {
            if span.start > span.end || span.end > previous_start || span.end > member.payload.len()
            {
                return Err(AdapterError::SynchronizationMismatch);
            }
            member
                .payload
                .splice(span.start..span.end, replacement.iter().copied());
            previous_start = span.start;
            edits.push(edit.clone());
        }
    }
    edits.sort_by(|left, right| {
        left.member
            .cmp(&right.member)
            .then_with(|| left.span.start.cmp(&right.span.start))
    });
    let bytes = write_archive(&members)?;
    let imported = import_package(&bytes)?;
    if imported.document != *edited {
        return Err(AdapterError::SynchronizationMismatch);
    }
    Ok(SynchronizedPackage {
        bytes,
        edits,
        report: imported.retentive.report,
    })
}

fn read_archive(bytes: &[u8]) -> Result<Vec<PackageMember>, AdapterError> {
    if bytes.len() > MAX_PACKAGE_BYTES {
        return Err(AdapterError::PackageTooLarge);
    }
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| AdapterError::Zip(error.to_string()))?;
    if archive.len() > MAX_MEMBER_COUNT {
        return Err(AdapterError::TooManyMembers);
    }
    if archive
        .decompressed_size()
        .is_some_and(|size| size > MAX_EXPANDED_BYTES as u128)
    {
        return Err(AdapterError::ExpandedPackageTooLarge);
    }
    let mut names = BTreeSet::new();
    let mut expanded = 0_usize;
    let mut members = Vec::with_capacity(archive.len());
    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .map_err(|error| AdapterError::Zip(error.to_string()))?;
        let name = file.name().to_owned();
        if !portable_member_name(&file, &name) {
            return Err(AdapterError::UnsafeMemberName(name));
        }
        if !names.insert(name.clone()) {
            return Err(AdapterError::DuplicateMember(name));
        }
        if file.encrypted() {
            return Err(AdapterError::EncryptedMember(name));
        }
        if file.is_dir() || file.is_symlink() {
            return Err(AdapterError::UnsafeMemberName(name));
        }
        let compression = file.compression();
        if !matches!(
            compression,
            CompressionMethod::Stored | CompressionMethod::Deflated
        ) {
            return Err(AdapterError::UnsupportedCompression {
                member: name,
                method: format!("{compression:?}"),
            });
        }
        if file.size() > MAX_MEMBER_BYTES as u64 {
            return Err(AdapterError::MemberTooLarge { member: name });
        }
        let compressed = file.compressed_size().max(1);
        if file.size() > compressed.saturating_mul(MAX_COMPRESSION_RATIO) {
            return Err(AdapterError::MemberTooLarge { member: name });
        }
        let capacity = usize::try_from(file.size()).map_err(|_| AdapterError::MemberTooLarge {
            member: name.clone(),
        })?;
        let mut payload = Vec::with_capacity(capacity);
        file.by_ref()
            .take(MAX_MEMBER_BYTES as u64 + 1)
            .read_to_end(&mut payload)
            .map_err(|error| AdapterError::Zip(error.to_string()))?;
        if payload.len() > MAX_MEMBER_BYTES {
            return Err(AdapterError::MemberTooLarge { member: name });
        }
        expanded = expanded.saturating_add(payload.len());
        if expanded > MAX_EXPANDED_BYTES {
            return Err(AdapterError::ExpandedPackageTooLarge);
        }
        members.push(PackageMember {
            name,
            payload,
            compression,
        });
    }
    Ok(members)
}

fn portable_member_name<R: Read>(file: &zip::read::ZipFile<'_, R>, name: &str) -> bool {
    name.is_ascii()
        && !name.contains('\\')
        && file
            .enclosed_name()
            .is_some_and(|path| path.to_string_lossy() == name)
}

fn write_archive(members: &[PackageMember]) -> Result<Vec<u8>, AdapterError> {
    let cursor = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(cursor);
    for member in members {
        let options = SimpleFileOptions::default()
            .compression_method(member.compression)
            .last_modified_time(
                zip::DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0)
                    .map_err(|error| AdapterError::Zip(error.to_string()))?,
            )
            .unix_permissions(0o644);
        writer
            .start_file(&member.name, options)
            .map_err(|error| AdapterError::Zip(error.to_string()))?;
        writer
            .write_all(&member.payload)
            .map_err(|error| AdapterError::Zip(error.to_string()))?;
    }
    writer
        .finish()
        .map(Cursor::into_inner)
        .map_err(|error| AdapterError::Zip(error.to_string()))
}

fn render_members(
    document: &Document,
    page_id: EntityId,
) -> Result<Vec<PackageMember>, AdapterError> {
    let file_uuid = uuid(document.id);
    let page_uuid = uuid(page_id);
    let surface = &document.entities[&document.roots[0]];
    let mut members = Vec::new();
    let manifest = serde_json::json!({
        "type": "penpot/export-files",
        "version": 1,
        "generatedBy": "nuif/0.1.0-alpha.2",
        "referer": "nuif",
        "files": [{"id": file_uuid, "name": "NUIF Penpot profile", "features": FEATURES}],
        "relations": [],
    });
    push_json(&mut members, "manifest.json".to_owned(), &manifest)?;
    let file = serde_json::json!({
        "features": FEATURES,
        "name": "NUIF Penpot profile",
        "id": file_uuid,
        "isShared": false,
        "migrations": MIGRATIONS,
        "version": 67,
        "options": {"componentsV2": true, "baseFontSize": "16px"},
    });
    push_json(&mut members, format!("files/{file_uuid}.json"), &file)?;
    push_json(
        &mut members,
        format!("files/{file_uuid}/pages/{page_uuid}.json"),
        &serde_json::json!({"id": page_uuid, "name": "Profile page", "index": 0}),
    )?;
    let prefix = format!("files/{file_uuid}/pages/{page_uuid}");
    let root = base_shape(
        EntityId::new(0),
        "Root Frame",
        "frame",
        Point::default(),
        0.01,
        0.01,
        ROOT_UUID,
        ROOT_UUID,
        &page_uuid,
    );
    let mut root = root;
    root.insert(
        "fills".to_owned(),
        serde_json::json!([{"fillColor":"#FFFFFF","fillOpacity":1}]),
    );
    root.insert("shapes".to_owned(), serde_json::json!([uuid(surface.id)]));
    push_json(
        &mut members,
        format!("{prefix}/{ROOT_UUID}.json"),
        &Value::Object(root),
    )?;
    let mut board = render_shape(surface, ROOT_UUID, ROOT_UUID, &page_uuid);
    board.insert(
        "shapes".to_owned(),
        Value::Array(
            surface
                .children
                .iter()
                .map(|id| Value::String(uuid(*id)))
                .collect(),
        ),
    );
    push_json(
        &mut members,
        format!("{prefix}/{}.json", uuid(surface.id)),
        &Value::Object(board),
    )?;
    for child in &surface.children {
        let entity = &document.entities[child];
        let value = Value::Object(render_shape(
            entity,
            &uuid(surface.id),
            &uuid(surface.id),
            &page_uuid,
        ));
        push_json(
            &mut members,
            format!("{prefix}/{}.json", uuid(entity.id)),
            &value,
        )?;
    }
    Ok(members)
}

fn push_json(
    members: &mut Vec<PackageMember>,
    name: String,
    value: &Value,
) -> Result<(), AdapterError> {
    let payload = serde_json::to_vec(value).map_err(|error| AdapterError::Json {
        member: name.clone(),
        reason: error.to_string(),
    })?;
    members.push(PackageMember {
        name,
        compression: if payload.len() >= MIN_DEFLATE_BYTES {
            CompressionMethod::Deflated
        } else {
            CompressionMethod::Stored
        },
        payload,
    });
    Ok(())
}

fn render_shape(
    entity: &Entity,
    parent_uuid: &str,
    frame_uuid: &str,
    page_uuid: &str,
) -> Map<String, Value> {
    let kind = match entity.kind {
        EntityKind::Surface => "frame",
        EntityKind::Shape(ShapeKind::Rectangle) => "rect",
        EntityKind::Shape(ShapeKind::Ellipse) => "circle",
        EntityKind::Text => "text",
        _ => unreachable!("profile validation rejects unsupported kinds"),
    };
    let width = fixed(&entity.authored.width);
    let height = fixed(&entity.authored.height);
    let mut shape = base_shape(
        entity.id,
        entity.name.as_deref().expect("profile requires names"),
        kind,
        entity.authored.position,
        width,
        height,
        parent_uuid,
        frame_uuid,
        page_uuid,
    );
    if let Some(fill) = entity.authored.fill {
        shape.insert(
            "fills".to_owned(),
            serde_json::json!([{"fillColor": color(fill), "fillOpacity": 1}]),
        );
    }
    if let Some(text) = &entity.authored.text {
        shape.insert("content".to_owned(), Value::String(text.content.clone()));
        shape.insert(
            "pluginData".to_owned(),
            serde_json::json!({
                "org-nuif": {
                    "profile": PROFILE_NAME,
                    "font": text.font,
                    "font-sha256": text.font_sha256,
                    "size": number(text.size),
                    "line-height": number(text.line_height),
                }
            }),
        );
    }
    shape
}

#[allow(clippy::too_many_arguments)]
fn base_shape(
    id: EntityId,
    name: &str,
    kind: &str,
    position: Point,
    width: f64,
    height: f64,
    parent_uuid: &str,
    frame_uuid: &str,
    page_uuid: &str,
) -> Map<String, Value> {
    let x2 = position.x + width;
    let y2 = position.y + height;
    let mut shape = Map::new();
    shape.insert("id".to_owned(), Value::String(uuid(id)));
    shape.insert("name".to_owned(), Value::String(name.to_owned()));
    shape.insert("type".to_owned(), Value::String(kind.to_owned()));
    shape.insert("x".to_owned(), json_number(position.x));
    shape.insert("y".to_owned(), json_number(position.y));
    shape.insert("width".to_owned(), json_number(width));
    shape.insert("height".to_owned(), json_number(height));
    shape.insert("rotation".to_owned(), Value::from(0));
    shape.insert(
        "selrect".to_owned(),
        serde_json::json!({
            "x": position.x, "y": position.y, "width": width, "height": height,
            "x1": position.x, "y1": position.y, "x2": x2, "y2": y2,
        }),
    );
    shape.insert(
        "points".to_owned(),
        serde_json::json!([
            {"x":position.x,"y":position.y}, {"x":x2,"y":position.y},
            {"x":x2,"y":y2}, {"x":position.x,"y":y2}
        ]),
    );
    shape.insert(
        "transform".to_owned(),
        serde_json::json!({"a":1,"b":0,"c":0,"d":1,"e":0,"f":0}),
    );
    shape.insert(
        "transformInverse".to_owned(),
        serde_json::json!({"a":1,"b":0,"c":0,"d":1,"e":0,"f":0}),
    );
    shape.insert("parentId".to_owned(), Value::String(parent_uuid.to_owned()));
    shape.insert("frameId".to_owned(), Value::String(frame_uuid.to_owned()));
    shape.insert("flipX".to_owned(), Value::Null);
    shape.insert("flipY".to_owned(), Value::Null);
    shape.insert("strokes".to_owned(), Value::Array(Vec::new()));
    shape.insert("pageId".to_owned(), Value::String(page_uuid.to_owned()));
    shape
}

fn json_number(value: f64) -> Value {
    Value::Number(serde_json::Number::from_f64(value).expect("profile numbers are finite"))
}

#[derive(Deserialize)]
struct RawShape<'a> {
    #[serde(borrow)]
    id: &'a RawValue,
    #[serde(borrow)]
    name: &'a RawValue,
    #[serde(rename = "type", borrow)]
    kind: &'a RawValue,
    #[serde(borrow)]
    x: &'a RawValue,
    #[serde(borrow)]
    y: &'a RawValue,
    #[serde(borrow)]
    width: &'a RawValue,
    #[serde(borrow)]
    height: &'a RawValue,
    #[serde(borrow)]
    selrect: &'a RawValue,
    #[serde(borrow)]
    points: &'a RawValue,
    #[serde(default, borrow)]
    fills: Option<&'a RawValue>,
    #[serde(default, borrow)]
    content: Option<&'a RawValue>,
    #[serde(rename = "pluginData", default, borrow)]
    plugin_data: Option<&'a RawValue>,
    #[serde(default, borrow)]
    shapes: Option<&'a RawValue>,
    #[serde(rename = "pageId", default, borrow)]
    page_id: Option<&'a RawValue>,
    #[serde(rename = "parentId", default, borrow)]
    parent_id: Option<&'a RawValue>,
    #[serde(rename = "frameId", default, borrow)]
    frame_id: Option<&'a RawValue>,
    #[serde(default, borrow)]
    rotation: Option<&'a RawValue>,
    #[serde(rename = "flipX", default, borrow)]
    flip_x: Option<&'a RawValue>,
    #[serde(rename = "flipY", default, borrow)]
    flip_y: Option<&'a RawValue>,
    #[serde(default, borrow)]
    strokes: Option<&'a RawValue>,
    #[serde(default, borrow)]
    transform: Option<&'a RawValue>,
    #[serde(rename = "transformInverse", default, borrow)]
    transform_inverse: Option<&'a RawValue>,
}

#[derive(Deserialize)]
struct RawRect<'a> {
    #[serde(borrow)]
    x: &'a RawValue,
    #[serde(borrow)]
    y: &'a RawValue,
    #[serde(borrow)]
    width: &'a RawValue,
    #[serde(borrow)]
    height: &'a RawValue,
    #[serde(borrow)]
    x1: &'a RawValue,
    #[serde(borrow)]
    y1: &'a RawValue,
    #[serde(borrow)]
    x2: &'a RawValue,
    #[serde(borrow)]
    y2: &'a RawValue,
}

#[derive(Deserialize)]
struct RawPoint<'a> {
    #[serde(borrow)]
    x: &'a RawValue,
    #[serde(borrow)]
    y: &'a RawValue,
}

#[derive(Deserialize)]
struct RawFill<'a> {
    #[serde(rename = "fillColor", borrow)]
    color: &'a RawValue,
    #[serde(rename = "fillOpacity", borrow)]
    opacity: &'a RawValue,
}

#[derive(Deserialize)]
struct RawNuifText<'a> {
    #[serde(borrow)]
    profile: &'a RawValue,
    #[serde(borrow)]
    font: &'a RawValue,
    #[serde(rename = "font-sha256", borrow)]
    font_hash: &'a RawValue,
    #[serde(borrow)]
    size: &'a RawValue,
    #[serde(rename = "line-height", borrow)]
    line_height: &'a RawValue,
}

#[allow(clippy::too_many_arguments)]
#[expect(
    clippy::too_many_lines,
    reason = "one member parser keeps span capture and semantic validation adjacent"
)]
fn parse_shape(
    member: &PackageMember,
    page_uuid: &str,
    parent_uuid: &str,
    frame_uuid: &str,
    forced_kind: Option<EntityKind>,
    fields: &mut Vec<FieldRecord>,
    fidelity: &mut Vec<FidelityEntry>,
) -> Result<(Entity, Vec<String>), AdapterError> {
    let source = std::str::from_utf8(&member.payload).map_err(|error| AdapterError::Json {
        member: member.name.clone(),
        reason: error.to_string(),
    })?;
    let raw: RawShape<'_> = serde_json::from_str(source).map_err(|error| AdapterError::Json {
        member: member.name.clone(),
        reason: error.to_string(),
    })?;
    let shape_value: Value = serde_json::from_str(source).map_err(|error| AdapterError::Json {
        member: member.name.clone(),
        reason: error.to_string(),
    })?;
    json_resources(&shape_value, &member.name)?;
    let id = parse_uuid(&raw_string(raw.id, &member.name)?, "/shape/id")?;
    let kind_name = raw_string(raw.kind, &member.name)?;
    let kind = if let Some(kind) = forced_kind {
        kind
    } else {
        match kind_name.as_str() {
            "rect" => EntityKind::Shape(ShapeKind::Rectangle),
            "circle" => EntityKind::Shape(ShapeKind::Ellipse),
            "text" => EntityKind::Text,
            _ => {
                return Err(AdapterError::InvalidValue {
                    pointer: format!("/entities/{id}/kind"),
                    reason: format!("shape type {kind_name} is outside {PROFILE_NAME}"),
                });
            }
        }
    };
    let expected_kind = match kind {
        EntityKind::Surface => "frame",
        EntityKind::Shape(ShapeKind::Rectangle) => "rect",
        EntityKind::Shape(ShapeKind::Ellipse) => "circle",
        EntityKind::Text => "text",
        _ => {
            return Err(AdapterError::InvalidValue {
                pointer: format!("/entities/{id}/kind"),
                reason: format!("shape type {kind_name} is outside {PROFILE_NAME}"),
            });
        }
    };
    if kind_name != expected_kind {
        return Err(AdapterError::InvalidValue {
            pointer: format!("/entities/{id}/kind"),
            reason: "shape type does not match its containment role".to_owned(),
        });
    }
    validate_shape_defaults(
        &raw,
        &shape_value,
        page_uuid,
        parent_uuid,
        frame_uuid,
        id,
        fidelity,
    )?;
    let x = raw_number(raw.x, "/shape/x")?;
    let y = raw_number(raw.y, "/shape/y")?;
    let width = raw_number(raw.width, "/shape/width")?;
    let height = raw_number(raw.height, "/shape/height")?;
    if ![x, y, width, height].into_iter().all(f64::is_finite) || width < 0.0 || height < 0.0 {
        return Err(AdapterError::InvalidValue {
            pointer: format!("/entities/{id}/authored"),
            reason: "geometry must be finite and dimensions non-negative".to_owned(),
        });
    }
    let mut entity = Entity::new(id, kind);
    entity.name = Some(raw_string(raw.name, &member.name)?);
    entity.authored.position = Point { x, y };
    entity.authored.width = SizeIntent::Fixed(width);
    entity.authored.height = SizeIntent::Fixed(height);
    entity.authored.fill = parse_fill(raw.fills, &member.name)?;

    let target = CorrespondenceTarget::Entity { id };
    add_field(fields, &target, member, raw.name, "/name", FieldKind::Name)?;
    add_geometry_fields(fields, &target, member, &raw, id)?;
    if let Some(fills) = raw.fills {
        let parsed: Vec<RawFill<'_>> =
            serde_json::from_str(fills.get()).map_err(|error| AdapterError::Json {
                member: member.name.clone(),
                reason: error.to_string(),
            })?;
        if let Some(fill) = parsed.first() {
            add_field(
                fields,
                &target,
                member,
                fill.color,
                "/authored/fill",
                FieldKind::Fill,
            )?;
        }
    }

    if matches!(entity.kind, EntityKind::Text) {
        let content = raw.content.ok_or_else(|| AdapterError::InvalidValue {
            pointer: format!("/entities/{id}/authored/text/content"),
            reason: "literal text content is missing".to_owned(),
        })?;
        let plugin_data = raw.plugin_data.ok_or_else(|| AdapterError::InvalidValue {
            pointer: format!("/entities/{id}/authored/text"),
            reason: "org-nuif pinned-font metadata is missing".to_owned(),
        })?;
        let metadata = raw_nuif_metadata(plugin_data, &member.name)?;
        if raw_string(metadata.profile, &member.name)? != PROFILE_NAME {
            return Err(AdapterError::InvalidValue {
                pointer: format!("/entities/{id}/authored/text"),
                reason: "org-nuif profile marker differs".to_owned(),
            });
        }
        let font = raw_string(metadata.font, &member.name)?;
        let font_sha256 = raw_string(metadata.font_hash, &member.name)?;
        let size = raw_string(metadata.size, &member.name)?
            .parse::<f64>()
            .map_err(|error| AdapterError::InvalidValue {
                pointer: format!("/entities/{id}/authored/text/size"),
                reason: error.to_string(),
            })?;
        let line_height = raw_string(metadata.line_height, &member.name)?
            .parse::<f64>()
            .map_err(|error| AdapterError::InvalidValue {
                pointer: format!("/entities/{id}/authored/text/line_height"),
                reason: error.to_string(),
            })?;
        entity.authored.text = Some(TextContent {
            content: raw_string(content, &member.name)?,
            font,
            font_sha256,
            size,
            line_height,
        });
        for (raw_value, suffix, kind) in [
            (content, "/authored/text/content", FieldKind::TextContent),
            (metadata.font, "/authored/text/font", FieldKind::TextFont),
            (
                metadata.font_hash,
                "/authored/text/font_sha256",
                FieldKind::TextFontHash,
            ),
            (metadata.size, "/authored/text/size", FieldKind::TextSize),
            (
                metadata.line_height,
                "/authored/text/line_height",
                FieldKind::TextLineHeight,
            ),
        ] {
            add_field(fields, &target, member, raw_value, suffix, kind)?;
        }
    } else if raw.content.is_some() || raw.plugin_data.is_some() {
        unsupported_fidelity(
            fidelity,
            target.clone(),
            format!("/entities/{id}"),
            "non-text content or plug-in data is retained but not mapped",
        );
    }
    let children = raw.shapes.map_or_else(
        || Ok(Vec::new()),
        |value| raw_string_array(value, "/shape/shapes"),
    )?;
    Ok((entity, children))
}

fn raw_nuif_metadata<'a>(
    plugin_data: &'a RawValue,
    member: &str,
) -> Result<RawNuifText<'a>, AdapterError> {
    #[derive(Deserialize)]
    struct Wrapper<'a> {
        #[serde(rename = "org-nuif", borrow)]
        nuif: &'a RawValue,
    }
    let wrapper: Wrapper<'_> =
        serde_json::from_str(plugin_data.get()).map_err(|error| AdapterError::Json {
            member: member.to_owned(),
            reason: error.to_string(),
        })?;
    serde_json::from_str(wrapper.nuif.get()).map_err(|error| AdapterError::Json {
        member: member.to_owned(),
        reason: error.to_string(),
    })
}

#[expect(
    clippy::too_many_lines,
    reason = "the accepted Penpot default inventory is intentionally explicit and locally auditable"
)]
fn validate_shape_defaults(
    shape: &RawShape<'_>,
    shape_value: &Value,
    page_uuid: &str,
    parent_uuid: &str,
    frame_uuid: &str,
    id: EntityId,
    fidelity: &mut Vec<FidelityEntry>,
) -> Result<(), AdapterError> {
    let target = CorrespondenceTarget::Entity { id };
    for (name, raw, expected, required) in [
        (
            "pageId",
            shape.page_id,
            Value::String(page_uuid.to_owned()),
            true,
        ),
        (
            "parentId",
            shape.parent_id,
            Value::String(parent_uuid.to_owned()),
            true,
        ),
        (
            "frameId",
            shape.frame_id,
            Value::String(frame_uuid.to_owned()),
            true,
        ),
        ("rotation", shape.rotation, Value::from(0), false),
        ("flipX", shape.flip_x, Value::Null, false),
        ("flipY", shape.flip_y, Value::Null, false),
        ("strokes", shape.strokes, Value::Array(Vec::new()), false),
    ] {
        let Some(raw) = raw else {
            if required {
                return Err(AdapterError::InvalidValue {
                    pointer: format!("/entities/{id}/{name}"),
                    reason: "required Penpot relationship is missing".to_owned(),
                });
            }
            continue;
        };
        let observed: Value =
            serde_json::from_str(raw.get()).map_err(|error| AdapterError::InvalidValue {
                pointer: format!("/entities/{id}/{name}"),
                reason: error.to_string(),
            })?;
        if observed != expected
            && !(matches!(name, "flipX" | "flipY") && observed == Value::Bool(false))
        {
            return Err(AdapterError::InvalidValue {
                pointer: format!("/entities/{id}/{name}"),
                reason: "relationship, transform or stroke default differs".to_owned(),
            });
        }
    }
    for (name, raw) in [
        ("transform", shape.transform),
        ("transformInverse", shape.transform_inverse),
    ] {
        if let Some(raw) = raw {
            let value: Value =
                serde_json::from_str(raw.get()).map_err(|error| AdapterError::InvalidValue {
                    pointer: format!("/entities/{id}/{name}"),
                    reason: error.to_string(),
                })?;
            if !identity_transform(&value) {
                return Err(AdapterError::InvalidValue {
                    pointer: format!("/entities/{id}/{name}"),
                    reason: "only identity transforms are mapped".to_owned(),
                });
            }
        }
    }
    let object = shape_value
        .as_object()
        .ok_or_else(|| AdapterError::InvalidValue {
            pointer: format!("/entities/{id}"),
            reason: "shape must be a JSON object".to_owned(),
        })?;
    for (name, expected) in [
        ("r1", Value::from(0)),
        ("r2", Value::from(0)),
        ("r3", Value::from(0)),
        ("r4", Value::from(0)),
        ("proportion", Value::from(1)),
        ("proportionLock", Value::Bool(false)),
        ("growType", Value::String("fixed".to_owned())),
        ("hideInViewer", Value::Bool(false)),
        ("hideFillOnExport", Value::Bool(false)),
    ] {
        if let Some(observed) = object.get(name)
            && observed != &expected
        {
            return Err(AdapterError::InvalidValue {
                pointer: format!("/entities/{id}/{name}"),
                reason: "optional Penpot default differs from the bounded profile".to_owned(),
            });
        }
    }
    let known = [
        "id",
        "name",
        "type",
        "x",
        "y",
        "width",
        "height",
        "selrect",
        "points",
        "fills",
        "content",
        "pluginData",
        "shapes",
        "pageId",
        "parentId",
        "frameId",
        "rotation",
        "flipX",
        "flipY",
        "strokes",
        "transform",
        "transformInverse",
        "r1",
        "r2",
        "r3",
        "r4",
        "proportion",
        "proportionLock",
        "growType",
        "hideInViewer",
        "hideFillOnExport",
    ];
    for name in object.keys().filter(|name| !known.contains(&name.as_str())) {
        unsupported_fidelity(
            fidelity,
            target.clone(),
            format!("/entities/{id}"),
            &format!("unrecognized Penpot field {name} is retained but not interpreted"),
        );
    }
    Ok(())
}

fn identity_transform(value: &Value) -> bool {
    if let Some(object) = value.as_object() {
        return ["a", "d"]
            .into_iter()
            .all(|key| object.get(key).and_then(Value::as_f64) == Some(1.0))
            && ["b", "c", "e", "f"]
                .into_iter()
                .all(|key| object.get(key).and_then(Value::as_f64) == Some(0.0));
    }
    value.as_array().is_some_and(|array| {
        array.len() == 6
            && array
                .iter()
                .filter_map(Value::as_f64)
                .eq([1.0, 0.0, 0.0, 1.0, 0.0, 0.0])
    })
}

fn parse_fill(raw: Option<&RawValue>, member: &str) -> Result<Option<Color>, AdapterError> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let fills: Vec<RawFill<'_>> =
        serde_json::from_str(raw.get()).map_err(|error| AdapterError::Json {
            member: member.to_owned(),
            reason: error.to_string(),
        })?;
    if fills.is_empty() {
        return Ok(None);
    }
    let fill_values: Value =
        serde_json::from_str(raw.get()).map_err(|error| AdapterError::Json {
            member: member.to_owned(),
            reason: error.to_string(),
        })?;
    let exact_fields = fill_values
        .as_array()
        .and_then(|values| values.first())
        .and_then(Value::as_object)
        .is_some_and(|object| {
            object.len() == 2
                && object.contains_key("fillColor")
                && object.contains_key("fillOpacity")
        });
    if fills.len() != 1
        || !exact_fields
        || !same_number(raw_number(fills[0].opacity, "/fills/0/fillOpacity")?, 1.0)
    {
        return Err(AdapterError::InvalidValue {
            pointer: "/shape/fills".to_owned(),
            reason: "profile maps at most one opaque solid fill".to_owned(),
        });
    }
    parse_color(&raw_string(fills[0].color, member)?).map(Some)
}

fn add_geometry_fields(
    fields: &mut Vec<FieldRecord>,
    target: &CorrespondenceTarget,
    member: &PackageMember,
    raw: &RawShape<'_>,
    id: EntityId,
) -> Result<(), AdapterError> {
    let rect: RawRect<'_> =
        serde_json::from_str(raw.selrect.get()).map_err(|error| AdapterError::InvalidValue {
            pointer: format!("/entities/{id}/authored"),
            reason: error.to_string(),
        })?;
    let points: Vec<RawPoint<'_>> =
        serde_json::from_str(raw.points.get()).map_err(|error| AdapterError::InvalidValue {
            pointer: format!("/entities/{id}/authored"),
            reason: error.to_string(),
        })?;
    if points.len() != 4 {
        return Err(AdapterError::InvalidValue {
            pointer: format!("/entities/{id}/authored"),
            reason: "axis-aligned profile geometry requires four points".to_owned(),
        });
    }
    let entries = [
        (raw.x, "/authored/position/x", FieldKind::X),
        (rect.x, "/authored/position/x", FieldKind::X),
        (rect.x1, "/authored/position/x", FieldKind::X),
        (points[0].x, "/authored/position/x", FieldKind::X),
        (points[3].x, "/authored/position/x", FieldKind::X),
        (raw.y, "/authored/position/y", FieldKind::Y),
        (rect.y, "/authored/position/y", FieldKind::Y),
        (rect.y1, "/authored/position/y", FieldKind::Y),
        (points[0].y, "/authored/position/y", FieldKind::Y),
        (points[1].y, "/authored/position/y", FieldKind::Y),
        (raw.width, "/authored/width", FieldKind::Width),
        (rect.width, "/authored/width", FieldKind::Width),
        (raw.height, "/authored/height", FieldKind::Height),
        (rect.height, "/authored/height", FieldKind::Height),
        (rect.x2, "/authored", FieldKind::X2),
        (points[1].x, "/authored", FieldKind::X2),
        (points[2].x, "/authored", FieldKind::X2),
        (rect.y2, "/authored", FieldKind::Y2),
        (points[2].y, "/authored", FieldKind::Y2),
        (points[3].y, "/authored", FieldKind::Y2),
    ];
    for (value, suffix, kind) in entries {
        add_field(fields, target, member, value, suffix, kind)?;
    }
    let x = raw_number(raw.x, "/shape/x")?;
    let y = raw_number(raw.y, "/shape/y")?;
    let width = raw_number(raw.width, "/shape/width")?;
    let height = raw_number(raw.height, "/shape/height")?;
    for (value, expected) in [
        (rect.x, x),
        (rect.x1, x),
        (rect.y, y),
        (rect.y1, y),
        (rect.width, width),
        (rect.height, height),
        (rect.x2, x + width),
        (rect.y2, y + height),
        (points[0].x, x),
        (points[0].y, y),
        (points[1].x, x + width),
        (points[1].y, y),
        (points[2].x, x + width),
        (points[2].y, y + height),
        (points[3].x, x),
        (points[3].y, y + height),
    ] {
        if !same_number(raw_number(value, "/shape/derived-geometry")?, expected) {
            return Err(AdapterError::InvalidValue {
                pointer: format!("/entities/{id}/authored"),
                reason: "derived selection rectangle or points are inconsistent".to_owned(),
            });
        }
    }
    Ok(())
}

fn add_field(
    fields: &mut Vec<FieldRecord>,
    target: &CorrespondenceTarget,
    member: &PackageMember,
    raw: &RawValue,
    suffix: &str,
    kind: FieldKind,
) -> Result<(), AdapterError> {
    let id = match target {
        CorrespondenceTarget::Entity { id } => *id,
        _ => unreachable!("Penpot fields map to entities"),
    };
    fields.push(FieldRecord {
        target: target.clone(),
        member: member.name.clone(),
        pointer: format!("/entities/{id}{suffix}"),
        span: raw_span(&member.payload, raw)?,
        kind,
    });
    Ok(())
}

fn raw_span(source: &[u8], raw: &RawValue) -> Result<SourceSpan, AdapterError> {
    let start = (raw.get().as_ptr() as usize)
        .checked_sub(source.as_ptr() as usize)
        .ok_or(AdapterError::SynchronizationMismatch)?;
    let end = start + raw.get().len();
    if end > source.len() {
        return Err(AdapterError::SynchronizationMismatch);
    }
    Ok(SourceSpan { start, end })
}

fn field_value(document: &Document, field: &FieldRecord) -> Result<Vec<u8>, AdapterError> {
    let CorrespondenceTarget::Entity { id } = field.target else {
        return Err(AdapterError::SynchronizationMismatch);
    };
    let entity = document
        .entities
        .get(&id)
        .ok_or(AdapterError::SynchronizationMismatch)?;
    let value = match field.kind {
        FieldKind::Name => serde_json::to_string(
            entity
                .name
                .as_deref()
                .ok_or(AdapterError::SynchronizationMismatch)?,
        ),
        FieldKind::X => Ok(number(entity.authored.position.x)),
        FieldKind::Y => Ok(number(entity.authored.position.y)),
        FieldKind::Width => Ok(number(fixed(&entity.authored.width))),
        FieldKind::Height => Ok(number(fixed(&entity.authored.height))),
        FieldKind::X2 => Ok(number(
            entity.authored.position.x + fixed(&entity.authored.width),
        )),
        FieldKind::Y2 => Ok(number(
            entity.authored.position.y + fixed(&entity.authored.height),
        )),
        FieldKind::Fill => serde_json::to_string(&color(
            entity
                .authored
                .fill
                .ok_or(AdapterError::SynchronizationMismatch)?,
        )),
        FieldKind::TextContent => serde_json::to_string(&text(entity)?.content),
        FieldKind::TextFont => serde_json::to_string(&text(entity)?.font),
        FieldKind::TextFontHash => serde_json::to_string(&text(entity)?.font_sha256),
        FieldKind::TextSize => serde_json::to_string(&number(text(entity)?.size)),
        FieldKind::TextLineHeight => serde_json::to_string(&number(text(entity)?.line_height)),
    }
    .map_err(|error| AdapterError::Json {
        member: field.member.clone(),
        reason: error.to_string(),
    })?;
    Ok(value.into_bytes())
}

fn mapped_structure_equal(before: &Document, after: &Document) -> bool {
    if before.id != after.id
        || before.roots != after.roots
        || before.entities.keys().ne(after.entities.keys())
        || before.tokens != after.tokens
        || before.relations != after.relations
        || before.extension_declarations != after.extension_declarations
        || before.extensions != after.extensions
    {
        return false;
    }
    before.entities.iter().all(|(id, left)| {
        let right = &after.entities[id];
        left.id == right.id
            && left.kind == right.kind
            && left.children == right.children
            && left.semantics == right.semantics
            && left.extensions == right.extensions
            && left.authored.layout == right.authored.layout
            && left.authored.responsive == right.authored.responsive
            && left.authored.values == right.authored.values
            && left.name.is_some() == right.name.is_some()
            && left.authored.fill.is_some() == right.authored.fill.is_some()
            && left.authored.text.is_some() == right.authored.text.is_some()
    })
}

fn text(entity: &Entity) -> Result<&TextContent, AdapterError> {
    entity
        .authored
        .text
        .as_ref()
        .ok_or(AdapterError::SynchronizationMismatch)
}

fn package_report(
    document: &Document,
    fidelity: Vec<FidelityEntry>,
    correspondences: Vec<PackageCorrespondenceRecord>,
    preserved: bool,
) -> Result<PackageReport, AdapterError> {
    Ok(PackageReport {
        schema_version: 1,
        source_format: SOURCE_FORMAT.to_owned(),
        canonical_hash: Some(canonical_hash(document).map_err(|error| {
            AdapterError::InvalidValue {
                pointer: String::new(),
                reason: error.to_string(),
            }
        })?),
        fidelity,
        correspondences,
        unmapped_member_payloads_preserved: preserved,
    })
}

fn unsupported_fidelity(
    fidelity: &mut Vec<FidelityEntry>,
    target: CorrespondenceTarget,
    pointer: String,
    reason: &str,
) {
    fidelity.push(FidelityEntry {
        target,
        pointer,
        status: Fidelity::Unsupported {
            reason: reason.to_owned(),
        },
    });
}

fn json_value(member: &PackageMember) -> Result<Value, AdapterError> {
    let value: Value =
        serde_json::from_slice(&member.payload).map_err(|error| AdapterError::Json {
            member: member.name.clone(),
            reason: error.to_string(),
        })?;
    json_resources(&value, &member.name)?;
    Ok(value)
}

fn json_resources(value: &Value, member: &str) -> Result<(), AdapterError> {
    let mut values = 0_usize;
    let mut pending = vec![(value, 1_usize)];
    while let Some((value, depth)) = pending.pop() {
        values += 1;
        if values > MAX_JSON_VALUES || depth > MAX_JSON_DEPTH {
            return Err(AdapterError::Json {
                member: member.to_owned(),
                reason: "JSON depth or value-count limit exceeded".to_owned(),
            });
        }
        match value {
            Value::Array(items) => pending.extend(items.iter().map(|item| (item, depth + 1))),
            Value::Object(items) => pending.extend(items.values().map(|item| (item, depth + 1))),
            _ => {}
        }
    }
    Ok(())
}

fn require_integer(
    value: &Value,
    key: &str,
    expected: i64,
    pointer: &str,
) -> Result<(), AdapterError> {
    if value.get(key).and_then(Value::as_i64) == Some(expected) {
        Ok(())
    } else {
        Err(AdapterError::InvalidValue {
            pointer: pointer.to_owned(),
            reason: format!("expected integer {expected}"),
        })
    }
}

fn require_string(
    value: &Value,
    key: &str,
    expected: &str,
    pointer: &str,
) -> Result<(), AdapterError> {
    if value.get(key).and_then(Value::as_str) == Some(expected) {
        Ok(())
    } else {
        Err(AdapterError::InvalidValue {
            pointer: pointer.to_owned(),
            reason: format!("expected {expected}"),
        })
    }
}

fn string_array(value: &Value, key: &str, pointer: &str) -> Result<Vec<String>, AdapterError> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| AdapterError::InvalidValue {
            pointer: pointer.to_owned(),
            reason: "expected an array".to_owned(),
        })?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| AdapterError::InvalidValue {
                    pointer: pointer.to_owned(),
                    reason: "expected string array members".to_owned(),
                })
        })
        .collect()
}

fn raw_string(raw: &RawValue, member: &str) -> Result<String, AdapterError> {
    serde_json::from_str(raw.get()).map_err(|error| AdapterError::Json {
        member: member.to_owned(),
        reason: error.to_string(),
    })
}

fn raw_number(raw: &RawValue, pointer: &str) -> Result<f64, AdapterError> {
    serde_json::from_str(raw.get()).map_err(|error| AdapterError::InvalidValue {
        pointer: pointer.to_owned(),
        reason: error.to_string(),
    })
}

fn raw_string_array(raw: &RawValue, pointer: &str) -> Result<Vec<String>, AdapterError> {
    serde_json::from_str(raw.get()).map_err(|error| AdapterError::InvalidValue {
        pointer: pointer.to_owned(),
        reason: error.to_string(),
    })
}

fn parse_uuid(value: &str, pointer: &str) -> Result<EntityId, AdapterError> {
    let valid = value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        });
    if !valid {
        return Err(AdapterError::InvalidValue {
            pointer: pointer.to_owned(),
            reason: "expected a canonical UUID string".to_owned(),
        });
    }
    EntityId::from_str(&value.replace('-', "")).map_err(|error| AdapterError::InvalidValue {
        pointer: pointer.to_owned(),
        reason: error.to_string(),
    })
}

fn uuid(id: EntityId) -> String {
    let compact = id.to_string();
    format!(
        "{}-{}-{}-{}-{}",
        &compact[0..8],
        &compact[8..12],
        &compact[12..16],
        &compact[16..20],
        &compact[20..32]
    )
}

fn derived_page_id(document: &Document) -> EntityId {
    let mut hasher = Sha256::new();
    hasher.update(b"nuif-penpot-v3-0/page/");
    hasher.update(document.id.to_string());
    let digest = hasher.finalize();
    EntityId::new(u128::from_be_bytes(
        digest[..16].try_into().expect("slice length is fixed"),
    ))
}

fn fixed(value: &SizeIntent) -> f64 {
    let SizeIntent::Fixed(value) = value else {
        unreachable!("profile validation requires fixed dimensions");
    };
    *value
}

fn number(value: f64) -> String {
    if value == 0.0 {
        "0".to_owned()
    } else {
        value.to_string()
    }
}

fn same_number(left: f64, right: f64) -> bool {
    let scale = left.abs().max(right.abs()).max(1.0);
    (left - right).abs() <= f64::EPSILON * scale * 4.0
}

fn color(value: Color) -> String {
    format!(
        "#{:02x}{:02x}{:02x}",
        color_byte(value.red),
        color_byte(value.green),
        color_byte(value.blue)
    )
}

fn parse_color(value: &str) -> Result<Color, AdapterError> {
    if value.len() != 7
        || !value.starts_with('#')
        || !value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(AdapterError::InvalidValue {
            pointer: "/shape/fills/0/fillColor".to_owned(),
            reason: "expected six-digit hexadecimal sRGB color".to_owned(),
        });
    }
    let channel = |range: std::ops::Range<usize>| {
        u8::from_str_radix(&value[range], 16).map_err(|error| AdapterError::InvalidValue {
            pointer: "/shape/fills/0/fillColor".to_owned(),
            reason: error.to_string(),
        })
    };
    Ok(Color {
        space: ColorSpace::Srgb,
        red: f32::from(channel(1..3)?) / 255.0,
        green: f32::from(channel(3..5)?) / 255.0,
        blue: f32::from(channel(5..7)?) / 255.0,
        alpha: 1.0,
    })
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "profile validation restricts channels to exact values in the u8 domain"
)]
fn color_byte(value: f32) -> u8 {
    (value * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exported_profile_round_trips_and_is_deterministic() {
        let fixture = profile_fixture();
        let first = export_document(&fixture).unwrap();
        let second = export_document(&fixture).unwrap();
        assert_eq!(first.bytes, second.bytes);
        assert_eq!(import_package(&first.bytes).unwrap().document, fixture);
        assert!(first.report.is_lossless());
    }

    #[test]
    fn unmodified_sync_returns_original_archive_bytes() {
        let fixture = profile_fixture();
        let exported = export_document(&fixture).unwrap();
        let imported = import_package(&exported.bytes).unwrap();
        let synchronized = synchronize(&imported.retentive, &fixture).unwrap();
        assert_eq!(synchronized.bytes, exported.bytes);
        assert!(synchronized.edits.is_empty());
    }

    #[test]
    fn official_library_fixture_imports_to_the_profile_document() {
        let bytes = include_bytes!("../../../conformance/foreign/penpot/fixture.penpot");
        let imported = import_package(bytes).unwrap();
        assert_eq!(imported.document, profile_fixture());
        assert!(imported.retentive.report.is_lossless());
    }

    #[test]
    fn mapped_sync_preserves_an_unmapped_member_payload() {
        let fixture = profile_fixture();
        let exported = export_document(&fixture).unwrap();
        let mut members = read_archive(&exported.bytes).unwrap();
        members.push(PackageMember {
            name: "objects/org.nuif-probe.bin".to_owned(),
            payload: b"opaque\0member\xff".to_vec(),
            compression: CompressionMethod::Stored,
        });
        let retained_bytes = write_archive(&members).unwrap();
        let imported = import_package(&retained_bytes).unwrap();
        assert!(!imported.retentive.report.is_lossless());
        let mut edited = fixture;
        edited.entities.get_mut(&EntityId::new(0x21)).unwrap().name =
            Some("Edited card".to_owned());
        edited
            .entities
            .get_mut(&EntityId::new(0x21))
            .unwrap()
            .authored
            .width = SizeIntent::Fixed(280.0);
        edited
            .entities
            .get_mut(&EntityId::new(0x22))
            .unwrap()
            .authored
            .fill = Some(Color {
            space: ColorSpace::Srgb,
            red: 1.0,
            green: 128.0 / 255.0,
            blue: 0.0,
            alpha: 1.0,
        });
        edited
            .entities
            .get_mut(&EntityId::new(0x23))
            .unwrap()
            .authored
            .text
            .as_mut()
            .unwrap()
            .content = "Edited Penpot text".to_owned();
        let synchronized = synchronize(&imported.retentive, &edited).unwrap();
        assert!(!synchronized.edits.is_empty());
        let reimported = import_package(&synchronized.bytes).unwrap();
        assert_eq!(reimported.document, edited);
        let opaque = reimported
            .retentive
            .members
            .iter()
            .find(|member| member.name == "objects/org.nuif-probe.bin")
            .unwrap();
        assert_eq!(opaque.payload, b"opaque\0member\xff");
    }

    #[test]
    fn package_byte_limit_is_typed() {
        assert!(matches!(
            import_package(&vec![0; MAX_PACKAGE_BYTES + 1]),
            Err(AdapterError::PackageTooLarge)
        ));
    }

    #[test]
    fn writer_rejects_duplicate_member_names() {
        let cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(cursor);
        let options = SimpleFileOptions::default();
        writer.start_file("manifest.json", options).unwrap();
        writer.write_all(b"{}").unwrap();
        assert!(writer.start_file("manifest.json", options).is_err());
    }

    #[test]
    fn native_writer_deflates_only_larger_json_members() {
        let mut members = Vec::new();
        push_json(
            &mut members,
            "small.json".to_owned(),
            &serde_json::json!({"value": "small"}),
        )
        .unwrap();
        push_json(
            &mut members,
            "large.json".to_owned(),
            &serde_json::json!({"value": "x".repeat(MIN_DEFLATE_BYTES)}),
        )
        .unwrap();
        assert_eq!(members[0].compression, CompressionMethod::Stored);
        assert_eq!(members[1].compression, CompressionMethod::Deflated);
    }
}
