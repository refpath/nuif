#![doc = "Stable semantic query surface for headless tooling."]

use nuif_core::{Document, Entity, EntityId, EntityKind};

/// Returns the entity with the requested stable identity.
#[must_use]
pub fn entity(document: &Document, id: EntityId) -> Option<&Entity> {
    document.entities.get(&id)
}

/// Iterates the document's root entities in authored root order.
#[must_use]
pub fn roots(document: &Document) -> impl Iterator<Item = &Entity> {
    document
        .roots
        .iter()
        .filter_map(|id| document.entities.get(id))
}

/// Returns all entities whose kind satisfies `predicate`.
#[must_use]
pub fn by_kind(document: &Document, predicate: fn(&EntityKind) -> bool) -> Vec<&Entity> {
    document
        .entities
        .values()
        .filter(|entity| predicate(&entity.kind))
        .collect()
}
