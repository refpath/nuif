#![doc = "Semantic operations, atomic patches, replay and inversion for NUIF."]

use nuif_codec::canonical_hash;
use nuif_core::{
    Asset, AssetId, Color, Document, Entity, EntityId, ExtensionDeclarations, Extensions,
    GridPlacement, ImagePaint, LayoutStyle, OpaquePayload, Point, PropertyValue, Relation,
    ResourceDigest, Severity, SizeIntent, TextContent, Token, validate,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "entity")]
pub enum Anchor {
    Start,
    After(EntityId),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transaction {
    pub id: u128,
    #[serde(default)]
    pub operations: Vec<Operation>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "op")]
pub enum Operation {
    Insert {
        parent: Option<EntityId>,
        anchor: Anchor,
        entity: Box<Entity>,
    },
    Remove {
        entity: EntityId,
    },
    Move {
        entity: EntityId,
        new_parent: Option<EntityId>,
        anchor: Anchor,
    },
    Rename {
        entity: EntityId,
        name: Option<String>,
    },
    SetSize {
        entity: EntityId,
        axis: Axis,
        value: SizeIntent,
    },
    SetLayout {
        entity: EntityId,
        value: LayoutStyle,
    },
    SetGridPlacement {
        entity: EntityId,
        value: GridPlacement,
    },
    SetPosition {
        entity: EntityId,
        value: Point,
    },
    SetFill {
        entity: EntityId,
        value: Option<Color>,
    },
    SetText {
        entity: EntityId,
        value: Option<TextContent>,
    },
    SetImage {
        entity: EntityId,
        value: Option<ImagePaint>,
    },
    SetAsset {
        asset: Asset,
    },
    RemoveAsset {
        asset: AssetId,
    },
    BindAssetResource {
        asset: AssetId,
        digest: Option<ResourceDigest>,
    },
    SetToken {
        token: Token,
    },
    RemoveToken {
        token: EntityId,
    },
    SetExtensionDeclarations {
        value: ExtensionDeclarations,
    },
    SetValue {
        entity: EntityId,
        key: String,
        value: PropertyValue,
    },
    RemoveValue {
        entity: EntityId,
        key: String,
    },
    SetExtension {
        entity: EntityId,
        namespace: String,
        payload: OpaquePayload,
    },
    RemoveExtension {
        entity: EntityId,
        namespace: String,
    },
    SetUnknownPayload {
        entity: EntityId,
        payload: OpaquePayload,
    },
    #[serde(rename = "_restore_subtree")]
    RestoreSubtree {
        parent: Option<EntityId>,
        anchor: Anchor,
        root: EntityId,
        entities: Vec<Entity>,
        relations: Vec<Relation>,
    },
}

impl Operation {
    fn inserted_entity(&self) -> Option<EntityId> {
        match self {
            Self::Insert { entity, .. } => Some(entity.id),
            Self::Move { entity, .. } => Some(*entity),
            Self::RestoreSubtree { root, .. } => Some(*root),
            _ => None,
        }
    }

    fn requested_position(&self) -> Option<(Option<EntityId>, Anchor)> {
        match self {
            Self::Insert { parent, anchor, .. } | Self::RestoreSubtree { parent, anchor, .. } => {
                Some((*parent, *anchor))
            }
            Self::Move {
                new_parent, anchor, ..
            } => Some((*new_parent, *anchor)),
            _ => None,
        }
    }

    fn with_anchor(&self, anchor: Anchor) -> Self {
        let mut operation = self.clone();
        match &mut operation {
            Self::Insert {
                anchor: current, ..
            }
            | Self::Move {
                anchor: current, ..
            }
            | Self::RestoreSubtree {
                anchor: current, ..
            } => *current = anchor,
            _ => {}
        }
        operation
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Patch {
    pub base_revision: Option<String>,
    #[serde(default)]
    pub transactions: Vec<Transaction>,
}

/// Caller-selected structural limits for an untrusted patch envelope.
///
/// Byte limits belong to the transport that decodes the patch. These limits
/// cover the semantic cardinalities shared by WASM, MCP and future bindings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PatchLimits {
    pub transactions: usize,
    pub operations: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PatchUsage {
    pub transactions: usize,
    pub operations: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Error)]
pub enum PatchLimitExceeded {
    #[error("patch has {observed} transactions; limit is {limit}")]
    Transactions { limit: usize, observed: usize },
    #[error("patch operation count overflowed")]
    OperationCountOverflow,
    #[error("patch has {observed} operations; limit is {limit}")]
    Operations { limit: usize, observed: usize },
}

/// Measures a decoded patch and rejects cardinalities above caller-selected
/// limits before any operation is applied.
///
/// # Errors
///
/// Returns the first transaction or aggregate-operation bound exceeded.
pub fn enforce_patch_limits(
    patch: &Patch,
    limits: PatchLimits,
) -> Result<PatchUsage, PatchLimitExceeded> {
    let transactions = patch.transactions.len();
    if transactions > limits.transactions {
        return Err(PatchLimitExceeded::Transactions {
            limit: limits.transactions,
            observed: transactions,
        });
    }
    let operations = patch
        .transactions
        .iter()
        .try_fold(0_usize, |total, transaction| {
            total.checked_add(transaction.operations.len())
        })
        .ok_or(PatchLimitExceeded::OperationCountOverflow)?;
    if operations > limits.operations {
        return Err(PatchLimitExceeded::Operations {
            limit: limits.operations,
            observed: operations,
        });
    }
    Ok(PatchUsage {
        transactions,
        operations,
    })
}

#[derive(Clone, Debug, Eq, PartialEq, Error, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "code")]
pub enum ApplyError {
    #[error("patch base revision {expected} does not match document revision {actual}")]
    BaseRevisionMismatch { expected: String, actual: String },
    #[error("document revision cannot be computed: {reason}")]
    BaseRevisionUnavailable { reason: String },
    #[error("entity {entity} does not exist")]
    EntityMissing { entity: EntityId },
    #[error("entity {entity} already exists")]
    EntityExists { entity: EntityId },
    #[error("asset {asset} does not exist")]
    AssetMissing { asset: AssetId },
    #[error("parent {parent} does not exist")]
    ParentMissing { parent: EntityId },
    #[error("anchor {anchor:?} does not exist under parent {parent:?}")]
    AnchorMissing {
        parent: Option<EntityId>,
        anchor: Anchor,
    },
    #[error("moving {entity} under {new_parent} would create a containment cycle")]
    CycleRejected {
        entity: EntityId,
        new_parent: EntityId,
    },
    #[error("entity {entity} is still referenced outside its removed subtree")]
    ReferencedEntity { entity: EntityId },
    #[error("operation requires unknown entity {entity}")]
    NotUnknown { entity: EntityId },
    #[error("patch produced an invalid document: {codes:?}")]
    InvalidResult { codes: Vec<String> },
}

/// Applies the complete patch atomically. A failure leaves `document` unchanged.
///
/// # Errors
///
/// Returns a typed conflict when an entity, anchor, or precondition is invalid,
/// or when the resulting canonical document fails structural validation.
pub fn apply_patch(document: &mut Document, patch: &Patch) -> Result<(), ApplyError> {
    apply_patch_with_inverse(document, patch).map(|_| ())
}

/// Applies a patch and returns the inverse patch in undo order.
///
/// # Errors
///
/// Has the same failure conditions and atomicity as [`apply_patch`].
pub fn apply_patch_with_inverse(
    document: &mut Document,
    patch: &Patch,
) -> Result<Patch, ApplyError> {
    if let Some(expected) = &patch.base_revision {
        let actual =
            canonical_hash(document).map_err(|error| ApplyError::BaseRevisionUnavailable {
                reason: error.to_string(),
            })?;
        if expected != &actual {
            return Err(ApplyError::BaseRevisionMismatch {
                expected: expected.clone(),
                actual,
            });
        }
    }
    let mut candidate = document.clone();
    let mut inverse_transactions = Vec::with_capacity(patch.transactions.len());
    let mut same_anchor_cursor: BTreeMap<(Option<EntityId>, Anchor), EntityId> = BTreeMap::new();

    for transaction in &patch.transactions {
        let mut inverses = Vec::with_capacity(transaction.operations.len());
        for operation in &transaction.operations {
            let requested = operation.requested_position();
            let effective = requested
                .and_then(|position| same_anchor_cursor.get(&position).copied())
                .map_or_else(
                    || operation.clone(),
                    |prior| operation.with_anchor(Anchor::After(prior)),
                );
            let inverse = apply_operation(&mut candidate, &effective)?;
            if let (Some(position), Some(inserted)) = (requested, operation.inserted_entity()) {
                same_anchor_cursor.insert(position, inserted);
            }
            inverses.push(inverse);
        }
        inverses.reverse();
        inverse_transactions.push(Transaction {
            id: transaction.id,
            operations: inverses,
        });
    }

    let errors = validate(&candidate)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| diagnostic.code)
        .collect::<Vec<_>>();
    if !errors.is_empty() {
        return Err(ApplyError::InvalidResult { codes: errors });
    }

    let inverse_base =
        canonical_hash(&candidate).map_err(|error| ApplyError::BaseRevisionUnavailable {
            reason: error.to_string(),
        })?;
    *document = candidate;
    inverse_transactions.reverse();
    Ok(Patch {
        base_revision: Some(inverse_base),
        transactions: inverse_transactions,
    })
}

#[expect(
    clippy::too_many_lines,
    reason = "operation cases stay together to make inverse coverage auditable"
)]
fn apply_operation(
    document: &mut Document,
    operation: &Operation,
) -> Result<Operation, ApplyError> {
    match operation {
        Operation::Insert {
            parent,
            anchor,
            entity,
        } => {
            if document.entities.contains_key(&entity.id) {
                return Err(ApplyError::EntityExists { entity: entity.id });
            }
            for child in &entity.children {
                if !document.entities.contains_key(child) {
                    return Err(ApplyError::EntityMissing { entity: *child });
                }
                if document.parent_of(*child).is_some() || document.roots.contains(child) {
                    return Err(ApplyError::ReferencedEntity { entity: *child });
                }
            }
            insert_at(document, *parent, *anchor, entity.id)?;
            document.entities.insert(entity.id, entity.as_ref().clone());
            Ok(Operation::Remove { entity: entity.id })
        }
        Operation::Remove { entity } => remove_subtree(document, *entity),
        Operation::Move {
            entity,
            new_parent,
            anchor,
        } => {
            if !document.entities.contains_key(entity) {
                return Err(ApplyError::EntityMissing { entity: *entity });
            }
            if let Some(parent) = new_parent
                && (*parent == *entity || document.contains_descendant(*entity, *parent))
            {
                return Err(ApplyError::CycleRejected {
                    entity: *entity,
                    new_parent: *parent,
                });
            }
            if matches!(anchor, Anchor::After(anchor_id) if anchor_id == entity) {
                return Err(ApplyError::AnchorMissing {
                    parent: *new_parent,
                    anchor: *anchor,
                });
            }
            let old_parent = document.parent_of(*entity);
            let old_anchor = anchor_of(document, old_parent, *entity)?;
            detach(document, old_parent, *entity)?;
            if let Err(error) = insert_at(document, *new_parent, *anchor, *entity) {
                insert_at(document, old_parent, old_anchor, *entity)
                    .expect("the original position was validated before detaching");
                return Err(error);
            }
            Ok(Operation::Move {
                entity: *entity,
                new_parent: old_parent,
                anchor: old_anchor,
            })
        }
        Operation::Rename { entity, name } => {
            let item = document
                .entities
                .get_mut(entity)
                .ok_or(ApplyError::EntityMissing { entity: *entity })?;
            let previous = std::mem::replace(&mut item.name, name.clone());
            Ok(Operation::Rename {
                entity: *entity,
                name: previous,
            })
        }
        Operation::SetSize {
            entity,
            axis,
            value,
        } => {
            let item = document
                .entities
                .get_mut(entity)
                .ok_or(ApplyError::EntityMissing { entity: *entity })?;
            let slot = match axis {
                Axis::Horizontal => &mut item.authored.width,
                Axis::Vertical => &mut item.authored.height,
            };
            let previous = std::mem::replace(slot, value.clone());
            Ok(Operation::SetSize {
                entity: *entity,
                axis: *axis,
                value: previous,
            })
        }
        Operation::SetLayout { entity, value } => {
            let item = document
                .entities
                .get_mut(entity)
                .ok_or(ApplyError::EntityMissing { entity: *entity })?;
            let previous = std::mem::replace(&mut item.authored.layout, value.clone());
            Ok(Operation::SetLayout {
                entity: *entity,
                value: previous,
            })
        }
        Operation::SetGridPlacement { entity, value } => {
            let item = document
                .entities
                .get_mut(entity)
                .ok_or(ApplyError::EntityMissing { entity: *entity })?;
            let previous = std::mem::replace(&mut item.authored.grid_placement, *value);
            Ok(Operation::SetGridPlacement {
                entity: *entity,
                value: previous,
            })
        }
        Operation::SetPosition { entity, value } => {
            let item = document
                .entities
                .get_mut(entity)
                .ok_or(ApplyError::EntityMissing { entity: *entity })?;
            let previous = std::mem::replace(&mut item.authored.position, *value);
            Ok(Operation::SetPosition {
                entity: *entity,
                value: previous,
            })
        }
        Operation::SetFill { entity, value } => {
            let item = document
                .entities
                .get_mut(entity)
                .ok_or(ApplyError::EntityMissing { entity: *entity })?;
            let previous = std::mem::replace(&mut item.authored.fill, *value);
            Ok(Operation::SetFill {
                entity: *entity,
                value: previous,
            })
        }
        Operation::SetText { entity, value } => {
            let item = document
                .entities
                .get_mut(entity)
                .ok_or(ApplyError::EntityMissing { entity: *entity })?;
            let previous = std::mem::replace(&mut item.authored.text, value.clone());
            Ok(Operation::SetText {
                entity: *entity,
                value: previous,
            })
        }
        Operation::SetImage { entity, value } => {
            let item = document
                .entities
                .get_mut(entity)
                .ok_or(ApplyError::EntityMissing { entity: *entity })?;
            let previous = std::mem::replace(&mut item.authored.image, value.clone());
            Ok(Operation::SetImage {
                entity: *entity,
                value: previous,
            })
        }
        Operation::SetAsset { asset } => Ok(document
            .assets
            .insert(asset.id, asset.clone())
            .map_or(Operation::RemoveAsset { asset: asset.id }, |previous| {
                Operation::SetAsset { asset: previous }
            })),
        Operation::RemoveAsset { asset } => Ok(document
            .assets
            .remove(asset)
            .map_or(Operation::RemoveAsset { asset: *asset }, |previous| {
                Operation::SetAsset { asset: previous }
            })),
        Operation::BindAssetResource { asset, digest } => {
            let item = document
                .assets
                .get_mut(asset)
                .ok_or(ApplyError::AssetMissing { asset: *asset })?;
            let previous = std::mem::replace(&mut item.resource, digest.clone());
            Ok(Operation::BindAssetResource {
                asset: *asset,
                digest: previous,
            })
        }
        Operation::SetToken { token } => Ok(document
            .tokens
            .insert(token.id, token.clone())
            .map_or(Operation::RemoveToken { token: token.id }, |previous| {
                Operation::SetToken { token: previous }
            })),
        Operation::RemoveToken { token } => Ok(document
            .tokens
            .remove(token)
            .map_or(Operation::RemoveToken { token: *token }, |previous| {
                Operation::SetToken { token: previous }
            })),
        Operation::SetExtensionDeclarations { value } => {
            let previous = std::mem::replace(&mut document.extension_declarations, value.clone());
            Ok(Operation::SetExtensionDeclarations { value: previous })
        }
        Operation::SetValue { entity, key, value } => {
            let item = document
                .entities
                .get_mut(entity)
                .ok_or(ApplyError::EntityMissing { entity: *entity })?;
            Ok(item
                .authored
                .values
                .insert(key.clone(), value.clone())
                .map_or(
                    Operation::RemoveValue {
                        entity: *entity,
                        key: key.clone(),
                    },
                    |previous| Operation::SetValue {
                        entity: *entity,
                        key: key.clone(),
                        value: previous,
                    },
                ))
        }
        Operation::RemoveValue { entity, key } => {
            let item = document
                .entities
                .get_mut(entity)
                .ok_or(ApplyError::EntityMissing { entity: *entity })?;
            Ok(item.authored.values.remove(key).map_or(
                Operation::RemoveValue {
                    entity: *entity,
                    key: key.clone(),
                },
                |previous| Operation::SetValue {
                    entity: *entity,
                    key: key.clone(),
                    value: previous,
                },
            ))
        }
        Operation::SetExtension {
            entity,
            namespace,
            payload,
        } => {
            let item = document
                .entities
                .get_mut(entity)
                .ok_or(ApplyError::EntityMissing { entity: *entity })?;
            Ok(item
                .extensions
                .0
                .insert(namespace.clone(), payload.clone())
                .map_or(
                    Operation::RemoveExtension {
                        entity: *entity,
                        namespace: namespace.clone(),
                    },
                    |previous| Operation::SetExtension {
                        entity: *entity,
                        namespace: namespace.clone(),
                        payload: previous,
                    },
                ))
        }
        Operation::RemoveExtension { entity, namespace } => {
            let item = document
                .entities
                .get_mut(entity)
                .ok_or(ApplyError::EntityMissing { entity: *entity })?;
            Ok(item.extensions.0.remove(namespace).map_or(
                Operation::RemoveExtension {
                    entity: *entity,
                    namespace: namespace.clone(),
                },
                |previous| Operation::SetExtension {
                    entity: *entity,
                    namespace: namespace.clone(),
                    payload: previous,
                },
            ))
        }
        Operation::SetUnknownPayload { entity, payload } => {
            let item = document
                .entities
                .get_mut(entity)
                .ok_or(ApplyError::EntityMissing { entity: *entity })?;
            let nuif_core::EntityKind::Unknown(unknown) = &mut item.kind else {
                return Err(ApplyError::NotUnknown { entity: *entity });
            };
            let previous = std::mem::replace(&mut unknown.payload, payload.clone());
            Ok(Operation::SetUnknownPayload {
                entity: *entity,
                payload: previous,
            })
        }
        Operation::RestoreSubtree {
            parent,
            anchor,
            root,
            entities,
            relations,
        } => {
            for entity in entities {
                if document.entities.contains_key(&entity.id) {
                    return Err(ApplyError::EntityExists { entity: entity.id });
                }
            }
            insert_at(document, *parent, *anchor, *root)?;
            for entity in entities {
                document.entities.insert(entity.id, entity.clone());
            }
            document.relations.extend(relations.iter().cloned());
            Ok(Operation::Remove { entity: *root })
        }
    }
}

fn remove_subtree(document: &mut Document, root: EntityId) -> Result<Operation, ApplyError> {
    if !document.entities.contains_key(&root) {
        return Err(ApplyError::EntityMissing { entity: root });
    }
    let parent = document.parent_of(root);
    let anchor = anchor_of(document, parent, root)?;
    let mut pending = vec![root];
    let mut ids = BTreeSet::new();
    while let Some(id) = pending.pop() {
        ids.insert(id);
        if let Some(entity) = document.entities.get(&id) {
            pending.extend(entity.children.iter().copied());
        }
    }
    for entity in document.entities.values() {
        if !ids.contains(&entity.id)
            && matches!(entity.kind, nuif_core::EntityKind::Instance { component } if ids.contains(&component))
        {
            return Err(ApplyError::ReferencedEntity { entity: root });
        }
    }
    detach(document, parent, root)?;
    let entities = ids
        .iter()
        .filter_map(|id| document.entities.remove(id))
        .collect::<Vec<_>>();
    let mut removed_relations = Vec::new();
    document.relations.retain(|relation| {
        if ids.contains(&relation.source) || ids.contains(&relation.target) {
            removed_relations.push(relation.clone());
            false
        } else {
            true
        }
    });
    Ok(Operation::RestoreSubtree {
        parent,
        anchor,
        root,
        entities,
        relations: removed_relations,
    })
}

fn siblings(document: &Document, parent: Option<EntityId>) -> Result<&[EntityId], ApplyError> {
    parent.map_or(Ok(document.roots.as_slice()), |id| {
        document
            .entities
            .get(&id)
            .map(|entity| entity.children.as_slice())
            .ok_or(ApplyError::ParentMissing { parent: id })
    })
}

fn siblings_mut(
    document: &mut Document,
    parent: Option<EntityId>,
) -> Result<&mut Vec<EntityId>, ApplyError> {
    parent.map_or(Ok(&mut document.roots), |id| {
        document
            .entities
            .get_mut(&id)
            .map(|entity| &mut entity.children)
            .ok_or(ApplyError::ParentMissing { parent: id })
    })
}

fn insert_at(
    document: &mut Document,
    parent: Option<EntityId>,
    anchor: Anchor,
    entity: EntityId,
) -> Result<(), ApplyError> {
    let siblings = siblings_mut(document, parent)?;
    let index = match anchor {
        Anchor::Start => 0,
        Anchor::After(id) => siblings
            .iter()
            .position(|candidate| *candidate == id)
            .map(|index| index + 1)
            .ok_or(ApplyError::AnchorMissing { parent, anchor })?,
    };
    siblings.insert(index, entity);
    Ok(())
}

fn detach(
    document: &mut Document,
    parent: Option<EntityId>,
    entity: EntityId,
) -> Result<(), ApplyError> {
    let siblings = siblings_mut(document, parent)?;
    let index = siblings
        .iter()
        .position(|candidate| *candidate == entity)
        .ok_or(ApplyError::EntityMissing { entity })?;
    siblings.remove(index);
    Ok(())
}

fn anchor_of(
    document: &Document,
    parent: Option<EntityId>,
    entity: EntityId,
) -> Result<Anchor, ApplyError> {
    let siblings = siblings(document, parent)?;
    let index = siblings
        .iter()
        .position(|candidate| *candidate == entity)
        .ok_or(ApplyError::EntityMissing { entity })?;
    Ok(if index == 0 {
        Anchor::Start
    } else {
        Anchor::After(siblings[index - 1])
    })
}

#[must_use]
pub fn empty_extensions() -> Extensions {
    Extensions::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nuif_core::{EntityKind, PropertyValue};

    fn base() -> Document {
        let mut document = Document::empty(EntityId::new(1));
        let root = Entity::new(EntityId::new(2), EntityKind::Container);
        document.roots.push(root.id);
        document.entities.insert(root.id, root);
        document
    }

    #[test]
    fn same_anchor_inserts_keep_patch_order() {
        let mut document = base();
        let first = Entity::new(EntityId::new(3), EntityKind::Container);
        let second = Entity::new(EntityId::new(4), EntityKind::Container);
        let patch = Patch {
            base_revision: None,
            transactions: vec![Transaction {
                id: 1,
                operations: vec![
                    Operation::Insert {
                        parent: Some(EntityId::new(2)),
                        anchor: Anchor::Start,
                        entity: Box::new(first),
                    },
                    Operation::Insert {
                        parent: Some(EntityId::new(2)),
                        anchor: Anchor::Start,
                        entity: Box::new(second),
                    },
                ],
            }],
        };
        apply_patch(&mut document, &patch).unwrap();
        assert_eq!(
            document.entities[&EntityId::new(2)].children,
            [EntityId::new(3), EntityId::new(4)]
        );
    }

    #[test]
    fn same_anchor_order_spans_transactions_in_one_patch() {
        let mut document = base();
        let transactions = [EntityId::new(3), EntityId::new(4)]
            .into_iter()
            .enumerate()
            .map(|(index, id)| Transaction {
                id: u128::try_from(index).expect("fixture index fits u128"),
                operations: vec![Operation::Insert {
                    parent: Some(EntityId::new(2)),
                    anchor: Anchor::Start,
                    entity: Box::new(Entity::new(id, EntityKind::Container)),
                }],
            })
            .collect();
        let patch = Patch {
            base_revision: None,
            transactions,
        };

        apply_patch(&mut document, &patch).unwrap();
        assert_eq!(
            document.entities[&EntityId::new(2)].children,
            [EntityId::new(3), EntityId::new(4)]
        );
    }

    #[test]
    fn inverse_restores_exact_document() {
        let mut document = base();
        let original = document.clone();
        let patch = Patch {
            base_revision: None,
            transactions: vec![Transaction {
                id: 7,
                operations: vec![Operation::Rename {
                    entity: EntityId::new(2),
                    name: Some("renamed".to_owned()),
                }],
            }],
        };
        let inverse = apply_patch_with_inverse(&mut document, &patch).unwrap();
        apply_patch(&mut document, &inverse).unwrap();
        assert_eq!(document, original);
    }

    #[test]
    fn document_level_token_and_declaration_operations_are_invertible() {
        let mut document = base();
        let original = document.clone();
        let token_id = EntityId::new(10);
        let patch = Patch {
            base_revision: None,
            transactions: vec![Transaction {
                id: 8,
                operations: vec![
                    Operation::SetExtensionDeclarations {
                        value: ExtensionDeclarations {
                            used: BTreeSet::from(["vendor.probe".to_owned()]),
                            required: BTreeSet::new(),
                            fallback_kind: BTreeMap::from([(
                                "vendor.probe".to_owned(),
                                "container".to_owned(),
                            )]),
                        },
                    },
                    Operation::SetToken {
                        token: Token {
                            id: token_id,
                            name: "space.probe".to_owned(),
                            value: PropertyValue::Real(8.0),
                        },
                    },
                ],
            }],
        };
        let inverse = apply_patch_with_inverse(&mut document, &patch).unwrap();
        assert!(document.tokens.contains_key(&token_id));
        assert!(
            document
                .extension_declarations
                .used
                .contains("vendor.probe")
        );
        apply_patch(&mut document, &inverse).unwrap();
        assert_eq!(document, original);
    }

    #[test]
    fn cycle_is_rejected_atomically() {
        let mut document = base();
        let child = Entity::new(EntityId::new(3), EntityKind::Container);
        document
            .entities
            .get_mut(&EntityId::new(2))
            .unwrap()
            .children
            .push(child.id);
        document.entities.insert(child.id, child);
        let original = document.clone();
        let patch = Patch {
            base_revision: None,
            transactions: vec![Transaction {
                id: 2,
                operations: vec![Operation::Move {
                    entity: EntityId::new(2),
                    new_parent: Some(EntityId::new(3)),
                    anchor: Anchor::Start,
                }],
            }],
        };
        assert!(matches!(
            apply_patch(&mut document, &patch),
            Err(ApplyError::CycleRejected { .. })
        ));
        assert_eq!(document, original);
    }

    #[test]
    fn stale_base_revision_is_rejected_atomically() {
        let mut document = base();
        let original = document.clone();
        let patch = Patch {
            base_revision: Some("nuif-cbor-0:sha256:stale".to_owned()),
            transactions: vec![Transaction {
                id: 3,
                operations: vec![Operation::Rename {
                    entity: EntityId::new(2),
                    name: Some("must not apply".to_owned()),
                }],
            }],
        };

        assert!(matches!(
            apply_patch(&mut document, &patch),
            Err(ApplyError::BaseRevisionMismatch { .. })
        ));
        assert_eq!(document, original);
    }

    #[test]
    fn visual_editor_properties_are_atomic_and_invertible() {
        let mut document = base();
        document.entities.get_mut(&EntityId::new(2)).unwrap().kind = EntityKind::Text;
        let original = document.clone();
        let entity = EntityId::new(2);
        let patch = Patch {
            base_revision: None,
            transactions: vec![Transaction {
                id: 9,
                operations: vec![
                    Operation::SetPosition {
                        entity,
                        value: Point { x: 12.0, y: 24.0 },
                    },
                    Operation::SetFill {
                        entity,
                        value: Some(Color {
                            space: nuif_core::ColorSpace::Srgb,
                            red: 0.2,
                            green: 0.4,
                            blue: 0.6,
                            alpha: 0.8,
                        }),
                    },
                    Operation::SetText {
                        entity,
                        value: Some(TextContent {
                            content: "Editable".to_owned(),
                            font: "Ahem".to_owned(),
                            font_sha256: "0".repeat(64),
                            size: 16.0,
                            line_height: 20.0,
                        }),
                    },
                ],
            }],
        };
        let inverse = apply_patch_with_inverse(&mut document, &patch).unwrap();
        assert!((document.entities[&entity].authored.position.x - 12.0).abs() < f64::EPSILON);
        assert_eq!(
            document.entities[&entity]
                .authored
                .text
                .as_ref()
                .unwrap()
                .content,
            "Editable"
        );
        apply_patch(&mut document, &inverse).unwrap();
        assert_eq!(document, original);
    }

    #[test]
    fn grid_item_placement_is_atomic_and_invertible() {
        let mut document = base();
        let root = EntityId::new(2);
        let child = EntityId::new(3);
        document
            .entities
            .get_mut(&root)
            .unwrap()
            .authored
            .layout
            .family = nuif_core::LayoutFamily::Grid;
        document
            .entities
            .get_mut(&root)
            .unwrap()
            .authored
            .layout
            .grid
            .columns = vec![
            nuif_core::GridTrack::Fraction(1.0),
            nuif_core::GridTrack::Fraction(1.0),
        ];
        document
            .entities
            .get_mut(&root)
            .unwrap()
            .authored
            .layout
            .grid
            .rows = vec![nuif_core::GridTrack::Fraction(1.0)];
        document
            .entities
            .get_mut(&root)
            .unwrap()
            .children
            .push(child);
        document
            .entities
            .insert(child, Entity::new(child, EntityKind::Container));
        let original = document.clone();
        let patch = Patch {
            base_revision: None,
            transactions: vec![Transaction {
                id: 10,
                operations: vec![Operation::SetGridPlacement {
                    entity: child,
                    value: GridPlacement {
                        column: Some(1),
                        row: Some(0),
                        ..GridPlacement::default()
                    },
                }],
            }],
        };

        let inverse = apply_patch_with_inverse(&mut document, &patch).unwrap();
        assert_eq!(
            document.entities[&child].authored.grid_placement.column,
            Some(1)
        );
        apply_patch(&mut document, &inverse).unwrap();
        assert_eq!(document, original);
    }

    #[test]
    fn asset_binding_is_semantic_and_invertible() {
        let mut document = base();
        let original = document.clone();
        let asset_id = AssetId::new(0xa0);
        let first = ResourceDigest::from_sha256_hex("a".repeat(64));
        let second = ResourceDigest::from_sha256_hex("b".repeat(64));
        let patch = Patch {
            base_revision: None,
            transactions: vec![Transaction {
                id: 10,
                operations: vec![
                    Operation::SetAsset {
                        asset: Asset {
                            schema_version: nuif_core::CURRENT_SCHEMA_VERSION,
                            id: asset_id,
                            name: Some("image".to_owned()),
                            resource: Some(first),
                            portability: nuif_core::AssetPortability::Portable,
                            kind: nuif_core::AssetKind::Image(nuif_core::ImageAsset {
                                width: 16,
                                height: 16,
                                decoder_profile: "nuif-png-0".to_owned(),
                            }),
                        },
                    },
                    Operation::BindAssetResource {
                        asset: asset_id,
                        digest: Some(second.clone()),
                    },
                ],
            }],
        };
        let inverse = apply_patch_with_inverse(&mut document, &patch).unwrap();
        assert_eq!(document.assets[&asset_id].id, asset_id);
        assert_eq!(document.assets[&asset_id].resource.as_ref(), Some(&second));
        apply_patch(&mut document, &inverse).unwrap();
        assert_eq!(document, original);
    }

    #[test]
    fn patch_limits_measure_the_shared_semantic_envelope() {
        let patch = Patch {
            base_revision: None,
            transactions: vec![
                Transaction {
                    id: 1,
                    operations: vec![Operation::Rename {
                        entity: EntityId::new(1),
                        name: Some("one".to_owned()),
                    }],
                },
                Transaction {
                    id: 2,
                    operations: vec![
                        Operation::Rename {
                            entity: EntityId::new(2),
                            name: Some("two".to_owned()),
                        },
                        Operation::Rename {
                            entity: EntityId::new(3),
                            name: Some("three".to_owned()),
                        },
                    ],
                },
            ],
        };
        assert_eq!(
            enforce_patch_limits(
                &patch,
                PatchLimits {
                    transactions: 2,
                    operations: 3,
                },
            ),
            Ok(PatchUsage {
                transactions: 2,
                operations: 3,
            })
        );
        assert_eq!(
            enforce_patch_limits(
                &patch,
                PatchLimits {
                    transactions: 1,
                    operations: 3,
                },
            ),
            Err(PatchLimitExceeded::Transactions {
                limit: 1,
                observed: 2,
            })
        );
        assert_eq!(
            enforce_patch_limits(
                &patch,
                PatchLimits {
                    transactions: 2,
                    operations: 2,
                },
            ),
            Err(PatchLimitExceeded::Operations {
                limit: 2,
                observed: 3,
            })
        );
    }
}
