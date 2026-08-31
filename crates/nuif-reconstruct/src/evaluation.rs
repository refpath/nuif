use crate::{Bounds, ReconstructionError};
use nuif_core::{ResourceDigest, is_identifier};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

pub const EVALUATION_PROFILE: &str = "nuif-reconstruction-evaluation-0";
pub const MAX_EVALUATION_ITEMS: usize = 100_000;
pub const MAX_EDIT_DISTANCE_CELLS: usize = 16 * 1024 * 1024;
pub const MAX_PERCEPTUAL_DIAGNOSTICS: usize = 64;
pub const MAX_CALIBRATION_BINS: usize = 100;
pub const MAX_PIXEL_COUNT: usize = 16 * 1024 * 1024;
const MAX_CALIBRATION_BINS_U32: u32 = 100;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationSuite {
    SyntheticExact,
    RealScreenshot,
    SourceBacked,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RateMetric {
    pub numerator: u64,
    pub denominator: u64,
    pub value: Option<f64>,
}

impl RateMetric {
    /// Constructs a rate while retaining its exact integer evidence.
    ///
    /// A zero denominator is reported as `None`, never as a perfect or failed
    /// score.
    #[must_use]
    pub fn new(numerator: u64, denominator: u64) -> Self {
        Self {
            numerator,
            denominator,
            value: (denominator != 0).then(|| ratio(numerator, denominator)),
        }
    }

    fn validate(self) -> Result<(), EvaluationError> {
        if self.numerator > self.denominator
            || self.denominator > MAX_EVALUATION_ITEMS as u64
            || self.value.is_some() != (self.denominator != 0)
            || self
                .value
                .is_some_and(|value| !probability(value) || !same(value, self.expected_value()))
        {
            Err(EvaluationError::InvalidMetric("rate"))
        } else {
            Ok(())
        }
    }

    fn expected_value(self) -> f64 {
        if self.denominator == 0 {
            0.0
        } else {
            ratio(self.numerator, self.denominator)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetectionMetrics {
    pub true_positive: u64,
    pub false_positive: u64,
    pub false_negative: u64,
    pub precision: RateMetric,
    pub recall: RateMetric,
    pub f1: Option<f64>,
}

impl DetectionMetrics {
    #[must_use]
    pub fn new(true_positive: u64, false_positive: u64, false_negative: u64) -> Self {
        let precision =
            RateMetric::new(true_positive, true_positive.saturating_add(false_positive));
        let recall = RateMetric::new(true_positive, true_positive.saturating_add(false_negative));
        let f1 = match (precision.value, recall.value) {
            (Some(precision), Some(recall)) if precision + recall > 0.0 => {
                Some(2.0 * precision * recall / (precision + recall))
            }
            (Some(_), Some(_)) => Some(0.0),
            _ => None,
        };
        Self {
            true_positive,
            false_positive,
            false_negative,
            precision,
            recall,
            f1,
        }
    }

    fn validate(self) -> Result<(), EvaluationError> {
        self.precision.validate()?;
        self.recall.validate()?;
        let expected = Self::new(self.true_positive, self.false_positive, self.false_negative);
        if self.true_positive > MAX_EVALUATION_ITEMS as u64
            || self.false_positive > MAX_EVALUATION_ITEMS as u64
            || self.false_negative > MAX_EVALUATION_ITEMS as u64
            || self.precision != expected.precision
            || self.recall != expected.recall
            || !optional_same(self.f1, expected.f1)
        {
            Err(EvaluationError::InvalidMetric("detection"))
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditMetrics {
    pub distance: u64,
    pub reference_units: u64,
    pub normalized_error: Option<f64>,
}

impl EditMetrics {
    #[must_use]
    pub fn new(distance: u64, reference_units: u64) -> Self {
        Self {
            distance,
            reference_units,
            normalized_error: (reference_units != 0).then(|| ratio(distance, reference_units)),
        }
    }

    fn validate(self) -> Result<(), EvaluationError> {
        let expected = Self::new(self.distance, self.reference_units);
        if self.distance > MAX_EVALUATION_ITEMS as u64
            || self.reference_units > MAX_EVALUATION_ITEMS as u64
            || !optional_same(self.normalized_error, expected.normalized_error)
        {
            Err(EvaluationError::InvalidMetric("edit distance"))
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorMetrics {
    pub samples: u64,
    pub absolute_error_sum: f64,
    pub mean_absolute_error: Option<f64>,
}

impl ErrorMetrics {
    #[must_use]
    pub fn new(samples: u64, absolute_error_sum: f64) -> Self {
        Self {
            samples,
            absolute_error_sum,
            mean_absolute_error: (samples != 0)
                .then(|| absolute_error_sum / bounded_count_f64(samples)),
        }
    }

    fn validate(self, name: &'static str) -> Result<(), EvaluationError> {
        let expected = Self::new(self.samples, self.absolute_error_sum);
        if self.samples > MAX_EVALUATION_ITEMS as u64
            || !self.absolute_error_sum.is_finite()
            || self.absolute_error_sum < 0.0
            || (self.samples == 0 && self.absolute_error_sum != 0.0)
            || !optional_same(self.mean_absolute_error, expected.mean_absolute_error)
        {
            Err(EvaluationError::InvalidMetric(name))
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextMetrics {
    pub regions: DetectionMetrics,
    pub characters: EditMetrics,
    pub words: EditMetrics,
    pub baseline_geometry: ErrorMetrics,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeMetrics {
    pub parents: RateMetric,
    pub sibling_pairs: RateMetric,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropertyMetrics {
    pub exact: RateMetric,
    pub numeric: ErrorMetrics,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeometryMetrics {
    pub matched_regions: u64,
    pub mean_iou: Option<f64>,
    pub normalized_absolute_error: ErrorMetrics,
}

impl GeometryMetrics {
    /// Aggregates matched rectangle comparisons. Normalization uses the
    /// reference width and height for position and size components.
    ///
    /// # Errors
    ///
    /// Rejects invalid bounds, mismatched inputs, or excessive item counts.
    pub fn compare(references: &[Bounds], candidates: &[Bounds]) -> Result<Self, EvaluationError> {
        if references.len() != candidates.len() {
            return Err(EvaluationError::MismatchedInputs);
        }
        check_items(references.len(), "geometry pairs")?;
        let mut iou_sum = 0.0;
        let mut normalized_error = 0.0;
        for (reference, candidate) in references.iter().zip(candidates) {
            validate_bounds(*reference)?;
            validate_bounds(*candidate)?;
            if reference.width == 0.0 || reference.height == 0.0 {
                return Err(EvaluationError::InvalidBounds);
            }
            iou_sum += bounds_iou(*reference, *candidate)?;
            normalized_error += (reference.x - candidate.x).abs() / reference.width
                + (reference.y - candidate.y).abs() / reference.height
                + (reference.width - candidate.width).abs() / reference.width
                + (reference.height - candidate.height).abs() / reference.height;
        }
        let matched_regions = references.len();
        Ok(Self {
            matched_regions: u64::try_from(matched_regions).unwrap_or(u64::MAX),
            mean_iou: (matched_regions != 0).then(|| iou_sum / bounded_usize_f64(matched_regions)),
            normalized_absolute_error: ErrorMetrics::new(
                u64::try_from(matched_regions.saturating_mul(4)).unwrap_or(u64::MAX),
                normalized_error,
            ),
        })
    }

    fn validate(&self) -> Result<(), EvaluationError> {
        if self.matched_regions > MAX_EVALUATION_ITEMS as u64
            || self.mean_iou.is_some() != (self.matched_regions != 0)
            || self.mean_iou.is_some_and(|value| !probability(value))
            || self.normalized_absolute_error.samples != self.matched_regions.saturating_mul(4)
        {
            return Err(EvaluationError::InvalidMetric("geometry"));
        }
        self.normalized_absolute_error
            .validate("geometry normalized error")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PixelMetrics {
    pub width: u32,
    pub height: u32,
    pub pixels: u64,
    pub differing_pixels: u64,
    pub maximum_channel_delta: u8,
    pub mean_absolute_channel_error: f64,
    pub exact_pixel_rate: RateMetric,
}

impl PixelMetrics {
    /// Compares exact unpremultiplied RGBA8 buffers.
    ///
    /// # Errors
    ///
    /// Rejects zero, excessive, overflowed, or mismatched buffers.
    pub fn compare(
        width: u32,
        height: u32,
        reference: &[u8],
        candidate: &[u8],
    ) -> Result<Self, EvaluationError> {
        let pixels = usize::try_from(width)
            .ok()
            .and_then(|width| {
                usize::try_from(height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .ok_or(EvaluationError::ResourceLimit("pixels"))?;
        if pixels == 0 || pixels > MAX_PIXEL_COUNT {
            return Err(EvaluationError::ResourceLimit("pixels"));
        }
        let expected = pixels
            .checked_mul(4)
            .ok_or(EvaluationError::ResourceLimit("pixel bytes"))?;
        if reference.len() != expected || candidate.len() != expected {
            return Err(EvaluationError::MismatchedInputs);
        }
        let mut differing_pixels = 0_usize;
        let mut maximum_channel_delta = 0_u8;
        let mut absolute_channel_error = 0_u64;
        let (reference_pixels, reference_remainder) = reference.as_chunks::<4>();
        let (candidate_pixels, candidate_remainder) = candidate.as_chunks::<4>();
        debug_assert!(reference_remainder.is_empty() && candidate_remainder.is_empty());
        for (reference, candidate) in reference_pixels.iter().zip(candidate_pixels) {
            let mut differs = false;
            for (&reference, &candidate) in reference.iter().zip(candidate) {
                let delta = reference.abs_diff(candidate);
                differs |= delta != 0;
                maximum_channel_delta = maximum_channel_delta.max(delta);
                absolute_channel_error = absolute_channel_error.saturating_add(u64::from(delta));
            }
            differing_pixels += usize::from(differs);
        }
        Ok(Self {
            width,
            height,
            pixels: u64::try_from(pixels).unwrap_or(u64::MAX),
            differing_pixels: u64::try_from(differing_pixels).unwrap_or(u64::MAX),
            maximum_channel_delta,
            mean_absolute_channel_error: exact_u64_f64(absolute_channel_error)
                / bounded_usize_f64(expected),
            exact_pixel_rate: RateMetric::new(
                u64::try_from(pixels - differing_pixels).unwrap_or(u64::MAX),
                u64::try_from(pixels).unwrap_or(u64::MAX),
            ),
        })
    }

    fn validate(self) -> Result<(), EvaluationError> {
        let pixels = u64::from(self.width)
            .checked_mul(u64::from(self.height))
            .ok_or(EvaluationError::InvalidMetric("pixels"))?;
        self.exact_pixel_rate.validate()?;
        if pixels == 0
            || pixels > MAX_PIXEL_COUNT as u64
            || self.pixels != pixels
            || self.differing_pixels > pixels
            || self.exact_pixel_rate.numerator != pixels - self.differing_pixels
            || self.exact_pixel_rate.denominator != pixels
            || !self.mean_absolute_channel_error.is_finite()
            || !(0.0..=255.0).contains(&self.mean_absolute_channel_error)
            || (self.differing_pixels == 0
                && (self.maximum_channel_delta != 0 || self.mean_absolute_channel_error != 0.0))
            || (self.differing_pixels != 0
                && (self.maximum_channel_delta == 0 || self.mean_absolute_channel_error == 0.0))
        {
            Err(EvaluationError::InvalidMetric("pixels"))
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PerceptualDiagnostic {
    pub method: String,
    pub value: f64,
    pub lower_is_better: bool,
    pub artifact: Option<ResourceDigest>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationSample {
    pub confidence: f64,
    pub correct: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectivePoint {
    pub threshold: f64,
    pub coverage: f64,
    pub risk: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationMetrics {
    pub samples: u64,
    pub bins: u32,
    pub expected_calibration_error: Option<f64>,
    pub brier_score: Option<f64>,
    pub selective: Vec<SelectivePoint>,
}

impl CalibrationMetrics {
    /// Computes equal-width ECE, Brier score, and risk/coverage points.
    ///
    /// # Errors
    ///
    /// Rejects invalid confidence, bin, threshold, order, or item bounds.
    pub fn evaluate(
        samples: &[CalibrationSample],
        bins: usize,
        thresholds: &[f64],
    ) -> Result<Self, EvaluationError> {
        check_items(samples.len(), "calibration samples")?;
        if bins == 0
            || bins > MAX_CALIBRATION_BINS
            || thresholds.len() > MAX_CALIBRATION_BINS
            || samples.iter().any(|sample| !probability(sample.confidence))
            || thresholds.iter().any(|threshold| !probability(*threshold))
            || !thresholds.windows(2).all(|pair| pair[0] < pair[1])
        {
            return Err(EvaluationError::InvalidMetric("calibration input"));
        }
        let bins_u32 =
            u32::try_from(bins).map_err(|_| EvaluationError::InvalidMetric("calibration input"))?;
        if samples.is_empty() {
            return Ok(Self {
                samples: 0,
                bins: bins_u32,
                expected_calibration_error: None,
                brier_score: None,
                selective: thresholds
                    .iter()
                    .map(|threshold| SelectivePoint {
                        threshold: *threshold,
                        coverage: 0.0,
                        risk: None,
                    })
                    .collect(),
            });
        }
        let mut bin_counts = vec![0_u64; bins];
        let mut bin_confidence = vec![0.0_f64; bins];
        let mut bin_correct = vec![0_u64; bins];
        let mut brier = 0.0;
        for sample in samples {
            let bin = calibration_bin(sample.confidence, bins_u32);
            bin_counts[bin] += 1;
            bin_confidence[bin] += sample.confidence;
            bin_correct[bin] += u64::from(sample.correct);
            let target = f64::from(sample.correct);
            brier += (sample.confidence - target).powi(2);
        }
        let count = bounded_usize_f64(samples.len());
        let expected_calibration_error = bin_counts
            .iter()
            .zip(bin_confidence)
            .zip(bin_correct)
            .filter(|((bin_count, _), _)| **bin_count != 0)
            .map(|((bin_count, confidence), correct)| {
                let bin_count = bounded_count_f64(*bin_count);
                (bin_count / count)
                    * (confidence / bin_count - bounded_count_f64(correct) / bin_count).abs()
            })
            .sum();
        let selective = thresholds
            .iter()
            .map(|threshold| {
                let selected = samples
                    .iter()
                    .filter(|sample| sample.confidence >= *threshold)
                    .collect::<Vec<_>>();
                let errors = selected.iter().filter(|sample| !sample.correct).count();
                SelectivePoint {
                    threshold: *threshold,
                    coverage: bounded_usize_f64(selected.len()) / count,
                    risk: (!selected.is_empty())
                        .then(|| bounded_usize_f64(errors) / bounded_usize_f64(selected.len())),
                }
            })
            .collect();
        Ok(Self {
            samples: u64::try_from(samples.len()).unwrap_or(u64::MAX),
            bins: bins_u32,
            expected_calibration_error: Some(expected_calibration_error),
            brier_score: Some(brier / count),
            selective,
        })
    }

    fn validate(&self) -> Result<(), EvaluationError> {
        if self.samples > MAX_EVALUATION_ITEMS as u64
            || self.bins == 0
            || self.bins > MAX_CALIBRATION_BINS_U32
            || self.selective.len() > MAX_CALIBRATION_BINS
            || self.expected_calibration_error.is_some() != (self.samples != 0)
            || self.brier_score.is_some() != (self.samples != 0)
            || self
                .expected_calibration_error
                .is_some_and(|value| !probability(value))
            || self.brier_score.is_some_and(|value| !probability(value))
            || !self
                .selective
                .windows(2)
                .all(|pair| pair[0].threshold < pair[1].threshold)
            || !self
                .selective
                .windows(2)
                .all(|pair| pair[0].coverage >= pair[1].coverage)
            || self.selective.iter().any(|point| {
                !probability(point.threshold)
                    || !probability(point.coverage)
                    || point.risk.is_some_and(|risk| !probability(risk))
                    || point.risk.is_some() != (point.coverage != 0.0)
                    || (self.samples == 0 && (point.coverage != 0.0 || point.risk.is_some()))
            })
        {
            Err(EvaluationError::InvalidMetric("calibration"))
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CostMetrics {
    pub latency_microseconds: Option<u64>,
    pub peak_ram_bytes: Option<u64>,
    pub peak_vram_bytes: Option<u64>,
    pub iterations: u32,
    pub external_cost_microunits: u64,
    pub currency: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationReport {
    pub schema_version: u32,
    pub profile: String,
    pub example_id: String,
    pub suite: EvaluationSuite,
    pub validity: RateMetric,
    pub text: TextMetrics,
    pub elements: DetectionMetrics,
    pub tree: TreeMetrics,
    pub properties: PropertyMetrics,
    pub geometry: GeometryMetrics,
    pub held_out_layout: Option<GeometryMetrics>,
    pub resources: RateMetric,
    pub provenance_honesty: RateMetric,
    pub accessibility: Option<RateMetric>,
    pub pixels: PixelMetrics,
    pub perceptual_diagnostics: Vec<PerceptualDiagnostic>,
    pub confidence: CalibrationMetrics,
    pub cost: CostMetrics,
}

impl EvaluationReport {
    /// Validates a complete per-example editable-reconstruction report.
    ///
    /// # Errors
    ///
    /// Rejects missing families, derived-rate drift, invalid identities,
    /// non-finite metrics, excessive records, and source-byte recall claims in
    /// a screenshot-only suite.
    pub fn validate(&self) -> Result<(), EvaluationError> {
        if self.schema_version != 1
            || self.profile != EVALUATION_PROFILE
            || !is_identifier(&self.example_id)
        {
            return Err(EvaluationError::InvalidReport(
                "schema, profile, or example identity is invalid",
            ));
        }
        self.validity.validate()?;
        self.text.regions.validate()?;
        self.text.characters.validate()?;
        self.text.words.validate()?;
        self.text
            .baseline_geometry
            .validate("text baseline geometry")?;
        self.elements.validate()?;
        self.tree.parents.validate()?;
        self.tree.sibling_pairs.validate()?;
        self.properties.exact.validate()?;
        self.properties.numeric.validate("numeric properties")?;
        self.geometry.validate()?;
        if let Some(held_out) = &self.held_out_layout {
            held_out.validate()?;
        }
        self.resources.validate()?;
        if self.suite == EvaluationSuite::RealScreenshot && self.resources.denominator != 0 {
            return Err(EvaluationError::InvalidReport(
                "screenshot-only evaluation cannot claim exact source-resource recall",
            ));
        }
        self.provenance_honesty.validate()?;
        if let Some(accessibility) = self.accessibility {
            accessibility.validate()?;
        }
        self.pixels.validate()?;
        self.confidence.validate()?;
        if self.perceptual_diagnostics.len() > MAX_PERCEPTUAL_DIAGNOSTICS {
            return Err(EvaluationError::ResourceLimit("perceptual diagnostics"));
        }
        let mut methods = BTreeSet::new();
        for diagnostic in &self.perceptual_diagnostics {
            if !is_identifier(&diagnostic.method)
                || !diagnostic.value.is_finite()
                || !methods.insert(&diagnostic.method)
                || diagnostic
                    .artifact
                    .as_ref()
                    .is_some_and(|artifact| !artifact.is_valid())
            {
                return Err(EvaluationError::InvalidMetric("perceptual diagnostic"));
            }
        }
        if self.cost.currency.is_some() != (self.cost.external_cost_microunits != 0)
            || self
                .cost
                .currency
                .as_ref()
                .is_some_and(|currency| !is_identifier(currency))
        {
            return Err(EvaluationError::InvalidMetric("cost"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Error)]
pub enum EvaluationError {
    #[error("evaluation inputs have different shapes")]
    MismatchedInputs,
    #[error("evaluation bounds are non-finite, negative, or degenerate")]
    InvalidBounds,
    #[error("evaluation resource limit exceeded for {0}")]
    ResourceLimit(&'static str),
    #[error("evaluation metric is invalid or internally inconsistent: {0}")]
    InvalidMetric(&'static str),
    #[error("evaluation report is invalid: {0}")]
    InvalidReport(&'static str),
}

/// Computes Unicode-scalar Levenshtein distance with bounded memory and work.
///
/// # Errors
///
/// Rejects inputs beyond the item or edit-cell budgets.
pub fn character_error(reference: &str, candidate: &str) -> Result<EditMetrics, EvaluationError> {
    let reference = reference.chars().collect::<Vec<_>>();
    let candidate = candidate.chars().collect::<Vec<_>>();
    let distance = levenshtein(&reference, &candidate)?;
    Ok(EditMetrics::new(
        u64::try_from(distance).unwrap_or(u64::MAX),
        u64::try_from(reference.len()).unwrap_or(u64::MAX),
    ))
}

/// Computes whitespace-token word error with bounded memory and work.
///
/// # Errors
///
/// Rejects inputs beyond the item or edit-cell budgets.
pub fn word_error(reference: &str, candidate: &str) -> Result<EditMetrics, EvaluationError> {
    let reference = reference.split_whitespace().collect::<Vec<_>>();
    let candidate = candidate.split_whitespace().collect::<Vec<_>>();
    let distance = levenshtein(&reference, &candidate)?;
    Ok(EditMetrics::new(
        u64::try_from(distance).unwrap_or(u64::MAX),
        u64::try_from(reference.len()).unwrap_or(u64::MAX),
    ))
}

/// Computes axis-aligned rectangle intersection-over-union.
///
/// # Errors
///
/// Rejects non-finite, negative, or degenerate rectangles.
pub fn bounds_iou(reference: Bounds, candidate: Bounds) -> Result<f64, EvaluationError> {
    validate_bounds(reference)?;
    validate_bounds(candidate)?;
    if reference.width == 0.0
        || reference.height == 0.0
        || candidate.width == 0.0
        || candidate.height == 0.0
    {
        return Err(EvaluationError::InvalidBounds);
    }
    let intersection_width = (reference.x + reference.width).min(candidate.x + candidate.width)
        - reference.x.max(candidate.x);
    let intersection_height = (reference.y + reference.height).min(candidate.y + candidate.height)
        - reference.y.max(candidate.y);
    let intersection = intersection_width.max(0.0) * intersection_height.max(0.0);
    let union =
        reference.width * reference.height + candidate.width * candidate.height - intersection;
    Ok(intersection / union)
}

fn levenshtein<T: Eq>(reference: &[T], candidate: &[T]) -> Result<usize, EvaluationError> {
    check_items(reference.len(), "reference edit units")?;
    check_items(candidate.len(), "candidate edit units")?;
    let cells = reference
        .len()
        .saturating_add(1)
        .checked_mul(candidate.len().saturating_add(1))
        .ok_or(EvaluationError::ResourceLimit("edit distance cells"))?;
    if cells > MAX_EDIT_DISTANCE_CELLS {
        return Err(EvaluationError::ResourceLimit("edit distance cells"));
    }
    let (rows, columns) = if candidate.len() <= reference.len() {
        (reference, candidate)
    } else {
        (candidate, reference)
    };
    let mut previous = (0..=columns.len()).collect::<Vec<_>>();
    let mut current = vec![0_usize; columns.len() + 1];
    for (row_index, row) in rows.iter().enumerate() {
        current[0] = row_index + 1;
        for (column_index, column) in columns.iter().enumerate() {
            current[column_index + 1] = (previous[column_index + 1] + 1)
                .min(current[column_index] + 1)
                .min(previous[column_index] + usize::from(row != column));
        }
        std::mem::swap(&mut previous, &mut current);
    }
    Ok(previous[columns.len()])
}

fn validate_bounds(bounds: Bounds) -> Result<(), EvaluationError> {
    if [bounds.x, bounds.y, bounds.width, bounds.height]
        .into_iter()
        .all(f64::is_finite)
        && bounds.width >= 0.0
        && bounds.height >= 0.0
    {
        Ok(())
    } else {
        Err(EvaluationError::InvalidBounds)
    }
}

fn check_items(items: usize, resource: &'static str) -> Result<(), EvaluationError> {
    if items > MAX_EVALUATION_ITEMS {
        Err(EvaluationError::ResourceLimit(resource))
    } else {
        Ok(())
    }
}

fn ratio(numerator: u64, denominator: u64) -> f64 {
    bounded_count_f64(numerator) / bounded_count_f64(denominator)
}

fn bounded_count_f64(value: u64) -> f64 {
    u32::try_from(value).map_or(f64::INFINITY, f64::from)
}

fn bounded_usize_f64(value: usize) -> f64 {
    u32::try_from(value).map_or(f64::INFINITY, f64::from)
}

#[expect(
    clippy::cast_precision_loss,
    reason = "RGBA8 sum is bounded below 2^53 by MAX_PIXEL_COUNT"
)]
fn exact_u64_f64(value: u64) -> f64 {
    value as f64
}

fn calibration_bin(confidence: f64, bins: u32) -> usize {
    let scaled = confidence * f64::from(bins);
    for boundary in 1..bins {
        if scaled < f64::from(boundary) {
            return usize::try_from(boundary - 1).unwrap_or(usize::MAX);
        }
    }
    usize::try_from(bins - 1).unwrap_or(usize::MAX)
}

fn probability(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn same(left: f64, right: f64) -> bool {
    (left - right).abs() <= f64::EPSILON * left.abs().max(right.abs()).max(1.0) * 4.0
}

fn optional_same(left: Option<f64>, right: Option<f64>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => same(left, right),
        (None, None) => true,
        _ => false,
    }
}

impl From<EvaluationError> for ReconstructionError {
    fn from(error: EvaluationError) -> Self {
        Self::Evaluator(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds(x: f64, y: f64, width: f64, height: f64) -> Bounds {
        Bounds {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn empty_denominators_are_not_reported_as_perfect() {
        let rate = RateMetric::new(0, 0);
        let detection = DetectionMetrics::new(0, 0, 0);
        assert_eq!(rate.value, None);
        assert_eq!(detection.precision.value, None);
        assert_eq!(detection.recall.value, None);
        assert_eq!(detection.f1, None);
    }

    #[test]
    fn text_geometry_pixels_and_calibration_are_exact() {
        assert_eq!(character_error("NUIF", "NU1F").unwrap().distance, 1);
        assert_eq!(word_error("one two", "one three").unwrap().distance, 1);
        assert!(same(
            bounds_iou(bounds(0.0, 0.0, 10.0, 10.0), bounds(5.0, 0.0, 10.0, 10.0)).unwrap(),
            1.0 / 3.0
        ));
        let pixels = PixelMetrics::compare(
            2,
            1,
            &[0, 0, 0, 255, 0, 0, 0, 255],
            &[0, 0, 0, 255, 1, 0, 0, 255],
        )
        .unwrap();
        assert_eq!(pixels.differing_pixels, 1);
        assert_eq!(pixels.maximum_channel_delta, 1);
        assert_eq!(pixels.exact_pixel_rate, RateMetric::new(1, 2));

        let calibration = CalibrationMetrics::evaluate(
            &[
                CalibrationSample {
                    confidence: 0.9,
                    correct: true,
                },
                CalibrationSample {
                    confidence: 0.8,
                    correct: false,
                },
            ],
            2,
            &[0.5, 0.85],
        )
        .unwrap();
        assert!(same(calibration.brier_score.unwrap(), 0.325));
        assert!(same(calibration.selective[1].coverage, 0.5));
        assert_eq!(calibration.selective[1].risk, Some(0.0));
    }

    #[test]
    fn report_requires_every_family_and_honest_resource_scope() {
        let geometry = GeometryMetrics::compare(
            &[bounds(0.0, 0.0, 10.0, 10.0)],
            &[bounds(0.0, 0.0, 10.0, 10.0)],
        )
        .unwrap();
        let mut report = EvaluationReport {
            schema_version: 1,
            profile: EVALUATION_PROFILE.to_owned(),
            example_id: "example-1".to_owned(),
            suite: EvaluationSuite::SyntheticExact,
            validity: RateMetric::new(1, 1),
            text: TextMetrics {
                regions: DetectionMetrics::new(1, 0, 0),
                characters: EditMetrics::new(0, 4),
                words: EditMetrics::new(0, 1),
                baseline_geometry: ErrorMetrics::new(1, 0.0),
            },
            elements: DetectionMetrics::new(1, 0, 0),
            tree: TreeMetrics {
                parents: RateMetric::new(1, 1),
                sibling_pairs: RateMetric::new(0, 0),
            },
            properties: PropertyMetrics {
                exact: RateMetric::new(2, 2),
                numeric: ErrorMetrics::new(1, 0.0),
            },
            geometry,
            held_out_layout: None,
            resources: RateMetric::new(1, 1),
            provenance_honesty: RateMetric::new(1, 1),
            accessibility: Some(RateMetric::new(1, 1)),
            pixels: PixelMetrics::compare(1, 1, &[0, 0, 0, 255], &[0, 0, 0, 255]).unwrap(),
            perceptual_diagnostics: Vec::new(),
            confidence: CalibrationMetrics::evaluate(&[], 10, &[0.5]).unwrap(),
            cost: CostMetrics {
                latency_microseconds: Some(1),
                peak_ram_bytes: Some(1),
                peak_vram_bytes: None,
                iterations: 1,
                external_cost_microunits: 0,
                currency: None,
            },
        };
        report.validate().unwrap();
        report.suite = EvaluationSuite::RealScreenshot;
        assert!(matches!(
            report.validate(),
            Err(EvaluationError::InvalidReport(_))
        ));
    }

    #[test]
    fn edit_distance_and_pixels_enforce_work_before_allocation() {
        let large = "x".repeat(4_097);
        assert!(matches!(
            character_error(&large, &large),
            Err(EvaluationError::ResourceLimit("edit distance cells"))
        ));
        assert!(matches!(
            PixelMetrics::compare(2, 2, &[0; 4], &[0; 4]),
            Err(EvaluationError::MismatchedInputs)
        ));

        let inconsistent_error = ErrorMetrics {
            samples: 0,
            absolute_error_sum: 1.0,
            mean_absolute_error: None,
        };
        assert!(inconsistent_error.validate("fixture").is_err());

        let mut inconsistent_pixels =
            PixelMetrics::compare(1, 1, &[0, 0, 0, 255], &[0, 0, 0, 255]).unwrap();
        inconsistent_pixels.maximum_channel_delta = 1;
        assert!(inconsistent_pixels.validate().is_err());
    }
}
