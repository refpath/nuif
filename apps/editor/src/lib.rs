#![doc = "Headless-testable reference-editor session and accessibility action surface."]

use nuif_api::{EngineError, Session, profile_zero_context};
use nuif_codec::canonical_hash;
use nuif_core::{Document, EntityId, EntityKind, SizeIntent};
use nuif_protocol::{Axis, Operation, Patch, Transaction};
use serde::{Deserialize, Serialize};
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
    SelectionChanged {
        entities: Vec<EntityId>,
    },
    PatchApplied {
        transaction: u128,
    },
    HistoryChanged,
    Snapshot {
        canonical_hash: String,
        layout_boxes: usize,
        render_commands: usize,
        png: Vec<u8>,
    },
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
                let snapshot = self
                    .session
                    .snapshot(&profile_zero_context(f64::from(width), f64::from(height)))?;
                Ok(EditorEvent::Snapshot {
                    canonical_hash: snapshot.canonical_hash,
                    layout_boxes: snapshot.layout.boxes.len(),
                    render_commands: snapshot.scene.commands.len(),
                    png: snapshot
                        .raster
                        .to_png()
                        .map_err(|error| EditorError::Snapshot(error.to_string()))?,
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
    use nuif_protocol::apply_patch;
    use nuif_testing::responsive_card_fixture;

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
}
