//! Deterministic, validity-preserving document reduction and fixture emission.

use nuif_codec::{CanonicalText, Encoder, canonical_hash};
use nuif_core::{
    Document, EntityId, Extensions, LayoutStyle, Point, PropertyValue, Semantics, Severity,
    SizeIntent, validate,
};
use nuif_protocol::Operation;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Stable failure classes for document reduction preconditions.
#[derive(Debug, Error, Eq, PartialEq)]
pub enum ReductionError {
    #[error("the initial document is invalid: {codes}")]
    InvalidInput { codes: String },
    #[error("the initial document does not satisfy the interestingness predicate")]
    NotInteresting,
    #[error("the initial document cannot be canonicalized: {message}")]
    Canonicalization { message: String },
}

/// One accepted, strictly simplifying reducer transformation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReductionStep {
    pub pass: String,
    pub detail: String,
    pub before_hash: String,
    pub after_hash: String,
    pub entities_before: usize,
    pub entities_after: usize,
}

/// Machine-readable evidence for one complete reduction run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReductionReport {
    pub schema_version: u32,
    pub original_hash: String,
    pub minimized_hash: String,
    pub original_entities: usize,
    pub minimized_entities: usize,
    pub predicate_evaluations: u64,
    pub invalid_candidates: u64,
    pub uninteresting_candidates: u64,
    pub duplicate_candidates: u64,
    pub steps: Vec<ReductionStep>,
}

/// A minimized document together with the exact reduction evidence.
#[derive(Clone, Debug, PartialEq)]
pub struct DocumentReduction {
    pub document: Document,
    pub report: ReductionReport,
}

/// Manifest stored beside an automatically emitted regression fixture.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReductionFixtureManifest {
    pub schema_version: u32,
    pub predicate: String,
    pub seed: Option<u64>,
    pub document: String,
    pub operations: String,
    pub reduction_report: String,
    pub canonical_hash: String,
    pub operation_count: usize,
}

struct Reducer<'a, F> {
    current: Document,
    predicate: &'a mut F,
    seen: BTreeSet<String>,
    original_hash: String,
    original_entities: usize,
    predicate_evaluations: u64,
    invalid_candidates: u64,
    uninteresting_candidates: u64,
    duplicate_candidates: u64,
    steps: Vec<ReductionStep>,
}

impl<F> Reducer<'_, F>
where
    F: FnMut(&Document) -> bool,
{
    fn try_transform<T>(
        &mut self,
        pass: &str,
        detail: T,
        transform: impl FnOnce(&mut Document) -> bool,
    ) -> bool
    where
        T: Into<String>,
    {
        let mut candidate = self.current.clone();
        if !transform(&mut candidate) {
            return false;
        }
        if validate(&candidate)
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
        {
            self.invalid_candidates += 1;
            return false;
        }
        let Ok(after_hash) = canonical_hash(&candidate) else {
            self.invalid_candidates += 1;
            return false;
        };
        if !self.seen.insert(after_hash.clone()) {
            self.duplicate_candidates += 1;
            return false;
        }
        self.predicate_evaluations += 1;
        if !(self.predicate)(&candidate) {
            self.uninteresting_candidates += 1;
            return false;
        }
        let Ok(before_hash) = canonical_hash(&self.current) else {
            self.invalid_candidates += 1;
            return false;
        };
        self.steps.push(ReductionStep {
            pass: pass.to_owned(),
            detail: detail.into(),
            before_hash,
            after_hash,
            entities_before: self.current.entities.len(),
            entities_after: candidate.entities.len(),
        });
        self.current = candidate;
        true
    }
}

/// Reduces a valid interesting document through deterministic subtree, graph
/// collection, extension and scalar passes. Every predicate invocation receives
/// a structurally valid document.
///
/// Opaque unknown-kind payload bytes are deliberately held fixed because the
/// reducer cannot know which bytes are semantically relevant to their owner.
///
/// # Errors
///
/// Returns [`ReductionError::InvalidInput`] before invoking the predicate when
/// the document has structural errors, or [`ReductionError::NotInteresting`]
/// when the initial predicate is false.
pub fn minimize_document<F>(
    document: &Document,
    mut interesting: F,
) -> Result<DocumentReduction, ReductionError>
where
    F: FnMut(&Document) -> bool,
{
    let errors = validate(document)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| diagnostic.code)
        .collect::<Vec<_>>();
    if !errors.is_empty() {
        return Err(ReductionError::InvalidInput {
            codes: errors.join(","),
        });
    }
    if !interesting(document) {
        return Err(ReductionError::NotInteresting);
    }
    let original_hash =
        canonical_hash(document).map_err(|error| ReductionError::Canonicalization {
            message: error.to_string(),
        })?;
    let mut reducer = Reducer {
        current: document.clone(),
        predicate: &mut interesting,
        seen: BTreeSet::from([original_hash.clone()]),
        original_hash: original_hash.clone(),
        original_entities: document.entities.len(),
        predicate_evaluations: 1,
        invalid_candidates: 0,
        uninteresting_candidates: 0,
        duplicate_candidates: 0,
        steps: Vec::new(),
    };

    reduce_entity_subtrees(&mut reducer);
    reduce_relations(&mut reducer);
    reduce_tokens(&mut reducer);
    reduce_assets(&mut reducer);
    reduce_extensions(&mut reducer);
    reduce_entity_scalars(&mut reducer);

    let minimized_hash =
        canonical_hash(&reducer.current).map_err(|error| ReductionError::Canonicalization {
            message: error.to_string(),
        })?;
    let minimized_entities = reducer.current.entities.len();
    Ok(DocumentReduction {
        document: reducer.current,
        report: ReductionReport {
            schema_version: 1,
            original_hash: reducer.original_hash,
            minimized_hash,
            original_entities: reducer.original_entities,
            minimized_entities,
            predicate_evaluations: reducer.predicate_evaluations,
            invalid_candidates: reducer.invalid_candidates,
            uninteresting_candidates: reducer.uninteresting_candidates,
            duplicate_candidates: reducer.duplicate_candidates,
            steps: reducer.steps,
        },
    })
}

fn reduce_entity_subtrees<F>(reducer: &mut Reducer<'_, F>)
where
    F: FnMut(&Document) -> bool,
{
    let mut granularity = 2_usize;
    loop {
        let ids = reducer.current.entities.keys().copied().collect::<Vec<_>>();
        if ids.is_empty() {
            return;
        }
        let chunk_size = ids.len().div_ceil(granularity);
        let mut accepted = false;
        for chunk in ids.chunks(chunk_size) {
            let removals = chunk.to_vec();
            let detail = removals
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",");
            if reducer.try_transform("entity-subtrees", detail, |candidate| {
                remove_subtrees(candidate, &removals)
            }) {
                granularity = granularity.saturating_sub(1).max(2);
                accepted = true;
                break;
            }
        }
        if accepted {
            continue;
        }
        if granularity >= ids.len() {
            return;
        }
        granularity = (granularity * 2).min(ids.len());
    }
}

fn remove_subtrees(document: &mut Document, roots: &[EntityId]) -> bool {
    let mut pending = roots.to_vec();
    let mut removed = BTreeSet::new();
    while let Some(id) = pending.pop() {
        if !removed.insert(id) {
            continue;
        }
        if let Some(entity) = document.entities.get(&id) {
            pending.extend(entity.children.iter().copied());
        }
    }
    removed.retain(|id| document.entities.contains_key(id));
    if removed.is_empty() {
        return false;
    }
    document.entities.retain(|id, _| !removed.contains(id));
    document.roots.retain(|id| !removed.contains(id));
    for entity in document.entities.values_mut() {
        entity.children.retain(|id| !removed.contains(id));
    }
    document.relations.retain(|relation| {
        !removed.contains(&relation.source) && !removed.contains(&relation.target)
    });
    true
}

fn reduce_relations<F>(reducer: &mut Reducer<'_, F>)
where
    F: FnMut(&Document) -> bool,
{
    let _ = reducer.try_transform("relations", "clear all relations", |document| {
        if document.relations.is_empty() {
            false
        } else {
            document.relations.clear();
            true
        }
    });
}

fn reduce_tokens<F>(reducer: &mut Reducer<'_, F>)
where
    F: FnMut(&Document) -> bool,
{
    reduce_map_keys(
        reducer,
        "tokens",
        |document| document.tokens.keys().copied().collect(),
        |document, ids| {
            let before = document.tokens.len();
            document.tokens.retain(|id, _| !ids.contains(id));
            document.tokens.len() != before
        },
    );
    let ids = reducer.current.tokens.keys().copied().collect::<Vec<_>>();
    for id in ids {
        let _ = reducer.try_transform("token-values", id.to_string(), |document| {
            let Some(token) = document.tokens.get_mut(&id) else {
                return false;
            };
            if token.value == PropertyValue::Null {
                false
            } else {
                token.value = PropertyValue::Null;
                true
            }
        });
    }
}

fn reduce_assets<F>(reducer: &mut Reducer<'_, F>)
where
    F: FnMut(&Document) -> bool,
{
    reduce_map_keys(
        reducer,
        "assets",
        |document| document.assets.keys().copied().collect(),
        |document, ids| {
            let before = document.assets.len();
            document.assets.retain(|id, _| !ids.contains(id));
            document.assets.len() != before
        },
    );
    let ids = reducer.current.assets.keys().copied().collect::<Vec<_>>();
    for id in ids {
        let _ = reducer.try_transform("asset-names", id.to_string(), |document| {
            let Some(asset) = document.assets.get_mut(&id) else {
                return false;
            };
            asset.name.take().is_some()
        });
    }
}

fn reduce_map_keys<F, K, Keys, Remove>(
    reducer: &mut Reducer<'_, F>,
    pass: &str,
    keys: Keys,
    remove: Remove,
) where
    F: FnMut(&Document) -> bool,
    K: Copy + Ord + ToString,
    Keys: Fn(&Document) -> Vec<K>,
    Remove: Fn(&mut Document, &BTreeSet<K>) -> bool,
{
    let ids = keys(&reducer.current);
    if ids.is_empty() {
        return;
    }
    let all = ids.iter().copied().collect::<BTreeSet<_>>();
    if reducer.try_transform(pass, "clear all", |document| remove(document, &all)) {
        return;
    }
    for id in ids {
        let removal = BTreeSet::from([id]);
        let _ = reducer.try_transform(pass, id.to_string(), |document| remove(document, &removal));
    }
}

fn reduce_extensions<F>(reducer: &mut Reducer<'_, F>)
where
    F: FnMut(&Document) -> bool,
{
    let _ = reducer.try_transform("document-extensions", "clear payloads", |document| {
        if document.extensions.0.is_empty() {
            false
        } else {
            document.extensions = Extensions::default();
            true
        }
    });
    let namespaces = reducer
        .current
        .extension_declarations
        .used
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    for namespace in namespaces {
        let detail = namespace.clone();
        let _ = reducer.try_transform("extension-namespaces", detail, |document| {
            let mut changed = document.extension_declarations.used.remove(&namespace);
            changed |= document.extension_declarations.required.remove(&namespace);
            changed |= document
                .extension_declarations
                .fallback_kind
                .remove(&namespace)
                .is_some();
            changed |= document.extensions.0.remove(&namespace).is_some();
            for entity in document.entities.values_mut() {
                changed |= entity.extensions.0.remove(&namespace).is_some();
            }
            changed
        });
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "the ordered scalar pass list is intentionally explicit and reviewable"
)]
fn reduce_entity_scalars<F>(reducer: &mut Reducer<'_, F>)
where
    F: FnMut(&Document) -> bool,
{
    let ids = reducer.current.entities.keys().copied().collect::<Vec<_>>();
    for id in ids {
        scalar_edit(reducer, id, "name", |entity| entity.name.take().is_some());
        scalar_edit(reducer, id, "width", |entity| {
            if entity.authored.width == SizeIntent::Auto {
                false
            } else {
                entity.authored.width = SizeIntent::Auto;
                true
            }
        });
        scalar_edit(reducer, id, "height", |entity| {
            if entity.authored.height == SizeIntent::Auto {
                false
            } else {
                entity.authored.height = SizeIntent::Auto;
                true
            }
        });
        scalar_edit(reducer, id, "position", |entity| {
            if entity.authored.position == Point::default() {
                false
            } else {
                entity.authored.position = Point::default();
                true
            }
        });
        scalar_edit(reducer, id, "layout", |entity| {
            if entity.authored.layout == LayoutStyle::default() {
                false
            } else {
                entity.authored.layout = LayoutStyle::default();
                true
            }
        });
        scalar_edit(reducer, id, "fill", |entity| {
            entity.authored.fill.take().is_some()
        });
        scalar_edit(reducer, id, "text", |entity| {
            entity.authored.text.take().is_some()
        });
        scalar_edit(reducer, id, "text-content", |entity| {
            let Some(text) = entity.authored.text.as_mut() else {
                return false;
            };
            if text.content.is_empty() {
                false
            } else {
                text.content.clear();
                true
            }
        });
        scalar_edit(reducer, id, "image", |entity| {
            entity.authored.image.take().is_some()
        });
        scalar_edit(reducer, id, "responsive", |entity| {
            if entity.authored.responsive.is_empty() {
                false
            } else {
                entity.authored.responsive.clear();
                true
            }
        });
        scalar_edit(reducer, id, "values", |entity| {
            if entity.authored.values.is_empty() {
                false
            } else {
                entity.authored.values.clear();
                true
            }
        });
        scalar_edit(reducer, id, "semantics", |entity| {
            if entity.semantics == Semantics::default() {
                false
            } else {
                entity.semantics = Semantics::default();
                true
            }
        });
        scalar_edit(reducer, id, "extensions", |entity| {
            if entity.extensions.0.is_empty() {
                false
            } else {
                entity.extensions = Extensions::default();
                true
            }
        });
        let keys = reducer
            .current
            .entities
            .get(&id)
            .map(|entity| entity.authored.values.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        for key in keys {
            let detail = format!("{id}:value:{key}");
            let _ = reducer.try_transform("entity-scalars", detail, |document| {
                document
                    .entities
                    .get_mut(&id)
                    .and_then(|entity| entity.authored.values.remove(&key))
                    .is_some()
            });
        }
    }
}

fn scalar_edit<F>(
    reducer: &mut Reducer<'_, F>,
    id: EntityId,
    field: &str,
    edit: impl FnOnce(&mut nuif_core::Entity) -> bool,
) where
    F: FnMut(&Document) -> bool,
{
    let detail = format!("{id}:{field}");
    let _ = reducer.try_transform("entity-scalars", detail, |document| {
        document.entities.get_mut(&id).is_some_and(edit)
    });
}

/// Emits a reduced canonical document, semantic operation list and report into
/// a new fixture directory. Existing destinations are never overwritten.
///
/// # Errors
///
/// Returns an error for an existing destination, a non-directory parent,
/// serialization failure, I/O failure or failed atomic rename.
pub fn write_reduced_fixture(
    directory: &Path,
    predicate: &str,
    seed: Option<u64>,
    reduction: &DocumentReduction,
    operations: &[Operation],
) -> Result<ReductionFixtureManifest, String> {
    if directory.exists() {
        return Err(format!(
            "fixture destination already exists: {}",
            directory.display()
        ));
    }
    let parent = directory
        .parent()
        .ok_or_else(|| format!("fixture destination has no parent: {}", directory.display()))?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let staging = staging_path(parent, directory)?;
    fs::create_dir(&staging).map_err(|error| error.to_string())?;
    let manifest = ReductionFixtureManifest {
        schema_version: 1,
        predicate: predicate.to_owned(),
        seed,
        document: "input.nuif.json".to_owned(),
        operations: "operations.json".to_owned(),
        reduction_report: "reduction.json".to_owned(),
        canonical_hash: reduction.report.minimized_hash.clone(),
        operation_count: operations.len(),
    };
    let result = write_fixture_files(&staging, reduction, operations, &manifest)
        .and_then(|()| fs::rename(&staging, directory).map_err(|error| error.to_string()));
    if let Err(error) = result {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    Ok(manifest)
}

/// Reproduces and hierarchically minimizes the base document for a seeded
/// trial failure, preserving its already minimized operation sequence.
///
/// # Errors
///
/// Returns an error when the recorded failure no longer reproduces or the
/// initial fixture is invalid.
pub fn minimize_trial_reproduction(
    reproduction: &crate::Reproduction,
) -> Result<DocumentReduction, String> {
    let base = crate::responsive_card_fixture();
    minimize_document(&base, |candidate| {
        matches!(
            super::verify_document_iteration(
                candidate,
                reproduction.seed,
                reproduction.iteration,
                &reproduction.minimized_operations,
                reproduction.trial_width,
                reproduction.verify_snapshot,
            ),
            Err((ref code, _)) if code == &reproduction.failure_code
        )
    })
    .map_err(|error| error.to_string())
}

fn staging_path(parent: &Path, directory: &Path) -> Result<PathBuf, String> {
    let name = directory
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            format!(
                "fixture destination has no UTF-8 name: {}",
                directory.display()
            )
        })?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    Ok(parent.join(format!(".{name}.tmp-{}-{nonce}", std::process::id())))
}

fn write_fixture_files(
    staging: &Path,
    reduction: &DocumentReduction,
    operations: &[Operation],
    manifest: &ReductionFixtureManifest,
) -> Result<(), String> {
    let document = CanonicalText
        .encode(&reduction.document)
        .map_err(|error| error.to_string())?;
    let operation_bytes =
        serde_json::to_vec_pretty(operations).map_err(|error| error.to_string())?;
    let report = serde_json::to_vec_pretty(&reduction.report).map_err(|error| error.to_string())?;
    let manifest = serde_json::to_vec_pretty(manifest).map_err(|error| error.to_string())?;
    for (name, bytes) in [
        ("input.nuif.json", document),
        ("operations.json", operation_bytes),
        ("reduction.json", report),
        ("fixture.json", manifest),
    ] {
        fs::write(staging.join(name), bytes).map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nuif_codec::Decoder;
    use nuif_core::{Entity, EntityKind};

    #[test]
    fn reduction_keeps_only_the_interesting_valid_path() {
        let mut document = crate::responsive_card_fixture();
        let target = EntityId::new(0x22);
        document.entities.get_mut(&target).unwrap().name = Some("trigger".to_owned());
        let result = minimize_document(&document, |candidate| {
            candidate
                .entities
                .get(&target)
                .and_then(|entity| entity.name.as_deref())
                == Some("trigger")
        })
        .unwrap();
        assert_eq!(result.document.entities.len(), 3);
        assert!(result.document.entities.contains_key(&EntityId::new(0x10)));
        assert!(result.document.entities.contains_key(&EntityId::new(0x20)));
        assert!(result.document.entities.contains_key(&target));
        assert!(
            validate(&result.document)
                .iter()
                .all(|item| item.severity != Severity::Error)
        );
        assert!(result.report.predicate_evaluations > 1);
        assert!(!result.report.steps.is_empty());
    }

    #[test]
    fn referenced_components_cannot_be_reduced_away() {
        let root = EntityId::new(1);
        let component = EntityId::new(2);
        let instance = EntityId::new(3);
        let mut document = Document::empty(EntityId::new(100));
        let mut surface = Entity::new(root, EntityKind::Surface);
        surface.children = vec![component, instance];
        document.roots.push(root);
        document.entities.insert(root, surface);
        document
            .entities
            .insert(component, Entity::new(component, EntityKind::Component));
        document.entities.insert(
            instance,
            Entity::new(instance, EntityKind::Instance { component }),
        );
        let result = minimize_document(&document, |candidate| {
            candidate.entities.contains_key(&instance)
        })
        .unwrap();
        assert!(result.document.entities.contains_key(&component));
        assert!(
            validate(&result.document)
                .iter()
                .all(|item| item.severity != Severity::Error)
        );
        assert!(result.report.invalid_candidates > 0);
    }

    #[test]
    fn invalid_and_uninteresting_inputs_are_rejected_before_reduction() {
        let mut invalid = Document::empty(EntityId::new(1));
        invalid.roots.push(EntityId::new(9));
        assert!(matches!(
            minimize_document(&invalid, |_| true),
            Err(ReductionError::InvalidInput { .. })
        ));
        assert_eq!(
            minimize_document(&Document::empty(EntityId::new(1)), |_| false),
            Err(ReductionError::NotInteresting)
        );
    }

    #[test]
    fn fixture_writer_is_atomic_and_never_overwrites() {
        let document = crate::responsive_card_fixture();
        let reduction =
            minimize_document(&document, |candidate| !candidate.entities.is_empty()).unwrap();
        let root = std::env::temp_dir().join(format!(
            "nuif-reduction-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let destination = root.join("named-regression");
        let manifest =
            write_reduced_fixture(&destination, "test:interesting", Some(7), &reduction, &[])
                .unwrap();
        let bytes = fs::read(destination.join(&manifest.document)).unwrap();
        assert_eq!(CanonicalText.decode(&bytes).unwrap(), reduction.document);
        assert!(
            write_reduced_fixture(&destination, "test:interesting", Some(7), &reduction, &[])
                .is_err()
        );
        fs::remove_dir_all(root).unwrap();
    }
}
