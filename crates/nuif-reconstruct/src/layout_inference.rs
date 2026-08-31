use crate::{Bounds, Confidence, EvidenceClass, InferenceProvenance, ObservationId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

pub const LAYOUT_INFERENCE_PROFILE: &str = "nuif-layout-inference-0";
pub const MAX_LAYOUT_SNAPSHOTS: usize = 16;
pub const MAX_LAYOUT_ITEMS: usize = 4_096;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutItemObservation {
    pub id: String,
    pub bounds: Bounds,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutSnapshot {
    pub viewport_width: f64,
    pub parent: Bounds,
    pub items: Vec<LayoutItemObservation>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutCandidateKind {
    StackRow,
    StackColumn,
    Grid,
    Constraint,
    Freeform,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PredictedLayoutItem {
    pub id: String,
    pub bounds: Bounds,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutCandidateReport {
    pub kind: LayoutCandidateKind,
    pub prediction_model: String,
    pub training_score: f64,
    pub structural_penalty: f64,
    pub complexity_penalty: f64,
    pub heldout_error: f64,
    pub predicted_parent: Bounds,
    pub predicted_items: Vec<PredictedLayoutItem>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutInferenceReport {
    pub schema_version: u32,
    pub profile: String,
    pub evidence: EvidenceClass,
    pub provenance: InferenceProvenance,
    pub selection_basis: String,
    pub training_viewports: Vec<f64>,
    pub heldout_viewport: f64,
    pub selected: LayoutCandidateKind,
    pub candidates: Vec<LayoutCandidateReport>,
    pub selected_heldout_error: f64,
    pub freeform_heldout_error: f64,
}

impl LayoutInferenceReport {
    #[must_use]
    pub fn beats_freeform_on_heldout(&self) -> bool {
        self.selected_heldout_error < self.freeform_heldout_error
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum LayoutInferenceError {
    #[error("layout inference requires between 2 and {MAX_LAYOUT_SNAPSHOTS} training snapshots")]
    SnapshotCount,
    #[error("layout inference snapshots must contain the same bounded ordered item identities")]
    ItemIdentity,
    #[error("layout inference viewport and geometry values must be finite and bounded")]
    Geometry,
    #[error("layout inference provenance must reference at least one valid observation identity")]
    Provenance,
}

#[derive(Clone, Copy)]
enum PredictionModel {
    FixedOffsets,
    LinearConstraints,
}

/// Ranks bounded geometric layout-family hypotheses without promoting inferred
/// semantics to source-lossless evidence.
///
/// Selection uses only the training snapshots. The held-out snapshot is used
/// after selection to report falsification error and never changes the rank.
/// Stack, Flex, and Grid predictions keep the first observed item sizes and
/// gaps because pixels alone do not identify fill, intrinsic, or fractional
/// sizing. The constraint candidate linearly extrapolates observed geometry;
/// that is an explicit baseline rather than proof of the authored rule.
///
/// # Errors
///
/// Rejects missing or excessive snapshots/items, identity drift, duplicate or
/// non-increasing training viewports, invalid geometry, and empty provenance.
pub fn infer_layout(
    training: &[LayoutSnapshot],
    heldout: &LayoutSnapshot,
    observations: BTreeSet<ObservationId>,
) -> Result<LayoutInferenceReport, LayoutInferenceError> {
    validate_inputs(training, heldout, &observations)?;
    let first = &training[0];
    let last = &training[training.len() - 1];
    let mut candidates = LayoutCandidateKind::all()
        .into_iter()
        .map(|kind| {
            let model = kind.model();
            let (predicted_parent, predicted_items) =
                predict(first, last, heldout.viewport_width, model);
            let training_error = training
                .iter()
                .skip(1)
                .map(|snapshot| {
                    let (parent, items) = predict(first, last, snapshot.viewport_width, model);
                    normalized_error(snapshot, parent, &items)
                })
                .sum::<f64>()
                / bounded_f64(training.len() - 1);
            let structural_penalty = structural_penalty(kind, training);
            let complexity_penalty = complexity_penalty(kind, first.items.len());
            LayoutCandidateReport {
                kind,
                prediction_model: kind.model_name().to_owned(),
                training_score: training_error * 10.0 + structural_penalty + complexity_penalty,
                structural_penalty,
                complexity_penalty,
                heldout_error: normalized_error(heldout, predicted_parent, &predicted_items),
                predicted_parent,
                predicted_items,
            }
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        left.training_score
            .total_cmp(&right.training_score)
            .then_with(|| left.kind.cmp(&right.kind))
    });
    let selected = candidates[0].kind;
    let best = candidates[0].training_score;
    let second = candidates[1].training_score;
    let raw_confidence = if second <= f64::EPSILON {
        0.0
    } else {
        ((second - best) / second).clamp(0.0, 1.0)
    };
    let selected_heldout_error = candidates[0].heldout_error;
    let freeform_heldout_error = candidates
        .iter()
        .find(|candidate| candidate.kind == LayoutCandidateKind::Freeform)
        .map_or(f64::INFINITY, |candidate| candidate.heldout_error);
    Ok(LayoutInferenceReport {
        schema_version: 1,
        profile: LAYOUT_INFERENCE_PROFILE.to_owned(),
        evidence: EvidenceClass::Inferred,
        provenance: InferenceProvenance {
            method: "geometric-layout-candidate-ranking".to_owned(),
            artifact: None,
            observations,
            confidence: Confidence::raw(raw_confidence),
        },
        selection_basis: "training_score_only".to_owned(),
        training_viewports: training
            .iter()
            .map(|snapshot| snapshot.viewport_width)
            .collect(),
        heldout_viewport: heldout.viewport_width,
        selected,
        candidates,
        selected_heldout_error,
        freeform_heldout_error,
    })
}

impl LayoutCandidateKind {
    const fn all() -> [Self; 5] {
        [
            Self::StackRow,
            Self::StackColumn,
            Self::Grid,
            Self::Constraint,
            Self::Freeform,
        ]
    }

    const fn model(self) -> PredictionModel {
        match self {
            Self::Constraint => PredictionModel::LinearConstraints,
            Self::StackRow | Self::StackColumn | Self::Grid | Self::Freeform => {
                PredictionModel::FixedOffsets
            }
        }
    }

    const fn model_name(self) -> &'static str {
        match self {
            Self::Constraint => "linear_geometry_trajectory",
            Self::StackRow => "fixed_row_items_and_gaps",
            Self::StackColumn => "fixed_column_items_and_gaps",
            Self::Grid => "fixed_grid_tracks_and_gaps",
            Self::Freeform => "fixed_parent_relative_bounds",
        }
    }
}

fn validate_inputs(
    training: &[LayoutSnapshot],
    heldout: &LayoutSnapshot,
    observations: &BTreeSet<ObservationId>,
) -> Result<(), LayoutInferenceError> {
    if !(2..=MAX_LAYOUT_SNAPSHOTS).contains(&training.len()) {
        return Err(LayoutInferenceError::SnapshotCount);
    }
    if observations.is_empty()
        || observations
            .iter()
            .any(|observation| !nuif_core::is_identifier(&observation.0))
    {
        return Err(LayoutInferenceError::Provenance);
    }
    let expected = training[0]
        .items
        .iter()
        .map(|item| item.id.as_str())
        .collect::<Vec<_>>();
    if expected.is_empty()
        || expected.len() > MAX_LAYOUT_ITEMS
        || expected.iter().any(|id| !nuif_core::is_identifier(id))
        || expected.iter().collect::<BTreeSet<_>>().len() != expected.len()
    {
        return Err(LayoutInferenceError::ItemIdentity);
    }
    let mut previous_viewport = 0.0;
    for snapshot in training {
        if snapshot.viewport_width <= previous_viewport {
            return Err(LayoutInferenceError::Geometry);
        }
        previous_viewport = snapshot.viewport_width;
        validate_snapshot(snapshot, &expected)?;
    }
    validate_snapshot(heldout, &expected)
}

fn validate_snapshot(
    snapshot: &LayoutSnapshot,
    expected: &[&str],
) -> Result<(), LayoutInferenceError> {
    if snapshot.items.len() != expected.len()
        || snapshot
            .items
            .iter()
            .zip(expected)
            .any(|(item, expected)| item.id != *expected)
    {
        return Err(LayoutInferenceError::ItemIdentity);
    }
    if !snapshot.viewport_width.is_finite()
        || snapshot.viewport_width <= 0.0
        || snapshot.viewport_width > 1_000_000.0
        || !valid_bounds(snapshot.parent)
        || snapshot.items.iter().any(|item| !valid_bounds(item.bounds))
    {
        Err(LayoutInferenceError::Geometry)
    } else {
        Ok(())
    }
}

fn valid_bounds(bounds: Bounds) -> bool {
    [bounds.x, bounds.y, bounds.width, bounds.height]
        .into_iter()
        .all(|value| value.is_finite() && value.abs() <= 1_000_000.0)
        && bounds.width >= 0.0
        && bounds.height >= 0.0
}

fn predict(
    first: &LayoutSnapshot,
    last: &LayoutSnapshot,
    viewport_width: f64,
    model: PredictionModel,
) -> (Bounds, Vec<PredictedLayoutItem>) {
    let ratio =
        (viewport_width - first.viewport_width) / (last.viewport_width - first.viewport_width);
    let parent = interpolate_bounds(first.parent, last.parent, ratio);
    let items = first
        .items
        .iter()
        .zip(&last.items)
        .map(|(first_item, last_item)| {
            let bounds = match model {
                PredictionModel::LinearConstraints => {
                    interpolate_bounds(first_item.bounds, last_item.bounds, ratio)
                }
                PredictionModel::FixedOffsets => Bounds {
                    x: parent.x + first_item.bounds.x - first.parent.x,
                    y: parent.y + first_item.bounds.y - first.parent.y,
                    width: first_item.bounds.width,
                    height: first_item.bounds.height,
                },
            };
            PredictedLayoutItem {
                id: first_item.id.clone(),
                bounds,
            }
        })
        .collect();
    (parent, items)
}

fn interpolate_bounds(first: Bounds, last: Bounds, ratio: f64) -> Bounds {
    Bounds {
        x: interpolate(first.x, last.x, ratio),
        y: interpolate(first.y, last.y, ratio),
        width: interpolate(first.width, last.width, ratio),
        height: interpolate(first.height, last.height, ratio),
    }
}

fn interpolate(first: f64, last: f64, ratio: f64) -> f64 {
    first + ratio * (last - first)
}

fn normalized_error(
    actual: &LayoutSnapshot,
    predicted_parent: Bounds,
    predicted_items: &[PredictedLayoutItem],
) -> f64 {
    let scale = (actual.parent.width + actual.parent.height).max(1.0);
    let parent_error = bounds_error(actual.parent, predicted_parent) / scale;
    let item_error = actual
        .items
        .iter()
        .zip(predicted_items)
        .map(|(actual, predicted)| bounds_error(actual.bounds, predicted.bounds) / scale)
        .sum::<f64>();
    (parent_error + item_error) / bounded_f64(actual.items.len() + 1)
}

fn bounds_error(left: Bounds, right: Bounds) -> f64 {
    (left.x - right.x).abs()
        + (left.y - right.y).abs()
        + (left.width - right.width).abs()
        + (left.height - right.height).abs()
}

fn complexity_penalty(kind: LayoutCandidateKind, items: usize) -> f64 {
    let items = bounded_f64(items);
    match kind {
        LayoutCandidateKind::StackRow | LayoutCandidateKind::StackColumn => 0.005 * items,
        LayoutCandidateKind::Grid => 0.008 * items,
        LayoutCandidateKind::Freeform => 0.02 * items,
        LayoutCandidateKind::Constraint => 0.04 * items * 4.0 + 0.05,
    }
}

fn structural_penalty(kind: LayoutCandidateKind, snapshots: &[LayoutSnapshot]) -> f64 {
    match kind {
        LayoutCandidateKind::StackRow => {
            snapshots
                .iter()
                .map(|snapshot| axis_penalty(snapshot, true))
                .sum::<f64>()
                / bounded_f64(snapshots.len())
        }
        LayoutCandidateKind::StackColumn => {
            snapshots
                .iter()
                .map(|snapshot| axis_penalty(snapshot, false))
                .sum::<f64>()
                / bounded_f64(snapshots.len())
        }
        LayoutCandidateKind::Grid => {
            snapshots.iter().map(grid_penalty).sum::<f64>() / bounded_f64(snapshots.len())
        }
        LayoutCandidateKind::Constraint | LayoutCandidateKind::Freeform => 0.0,
    }
}

fn axis_penalty(snapshot: &LayoutSnapshot, row: bool) -> f64 {
    if snapshot.items.len() < 2 {
        return 1.0;
    }
    let scale = if row {
        snapshot.parent.width.max(1.0)
    } else {
        snapshot.parent.height.max(1.0)
    };
    let cross_scale = if row {
        snapshot.parent.height.max(1.0)
    } else {
        snapshot.parent.width.max(1.0)
    };
    let mut order_overlap = 0.0;
    let mut gaps = Vec::new();
    for pair in snapshot.items.windows(2) {
        let first_end = if row {
            pair[0].bounds.x + pair[0].bounds.width
        } else {
            pair[0].bounds.y + pair[0].bounds.height
        };
        let second_start = if row {
            pair[1].bounds.x
        } else {
            pair[1].bounds.y
        };
        let gap = second_start - first_end;
        if gap < 0.0 {
            order_overlap += -gap / scale;
        }
        gaps.push(gap);
    }
    let cross_centers = snapshot
        .items
        .iter()
        .map(|item| {
            if row {
                item.bounds.y + item.bounds.height / 2.0
            } else {
                item.bounds.x + item.bounds.width / 2.0
            }
        })
        .collect::<Vec<_>>();
    let cross_spread = range(&cross_centers) / cross_scale;
    let gap_spread = range(&gaps) / scale;
    (order_overlap + cross_spread + gap_spread).min(4.0)
}

fn grid_penalty(snapshot: &LayoutSnapshot) -> f64 {
    if snapshot.items.len() < 4 {
        return 1.0;
    }
    let x = clusters(
        snapshot
            .items
            .iter()
            .map(|item| item.bounds.x + item.bounds.width / 2.0),
    );
    let y = clusters(
        snapshot
            .items
            .iter()
            .map(|item| item.bounds.y + item.bounds.height / 2.0),
    );
    if x < 2 || y < 2 {
        return 1.0;
    }
    let capacity = x.saturating_mul(y);
    bounded_f64(capacity.saturating_sub(snapshot.items.len())) / bounded_f64(capacity)
}

fn clusters(values: impl Iterator<Item = f64>) -> usize {
    let mut values = values.collect::<Vec<_>>();
    values.sort_by(f64::total_cmp);
    let mut clusters = 0;
    let mut previous = None;
    for value in values {
        if previous.is_none_or(|previous| value - previous > 1.0) {
            clusters += 1;
            previous = Some(value);
        }
    }
    clusters
}

fn range(values: &[f64]) -> f64 {
    let Some(first) = values.first() else {
        return 0.0;
    };
    let (minimum, maximum) = values
        .iter()
        .skip(1)
        .fold((*first, *first), |(minimum, maximum), value| {
            (minimum.min(*value), maximum.max(*value))
        });
    maximum - minimum
}

fn bounded_f64(value: usize) -> f64 {
    u32::try_from(value).map_or(f64::from(u32::MAX), f64::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, x: f64, y: f64, width: f64, height: f64) -> LayoutItemObservation {
        LayoutItemObservation {
            id: id.to_owned(),
            bounds: Bounds {
                x,
                y,
                width,
                height,
            },
        }
    }

    fn snapshot(viewport_width: f64, item_x: [f64; 2]) -> LayoutSnapshot {
        LayoutSnapshot {
            viewport_width,
            parent: Bounds {
                x: 0.0,
                y: 0.0,
                width: viewport_width,
                height: 200.0,
            },
            items: vec![
                item("first", item_x[0], 20.0, 40.0, 40.0),
                item("second", item_x[1], 20.0, 40.0, 40.0),
            ],
        }
    }

    fn evidence() -> BTreeSet<ObservationId> {
        BTreeSet::from([ObservationId("capture-node-1-geometry".to_owned())])
    }

    #[test]
    fn responsive_constraint_candidate_beats_fixed_freeform() {
        let training = [
            snapshot(200.0, [20.0, 140.0]),
            snapshot(400.0, [40.0, 320.0]),
        ];
        let heldout = snapshot(500.0, [50.0, 410.0]);
        let report = infer_layout(&training, &heldout, evidence()).unwrap();
        assert_eq!(report.selected, LayoutCandidateKind::Constraint);
        assert!(report.beats_freeform_on_heldout());
        assert_eq!(report.evidence, EvidenceClass::Inferred);
        assert_eq!(report.provenance.confidence.calibrated, None);
        assert_eq!(report.candidates.len(), 5);
    }

    #[test]
    fn stable_regular_row_prefers_stack_over_node_coordinates() {
        let training = [snapshot(200.0, [20.0, 80.0]), snapshot(400.0, [20.0, 80.0])];
        let heldout = snapshot(500.0, [20.0, 80.0]);
        let report = infer_layout(&training, &heldout, evidence()).unwrap();
        assert_eq!(report.selected, LayoutCandidateKind::StackRow);
        assert!(report.selected_heldout_error.abs() < f64::EPSILON);
    }

    #[test]
    fn identity_drift_and_empty_provenance_fail_closed() {
        let training = [snapshot(200.0, [20.0, 80.0]), snapshot(400.0, [20.0, 80.0])];
        let mut heldout = snapshot(500.0, [20.0, 80.0]);
        heldout.items[1].id = "replacement".to_owned();
        assert_eq!(
            infer_layout(&training, &heldout, evidence()),
            Err(LayoutInferenceError::ItemIdentity)
        );
        let heldout = snapshot(500.0, [20.0, 80.0]);
        assert_eq!(
            infer_layout(&training, &heldout, BTreeSet::new()),
            Err(LayoutInferenceError::Provenance)
        );
    }

    #[test]
    fn candidate_report_reaches_a_canonical_record_fixpoint() {
        let training = [
            snapshot(200.0, [20.0, 140.0]),
            snapshot(400.0, [40.0, 320.0]),
        ];
        let heldout = snapshot(500.0, [50.0, 410.0]);
        let report = infer_layout(&training, &heldout, evidence()).unwrap();
        let bytes = nuif_codec::encode_canonical_record(&report).unwrap();
        let decoded: LayoutInferenceReport = nuif_codec::decode_canonical_record(&bytes).unwrap();
        assert_eq!(decoded, report);
    }
}
