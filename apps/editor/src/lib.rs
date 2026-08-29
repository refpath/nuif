#![doc = "Headless-testable reference-editor session and accessibility action surface."]

use nuif_api::{EngineError, Session, profile_zero_context};
use nuif_codec::{CanonicalText, Encoder, canonical_hash};
use nuif_core::{Document, Entity, EntityId, EntityKind, ExtensionDeclarations, SizeIntent, Token};
use nuif_layout::{EvaluationContext, LayoutSnapshot};
use nuif_protocol::{Anchor, Axis, Operation, Patch, Transaction};
use nuif_render::RenderScene;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

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
    Select { entity: EntityId },
    Rename { entity: EntityId, name: String },
    SetWidth { entity: EntityId, value: f64 },
    SetHeight { entity: EntityId, value: f64 },
    Undo,
    Redo,
    Snapshot { width: u32, height: u32 },
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
    #[error("accessibility node {label:?} for entity {entity} does not exist")]
    AccessibilityNodeMissing { entity: EntityId, label: String },
    #[error("accessibility value {value:?} is invalid for {label:?}")]
    AccessibilityValueInvalid { label: String, value: String },
    #[error("accessibility set-value action is unsupported for {label:?}")]
    AccessibilityActionUnsupported { label: String },
}

#[derive(Clone, Debug)]
pub struct EditorDriver {
    session: Session,
    next_transaction: u128,
    operation_log: Vec<Patch>,
}

impl EditorDriver {
    #[must_use]
    pub fn new(document: Document) -> Self {
        Self {
            session: Session::new(document),
            next_transaction: 1,
            operation_log: Vec::new(),
        }
    }

    #[must_use]
    pub const fn document(&self) -> &Document {
        self.session.document()
    }

    #[must_use]
    pub fn operation_log(&self) -> &[Patch] {
        &self.operation_log
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
            nodes.push(AccessibilityNode {
                role: AccessibilityRole::TreeItem,
                label: entity
                    .name
                    .clone()
                    .unwrap_or_else(|| kind_label(&entity.kind).to_owned()),
                author_id: Some(entity.id),
                value: None,
            });
            nodes.push(AccessibilityNode {
                role: AccessibilityRole::TextField,
                label: "name".to_owned(),
                author_id: Some(entity.id),
                value: entity.name.clone(),
            });
            nodes.push(AccessibilityNode {
                role: AccessibilityRole::SpinButton,
                label: "width".to_owned(),
                author_id: Some(entity.id),
                value: Some(size_label(&entity.authored.width)),
            });
            nodes.push(AccessibilityNode {
                role: AccessibilityRole::SpinButton,
                label: "height".to_owned(),
                author_id: Some(entity.id),
                value: Some(size_label(&entity.authored.height)),
            });
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
            EditorCommand::Undo => {
                let patch = self.session.undo()?;
                self.operation_log.push(patch);
                Ok(EditorEvent::HistoryChanged)
            }
            EditorCommand::Redo => {
                let patch = self.session.redo()?;
                self.operation_log.push(patch);
                Ok(EditorEvent::HistoryChanged)
            }
            EditorCommand::Snapshot { width, height } => {
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
                            png,
                        },
                    }),
                })
            }
        }
    }

    /// Dispatches an author-identity accessibility action through the same
    /// command and semantic-patch path as direct editor automation.
    ///
    /// # Errors
    ///
    /// Returns a typed error when the semantic node is missing, its action is
    /// unsupported, or the supplied value cannot be parsed.
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
                        let number = value.parse::<f64>().map_err(|_| {
                            EditorError::AccessibilityValueInvalid {
                                label: label.clone(),
                                value: value.clone(),
                            }
                        })?;
                        if label == "width" {
                            self.execute(EditorCommand::SetWidth {
                                entity: author_id,
                                value: number,
                            })
                        } else {
                            self.execute(EditorCommand::SetHeight {
                                entity: author_id,
                                value: number,
                            })
                        }
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
        let transaction = self.next_transaction;
        let base_revision = canonical_hash(self.session.document()).map_err(EngineError::from)?;
        let patch = Patch {
            base_revision: Some(base_revision),
            transactions: vec![Transaction {
                id: transaction,
                operations: vec![operation],
            }],
        };
        self.session.apply(&patch)?;
        self.next_transaction += 1;
        self.operation_log.push(patch);
        Ok(EditorEvent::PatchApplied { transaction })
    }
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
    use nuif_codec::Decoder;
    use nuif_protocol::apply_patch;
    use nuif_testing::responsive_card_fixture;

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
}
