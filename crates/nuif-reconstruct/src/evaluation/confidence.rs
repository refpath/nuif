use nuif_core::{ResourceDigest, is_identifier};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const CONFIDENCE_EVALUATION_PROFILE: &str = "nuif-confidence-evaluation-0";
pub const MAX_CONFIDENCE_CASES: usize = 10_000;
pub const MAX_CONFIDENCE_BINS: usize = 100;
pub const MAX_CONFIDENCE_CONDITIONS: usize = 64;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionKind {
    Text,
    RegionClass,
    Parent,
    LayoutFamily,
    Geometry,
    ResourceMatch,
    OperationAcceptance,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidencePartition {
    Calibration,
    Test,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShiftAxis {
    Font,
    Template,
    Resolution,
    Language,
    RenderingCondition,
    Other,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "class", rename_all = "snake_case", deny_unknown_fields)]
pub enum DistributionCondition {
    InDistribution,
    Shifted { axis: ShiftAxis, label: String },
}

impl DistributionCondition {
    fn validate(&self) -> bool {
        match self {
            Self::InDistribution => true,
            Self::Shifted { label, .. } => is_identifier(label),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfidenceCase {
    pub id: String,
    pub group_id: String,
    pub partition: ConfidencePartition,
    pub condition: DistributionCondition,
    pub decision: DecisionKind,
    pub raw_confidence: f64,
    pub calibrated_confidence: f64,
    pub correct: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfidenceEvaluationConfig {
    pub schema_version: u32,
    pub profile: String,
    pub corpus_manifest: ResourceDigest,
    pub samples_artifact: ResourceDigest,
    pub calibrator_artifact: ResourceDigest,
    pub evaluator_artifact: ResourceDigest,
    pub bins: u32,
    pub automatic_risk_limit: f64,
    pub review_risk_limit: f64,
}

impl ConfidenceEvaluationConfig {
    fn validate(&self) -> Result<(), ConfidenceEvaluationError> {
        if self.schema_version != 1
            || self.profile != CONFIDENCE_EVALUATION_PROFILE
            || !self.corpus_manifest.is_valid()
            || !self.samples_artifact.is_valid()
            || !self.calibrator_artifact.is_valid()
            || !self.evaluator_artifact.is_valid()
            || self.bins == 0
            || self.bins as usize > MAX_CONFIDENCE_BINS
            || !probability(self.automatic_risk_limit)
            || !probability(self.review_risk_limit)
            || self.automatic_risk_limit > self.review_risk_limit
        {
            Err(ConfidenceEvaluationError::InvalidConfig)
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReliabilityBin {
    pub index: u32,
    pub samples: u64,
    pub correct: u64,
    pub confidence_sum: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RiskCoveragePoint {
    pub threshold: f64,
    pub selected: u64,
    pub errors: u64,
    pub coverage: f64,
    pub risk: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfidenceScoreSummary {
    pub samples: u64,
    pub correct: u64,
    pub accuracy: f64,
    pub brier_error_sum: f64,
    pub brier_score: f64,
    pub expected_calibration_error: f64,
    pub reliability: Vec<ReliabilityBin>,
    pub risk_coverage: Vec<RiskCoveragePoint>,
    pub area_under_risk_coverage: f64,
}

impl ConfidenceScoreSummary {
    fn build(
        cases: &[&ConfidenceCase],
        bins: u32,
        calibrated: bool,
    ) -> Result<Self, ConfidenceEvaluationError> {
        if cases.is_empty() {
            return Err(ConfidenceEvaluationError::MissingEvidence);
        }
        let samples = cases.len() as u64;
        let correct = cases.iter().filter(|case| case.correct).count() as u64;
        let mut reliability = (0..bins)
            .map(|index| ReliabilityBin {
                index,
                samples: 0,
                correct: 0,
                confidence_sum: 0.0,
            })
            .collect::<Vec<_>>();
        let mut brier_error_sum = 0.0;
        for case in cases {
            let confidence = case_confidence(case, calibrated);
            let bin = confidence_bin(confidence, bins);
            let item = &mut reliability[bin];
            item.samples += 1;
            item.correct += u64::from(case.correct);
            item.confidence_sum += confidence;
            let target = f64::from(case.correct);
            brier_error_sum += (confidence - target).powi(2);
        }
        let expected_calibration_error = reliability
            .iter()
            .filter(|bin| bin.samples != 0)
            .map(|bin| {
                let count = bin.samples as f64;
                (count / samples as f64)
                    * (bin.confidence_sum / count - bin.correct as f64 / count).abs()
            })
            .sum::<f64>();
        let risk_coverage = risk_coverage(cases, calibrated);
        let area_under_risk_coverage = risk_coverage
            .iter()
            .scan(0_u64, |previous, point| {
                let added = point.selected - *previous;
                *previous = point.selected;
                Some(point.risk * added as f64 / samples as f64)
            })
            .sum();
        let summary = Self {
            samples,
            correct,
            accuracy: correct as f64 / samples as f64,
            brier_error_sum,
            brier_score: brier_error_sum / samples as f64,
            expected_calibration_error,
            reliability,
            risk_coverage,
            area_under_risk_coverage,
        };
        summary.validate(bins)?;
        Ok(summary)
    }

    fn validate(&self, bins: u32) -> Result<(), ConfidenceEvaluationError> {
        if self.samples == 0
            || self.samples > MAX_CONFIDENCE_CASES as u64
            || self.correct > self.samples
            || self.reliability.len() != bins as usize
            || self.risk_coverage.is_empty()
            || self.risk_coverage.len() > self.samples as usize
            || !probability(self.accuracy)
            || !probability(self.brier_score)
            || !probability(self.expected_calibration_error)
            || !probability(self.area_under_risk_coverage)
            || !self.brier_error_sum.is_finite()
            || self.brier_error_sum < 0.0
        {
            return Err(ConfidenceEvaluationError::InvalidReport);
        }
        let reliability_samples = self.reliability.iter().map(|bin| bin.samples).sum::<u64>();
        let reliability_correct = self.reliability.iter().map(|bin| bin.correct).sum::<u64>();
        if reliability_samples != self.samples
            || reliability_correct != self.correct
            || self.reliability.iter().enumerate().any(|(index, bin)| {
                bin.index as usize != index
                    || bin.correct > bin.samples
                    || !bin.confidence_sum.is_finite()
                    || bin.confidence_sum < 0.0
                    || bin.confidence_sum > bin.samples as f64
            })
            || !same(self.accuracy, self.correct as f64 / self.samples as f64)
            || !same(self.brier_score, self.brier_error_sum / self.samples as f64)
        {
            return Err(ConfidenceEvaluationError::InvalidReport);
        }
        let ece = self
            .reliability
            .iter()
            .filter(|bin| bin.samples != 0)
            .map(|bin| {
                let count = bin.samples as f64;
                (count / self.samples as f64)
                    * (bin.confidence_sum / count - bin.correct as f64 / count).abs()
            })
            .sum::<f64>();
        if !same(ece, self.expected_calibration_error) {
            return Err(ConfidenceEvaluationError::InvalidReport);
        }
        let mut previous_selected = 0_u64;
        let mut previous_errors = 0_u64;
        let mut previous_threshold = f64::INFINITY;
        let mut aurc = 0.0;
        for point in &self.risk_coverage {
            if !probability(point.threshold)
                || point.threshold >= previous_threshold
                || point.selected <= previous_selected
                || point.selected > self.samples
                || point.errors < previous_errors
                || point.errors > point.selected
                || !same(point.coverage, point.selected as f64 / self.samples as f64)
                || !same(point.risk, point.errors as f64 / point.selected as f64)
            {
                return Err(ConfidenceEvaluationError::InvalidReport);
            }
            aurc += point.risk * (point.selected - previous_selected) as f64 / self.samples as f64;
            previous_selected = point.selected;
            previous_errors = point.errors;
            previous_threshold = point.threshold;
        }
        if previous_selected != self.samples || !same(aurc, self.area_under_risk_coverage) {
            return Err(ConfidenceEvaluationError::InvalidReport);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectionOutcome {
    pub threshold: Option<f64>,
    pub selected: u64,
    pub errors: u64,
    pub coverage: f64,
    pub risk: Option<f64>,
}

impl SelectionOutcome {
    fn empty() -> Self {
        Self {
            threshold: None,
            selected: 0,
            errors: 0,
            coverage: 0.0,
            risk: None,
        }
    }

    fn validate(self, samples: u64) -> Result<(), ConfidenceEvaluationError> {
        if samples == 0
            || self.selected > samples
            || self.errors > self.selected
            || !probability(self.coverage)
            || self.threshold.is_some_and(|value| !probability(value))
            || self.risk.is_some_and(|value| !probability(value))
            || self.threshold.is_some() != (self.selected != 0)
            || self.risk.is_some() != (self.selected != 0)
            || !same(self.coverage, self.selected as f64 / samples as f64)
            || self
                .risk
                .is_some_and(|risk| !same(risk, self.errors as f64 / self.selected as f64))
        {
            Err(ConfidenceEvaluationError::InvalidReport)
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectivePolicy {
    pub calibration_samples: u64,
    pub automatic_risk_limit: f64,
    pub review_risk_limit: f64,
    pub automatic: SelectionOutcome,
    pub review_or_better: SelectionOutcome,
}

impl SelectivePolicy {
    fn build(cases: &[&ConfidenceCase], automatic_limit: f64, review_limit: f64) -> Self {
        let automatic = select_threshold(cases, automatic_limit);
        let mut review_or_better = select_threshold(cases, review_limit);
        if review_or_better.selected < automatic.selected {
            review_or_better = automatic;
        }
        Self {
            calibration_samples: cases.len() as u64,
            automatic_risk_limit: automatic_limit,
            review_risk_limit: review_limit,
            automatic,
            review_or_better,
        }
    }

    fn validate(&self) -> Result<(), ConfidenceEvaluationError> {
        self.automatic.validate(self.calibration_samples)?;
        self.review_or_better.validate(self.calibration_samples)?;
        if !probability(self.automatic_risk_limit)
            || !probability(self.review_risk_limit)
            || self.automatic_risk_limit > self.review_risk_limit
            || self.automatic.selected > self.review_or_better.selected
            || self
                .automatic
                .risk
                .is_some_and(|risk| risk > self.automatic_risk_limit)
            || self
                .review_or_better
                .risk
                .is_some_and(|risk| risk > self.review_risk_limit)
        {
            Err(ConfidenceEvaluationError::InvalidReport)
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyOutcome {
    pub samples: u64,
    pub automatic: SelectionOutcome,
    pub review_or_better: SelectionOutcome,
    pub always_answer_risk: f64,
}

impl PolicyOutcome {
    fn validate(&self) -> Result<(), ConfidenceEvaluationError> {
        self.automatic.validate(self.samples)?;
        self.review_or_better.validate(self.samples)?;
        if self.automatic.selected > self.review_or_better.selected
            || !probability(self.always_answer_risk)
        {
            Err(ConfidenceEvaluationError::InvalidReport)
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConditionEvaluation {
    pub condition: DistributionCondition,
    pub raw: ConfidenceScoreSummary,
    pub calibrated: ConfidenceScoreSummary,
    pub policy: PolicyOutcome,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionEvaluation {
    pub decision: DecisionKind,
    pub policy: SelectivePolicy,
    pub conditions: Vec<ConditionEvaluation>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfidenceEvaluationReport {
    pub schema_version: u32,
    pub profile: String,
    pub config: ConfidenceEvaluationConfig,
    pub calibration_cases: u64,
    pub test_cases: u64,
    pub decisions: Vec<DecisionEvaluation>,
}

impl ConfidenceEvaluationReport {
    /// Evaluates one frozen set of calibration and test correctness events.
    ///
    /// Policy thresholds are selected only from the calibration partition.
    /// Equal confidence scores are admitted or rejected as one group, so
    /// input order cannot change risk/coverage evidence.
    ///
    /// # Errors
    ///
    /// Rejects malformed identities/probabilities, split or group leakage,
    /// non-monotonic calibration, absent normal holdouts, or bounded-work
    /// excess.
    pub fn evaluate(
        config: ConfidenceEvaluationConfig,
        cases: &[ConfidenceCase],
    ) -> Result<Self, ConfidenceEvaluationError> {
        config.validate()?;
        validate_cases(cases)?;
        let decisions = cases
            .iter()
            .map(|case| case.decision)
            .collect::<BTreeSet<_>>();
        let mut reports = Vec::with_capacity(decisions.len());
        for decision in decisions {
            let calibration = cases
                .iter()
                .filter(|case| {
                    case.decision == decision && case.partition == ConfidencePartition::Calibration
                })
                .collect::<Vec<_>>();
            if calibration.is_empty() {
                return Err(ConfidenceEvaluationError::MissingEvidence);
            }
            let policy = SelectivePolicy::build(
                &calibration,
                config.automatic_risk_limit,
                config.review_risk_limit,
            );
            let mut conditions = BTreeMap::<DistributionCondition, Vec<&ConfidenceCase>>::new();
            for case in cases.iter().filter(|case| {
                case.decision == decision && case.partition == ConfidencePartition::Test
            }) {
                conditions
                    .entry(case.condition.clone())
                    .or_default()
                    .push(case);
            }
            if !conditions.contains_key(&DistributionCondition::InDistribution)
                || conditions.len() > MAX_CONFIDENCE_CONDITIONS
            {
                return Err(ConfidenceEvaluationError::MissingEvidence);
            }
            let conditions = conditions
                .into_iter()
                .map(|(condition, samples)| {
                    Ok(ConditionEvaluation {
                        condition,
                        raw: ConfidenceScoreSummary::build(&samples, config.bins, false)?,
                        calibrated: ConfidenceScoreSummary::build(&samples, config.bins, true)?,
                        policy: policy_outcome(&samples, &policy),
                    })
                })
                .collect::<Result<Vec<_>, ConfidenceEvaluationError>>()?;
            reports.push(DecisionEvaluation {
                decision,
                policy,
                conditions,
            });
        }
        let report = Self {
            schema_version: 1,
            profile: CONFIDENCE_EVALUATION_PROFILE.to_owned(),
            config,
            calibration_cases: cases
                .iter()
                .filter(|case| case.partition == ConfidencePartition::Calibration)
                .count() as u64,
            test_cases: cases
                .iter()
                .filter(|case| case.partition == ConfidencePartition::Test)
                .count() as u64,
            decisions: reports,
        };
        report.validate()?;
        Ok(report)
    }

    /// Validates the report's identities and all retained derived arithmetic.
    ///
    /// # Errors
    ///
    /// Rejects profile drift, duplicates, invalid distributions, malformed
    /// policy evidence, or inconsistent derived metrics.
    pub fn validate(&self) -> Result<(), ConfidenceEvaluationError> {
        self.config.validate()?;
        if self.schema_version != 1
            || self.profile != CONFIDENCE_EVALUATION_PROFILE
            || self.calibration_cases == 0
            || self.test_cases == 0
            || self.calibration_cases + self.test_cases > MAX_CONFIDENCE_CASES as u64
            || self.decisions.is_empty()
        {
            return Err(ConfidenceEvaluationError::InvalidReport);
        }
        let mut decisions = BTreeSet::new();
        for decision in &self.decisions {
            if !decisions.insert(decision.decision)
                || decision.policy.calibration_samples == 0
                || decision.conditions.is_empty()
                || decision.conditions.len() > MAX_CONFIDENCE_CONDITIONS
            {
                return Err(ConfidenceEvaluationError::InvalidReport);
            }
            decision.policy.validate()?;
            let mut conditions = BTreeSet::new();
            for condition in &decision.conditions {
                if !condition.condition.validate() || !conditions.insert(&condition.condition) {
                    return Err(ConfidenceEvaluationError::InvalidReport);
                }
                condition.raw.validate(self.config.bins)?;
                condition.calibrated.validate(self.config.bins)?;
                condition.policy.validate()?;
                if condition.raw.samples != condition.calibrated.samples
                    || condition.raw.correct != condition.calibrated.correct
                    || condition.policy.samples != condition.raw.samples
                    || !same(condition.raw.accuracy, condition.calibrated.accuracy)
                {
                    return Err(ConfidenceEvaluationError::InvalidReport);
                }
            }
            if !conditions.contains(&DistributionCondition::InDistribution) {
                return Err(ConfidenceEvaluationError::InvalidReport);
            }
        }
        Ok(())
    }
}

fn validate_cases(cases: &[ConfidenceCase]) -> Result<(), ConfidenceEvaluationError> {
    if cases.is_empty() || cases.len() > MAX_CONFIDENCE_CASES {
        return Err(ConfidenceEvaluationError::ResourceLimit);
    }
    let mut ids = BTreeSet::new();
    let mut group_partitions = BTreeMap::new();
    for case in cases {
        if !is_identifier(&case.id)
            || !is_identifier(&case.group_id)
            || !case.condition.validate()
            || !probability(case.raw_confidence)
            || !probability(case.calibrated_confidence)
            || negative_zero(case.raw_confidence)
            || negative_zero(case.calibrated_confidence)
            || !ids.insert(&case.id)
            || (case.partition == ConfidencePartition::Calibration
                && case.condition != DistributionCondition::InDistribution)
        {
            return Err(ConfidenceEvaluationError::InvalidCase);
        }
        if let Some(partition) = group_partitions.insert(&case.group_id, case.partition)
            && partition != case.partition
        {
            return Err(ConfidenceEvaluationError::GroupLeakage);
        }
    }
    for decision in cases
        .iter()
        .map(|case| case.decision)
        .collect::<BTreeSet<_>>()
    {
        let mut mapping = cases
            .iter()
            .filter(|case| case.decision == decision)
            .map(|case| (case.raw_confidence, case.calibrated_confidence))
            .collect::<Vec<_>>();
        mapping.sort_by(|left, right| left.0.total_cmp(&right.0));
        if mapping
            .windows(2)
            .any(|pair| (pair[0].0 == pair[1].0 && pair[0].1 != pair[1].1) || pair[0].1 > pair[1].1)
        {
            return Err(ConfidenceEvaluationError::NonMonotonicCalibration);
        }
    }
    Ok(())
}

fn risk_coverage(cases: &[&ConfidenceCase], calibrated: bool) -> Vec<RiskCoveragePoint> {
    let mut ordered = cases.to_vec();
    ordered.sort_by(|left, right| {
        case_confidence(right, calibrated).total_cmp(&case_confidence(left, calibrated))
    });
    let mut points = Vec::new();
    let mut selected = 0_u64;
    let mut errors = 0_u64;
    let mut index = 0;
    while index < ordered.len() {
        let threshold = case_confidence(ordered[index], calibrated);
        while index < ordered.len() && case_confidence(ordered[index], calibrated) == threshold {
            selected += 1;
            errors += u64::from(!ordered[index].correct);
            index += 1;
        }
        points.push(RiskCoveragePoint {
            threshold,
            selected,
            errors,
            coverage: selected as f64 / ordered.len() as f64,
            risk: errors as f64 / selected as f64,
        });
    }
    points
}

fn select_threshold(cases: &[&ConfidenceCase], risk_limit: f64) -> SelectionOutcome {
    risk_coverage(cases, true)
        .into_iter()
        .filter(|point| point.risk <= risk_limit)
        .map(|point| SelectionOutcome {
            threshold: Some(point.threshold),
            selected: point.selected,
            errors: point.errors,
            coverage: point.coverage,
            risk: Some(point.risk),
        })
        .max_by_key(|outcome| outcome.selected)
        .unwrap_or_else(SelectionOutcome::empty)
}

fn policy_outcome(cases: &[&ConfidenceCase], policy: &SelectivePolicy) -> PolicyOutcome {
    let automatic = apply_threshold(cases, policy.automatic.threshold);
    let review_or_better = apply_threshold(cases, policy.review_or_better.threshold);
    let errors = cases.iter().filter(|case| !case.correct).count() as u64;
    PolicyOutcome {
        samples: cases.len() as u64,
        automatic,
        review_or_better,
        always_answer_risk: errors as f64 / cases.len() as f64,
    }
}

fn apply_threshold(cases: &[&ConfidenceCase], threshold: Option<f64>) -> SelectionOutcome {
    let Some(threshold) = threshold else {
        return SelectionOutcome::empty();
    };
    let selected = cases
        .iter()
        .filter(|case| case.calibrated_confidence >= threshold)
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return SelectionOutcome::empty();
    }
    let errors = selected.iter().filter(|case| !case.correct).count() as u64;
    SelectionOutcome {
        threshold: Some(threshold),
        selected: selected.len() as u64,
        errors,
        coverage: selected.len() as f64 / cases.len() as f64,
        risk: Some(errors as f64 / selected.len() as f64),
    }
}

fn confidence_bin(confidence: f64, bins: u32) -> usize {
    ((confidence * bins as f64).floor() as usize).min(bins as usize - 1)
}

fn case_confidence(case: &ConfidenceCase, calibrated: bool) -> f64 {
    if calibrated {
        case.calibrated_confidence
    } else {
        case.raw_confidence
    }
}

fn probability(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn negative_zero(value: f64) -> bool {
    value.to_bits() == (-0.0_f64).to_bits()
}

fn same(left: f64, right: f64) -> bool {
    (left - right).abs() <= 1e-12_f64.max(left.abs().max(right.abs()) * 1e-12)
}

#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum ConfidenceEvaluationError {
    #[error("confidence evaluation configuration is invalid")]
    InvalidConfig,
    #[error("confidence evaluation case is invalid or duplicated")]
    InvalidCase,
    #[error("calibration and test groups overlap")]
    GroupLeakage,
    #[error("calibration mapping is not a deterministic monotonic function")]
    NonMonotonicCalibration,
    #[error("confidence evaluation is missing required calibration or normal holdout evidence")]
    MissingEvidence,
    #[error("confidence evaluation report is inconsistent")]
    InvalidReport,
    #[error("confidence evaluation resource limit exceeded")]
    ResourceLimit,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(value: char) -> ResourceDigest {
        ResourceDigest::from_sha256_hex(value.to_string().repeat(64))
    }

    fn config() -> ConfidenceEvaluationConfig {
        ConfidenceEvaluationConfig {
            schema_version: 1,
            profile: CONFIDENCE_EVALUATION_PROFILE.to_owned(),
            corpus_manifest: digest('a'),
            samples_artifact: digest('b'),
            calibrator_artifact: digest('c'),
            evaluator_artifact: digest('d'),
            bins: 4,
            automatic_risk_limit: 0.25,
            review_risk_limit: 0.5,
        }
    }

    fn case(
        id: &str,
        partition: ConfidencePartition,
        raw: f64,
        calibrated: f64,
        correct: bool,
    ) -> ConfidenceCase {
        ConfidenceCase {
            id: id.to_owned(),
            group_id: id.to_owned(),
            partition,
            condition: DistributionCondition::InDistribution,
            decision: DecisionKind::Text,
            raw_confidence: raw,
            calibrated_confidence: calibrated,
            correct,
        }
    }

    #[test]
    fn evaluation_is_tie_safe_and_derived_metrics_validate() {
        let cases = vec![
            case("cal-a", ConfidencePartition::Calibration, 0.9, 0.75, true),
            case("cal-b", ConfidencePartition::Calibration, 0.9, 0.75, false),
            case("test-a", ConfidencePartition::Test, 0.9, 0.75, true),
            case("test-b", ConfidencePartition::Test, 0.9, 0.75, false),
        ];
        let report = ConfidenceEvaluationReport::evaluate(config(), &cases).unwrap();
        assert_eq!(report.decisions[0].conditions[0].raw.risk_coverage.len(), 1);
        assert!(report.validate().is_ok());
    }

    #[test]
    fn leakage_and_non_monotonic_calibration_fail_closed() {
        let mut cases = vec![
            case("cal-a", ConfidencePartition::Calibration, 0.2, 0.4, false),
            case("test-a", ConfidencePartition::Test, 0.8, 0.6, true),
        ];
        let group_id = cases[0].group_id.clone();
        cases[1].group_id = group_id;
        assert!(matches!(
            ConfidenceEvaluationReport::evaluate(config(), &cases),
            Err(ConfidenceEvaluationError::GroupLeakage)
        ));
        cases[1].group_id = "test-a".to_owned();
        cases[1].calibrated_confidence = 0.3;
        assert!(matches!(
            ConfidenceEvaluationReport::evaluate(config(), &cases),
            Err(ConfidenceEvaluationError::NonMonotonicCalibration)
        ));
    }
}
