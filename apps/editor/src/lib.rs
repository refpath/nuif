#![doc = "Headless-testable reference-editor session and accessibility action surface."]

pub mod gui;

use nuif_api::{EngineError, Session, profile_zero_context};
use nuif_codec::{CanonicalText, Decoder, DeterministicCbor, Encoder};
use nuif_core::{
    Color, Document, Entity, EntityId, EntityKind, ExtensionDeclarations, GridPlacement, GridTrack,
    LayoutStyle, Point, SizeIntent, TextContent, Token,
};
use nuif_layout::{EvaluationContext, LayoutSnapshot};
use nuif_package::{NuifPackage, PackageCapabilityReport, PackageMode};
use nuif_protocol::{Anchor, Axis, Operation, Patch};
use nuif_render::RenderScene;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use thiserror::Error;

pub const MAX_SNAPSHOT_EDGE: u32 = 4_096;
pub const MAX_SNAPSHOT_PIXELS: u64 = 16_777_216;

/// Package capabilities the reference editor can evaluate completely.
///
/// Profile-zero images and fonts are ordinary verified resources rather than
/// required manifest capabilities. Behavior and future capability resources
/// remain structurally inspectable but read-only until the editor ships and
/// tests their complete authoring/evaluation contract.
#[must_use]
pub fn editor_package_capabilities() -> BTreeSet<String> {
    BTreeSet::new()
}

#[derive(Clone, Debug, PartialEq)]
pub struct EditorFile {
    pub document: Document,
    pub package: Option<NuifPackage>,
}

impl EditorFile {
    /// Reports package requirements against the editor's explicitly supported
    /// capability set. Bare documents have no package report.
    #[must_use]
    pub fn package_capability_report(&self) -> Option<PackageCapabilityReport> {
        self.package
            .as_ref()
            .map(|package| package.capability_report(&editor_package_capabilities()))
    }
}

/// Decodes a deterministic package or a historical alpha bare document.
///
/// # Errors
///
/// Returns a package or canonical-codec error string.
pub fn decode_editor_file(bytes: &[u8]) -> Result<EditorFile, String> {
    if bytes.starts_with(b"PK\x03\x04") {
        let package = NuifPackage::decode(bytes).map_err(|error| error.to_string())?;
        return Ok(EditorFile {
            document: package.document.clone(),
            package: Some(package),
        });
    }
    if let Ok(document) = CanonicalText.decode(bytes) {
        return Ok(EditorFile {
            document,
            package: None,
        });
    }
    Ok(EditorFile {
        document: DeterministicCbor
            .decode(bytes)
            .map_err(|error| error.to_string())?,
        package: None,
    })
}

/// Encodes editor state as a deterministic `.nuif` package while preserving
/// resources and authoring-mode policy from an opened package.
///
/// # Errors
///
/// Returns a package validation or encoding error string.
pub fn encode_editor_file(
    document: &Document,
    package: &mut Option<NuifPackage>,
) -> Result<Vec<u8>, String> {
    if let Some(opened) = package.as_ref()
        && opened.document != *document
    {
        let report = opened.capability_report(&editor_package_capabilities());
        if !report.fully_supported {
            return Err(format!(
                "package is read-only because the editor does not support required capabilities: {:?}",
                report.missing_required
            ));
        }
    }
    let mut value = package
        .clone()
        .unwrap_or_else(|| NuifPackage::new(document.clone(), PackageMode::Portable));
    value.document.clone_from(document);
    let bytes = value.encode().map_err(|error| error.to_string())?;
    *package = Some(value);
    Ok(bytes)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessibilityRole {
    Document,
    TreeItem,
    TextField,
    SpinButton,
    Canvas,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccessibilityNode {
    pub role: AccessibilityRole,
    pub label: String,
    pub author_id: Option<EntityId>,
    pub value: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "command")]
pub enum EditorCommand {
    Select {
        entity: EntityId,
    },
    ClearSelection,
    Rename {
        entity: EntityId,
        name: String,
    },
    SetWidth {
        entity: EntityId,
        value: f64,
    },
    SetHeight {
        entity: EntityId,
        value: f64,
    },
    SetSize {
        entity: EntityId,
        axis: Axis,
        value: SizeIntent,
    },
    SetPosition {
        entity: EntityId,
        value: Point,
    },
    SetLayout {
        entity: EntityId,
        value: LayoutStyle,
    },
    SetGridPlacement {
        entity: EntityId,
        value: GridPlacement,
    },
    SetFill {
        entity: EntityId,
        value: Option<Color>,
    },
    SetText {
        entity: EntityId,
        value: Option<TextContent>,
    },
    Remove {
        entity: EntityId,
    },
    Apply {
        operations: Vec<Operation>,
    },
    Undo,
    Redo,
    Snapshot {
        width: u32,
        height: u32,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "action")]
pub enum AccessibilityAction {
    Select {
        author_id: EntityId,
    },
    SetValue {
        author_id: EntityId,
        label: String,
        value: String,
    },
    Insert {
        parent: Option<EntityId>,
        anchor: Anchor,
        entity: Box<Entity>,
    },
    SetToken {
        token: Token,
    },
    SetExtensionDeclarations {
        value: ExtensionDeclarations,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EditorInput {
    Command(EditorCommand),
    Accessibility(AccessibilityAction),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "event")]
pub enum EditorEvent {
    SelectionChanged { entities: Vec<EntityId> },
    PatchApplied { transaction: u128 },
    HistoryChanged,
    Snapshot { snapshot: Box<EditorSnapshot> },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditorSnapshot {
    pub canonical_hash: String,
    pub canonical_document: String,
    pub context: EvaluationContext,
    pub layout: LayoutSnapshot,
    pub scene: RenderScene,
    pub raster: SnapshotRaster,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotRaster {
    pub width: u32,
    pub height: u32,
    pub rgba_sha256: String,
    #[serde(skip)]
    pub rgba: Vec<u8>,
    pub png: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum EditorError {
    #[error(transparent)]
    Engine(#[from] EngineError),
    #[error("entity {0} does not exist")]
    EntityMissing(EntityId),
    #[error("snapshot PNG encoding failed: {0}")]
    Snapshot(String),
    #[error(
        "snapshot dimensions {width}x{height} exceed the non-zero {MAX_SNAPSHOT_EDGE}-edge and {MAX_SNAPSHOT_PIXELS}-pixel limits"
    )]
    SnapshotDimensions { width: u32, height: u32 },
    #[error("accessibility node {label:?} for entity {entity} does not exist")]
    AccessibilityNodeMissing { entity: EntityId, label: String },
    #[error("accessibility value {value:?} is invalid for {label:?}")]
    AccessibilityValueInvalid { label: String, value: String },
    #[error("accessibility set-value action is unsupported for {label:?}")]
    AccessibilityActionUnsupported { label: String },
    #[error(
        "package is read-only because the editor does not support required capabilities: {capabilities:?}"
    )]
    PackageReadOnly { capabilities: BTreeSet<String> },
    #[error("editor document does not match the package document")]
    PackageDocumentMismatch,
}

#[derive(Clone, Debug)]
pub struct EditorDriver {
    session: Session,
    next_transaction: u128,
    operation_log: Vec<Patch>,
    package_capability_report: Option<PackageCapabilityReport>,
}

impl EditorDriver {
    #[must_use]
    pub fn new(document: Document) -> Self {
        Self {
            session: Session::new(document),
            next_transaction: 1,
            operation_log: Vec::new(),
            package_capability_report: None,
        }
    }

    /// Creates an editor session with the exact embedded resources from an
    /// already decoded package. Linked resources remain unresolved.
    ///
    /// # Errors
    ///
    /// Returns an engine error if the package's local resource collection no
    /// longer satisfies the session digest or resource limits.
    pub fn new_with_package(
        document: Document,
        package: Option<&NuifPackage>,
    ) -> Result<Self, EditorError> {
        if package.is_some_and(|package| package.document != document) {
            return Err(EditorError::PackageDocumentMismatch);
        }
        let resources = package.map_or_else(Default::default, NuifPackage::embedded_resources);
        let package_capability_report =
            package.map(|package| package.capability_report(&editor_package_capabilities()));
        Ok(Self {
            session: Session::with_resources(document, resources)?,
            next_transaction: 1,
            operation_log: Vec::new(),
            package_capability_report,
        })
    }

    #[must_use]
    pub const fn document(&self) -> &Document {
        self.session.document()
    }

    #[must_use]
    pub fn operation_log(&self) -> &[Patch] {
        &self.operation_log
    }

    /// Returns the deterministic package negotiation result captured when the
    /// editor session was opened. Bare documents return no report.
    #[must_use]
    pub const fn package_capability_report(&self) -> Option<&PackageCapabilityReport> {
        self.package_capability_report.as_ref()
    }

    /// Whether semantic mutation is disabled because the opened package
    /// declares a capability the reference editor cannot fully evaluate.
    #[must_use]
    pub fn is_read_only(&self) -> bool {
        self.package_capability_report
            .as_ref()
            .is_some_and(|report| !report.fully_supported)
    }

    #[must_use]
    pub fn missing_required_capabilities(&self) -> Option<&BTreeSet<String>> {
        self.package_capability_report
            .as_ref()
            .map(|report| &report.missing_required)
    }

    #[must_use]
    pub fn selection(&self) -> &[EntityId] {
        self.session.selection()
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        self.session.can_undo()
    }

    #[must_use]
    pub fn can_redo(&self) -> bool {
        self.session.can_redo()
    }

    #[must_use]
    pub fn accessibility_tree(&self) -> Vec<AccessibilityNode> {
        let mut nodes = vec![AccessibilityNode {
            role: AccessibilityRole::Document,
            label: "NUIF document".to_owned(),
            author_id: None,
            value: None,
        }];
        for entity in self.session.document().entities.values() {
            nodes.extend(entity_accessibility_nodes(self.session.document(), entity));
        }
        nodes.push(AccessibilityNode {
            role: AccessibilityRole::Canvas,
            label: "document canvas".to_owned(),
            author_id: None,
            value: None,
        });
        nodes
    }

    /// Executes one editor command through the same protocol used by the CLI.
    ///
    /// # Errors
    ///
    /// Returns a typed error if the target is absent or the semantic patch,
    /// history operation, snapshot, or PNG encoding fails.
    pub fn execute(&mut self, command: EditorCommand) -> Result<EditorEvent, EditorError> {
        match command {
            EditorCommand::Select { entity } => {
                if !self.session.document().entities.contains_key(&entity) {
                    return Err(EditorError::EntityMissing(entity));
                }
                self.session.select(vec![entity]);
                Ok(EditorEvent::SelectionChanged {
                    entities: vec![entity],
                })
            }
            EditorCommand::ClearSelection => {
                self.session.select(Vec::new());
                Ok(EditorEvent::SelectionChanged {
                    entities: Vec::new(),
                })
            }
            EditorCommand::Rename { entity, name } => self.apply(Operation::Rename {
                entity,
                name: Some(name),
            }),
            EditorCommand::SetWidth { entity, value } => self.apply(Operation::SetSize {
                entity,
                axis: Axis::Horizontal,
                value: SizeIntent::Fixed(value),
            }),
            EditorCommand::SetHeight { entity, value } => self.apply(Operation::SetSize {
                entity,
                axis: Axis::Vertical,
                value: SizeIntent::Fixed(value),
            }),
            EditorCommand::SetSize {
                entity,
                axis,
                value,
            } => self.apply(Operation::SetSize {
                entity,
                axis,
                value,
            }),
            EditorCommand::SetPosition { entity, value } => {
                self.apply(Operation::SetPosition { entity, value })
            }
            EditorCommand::SetLayout { entity, value } => {
                self.apply(Operation::SetLayout { entity, value })
            }
            EditorCommand::SetGridPlacement { entity, value } => {
                self.apply(Operation::SetGridPlacement { entity, value })
            }
            EditorCommand::SetFill { entity, value } => {
                self.apply(Operation::SetFill { entity, value })
            }
            EditorCommand::SetText { entity, value } => {
                self.apply(Operation::SetText { entity, value })
            }
            EditorCommand::Remove { entity } => {
                let event = self.apply(Operation::Remove { entity })?;
                self.session.select(Vec::new());
                Ok(event)
            }
            EditorCommand::Apply { operations } => self.apply_operations(operations),
            EditorCommand::Undo => {
                self.require_editable()?;
                let patch = self.session.undo()?;
                self.operation_log.push(patch);
                Ok(EditorEvent::HistoryChanged)
            }
            EditorCommand::Redo => {
                self.require_editable()?;
                let patch = self.session.redo()?;
                self.operation_log.push(patch);
                Ok(EditorEvent::HistoryChanged)
            }
            EditorCommand::Snapshot { width, height } => self.snapshot(width, height),
        }
    }

    /// Dispatches an author-identity accessibility action through the same
    /// command and semantic-patch path as direct editor automation.
    ///
    /// # Errors
    ///
    /// Returns a typed error when the semantic node is missing, its action is
    /// unsupported, or the supplied value cannot be parsed.
    #[expect(
        clippy::too_many_lines,
        reason = "semantic accessibility routing stays exhaustive and centralized"
    )]
    pub fn dispatch_accessibility_action(
        &mut self,
        action: AccessibilityAction,
    ) -> Result<EditorEvent, EditorError> {
        match action {
            AccessibilityAction::Select { author_id } => {
                self.execute(EditorCommand::Select { entity: author_id })
            }
            AccessibilityAction::SetValue {
                author_id,
                label,
                value,
            } => {
                let exists = self.accessibility_tree().iter().any(|node| {
                    node.author_id == Some(author_id)
                        && node.label == label
                        && matches!(
                            node.role,
                            AccessibilityRole::TextField | AccessibilityRole::SpinButton
                        )
                });
                if !exists {
                    return Err(EditorError::AccessibilityNodeMissing {
                        entity: author_id,
                        label,
                    });
                }
                match label.as_str() {
                    "name" => self.execute(EditorCommand::Rename {
                        entity: author_id,
                        name: value,
                    }),
                    "width" | "height" => {
                        let intent = parse_size_intent(&value).ok_or_else(|| {
                            EditorError::AccessibilityValueInvalid {
                                label: label.clone(),
                                value: value.clone(),
                            }
                        })?;
                        self.execute(EditorCommand::SetSize {
                            entity: author_id,
                            axis: if label == "width" {
                                Axis::Horizontal
                            } else {
                                Axis::Vertical
                            },
                            value: intent,
                        })
                    }
                    "x" | "y" => {
                        let number = parse_finite_number(&label, &value)?;
                        let mut position = self
                            .session
                            .document()
                            .entities
                            .get(&author_id)
                            .ok_or(EditorError::EntityMissing(author_id))?
                            .authored
                            .position;
                        if label == "x" {
                            position.x = number;
                        } else {
                            position.y = number;
                        }
                        self.execute(EditorCommand::SetPosition {
                            entity: author_id,
                            value: position,
                        })
                    }
                    "gap" | "padding_top" | "padding_right" | "padding_bottom" | "padding_left" => {
                        let number = parse_finite_number(&label, &value)?;
                        let mut layout = self
                            .session
                            .document()
                            .entities
                            .get(&author_id)
                            .ok_or(EditorError::EntityMissing(author_id))?
                            .authored
                            .layout
                            .clone();
                        match label.as_str() {
                            "gap" => layout.gap = number,
                            "padding_top" => layout.padding.top = number,
                            "padding_right" => layout.padding.right = number,
                            "padding_bottom" => layout.padding.bottom = number,
                            "padding_left" => layout.padding.left = number,
                            _ => unreachable!(),
                        }
                        self.execute(EditorCommand::SetLayout {
                            entity: author_id,
                            value: layout,
                        })
                    }
                    "grid_columns" | "grid_rows" => {
                        let tracks = parse_grid_tracks(&value).map_err(|_| {
                            EditorError::AccessibilityValueInvalid {
                                label: label.clone(),
                                value: value.clone(),
                            }
                        })?;
                        let mut layout = self
                            .session
                            .document()
                            .entities
                            .get(&author_id)
                            .ok_or(EditorError::EntityMissing(author_id))?
                            .authored
                            .layout
                            .clone();
                        if label == "grid_columns" {
                            layout.grid.columns = tracks;
                        } else {
                            layout.grid.rows = tracks;
                        }
                        self.execute(EditorCommand::SetLayout {
                            entity: author_id,
                            value: layout,
                        })
                    }
                    "grid_position" | "grid_column_span" | "grid_row_span" => {
                        let mut placement = self
                            .session
                            .document()
                            .entities
                            .get(&author_id)
                            .ok_or(EditorError::EntityMissing(author_id))?
                            .authored
                            .grid_placement;
                        let invalid = |_| EditorError::AccessibilityValueInvalid {
                            label: label.clone(),
                            value: value.clone(),
                        };
                        match label.as_str() {
                            "grid_position" => {
                                let (column, row) = parse_grid_position(&value).map_err(invalid)?;
                                placement.column = column;
                                placement.row = row;
                            }
                            "grid_column_span" => {
                                placement.column_span = parse_grid_span(&value).map_err(invalid)?;
                            }
                            "grid_row_span" => {
                                placement.row_span = parse_grid_span(&value).map_err(invalid)?;
                            }
                            _ => unreachable!(),
                        }
                        self.execute(EditorCommand::SetGridPlacement {
                            entity: author_id,
                            value: placement,
                        })
                    }
                    "fill" => self.execute(EditorCommand::SetFill {
                        entity: author_id,
                        value: parse_fill(&value).map_err(|()| {
                            EditorError::AccessibilityValueInvalid {
                                label: label.clone(),
                                value: value.clone(),
                            }
                        })?,
                    }),
                    "text" | "font_size" | "line_height" => {
                        let mut text = self
                            .session
                            .document()
                            .entities
                            .get(&author_id)
                            .ok_or(EditorError::EntityMissing(author_id))?
                            .authored
                            .text
                            .clone()
                            .ok_or_else(|| EditorError::AccessibilityActionUnsupported {
                                label: label.clone(),
                            })?;
                        match label.as_str() {
                            "text" => text.content = value,
                            "font_size" => text.size = parse_finite_number(&label, &value)?,
                            "line_height" => {
                                text.line_height = parse_finite_number(&label, &value)?;
                            }
                            _ => unreachable!(),
                        }
                        self.execute(EditorCommand::SetText {
                            entity: author_id,
                            value: Some(text),
                        })
                    }
                    _ => Err(EditorError::AccessibilityActionUnsupported { label }),
                }
            }
            AccessibilityAction::Insert {
                parent,
                anchor,
                entity,
            } => self.apply(Operation::Insert {
                parent,
                anchor,
                entity,
            }),
            AccessibilityAction::SetToken { token } => self.apply(Operation::SetToken { token }),
            AccessibilityAction::SetExtensionDeclarations { value } => {
                self.apply(Operation::SetExtensionDeclarations { value })
            }
        }
    }

    /// Executes either a direct automation command or an accessibility action.
    ///
    /// # Errors
    ///
    /// Returns the corresponding command or accessibility dispatch error.
    pub fn dispatch(&mut self, input: EditorInput) -> Result<EditorEvent, EditorError> {
        match input {
            EditorInput::Command(command) => self.execute(command),
            EditorInput::Accessibility(action) => self.dispatch_accessibility_action(action),
        }
    }

    fn apply(&mut self, operation: Operation) -> Result<EditorEvent, EditorError> {
        self.apply_operations(vec![operation])
    }

    fn apply_operations(&mut self, operations: Vec<Operation>) -> Result<EditorEvent, EditorError> {
        self.require_editable()?;
        let transaction = self.next_transaction;
        let patch = self.session.apply_transaction(transaction, operations)?;
        self.next_transaction += 1;
        self.operation_log.push(patch);
        Ok(EditorEvent::PatchApplied { transaction })
    }

    fn require_editable(&self) -> Result<(), EditorError> {
        if self.is_read_only() {
            Err(EditorError::PackageReadOnly {
                capabilities: self
                    .missing_required_capabilities()
                    .cloned()
                    .unwrap_or_default(),
            })
        } else {
            Ok(())
        }
    }

    fn snapshot(&self, width: u32, height: u32) -> Result<EditorEvent, EditorError> {
        let pixels = u64::from(width) * u64::from(height);
        if width == 0
            || height == 0
            || width > MAX_SNAPSHOT_EDGE
            || height > MAX_SNAPSHOT_EDGE
            || pixels > MAX_SNAPSHOT_PIXELS
        {
            return Err(EditorError::SnapshotDimensions { width, height });
        }
        let context = profile_zero_context(f64::from(width), f64::from(height));
        let snapshot = self.session.snapshot(&context)?;
        let canonical_document = String::from_utf8(
            CanonicalText
                .encode(self.session.document())
                .map_err(EngineError::from)?,
        )
        .map_err(|error| EditorError::Snapshot(error.to_string()))?;
        let png = snapshot
            .raster
            .to_png()
            .map_err(|error| EditorError::Snapshot(error.to_string()))?;
        Ok(EditorEvent::Snapshot {
            snapshot: Box::new(EditorSnapshot {
                canonical_hash: snapshot.canonical_hash,
                canonical_document,
                context,
                layout: snapshot.layout,
                scene: snapshot.scene,
                raster: SnapshotRaster {
                    width: snapshot.raster.width,
                    height: snapshot.raster.height,
                    rgba_sha256: format!("{:x}", Sha256::digest(&snapshot.raster.rgba)),
                    rgba: snapshot.raster.rgba,
                    png,
                },
            }),
        })
    }
}

fn accessibility_node(
    entity: &Entity,
    role: AccessibilityRole,
    label: &str,
    value: Option<String>,
) -> AccessibilityNode {
    AccessibilityNode {
        role,
        label: label.to_owned(),
        author_id: Some(entity.id),
        value,
    }
}

fn entity_accessibility_nodes(document: &Document, entity: &Entity) -> Vec<AccessibilityNode> {
    let mut nodes = vec![
        accessibility_node(
            entity,
            AccessibilityRole::TreeItem,
            &entity
                .name
                .clone()
                .unwrap_or_else(|| kind_label(&entity.kind).to_owned()),
            None,
        ),
        accessibility_node(
            entity,
            AccessibilityRole::TextField,
            "name",
            entity.name.clone(),
        ),
        accessibility_node(
            entity,
            AccessibilityRole::SpinButton,
            "width",
            Some(size_label(&entity.authored.width)),
        ),
        accessibility_node(
            entity,
            AccessibilityRole::SpinButton,
            "height",
            Some(size_label(&entity.authored.height)),
        ),
    ];
    for (label, value) in [
        ("x", entity.authored.position.x.to_string()),
        ("y", entity.authored.position.y.to_string()),
        ("gap", entity.authored.layout.gap.to_string()),
        (
            "padding_top",
            entity.authored.layout.padding.top.to_string(),
        ),
        (
            "padding_right",
            entity.authored.layout.padding.right.to_string(),
        ),
        (
            "padding_bottom",
            entity.authored.layout.padding.bottom.to_string(),
        ),
        (
            "padding_left",
            entity.authored.layout.padding.left.to_string(),
        ),
    ] {
        nodes.push(accessibility_node(
            entity,
            AccessibilityRole::SpinButton,
            label,
            Some(value),
        ));
    }
    nodes.extend(grid_accessibility_nodes(document, entity));
    nodes.push(accessibility_node(
        entity,
        AccessibilityRole::TextField,
        "fill",
        Some(fill_label(entity.authored.fill)),
    ));
    nodes.extend(text_accessibility_nodes(entity));
    nodes
}

fn grid_accessibility_nodes(document: &Document, entity: &Entity) -> Vec<AccessibilityNode> {
    let mut nodes = Vec::new();
    if entity.authored.layout.family == nuif_core::LayoutFamily::Grid {
        for (label, value) in [
            (
                "grid_columns",
                grid_tracks_label(&entity.authored.layout.grid.columns),
            ),
            (
                "grid_rows",
                grid_tracks_label(&entity.authored.layout.grid.rows),
            ),
        ] {
            nodes.push(accessibility_node(
                entity,
                AccessibilityRole::TextField,
                label,
                Some(value),
            ));
        }
    }
    let parent_is_grid = document.parent_of(entity.id).is_some_and(|parent| {
        document.entities[&parent].authored.layout.family == nuif_core::LayoutFamily::Grid
    });
    if parent_is_grid {
        for (role, label, value) in [
            (
                AccessibilityRole::TextField,
                "grid_position",
                grid_position_label(entity.authored.grid_placement),
            ),
            (
                AccessibilityRole::SpinButton,
                "grid_column_span",
                entity.authored.grid_placement.column_span.to_string(),
            ),
            (
                AccessibilityRole::SpinButton,
                "grid_row_span",
                entity.authored.grid_placement.row_span.to_string(),
            ),
        ] {
            nodes.push(accessibility_node(entity, role, label, Some(value)));
        }
    }
    nodes
}

fn text_accessibility_nodes(entity: &Entity) -> Vec<AccessibilityNode> {
    let Some(text) = &entity.authored.text else {
        return Vec::new();
    };
    [
        (AccessibilityRole::TextField, "text", text.content.clone()),
        (
            AccessibilityRole::SpinButton,
            "font_size",
            text.size.to_string(),
        ),
        (
            AccessibilityRole::SpinButton,
            "line_height",
            text.line_height.to_string(),
        ),
    ]
    .into_iter()
    .map(|(role, label, value)| accessibility_node(entity, role, label, Some(value)))
    .collect()
}

pub(crate) fn grid_tracks_label(tracks: &[GridTrack]) -> String {
    tracks
        .iter()
        .map(|track| match track {
            GridTrack::Fixed(value) => format!("{value}px"),
            GridTrack::Fraction(value) => format!("{value}fr"),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn parse_grid_tracks(value: &str) -> Result<Vec<GridTrack>, String> {
    let parts = value
        .split(|character: char| character.is_whitespace() || character == ',')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return Err("grid track list must not be empty".to_owned());
    }
    if parts.len() > nuif_core::PROFILE0_MAX_GRID_AXIS_TRACKS {
        return Err(format!(
            "grid track list exceeds {} tracks",
            nuif_core::PROFILE0_MAX_GRID_AXIS_TRACKS
        ));
    }
    parts
        .into_iter()
        .map(|part| {
            let normalized = part.to_ascii_lowercase();
            let (number, fraction) = normalized.strip_suffix("fr").map_or_else(
                || (normalized.strip_suffix("px").unwrap_or(&normalized), false),
                |number| (number, true),
            );
            let value = number
                .parse::<f64>()
                .map_err(|_| format!("invalid grid track {part:?}"))?;
            if !value.is_finite() || value <= 0.0 {
                return Err(format!("grid track {part:?} must be positive and finite"));
            }
            Ok(if fraction {
                GridTrack::Fraction(value)
            } else {
                GridTrack::Fixed(value)
            })
        })
        .collect()
}

pub(crate) fn grid_position_label(value: GridPlacement) -> String {
    match (value.column, value.row) {
        (Some(column), Some(row)) => format!("{} {}", column + 1, row + 1),
        _ => "auto".to_owned(),
    }
}

pub(crate) fn parse_grid_position(value: &str) -> Result<(Option<u32>, Option<u32>), String> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("auto") {
        return Ok((None, None));
    }
    let indices = value
        .split(|character: char| character.is_whitespace() || matches!(character, ',' | '/'))
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if indices.len() != 2 {
        return Err("grid position must be auto or two 1-based integers".to_owned());
    }
    Ok((parse_grid_index(indices[0])?, parse_grid_index(indices[1])?))
}

fn parse_grid_index(value: &str) -> Result<Option<u32>, String> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("auto") {
        return Ok(None);
    }
    let index = value
        .parse::<u32>()
        .map_err(|_| "grid index must be auto or a positive integer".to_owned())?;
    if index == 0
        || usize::try_from(index).unwrap_or(usize::MAX) > nuif_core::PROFILE0_MAX_GRID_AXIS_TRACKS
    {
        return Err(format!(
            "grid index must be between 1 and {}",
            nuif_core::PROFILE0_MAX_GRID_AXIS_TRACKS
        ));
    }
    Ok(Some(index - 1))
}

pub(crate) fn parse_grid_span(value: &str) -> Result<u32, String> {
    let span = value
        .trim()
        .parse::<u32>()
        .map_err(|_| "grid span must be a positive integer".to_owned())?;
    if span == 0
        || usize::try_from(span).unwrap_or(usize::MAX) > nuif_core::PROFILE0_MAX_GRID_AXIS_TRACKS
    {
        return Err(format!(
            "grid span must be between 1 and {}",
            nuif_core::PROFILE0_MAX_GRID_AXIS_TRACKS
        ));
    }
    Ok(span)
}

fn parse_size_intent(value: &str) -> Option<SizeIntent> {
    let value = value.trim();
    match value {
        "auto" => Some(SizeIntent::Auto),
        "fill" => Some(SizeIntent::Fill),
        "intrinsic" => Some(SizeIntent::Intrinsic),
        "min-content" => Some(SizeIntent::MinContent),
        "max-content" => Some(SizeIntent::MaxContent),
        _ if value.ends_with('%') => value
            .strip_suffix('%')?
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .map(SizeIntent::Percentage),
        _ if value.starts_with("fit-content(") && value.ends_with(')') => value
            .strip_prefix("fit-content(")?
            .strip_suffix(')')?
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .map(SizeIntent::FitContent),
        _ => value
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .map(SizeIntent::Fixed),
    }
}

fn parse_finite_number(label: &str, value: &str) -> Result<f64, EditorError> {
    value
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
        .ok_or_else(|| EditorError::AccessibilityValueInvalid {
            label: label.to_owned(),
            value: value.to_owned(),
        })
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "channels are clamped to the exact u8 range before conversion"
)]
fn fill_label(fill: Option<Color>) -> String {
    fill.map_or_else(
        || "none".to_owned(),
        |fill| {
            format!(
                "#{:02X}{:02X}{:02X}{:02X}",
                (fill.red.clamp(0.0, 1.0) * 255.0).round() as u8,
                (fill.green.clamp(0.0, 1.0) * 255.0).round() as u8,
                (fill.blue.clamp(0.0, 1.0) * 255.0).round() as u8,
                (fill.alpha.clamp(0.0, 1.0) * 255.0).round() as u8,
            )
        },
    )
}

fn parse_fill(value: &str) -> Result<Option<Color>, ()> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("none") {
        return Ok(None);
    }
    let hex = value.strip_prefix('#').unwrap_or(value);
    if !matches!(hex.len(), 6 | 8) || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(());
    }
    let channel = |start| {
        u8::from_str_radix(&hex[start..start + 2], 16)
            .ok()
            .map(f32::from)
            .map(|value| value / 255.0)
    };
    Ok(Some(Color {
        space: nuif_core::ColorSpace::Srgb,
        red: channel(0).ok_or(())?,
        green: channel(2).ok_or(())?,
        blue: channel(4).ok_or(())?,
        alpha: if hex.len() == 8 {
            channel(6).ok_or(())?
        } else {
            1.0
        },
    }))
}

fn kind_label(kind: &EntityKind) -> &'static str {
    match kind {
        EntityKind::Surface => "surface",
        EntityKind::Container => "container",
        EntityKind::Shape(_) => "shape",
        EntityKind::Text => "text",
        EntityKind::Image => "image",
        EntityKind::Component => "component",
        EntityKind::Instance { .. } => "instance",
        EntityKind::Unknown(_) => "unknown entity",
    }
}

fn size_label(intent: &SizeIntent) -> String {
    match intent {
        SizeIntent::Fixed(value) => value.to_string(),
        SizeIntent::Auto => "auto".to_owned(),
        SizeIntent::Fill => "fill".to_owned(),
        SizeIntent::Intrinsic => "intrinsic".to_owned(),
        SizeIntent::Percentage(value) => format!("{value}%"),
        SizeIntent::MinContent => "min-content".to_owned(),
        SizeIntent::MaxContent => "max-content".to_owned(),
        SizeIntent::FitContent(value) => format!("fit-content({value})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nuif_codec::{Decoder, canonical_hash};
    use nuif_core::ResourceRole;
    use nuif_protocol::apply_patch;
    use nuif_testing::{responsive_card_fixture, rgba8_image_package_fixture};

    fn insert_fixture_entity(
        driver: &mut EditorDriver,
        expected: &Document,
        entity: EntityId,
        parent: Option<EntityId>,
        anchor: Anchor,
    ) {
        let mut value = expected.entities[&entity].clone();
        value.children.clear();
        driver
            .dispatch_accessibility_action(AccessibilityAction::Insert {
                parent,
                anchor,
                entity: Box::new(value),
            })
            .unwrap();
    }

    #[test]
    fn accessibility_actions_and_replay_reach_the_same_hash() {
        let base = responsive_card_fixture();
        let mut driver = EditorDriver::new(base.clone());
        let card = EntityId::new(0x20);
        driver
            .dispatch_accessibility_action(AccessibilityAction::SetValue {
                author_id: card,
                label: "width".to_owned(),
                value: "640".to_owned(),
            })
            .unwrap();
        driver
            .dispatch_accessibility_action(AccessibilityAction::SetValue {
                author_id: card,
                label: "name".to_owned(),
                value: "Edited card".to_owned(),
            })
            .unwrap();
        let mut replayed = base;
        for patch in driver.operation_log() {
            apply_patch(&mut replayed, patch).unwrap();
        }
        assert_eq!(
            canonical_hash(driver.document()).unwrap(),
            canonical_hash(&replayed).unwrap()
        );
        assert!(driver.accessibility_tree().iter().any(|node| {
            node.author_id == Some(card)
                && node.role == AccessibilityRole::SpinButton
                && node.label == "width"
                && node.value.as_deref() == Some("640")
        }));
    }

    #[test]
    fn grid_controls_share_accessibility_protocol_and_replay() {
        let mut document = Document::empty(EntityId::new(1));
        let root = EntityId::new(2);
        let child = EntityId::new(3);
        let mut grid = Entity::new(root, EntityKind::Container);
        grid.authored.width = SizeIntent::Fill;
        grid.authored.height = SizeIntent::Fill;
        grid.authored.layout.family = nuif_core::LayoutFamily::Grid;
        grid.authored.layout.grid.columns = vec![GridTrack::Fraction(1.0); 2];
        grid.authored.layout.grid.rows = vec![GridTrack::Fraction(1.0)];
        grid.children.push(child);
        document.roots.push(root);
        document.entities.insert(root, grid);
        document
            .entities
            .insert(child, Entity::new(child, EntityKind::Container));
        let base = document.clone();
        let mut driver = EditorDriver::new(document);
        let tree = driver.accessibility_tree();
        assert!(tree.iter().any(|node| {
            node.author_id == Some(root)
                && node.label == "grid_columns"
                && node.value.as_deref() == Some("1fr 1fr")
        }));
        assert!(tree.iter().any(|node| {
            node.author_id == Some(child)
                && node.label == "grid_position"
                && node.value.as_deref() == Some("auto")
        }));

        driver
            .dispatch_accessibility_action(AccessibilityAction::SetValue {
                author_id: root,
                label: "grid_columns".to_owned(),
                value: "80px 2fr".to_owned(),
            })
            .unwrap();
        driver
            .dispatch_accessibility_action(AccessibilityAction::SetValue {
                author_id: child,
                label: "grid_position".to_owned(),
                value: "2 1".to_owned(),
            })
            .unwrap();
        assert_eq!(
            driver.document().entities[&root]
                .authored
                .layout
                .grid
                .columns,
            vec![GridTrack::Fixed(80.0), GridTrack::Fraction(2.0)]
        );
        assert_eq!(
            driver.document().entities[&child]
                .authored
                .grid_placement
                .column,
            Some(1)
        );
        let mut replayed = base;
        for patch in driver.operation_log() {
            apply_patch(&mut replayed, patch).unwrap();
        }
        assert_eq!(replayed, *driver.document());
    }

    #[test]
    fn grid_control_parsers_reject_ambiguous_or_unbounded_values() {
        assert_eq!(
            parse_grid_tracks("120px, 2fr").unwrap(),
            vec![GridTrack::Fixed(120.0), GridTrack::Fraction(2.0)]
        );
        assert_eq!(parse_grid_position("2 / 3").unwrap(), (Some(1), Some(2)));
        assert!(parse_grid_tracks("").is_err());
        assert!(parse_grid_tracks("0fr").is_err());
        assert!(parse_grid_tracks("NaNpx").is_err());
        assert!(parse_grid_position("1").is_err());
        assert!(parse_grid_position("0 1").is_err());
        assert!(parse_grid_span("0").is_err());
    }

    #[test]
    fn semantic_actions_author_the_complete_v0_fixture_from_empty() {
        let expected = responsive_card_fixture();
        let mut driver = EditorDriver::new(Document::empty(expected.id));
        driver
            .dispatch_accessibility_action(AccessibilityAction::SetExtensionDeclarations {
                value: expected.extension_declarations.clone(),
            })
            .unwrap();
        for token in expected.tokens.values() {
            driver
                .dispatch_accessibility_action(AccessibilityAction::SetToken {
                    token: token.clone(),
                })
                .unwrap();
        }

        let surface = EntityId::new(0x10);
        let card = EntityId::new(0x20);
        let media = EntityId::new(0x21);
        let copy = EntityId::new(0x22);
        let button = EntityId::new(0x23);
        let instance = EntityId::new(0x24);
        let opaque = EntityId::new(0x25);
        let icon = EntityId::new(0x26);
        insert_fixture_entity(&mut driver, &expected, surface, None, Anchor::Start);
        insert_fixture_entity(&mut driver, &expected, card, Some(surface), Anchor::Start);
        insert_fixture_entity(
            &mut driver,
            &expected,
            button,
            Some(surface),
            Anchor::After(card),
        );
        insert_fixture_entity(&mut driver, &expected, media, Some(card), Anchor::Start);
        insert_fixture_entity(
            &mut driver,
            &expected,
            copy,
            Some(card),
            Anchor::After(media),
        );
        insert_fixture_entity(
            &mut driver,
            &expected,
            instance,
            Some(card),
            Anchor::After(copy),
        );
        insert_fixture_entity(
            &mut driver,
            &expected,
            opaque,
            Some(card),
            Anchor::After(instance),
        );
        insert_fixture_entity(&mut driver, &expected, icon, Some(button), Anchor::Start);

        assert_eq!(driver.document(), &expected);
        let nodes = driver.accessibility_tree();
        for entity in expected.entities.keys() {
            assert!(nodes.iter().any(|node| {
                node.role == AccessibilityRole::TreeItem && node.author_id == Some(*entity)
            }));
        }

        let mut replayed = Document::empty(expected.id);
        for patch in driver.operation_log() {
            apply_patch(&mut replayed, patch).unwrap();
        }
        assert_eq!(replayed, expected);

        let event = driver
            .execute(EditorCommand::Snapshot {
                width: 768,
                height: 640,
            })
            .unwrap();
        let EditorEvent::Snapshot { snapshot } = event else {
            panic!("snapshot command returned the wrong event");
        };
        assert_eq!(snapshot.canonical_hash, canonical_hash(&expected).unwrap());
        assert_eq!(
            CanonicalText
                .decode(snapshot.canonical_document.as_bytes())
                .unwrap(),
            expected
        );
        assert!((snapshot.context.viewport.width - 768.0).abs() < f64::EPSILON);
        assert_eq!(snapshot.layout.boxes.len(), expected.entities.len());
        assert!(!snapshot.scene.commands.is_empty());
        assert_eq!((snapshot.raster.width, snapshot.raster.height), (768, 640));
        assert_eq!(snapshot.raster.rgba_sha256.len(), 64);
        assert!(snapshot.raster.png.starts_with(b"\x89PNG\r\n\x1a\n"));
    }

    #[test]
    fn hostile_accessibility_values_fail_without_mutation() {
        let base = responsive_card_fixture();
        let mut driver = EditorDriver::new(base.clone());
        let card = EntityId::new(0x20);
        let copy = EntityId::new(0x22);
        for (entity, label, value) in [
            (card, "width", "NaN"),
            (card, "width", "NaN%"),
            (card, "height", "fit-content(inf)"),
            (card, "x", "-inf"),
            (card, "gap", "NaN"),
            (card, "padding_left", "1e999"),
            (card, "fill", "#12345g"),
            (copy, "font_size", "NaN"),
            (copy, "line_height", "inf"),
        ] {
            let error = driver
                .dispatch_accessibility_action(AccessibilityAction::SetValue {
                    author_id: entity,
                    label: label.to_owned(),
                    value: value.to_owned(),
                })
                .unwrap_err();
            assert!(matches!(
                error,
                EditorError::AccessibilityValueInvalid { .. }
            ));
            assert_eq!(driver.document(), &base);
            assert!(driver.operation_log().is_empty());
            assert!(!driver.can_undo());
        }
    }

    #[test]
    fn snapshot_budget_rejects_before_rendering() {
        let base = responsive_card_fixture();
        let mut driver = EditorDriver::new(base.clone());
        for (width, height) in [
            (0, 100),
            (100, 0),
            (MAX_SNAPSHOT_EDGE + 1, 1),
            (1, MAX_SNAPSHOT_EDGE + 1),
            (u32::MAX, u32::MAX),
        ] {
            assert!(matches!(
                driver.execute(EditorCommand::Snapshot { width, height }),
                Err(EditorError::SnapshotDimensions { .. })
            ));
            assert_eq!(driver.document(), &base);
            assert!(driver.operation_log().is_empty());
        }
    }

    #[test]
    fn failed_transactions_and_history_boundaries_are_atomic_and_replayable() {
        let base = responsive_card_fixture();
        let mut driver = EditorDriver::new(base.clone());
        let card = EntityId::new(0x20);
        let missing = EntityId::new(u128::MAX);
        assert!(
            driver
                .execute(EditorCommand::Apply {
                    operations: vec![
                        Operation::Rename {
                            entity: card,
                            name: Some("must not commit".to_owned()),
                        },
                        Operation::SetPosition {
                            entity: missing,
                            value: Point { x: 1.0, y: 2.0 },
                        },
                    ],
                })
                .is_err()
        );
        assert_eq!(driver.document(), &base);
        assert!(driver.operation_log().is_empty());
        assert!(!driver.can_undo());

        driver
            .execute(EditorCommand::Rename {
                entity: card,
                name: "first".to_owned(),
            })
            .unwrap();
        driver.execute(EditorCommand::Undo).unwrap();
        assert!(driver.can_redo());
        driver.execute(EditorCommand::Redo).unwrap();
        driver.execute(EditorCommand::Undo).unwrap();
        driver
            .execute(EditorCommand::Rename {
                entity: card,
                name: "replacement".to_owned(),
            })
            .unwrap();
        assert!(!driver.can_redo());

        let mut replayed = base;
        for patch in driver.operation_log() {
            apply_patch(&mut replayed, patch).unwrap();
        }
        assert_eq!(&replayed, driver.document());
    }

    #[test]
    fn editor_file_round_trip_preserves_package_resources() {
        let document = responsive_card_fixture();
        let mut original = NuifPackage::new(document.clone(), PackageMode::Portable);
        let digest = original
            .add_embedded(
                b"inert source evidence".to_vec(),
                "text/plain",
                ResourceRole::Source,
                None,
            )
            .unwrap();
        let bytes = original.encode().unwrap();
        let mut opened = decode_editor_file(&bytes).unwrap();

        let root = opened.document.roots[0];
        opened.document.entities.get_mut(&root).unwrap().name =
            Some("Edited without dropping resources".to_owned());
        let saved = encode_editor_file(&opened.document, &mut opened.package).unwrap();
        let decoded = NuifPackage::decode(&saved).unwrap();

        assert_eq!(decoded.document, opened.document);
        assert_eq!(
            decoded.embedded(&digest),
            Some(b"inert source evidence".as_slice())
        );
        assert_eq!(decoded.mode, PackageMode::Portable);
    }

    #[test]
    fn unsupported_package_capabilities_are_inspectable_but_read_only() {
        let document = responsive_card_fixture();
        let mut original = NuifPackage::new(document.clone(), PackageMode::Portable);
        original
            .required_capabilities
            .insert("feature.example".to_owned());
        let bytes = original.encode().unwrap();
        let opened = decode_editor_file(&bytes).unwrap();
        let report = opened.package_capability_report().unwrap();
        assert!(!report.fully_supported);
        assert_eq!(
            report.missing_required,
            BTreeSet::from(["feature.example".to_owned()])
        );

        let mut driver =
            EditorDriver::new_with_package(opened.document.clone(), opened.package.as_ref())
                .unwrap();
        assert!(driver.is_read_only());
        driver
            .execute(EditorCommand::Select {
                entity: EntityId::new(0x20),
            })
            .unwrap();
        assert!(matches!(
            driver.execute(EditorCommand::Rename {
                entity: EntityId::new(0x20),
                name: "must not commit".to_owned(),
            }),
            Err(EditorError::PackageReadOnly { capabilities })
                if capabilities == BTreeSet::from(["feature.example".to_owned()])
        ));
        assert_eq!(driver.document(), &document);
        assert!(driver.operation_log().is_empty());

        let mut no_op_package = opened.package.clone();
        assert_eq!(
            encode_editor_file(&opened.document, &mut no_op_package).unwrap(),
            bytes
        );
        let package_before = opened.package.clone();
        let mut changed = opened.document;
        changed.entities.get_mut(&EntityId::new(0x20)).unwrap().name =
            Some("must not save".to_owned());
        let mut rejected_package = opened.package;
        assert!(encode_editor_file(&changed, &mut rejected_package).is_err());
        assert_eq!(rejected_package, package_before);

        assert!(matches!(
            EditorDriver::new_with_package(
                Document::empty(EntityId::new(0xff)),
                package_before.as_ref()
            ),
            Err(EditorError::PackageDocumentMismatch)
        ));
    }

    #[test]
    fn editor_snapshot_resolves_images_from_an_open_package() {
        let package = rgba8_image_package_fixture();
        let opened = decode_editor_file(&package.encode().unwrap()).unwrap();
        let mut driver =
            EditorDriver::new_with_package(opened.document, opened.package.as_ref()).unwrap();
        let event = driver
            .execute(EditorCommand::Snapshot {
                width: 2,
                height: 2,
            })
            .unwrap();
        let EditorEvent::Snapshot { snapshot } = event else {
            panic!("snapshot command must return a snapshot");
        };

        assert!(matches!(
            snapshot.scene.commands.as_slice(),
            [nuif_render::DrawCommand::Image { .. }]
        ));
        assert_eq!(
            snapshot.raster.rgba,
            [
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 127, 255,
            ]
        );
        assert_eq!(snapshot.scene.fidelity.len(), 1);
        assert_eq!(
            snapshot.scene.fidelity[0].status,
            nuif_core::Fidelity::Lossless
        );
    }

    #[test]
    fn editor_file_accepts_historical_bare_documents() {
        let document = responsive_card_fixture();
        let bytes = CanonicalText.encode(&document).unwrap();
        let opened = decode_editor_file(&bytes).unwrap();
        assert_eq!(opened.document, document);
        assert!(opened.package.is_none());
    }
}
