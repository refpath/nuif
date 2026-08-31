use super::{
    EVALUATION_PROFILE, EvaluationError, EvaluationReport, EvaluationSuite, MAX_CALIBRATION_BINS,
    MAX_EVALUATION_ITEMS, RateMetric, exact_u64_f64, probability, same,
};
use nuif_core::{ResourceDigest, is_identifier};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const AGGREGATE_PROFILE: &str = "nuif-reconstruction-evaluation-aggregate-0";

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AggregateRate {
    pub numerator: u64,
    pub denominator: u64,
    pub value: Option<f64>,
}

impl AggregateRate {
    fn from_metrics(metrics: impl Iterator<Item = RateMetric>) -> Result<Self, AggregationError> {
        let mut numerator = 0_u64;
        let mut denominator = 0_u64;
        for metric in metrics {
            numerator = numerator
                .checked_add(metric.numerator)
                .ok_or(AggregationError::ArithmeticOverflow)?;
            denominator = denominator
                .checked_add(metric.denominator)
                .ok_or(AggregationError::ArithmeticOverflow)?;
        }
        Ok(Self {
            numerator,
            denominator,
            value: (denominator != 0)
                .then(|| exact_u64_f64(numerator) / exact_u64_f64(denominator)),
        })
    }

    fn validate(self) -> Result<(), AggregationError> {
        let expected = (self.denominator != 0)
            .then(|| exact_u64_f64(self.numerator) / exact_u64_f64(self.denominator));
        if self.numerator > self.denominator || !optional_same(self.value, expected) {
            Err(AggregationError::InvalidAggregate("micro rate"))
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScalarDistribution {
    pub total_examples: u64,
    pub scored_examples: u64,
    pub unscored_examples: u64,
    pub minimum: Option<f64>,
    pub p50_nearest_rank: Option<f64>,
    pub p95_nearest_rank: Option<f64>,
    pub maximum: Option<f64>,
    pub mean: Option<f64>,
}

impl ScalarDistribution {
    fn build(
        values: impl Iterator<Item = Option<f64>>,
        total: usize,
    ) -> Result<Self, AggregationError> {
        let mut values = values.flatten().collect::<Vec<_>>();
        if values.iter().any(|value| !value.is_finite()) {
            return Err(AggregationError::InvalidAggregate("non-finite scalar"));
        }
        values.sort_by(f64::total_cmp);
        let scored = values.len();
        let mean = if values.is_empty() {
            None
        } else {
            let sum = values.iter().sum::<f64>();
            let mean = sum / f64::from(u32::try_from(scored).unwrap_or(u32::MAX));
            if !mean.is_finite() {
                return Err(AggregationError::InvalidAggregate("scalar mean overflow"));
            }
            Some(mean)
        };
        Ok(Self {
            total_examples: u64::try_from(total).unwrap_or(u64::MAX),
            scored_examples: u64::try_from(scored).unwrap_or(u64::MAX),
            unscored_examples: u64::try_from(total.saturating_sub(scored)).unwrap_or(u64::MAX),
            minimum: values.first().copied(),
            p50_nearest_rank: nearest_rank(&values, 50),
            p95_nearest_rank: nearest_rank(&values, 95),
            maximum: values.last().copied(),
            mean,
        })
    }

    fn validate(&self, expected_total: u64) -> Result<(), AggregationError> {
        let present = [
            self.minimum,
            self.p50_nearest_rank,
            self.p95_nearest_rank,
            self.maximum,
            self.mean,
        ];
        if self.total_examples != expected_total
            || self.scored_examples.saturating_add(self.unscored_examples) != expected_total
            || present
                .iter()
                .any(|value| value.is_some() != (self.scored_examples != 0))
            || present
                .into_iter()
                .flatten()
                .any(|value| !value.is_finite())
        {
            return Err(AggregationError::InvalidAggregate("scalar distribution"));
        }
        if let (Some(minimum), Some(p50), Some(p95), Some(maximum), Some(mean)) = (
            self.minimum,
            self.p50_nearest_rank,
            self.p95_nearest_rank,
            self.maximum,
            self.mean,
        ) && !(less_or_same(minimum, p50)
            && less_or_same(p50, p95)
            && less_or_same(p95, maximum)
            && less_or_same(minimum, mean)
            && less_or_same(mean, maximum))
        {
            return Err(AggregationError::InvalidAggregate("scalar ordering"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntegerDistribution {
    pub total_examples: u64,
    pub scored_examples: u64,
    pub unscored_examples: u64,
    pub minimum: Option<u64>,
    pub p50_nearest_rank: Option<u64>,
    pub p95_nearest_rank: Option<u64>,
    pub maximum: Option<u64>,
}

impl IntegerDistribution {
    fn build(values: impl Iterator<Item = Option<u64>>, total: usize) -> Self {
        let mut values = values.flatten().collect::<Vec<_>>();
        values.sort_unstable();
        let scored = values.len();
        Self {
            total_examples: u64::try_from(total).unwrap_or(u64::MAX),
            scored_examples: u64::try_from(scored).unwrap_or(u64::MAX),
            unscored_examples: u64::try_from(total.saturating_sub(scored)).unwrap_or(u64::MAX),
            minimum: values.first().copied(),
            p50_nearest_rank: nearest_rank(&values, 50),
            p95_nearest_rank: nearest_rank(&values, 95),
            maximum: values.last().copied(),
        }
    }

    fn validate(&self, expected_total: u64) -> Result<(), AggregationError> {
        let present = [
            self.minimum,
            self.p50_nearest_rank,
            self.p95_nearest_rank,
            self.maximum,
        ];
        if self.total_examples != expected_total
            || self.scored_examples.saturating_add(self.unscored_examples) != expected_total
            || present
                .iter()
                .any(|value| value.is_some() != (self.scored_examples != 0))
        {
            return Err(AggregationError::InvalidAggregate("integer distribution"));
        }
        if let (Some(minimum), Some(p50), Some(p95), Some(maximum)) = (
            self.minimum,
            self.p50_nearest_rank,
            self.p95_nearest_rank,
            self.maximum,
        ) && !(minimum <= p50 && p50 <= p95 && p95 <= maximum)
        {
            return Err(AggregationError::InvalidAggregate("integer ordering"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RateDistribution {
    pub micro: AggregateRate,
    pub per_example: ScalarDistribution,
}

impl RateDistribution {
    fn build(
        reports: &[&EvaluationReport],
        metric: impl Fn(&EvaluationReport) -> Option<RateMetric>,
    ) -> Result<Self, AggregationError> {
        let values = reports.iter().map(|report| metric(report));
        let micro = AggregateRate::from_metrics(values.clone().flatten())?;
        let per_example = ScalarDistribution::build(
            values.map(|value| value.and_then(|value| value.value)),
            reports.len(),
        )?;
        Ok(Self { micro, per_example })
    }

    fn validate(&self, total: u64) -> Result<(), AggregationError> {
        self.micro.validate()?;
        self.per_example.validate(total)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectiveDistribution {
    pub threshold: f64,
    pub coverage: ScalarDistribution,
    pub risk: ScalarDistribution,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PerceptualDistribution {
    pub lower_is_better: bool,
    pub artifact: Option<ResourceDigest>,
    pub parameters: BTreeMap<String, String>,
    pub values: ScalarDistribution,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AggregateMetrics {
    pub validity: RateDistribution,
    pub text_region_precision: RateDistribution,
    pub text_region_recall: RateDistribution,
    pub text_region_f1: ScalarDistribution,
    pub character_error: ScalarDistribution,
    pub word_error: ScalarDistribution,
    pub text_baseline_error: ScalarDistribution,
    pub element_precision: RateDistribution,
    pub element_recall: RateDistribution,
    pub element_f1: ScalarDistribution,
    pub parent_correctness: RateDistribution,
    pub sibling_correctness: RateDistribution,
    pub property_exactness: RateDistribution,
    pub property_numeric_error: ScalarDistribution,
    pub geometry_iou: ScalarDistribution,
    pub geometry_normalized_error: ScalarDistribution,
    pub held_out_geometry_iou: ScalarDistribution,
    pub held_out_geometry_normalized_error: ScalarDistribution,
    pub resource_recall: RateDistribution,
    pub provenance_honesty: RateDistribution,
    pub accessibility_accuracy: RateDistribution,
    pub exact_pixel_rate: RateDistribution,
    pub pixel_channel_error: ScalarDistribution,
    pub calibration_error: ScalarDistribution,
    pub brier_score: ScalarDistribution,
    pub latency_microseconds: IntegerDistribution,
    pub peak_ram_bytes: IntegerDistribution,
    pub peak_vram_bytes: IntegerDistribution,
    pub iterations: IntegerDistribution,
    pub external_cost_microunits: IntegerDistribution,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationAggregate {
    pub schema_version: u32,
    pub profile: String,
    pub source_profile: String,
    pub suite: EvaluationSuite,
    pub example_ids: Vec<String>,
    pub calibration_bins: u32,
    pub metrics: AggregateMetrics,
    pub selective: Vec<SelectiveDistribution>,
    pub perceptual_diagnostics: BTreeMap<String, PerceptualDistribution>,
    pub external_cost_currency: Option<String>,
}

impl EvaluationAggregate {
    /// Builds an order-independent corpus report from validated examples.
    ///
    /// P50 and p95 use the nearest-rank definition. Rates contain both pooled
    /// integer evidence (micro) and per-example distributions (macro).
    ///
    /// # Errors
    ///
    /// Rejects empty, excessive, mixed-suite, duplicate, invalid or
    /// evaluator-incompatible inputs.
    pub fn build(reports: &[EvaluationReport]) -> Result<Self, AggregationError> {
        if reports.is_empty() {
            return Err(AggregationError::EmptyCorpus);
        }
        if reports.len() > MAX_EVALUATION_ITEMS {
            return Err(AggregationError::TooManyExamples);
        }
        for report in reports {
            report.validate()?;
        }
        let suite = reports[0].suite;
        if reports.iter().any(|report| report.suite != suite) {
            return Err(AggregationError::MixedSuites);
        }
        let mut reports = reports.iter().collect::<Vec<_>>();
        reports.sort_by(|left, right| left.example_id.cmp(&right.example_id));
        if reports
            .windows(2)
            .any(|pair| pair[0].example_id == pair[1].example_id)
        {
            return Err(AggregationError::DuplicateExample);
        }
        let calibration_bins = reports[0].confidence.bins;
        let thresholds = reports[0]
            .confidence
            .selective
            .iter()
            .map(|point| point.threshold)
            .collect::<Vec<_>>();
        if reports.iter().any(|report| {
            report.confidence.bins != calibration_bins
                || report.confidence.selective.len() != thresholds.len()
                || report
                    .confidence
                    .selective
                    .iter()
                    .zip(&thresholds)
                    .any(|(point, threshold)| point.threshold.to_bits() != threshold.to_bits())
        }) {
            return Err(AggregationError::IncompatibleCalibration);
        }
        let external_cost_currency = common_currency(&reports)?;
        let metrics = build_metrics(&reports)?;
        let selective = thresholds
            .iter()
            .enumerate()
            .map(|(index, threshold)| {
                Ok(SelectiveDistribution {
                    threshold: *threshold,
                    coverage: scalar(&reports, |report| {
                        Some(report.confidence.selective[index].coverage)
                    })?,
                    risk: scalar(&reports, |report| report.confidence.selective[index].risk)?,
                })
            })
            .collect::<Result<Vec<_>, AggregationError>>()?;
        let perceptual_diagnostics = perceptual_distributions(&reports)?;
        let aggregate = Self {
            schema_version: 1,
            profile: AGGREGATE_PROFILE.to_owned(),
            source_profile: EVALUATION_PROFILE.to_owned(),
            suite,
            example_ids: reports
                .iter()
                .map(|report| report.example_id.clone())
                .collect(),
            calibration_bins,
            metrics,
            selective,
            perceptual_diagnostics,
            external_cost_currency,
        };
        aggregate.validate()?;
        Ok(aggregate)
    }

    /// Validates the aggregate's structural and derived-value invariants.
    ///
    /// # Errors
    ///
    /// Rejects identity, count, ordering, finite-value, and profile drift.
    pub fn validate(&self) -> Result<(), AggregationError> {
        let total = u64::try_from(self.example_ids.len()).unwrap_or(u64::MAX);
        if self.schema_version != 1
            || self.profile != AGGREGATE_PROFILE
            || self.source_profile != EVALUATION_PROFILE
            || self.example_ids.is_empty()
            || self.example_ids.len() > MAX_EVALUATION_ITEMS
            || self.example_ids.iter().any(|id| !is_identifier(id))
            || !self.example_ids.windows(2).all(|pair| pair[0] < pair[1])
            || self.calibration_bins == 0
            || usize::try_from(self.calibration_bins).unwrap_or(usize::MAX) > MAX_CALIBRATION_BINS
            || self.selective.len() > MAX_CALIBRATION_BINS
            || !self
                .selective
                .windows(2)
                .all(|pair| pair[0].threshold < pair[1].threshold)
            || self
                .external_cost_currency
                .as_ref()
                .is_some_and(|currency| !is_identifier(currency))
        {
            return Err(AggregationError::InvalidAggregate("aggregate header"));
        }
        self.metrics.validate(total)?;
        for point in &self.selective {
            if !probability(point.threshold) {
                return Err(AggregationError::InvalidAggregate("selective threshold"));
            }
            point.coverage.validate(total)?;
            point.risk.validate(total)?;
        }
        for (method, diagnostic) in &self.perceptual_diagnostics {
            if !is_identifier(method)
                || diagnostic
                    .artifact
                    .as_ref()
                    .is_some_and(|artifact| !artifact.is_valid())
                || diagnostic.parameters.is_empty()
            {
                return Err(AggregationError::InvalidAggregate("perceptual diagnostic"));
            }
            diagnostic.values.validate(total)?;
        }
        Ok(())
    }
}

impl AggregateMetrics {
    fn validate(&self, total: u64) -> Result<(), AggregationError> {
        for rate in [
            &self.validity,
            &self.text_region_precision,
            &self.text_region_recall,
            &self.element_precision,
            &self.element_recall,
            &self.parent_correctness,
            &self.sibling_correctness,
            &self.property_exactness,
            &self.resource_recall,
            &self.provenance_honesty,
            &self.accessibility_accuracy,
            &self.exact_pixel_rate,
        ] {
            rate.validate(total)?;
        }
        for scalar in [
            &self.text_region_f1,
            &self.character_error,
            &self.word_error,
            &self.text_baseline_error,
            &self.element_f1,
            &self.property_numeric_error,
            &self.geometry_iou,
            &self.geometry_normalized_error,
            &self.held_out_geometry_iou,
            &self.held_out_geometry_normalized_error,
            &self.pixel_channel_error,
            &self.calibration_error,
            &self.brier_score,
        ] {
            scalar.validate(total)?;
        }
        for integer in [
            &self.latency_microseconds,
            &self.peak_ram_bytes,
            &self.peak_vram_bytes,
            &self.iterations,
            &self.external_cost_microunits,
        ] {
            integer.validate(total)?;
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum AggregationError {
    #[error("evaluation corpus is empty")]
    EmptyCorpus,
    #[error("evaluation corpus exceeds the example limit")]
    TooManyExamples,
    #[error("evaluation corpus mixes source-backed and screenshot suites")]
    MixedSuites,
    #[error("evaluation corpus contains a duplicate example identity")]
    DuplicateExample,
    #[error("evaluation corpus uses incompatible calibration configurations")]
    IncompatibleCalibration,
    #[error("evaluation corpus uses incompatible perceptual diagnostic identities")]
    IncompatiblePerceptualDiagnostic,
    #[error("evaluation corpus mixes external-cost currencies")]
    MixedCurrencies,
    #[error("evaluation aggregate arithmetic overflowed")]
    ArithmeticOverflow,
    #[error("evaluation aggregate is invalid: {0}")]
    InvalidAggregate(&'static str),
    #[error(transparent)]
    Evaluation(#[from] EvaluationError),
}

fn build_metrics(reports: &[&EvaluationReport]) -> Result<AggregateMetrics, AggregationError> {
    Ok(AggregateMetrics {
        validity: rate(reports, |report| Some(report.validity))?,
        text_region_precision: rate(reports, |report| Some(report.text.regions.precision))?,
        text_region_recall: rate(reports, |report| Some(report.text.regions.recall))?,
        text_region_f1: scalar(reports, |report| report.text.regions.f1)?,
        character_error: scalar(reports, |report| report.text.characters.normalized_error)?,
        word_error: scalar(reports, |report| report.text.words.normalized_error)?,
        text_baseline_error: scalar(reports, |report| {
            report.text.baseline_geometry.mean_absolute_error
        })?,
        element_precision: rate(reports, |report| Some(report.elements.precision))?,
        element_recall: rate(reports, |report| Some(report.elements.recall))?,
        element_f1: scalar(reports, |report| report.elements.f1)?,
        parent_correctness: rate(reports, |report| Some(report.tree.parents))?,
        sibling_correctness: rate(reports, |report| Some(report.tree.sibling_pairs))?,
        property_exactness: rate(reports, |report| Some(report.properties.exact))?,
        property_numeric_error: scalar(reports, |report| {
            report.properties.numeric.mean_absolute_error
        })?,
        geometry_iou: scalar(reports, |report| report.geometry.mean_iou)?,
        geometry_normalized_error: scalar(reports, |report| {
            report
                .geometry
                .normalized_absolute_error
                .mean_absolute_error
        })?,
        held_out_geometry_iou: scalar(reports, |report| {
            report
                .held_out_layout
                .as_ref()
                .and_then(|geometry| geometry.mean_iou)
        })?,
        held_out_geometry_normalized_error: scalar(reports, |report| {
            report
                .held_out_layout
                .as_ref()
                .and_then(|geometry| geometry.normalized_absolute_error.mean_absolute_error)
        })?,
        resource_recall: rate(reports, |report| Some(report.resources))?,
        provenance_honesty: rate(reports, |report| Some(report.provenance_honesty))?,
        accessibility_accuracy: rate(reports, |report| report.accessibility)?,
        exact_pixel_rate: rate(reports, |report| Some(report.pixels.exact_pixel_rate))?,
        pixel_channel_error: scalar(reports, |report| {
            Some(report.pixels.mean_absolute_channel_error)
        })?,
        calibration_error: scalar(reports, |report| {
            report.confidence.expected_calibration_error
        })?,
        brier_score: scalar(reports, |report| report.confidence.brier_score)?,
        latency_microseconds: integer(reports, |report| report.cost.latency_microseconds),
        peak_ram_bytes: integer(reports, |report| report.cost.peak_ram_bytes),
        peak_vram_bytes: integer(reports, |report| report.cost.peak_vram_bytes),
        iterations: integer(reports, |report| Some(u64::from(report.cost.iterations))),
        external_cost_microunits: integer(reports, |report| {
            Some(report.cost.external_cost_microunits)
        }),
    })
}

fn rate(
    reports: &[&EvaluationReport],
    metric: impl Fn(&EvaluationReport) -> Option<RateMetric>,
) -> Result<RateDistribution, AggregationError> {
    RateDistribution::build(reports, metric)
}

fn scalar(
    reports: &[&EvaluationReport],
    metric: impl Fn(&EvaluationReport) -> Option<f64>,
) -> Result<ScalarDistribution, AggregationError> {
    ScalarDistribution::build(reports.iter().map(|report| metric(report)), reports.len())
}

fn integer(
    reports: &[&EvaluationReport],
    metric: impl Fn(&EvaluationReport) -> Option<u64>,
) -> IntegerDistribution {
    IntegerDistribution::build(reports.iter().map(|report| metric(report)), reports.len())
}

fn common_currency(reports: &[&EvaluationReport]) -> Result<Option<String>, AggregationError> {
    let currencies = reports
        .iter()
        .filter_map(|report| report.cost.currency.clone())
        .collect::<BTreeSet<_>>();
    if currencies.len() > 1 {
        Err(AggregationError::MixedCurrencies)
    } else {
        Ok(currencies.into_iter().next())
    }
}

fn perceptual_distributions(
    reports: &[&EvaluationReport],
) -> Result<BTreeMap<String, PerceptualDistribution>, AggregationError> {
    let methods = reports
        .iter()
        .flat_map(|report| {
            report
                .perceptual_diagnostics
                .iter()
                .map(|diagnostic| diagnostic.method.clone())
        })
        .collect::<BTreeSet<_>>();
    let mut distributions = BTreeMap::new();
    for method in methods {
        let matching = reports
            .iter()
            .filter_map(|report| {
                report
                    .perceptual_diagnostics
                    .iter()
                    .find(|diagnostic| diagnostic.method == method)
            })
            .collect::<Vec<_>>();
        let first = matching[0];
        if matching.iter().any(|diagnostic| {
            diagnostic.lower_is_better != first.lower_is_better
                || diagnostic.artifact != first.artifact
                || diagnostic.parameters != first.parameters
        }) {
            return Err(AggregationError::IncompatiblePerceptualDiagnostic);
        }
        distributions.insert(
            method,
            PerceptualDistribution {
                lower_is_better: first.lower_is_better,
                artifact: first.artifact.clone(),
                parameters: first.parameters.clone(),
                values: ScalarDistribution::build(
                    reports.iter().map(|report| {
                        report
                            .perceptual_diagnostics
                            .iter()
                            .find(|diagnostic| diagnostic.method == first.method)
                            .map(|diagnostic| diagnostic.value)
                    }),
                    reports.len(),
                )?,
            },
        );
    }
    Ok(distributions)
}

fn nearest_rank<T: Copy>(values: &[T], percentile: usize) -> Option<T> {
    if values.is_empty() {
        return None;
    }
    let rank = values.len().saturating_mul(percentile).saturating_add(99) / 100;
    values.get(rank.saturating_sub(1)).copied()
}

fn optional_same(left: Option<f64>, right: Option<f64>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => same(left, right),
        (None, None) => true,
        _ => false,
    }
}

fn less_or_same(left: f64, right: f64) -> bool {
    left <= right || same(left, right)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Bounds;
    use crate::evaluation::{
        CalibrationMetrics, CostMetrics, DetectionMetrics, ErrorMetrics, GeometryMetrics,
        PerceptualDiagnostic, PixelMetrics, PropertyMetrics, TextMetrics, TreeMetrics,
    };

    fn fixture(id: &str, validity: RateMetric) -> EvaluationReport {
        let bounds = [Bounds {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        }];
        EvaluationReport {
            schema_version: 1,
            profile: EVALUATION_PROFILE.to_owned(),
            example_id: id.to_owned(),
            suite: EvaluationSuite::SyntheticExact,
            validity,
            text: TextMetrics {
                regions: DetectionMetrics::new(1, 0, 0),
                characters: super::super::EditMetrics::new(0, 1),
                words: super::super::EditMetrics::new(0, 1),
                baseline_geometry: ErrorMetrics::new(1, 0.0),
            },
            elements: DetectionMetrics::new(1, 0, 0),
            tree: TreeMetrics {
                parents: RateMetric::new(1, 1),
                sibling_pairs: RateMetric::new(0, 0),
            },
            properties: PropertyMetrics {
                exact: RateMetric::new(1, 1),
                numeric: ErrorMetrics::new(1, 0.0),
            },
            geometry: GeometryMetrics::compare(&bounds, &bounds).unwrap(),
            held_out_layout: None,
            resources: RateMetric::new(0, 0),
            provenance_honesty: RateMetric::new(1, 1),
            accessibility: None,
            pixels: PixelMetrics::compare(1, 1, &[0, 0, 0, 255], &[0, 0, 0, 255]).unwrap(),
            perceptual_diagnostics: Vec::new(),
            confidence: CalibrationMetrics::evaluate(&[], 5, &[0.5]).unwrap(),
            cost: CostMetrics {
                latency_microseconds: None,
                peak_ram_bytes: None,
                peak_vram_bytes: None,
                iterations: 1,
                external_cost_microunits: 0,
                currency: None,
            },
        }
    }

    #[test]
    fn aggregation_is_order_independent_and_keeps_unscored_examples() {
        let first = fixture("example-a", RateMetric::new(1, 2));
        let second = fixture("example-b", RateMetric::new(2, 2));
        let forward = EvaluationAggregate::build(&[first.clone(), second.clone()]).unwrap();
        let reverse = EvaluationAggregate::build(&[second, first]).unwrap();
        assert_eq!(forward, reverse);
        assert_eq!(forward.metrics.validity.micro.numerator, 3);
        assert_eq!(forward.metrics.validity.micro.denominator, 4);
        assert_eq!(
            forward.metrics.validity.per_example.p50_nearest_rank,
            Some(0.5)
        );
        assert_eq!(
            forward.metrics.resource_recall.per_example.scored_examples,
            0
        );
        assert_eq!(
            forward
                .metrics
                .accessibility_accuracy
                .per_example
                .unscored_examples,
            2
        );
        forward.validate().unwrap();

        let mut tampered = forward;
        tampered.metrics.validity.per_example.p50_nearest_rank = Some(2.0);
        assert!(tampered.validate().is_err());
    }

    #[test]
    fn aggregation_rejects_mixed_or_incompatible_evidence() {
        let first = fixture("example-a", RateMetric::new(1, 1));
        let mut duplicate = first.clone();
        assert!(matches!(
            EvaluationAggregate::build(&[first.clone(), duplicate.clone()]),
            Err(AggregationError::DuplicateExample)
        ));
        duplicate.example_id = "example-b".to_owned();
        duplicate.suite = EvaluationSuite::SourceBacked;
        assert!(matches!(
            EvaluationAggregate::build(&[first.clone(), duplicate]),
            Err(AggregationError::MixedSuites)
        ));
        let mut incompatible = fixture("example-b", RateMetric::new(1, 1));
        incompatible.confidence = CalibrationMetrics::evaluate(&[], 10, &[0.6]).unwrap();
        assert!(matches!(
            EvaluationAggregate::build(&[first, incompatible]),
            Err(AggregationError::IncompatibleCalibration)
        ));
    }

    #[test]
    fn aggregation_rejects_mixed_currency_and_evaluator_artifacts() {
        let mut first = fixture("example-a", RateMetric::new(1, 1));
        let mut second = fixture("example-b", RateMetric::new(1, 1));
        first.cost.external_cost_microunits = 1;
        first.cost.currency = Some("usd".to_owned());
        second.cost.external_cost_microunits = 1;
        second.cost.currency = Some("eur".to_owned());
        assert!(matches!(
            EvaluationAggregate::build(&[first.clone(), second.clone()]),
            Err(AggregationError::MixedCurrencies)
        ));

        first.cost.external_cost_microunits = 0;
        first.cost.currency = None;
        second.cost.external_cost_microunits = 0;
        second.cost.currency = None;
        first.perceptual_diagnostics = vec![PerceptualDiagnostic {
            method: "fixture-metric".to_owned(),
            value: 0.5,
            lower_is_better: false,
            artifact: Some(ResourceDigest::from_sha256_hex("a".repeat(64))),
            parameters: BTreeMap::from([("profile".to_owned(), "fixture-1".to_owned())]),
        }];
        second.perceptual_diagnostics = vec![PerceptualDiagnostic {
            method: "fixture-metric".to_owned(),
            value: 0.5,
            lower_is_better: false,
            artifact: Some(ResourceDigest::from_sha256_hex("b".repeat(64))),
            parameters: BTreeMap::from([("profile".to_owned(), "fixture-1".to_owned())]),
        }];
        assert!(matches!(
            EvaluationAggregate::build(&[first, second]),
            Err(AggregationError::IncompatiblePerceptualDiagnostic)
        ));
    }
}
