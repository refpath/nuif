#![doc = "Stable semantic query surface for headless tooling."]

use nuif_core::{Document, Entity, EntityId, EntityKind};

/// Returns the entity with the requested stable identity.
#[must_use = "query results should be inspected"]
pub fn entity(document: &Document, id: EntityId) -> Option<&Entity> {
    document.entities.get(&id)
}

/// Iterates the document's root entities in authored root order.
#[must_use = "iterators are lazy and must be consumed"]
pub fn roots(document: &Document) -> impl Iterator<Item = &Entity> {
    document
        .roots
        .iter()
        .filter_map(|id| document.entities.get(id))
}

/// Returns all entities whose kind satisfies `predicate`.
#[must_use = "query results should be inspected"]
pub fn by_kind(document: &Document, predicate: fn(&EntityKind) -> bool) -> Vec<&Entity> {
    document
        .entities
        .values()
        .filter(|entity| predicate(&entity.kind))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nuif_core::{Entity, ShapeKind};

    fn fixture() -> Document {
        let mut document = Document::empty(EntityId::new(1));
        let surface = Entity::new(EntityId::new(2), EntityKind::Surface);
        let text = Entity::new(EntityId::new(3), EntityKind::Text);
        let shape = Entity::new(EntityId::new(4), EntityKind::Shape(ShapeKind::Rectangle));
        document.roots.extend([shape.id, surface.id]);
        for entity in [surface, text, shape] {
            document.entities.insert(entity.id, entity);
        }
        document
    }

    #[test]
    fn stable_identity_lookup_returns_only_the_requested_entity() {
        let document = fixture();
        assert_eq!(
            entity(&document, EntityId::new(3)).map(|entity| entity.id),
            Some(EntityId::new(3))
        );
        assert!(entity(&document, EntityId::new(99)).is_none());
    }

    #[test]
    fn roots_preserve_authored_order() {
        let document = fixture();
        assert_eq!(
            roots(&document).map(|entity| entity.id).collect::<Vec<_>>(),
            [EntityId::new(4), EntityId::new(2)]
        );
    }

    #[test]
    fn kind_scan_returns_matching_entities() {
        let document = fixture();
        assert_eq!(
            by_kind(&document, |kind| matches!(kind, EntityKind::Text))
                .into_iter()
                .map(|entity| entity.id)
                .collect::<Vec<_>>(),
            [EntityId::new(3)]
        );
    }
}
