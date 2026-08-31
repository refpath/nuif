use super::{EvaluationSuite, MAX_EVALUATION_ITEMS};
use nuif_core::{ResourceDigest, is_identifier};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const CORPUS_MANIFEST_PROFILE: &str = "nuif-reconstruction-corpus-manifest-0";
pub const CORPUS_AUDIT_PROFILE: &str = "nuif-reconstruction-corpus-audit-0";
pub const MAX_ARTIFACTS_PER_EXAMPLE: usize = 256;
pub const MAX_GROUPS_PER_DIMENSION: usize = 1_024;
pub const MAX_CORPUS_STRING_BYTES: usize = 4_096;
pub const MAX_CORPUS_METADATA_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorpusSplit {
    Adaptation,
    Calibration,
    Validation,
    Test,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Screenshot,
    SourceDocument,
    TargetDocument,
    Annotation,
    ResourceBundle,
    OperationTrace,
    CaptureContext,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactDisclosure {
    Public,
    Restricted,
    Withheld,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorpusArtifact {
    pub kind: ArtifactKind,
    pub digest: ResourceDigest,
    pub disclosure: ArtifactDisclosure,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectionClass {
    ProjectSynthetic,
    ContributorProvided,
    PublicWeb,
    HostExport,
    PrivateAuthenticated,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsentBasis {
    ProjectGenerated,
    ExplicitOptIn,
    ContractualAuthorization,
    PublicDomain,
    NotApplicable,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    Allowed,
    Prohibited,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PermittedUses {
    pub evaluation: Permission,
    pub calibration: Permission,
    pub adaptation: Permission,
    pub redistribution: Permission,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensitivityReview {
    SyntheticNoPersonalData,
    HumanReviewedNoSensitiveData,
    HumanReviewedRestricted,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RightsRecord {
    /// A recorded expression, not a legal conclusion by this validator.
    pub license_expression: String,
    pub evidence_artifact: ResourceDigest,
    pub consent_basis: ConsentBasis,
    pub permitted_uses: PermittedUses,
    pub sensitivity_review: SensitivityReview,
    pub withdrawal_policy_artifact: Option<ResourceDigest>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeakageGroups {
    pub origin: String,
    pub template: Option<String>,
    pub components: BTreeSet<String>,
    pub fonts: BTreeSet<String>,
    pub resources: BTreeSet<String>,
    pub generators: BTreeSet<String>,
    /// At least one group is required, even for a known-unique example.
    pub near_duplicates: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorpusExample {
    pub example_id: String,
    pub split: CorpusSplit,
    pub suite: EvaluationSuite,
    pub collection: CollectionClass,
    pub inputs: Vec<CorpusArtifact>,
    pub targets: Vec<CorpusArtifact>,
    pub rights: RightsRecord,
    pub leakage: LeakageGroups,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorpusManifest {
    pub schema_version: u32,
    pub profile: String,
    pub corpus_id: String,
    /// Digest of the immutable data/index snapshot, excluding this manifest.
    pub snapshot: ResourceDigest,
    pub dataset_card: ResourceDigest,
    pub evaluator_artifact: ResourceDigest,
    pub examples: Vec<CorpusExample>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorpusAudit {
    pub schema_version: u32,
    pub profile: String,
    pub corpus_id: String,
    pub snapshot: ResourceDigest,
    pub dataset_card: ResourceDigest,
    pub evaluator_artifact: ResourceDigest,
    pub examples: u64,
    pub split_examples: BTreeMap<CorpusSplit, u64>,
    pub suite_examples: BTreeMap<EvaluationSuite, u64>,
    pub artifact_disclosures: BTreeMap<ArtifactDisclosure, u64>,
    pub redistributable_examples: u64,
    pub non_redistributable_examples: u64,
    pub leakage_groups: BTreeMap<String, u64>,
}

impl CorpusManifest {
    /// Audits immutable identity, evidence-suite boundaries, rights declarations,
    /// artifact disclosure and family-level split isolation.
    ///
    /// This verifies internally declared evidence. It does not interpret a
    /// license, prove consent, detect unrecorded near duplicates, or establish
    /// that a corpus is representative.
    ///
    /// # Errors
    ///
    /// Rejects malformed or excessive metadata, duplicate examples, incomplete
    /// evidence, impermissible uses, and artifacts or leakage groups shared by
    /// different splits.
    pub fn audit(&self) -> Result<CorpusAudit, CorpusError> {
        validate_manifest_header(self)?;
        let mut example_ids = BTreeSet::new();
        let mut split_examples = BTreeMap::new();
        let mut suite_examples = BTreeMap::new();
        let mut disclosures = BTreeMap::new();
        let mut groups = BTreeMap::<(String, String), CorpusSplit>::new();
        let mut group_counts = BTreeMap::<String, BTreeSet<String>>::new();
        let mut artifacts = BTreeMap::<ResourceDigest, CorpusSplit>::new();
        let mut metadata_bytes = 0_usize;
        let mut redistributable_examples = 0_u64;

        for example in &self.examples {
            if !example_ids.insert(&example.example_id) {
                return Err(CorpusError::DuplicateExample(example.example_id.clone()));
            }
            validate_example(example, &mut metadata_bytes)?;
            increment(&mut split_examples, example.split);
            increment(&mut suite_examples, example.suite);
            if example.rights.permitted_uses.redistribution == Permission::Allowed {
                redistributable_examples = redistributable_examples.saturating_add(1);
            }

            let mut example_artifacts = BTreeSet::new();
            for artifact in example.inputs.iter().chain(&example.targets) {
                increment(&mut disclosures, artifact.disclosure);
                if !example_artifacts.insert(&artifact.digest) {
                    return Err(CorpusError::DuplicateArtifact {
                        example: example.example_id.clone(),
                        digest: artifact.digest.clone(),
                    });
                }
                if artifact.kind != ArtifactKind::CaptureContext {
                    insert_split_identity(
                        &mut artifacts,
                        artifact.digest.clone(),
                        example.split,
                        |first| CorpusError::ArtifactLeakage {
                            digest: artifact.digest.clone(),
                            first,
                            second: example.split,
                        },
                    )?;
                    if artifacts.len() > MAX_EVALUATION_ITEMS {
                        return Err(CorpusError::ResourceLimit("artifact identities"));
                    }
                }
            }

            for (dimension, value) in leakage_identities(&example.leakage) {
                group_counts
                    .entry(dimension.to_owned())
                    .or_default()
                    .insert(value.to_owned());
                let key = (dimension.to_owned(), value.to_owned());
                insert_split_identity(&mut groups, key, example.split, |first| {
                    CorpusError::GroupLeakage {
                        dimension: dimension.to_owned(),
                        group: value.to_owned(),
                        first,
                        second: example.split,
                    }
                })?;
                if groups.len() > MAX_EVALUATION_ITEMS {
                    return Err(CorpusError::ResourceLimit("leakage identities"));
                }
            }
        }
        let examples = u64::try_from(self.examples.len()).unwrap_or(u64::MAX);
        let non_redistributable_examples = examples.saturating_sub(redistributable_examples);
        Ok(CorpusAudit {
            schema_version: 1,
            profile: CORPUS_AUDIT_PROFILE.to_owned(),
            corpus_id: self.corpus_id.clone(),
            snapshot: self.snapshot.clone(),
            dataset_card: self.dataset_card.clone(),
            evaluator_artifact: self.evaluator_artifact.clone(),
            examples,
            split_examples,
            suite_examples,
            artifact_disclosures: disclosures,
            redistributable_examples,
            non_redistributable_examples,
            leakage_groups: group_counts
                .into_iter()
                .map(|(dimension, values)| {
                    (dimension, u64::try_from(values.len()).unwrap_or(u64::MAX))
                })
                .collect(),
        })
    }
}

impl CorpusAudit {
    /// Confirms that a decoded audit is exactly derivable from a manifest.
    ///
    /// # Errors
    ///
    /// Returns an error when the manifest fails audit or any derived value was
    /// modified.
    pub fn validate_against(&self, manifest: &CorpusManifest) -> Result<(), CorpusError> {
        let expected = manifest.audit()?;
        if self == &expected {
            Ok(())
        } else {
            Err(CorpusError::AuditDrift)
        }
    }
}

fn validate_manifest_header(manifest: &CorpusManifest) -> Result<(), CorpusError> {
    if manifest.schema_version != 1
        || manifest.profile != CORPUS_MANIFEST_PROFILE
        || !bounded_identifier(&manifest.corpus_id)
        || !manifest.snapshot.is_valid()
        || !manifest.dataset_card.is_valid()
        || !manifest.evaluator_artifact.is_valid()
        || manifest.examples.is_empty()
    {
        return Err(CorpusError::InvalidManifest(
            "schema, profile, identities, or examples are invalid",
        ));
    }
    if manifest.examples.len() > MAX_EVALUATION_ITEMS {
        return Err(CorpusError::ResourceLimit("examples"));
    }
    Ok(())
}

fn validate_example(
    example: &CorpusExample,
    metadata_bytes: &mut usize,
) -> Result<(), CorpusError> {
    if !bounded_identifier(&example.example_id)
        || example.inputs.is_empty()
        || example.targets.is_empty()
    {
        return Err(CorpusError::InvalidExample(
            "identity, inputs, or targets are invalid",
        ));
    }
    if example.inputs.len() > MAX_ARTIFACTS_PER_EXAMPLE
        || example.targets.len() > MAX_ARTIFACTS_PER_EXAMPLE
    {
        return Err(CorpusError::ResourceLimit("example artifacts"));
    }
    for artifact in example.inputs.iter().chain(&example.targets) {
        if !artifact.digest.is_valid() {
            return Err(CorpusError::InvalidExample("artifact digest is invalid"));
        }
    }
    if !example
        .inputs
        .iter()
        .any(|artifact| artifact.kind == ArtifactKind::Screenshot)
    {
        return Err(CorpusError::InvalidExample(
            "every reconstruction input requires a screenshot",
        ));
    }
    if !example.targets.iter().any(|artifact| {
        matches!(
            artifact.kind,
            ArtifactKind::TargetDocument | ArtifactKind::Annotation | ArtifactKind::OperationTrace
        )
    }) {
        return Err(CorpusError::InvalidExample(
            "a reconstruction target is missing",
        ));
    }
    validate_suite(example)?;
    validate_rights(example)?;
    validate_groups(&example.leakage, metadata_bytes)?;
    add_metadata_bytes(metadata_bytes, example.example_id.len())?;
    add_metadata_bytes(metadata_bytes, example.rights.license_expression.len())?;
    Ok(())
}

fn validate_suite(example: &CorpusExample) -> Result<(), CorpusError> {
    let has_source = example
        .inputs
        .iter()
        .any(|artifact| artifact.kind == ArtifactKind::SourceDocument);
    let has_resource_bundle = example
        .inputs
        .iter()
        .any(|artifact| artifact.kind == ArtifactKind::ResourceBundle);
    match example.suite {
        EvaluationSuite::SyntheticExact => {
            if example.collection != CollectionClass::ProjectSynthetic
                || example.rights.consent_basis != ConsentBasis::ProjectGenerated
            {
                Err(CorpusError::SuiteEvidence(
                    "synthetic-exact examples must be project-generated",
                ))
            } else {
                Ok(())
            }
        }
        EvaluationSuite::RealScreenshot => {
            if has_source || has_resource_bundle {
                Err(CorpusError::SuiteEvidence(
                    "screenshot-only examples cannot carry exact source or resource bundles",
                ))
            } else {
                Ok(())
            }
        }
        EvaluationSuite::SourceBacked => {
            if has_source {
                Ok(())
            } else {
                Err(CorpusError::SuiteEvidence(
                    "source-backed examples require an exact source document",
                ))
            }
        }
    }
}

fn validate_rights(example: &CorpusExample) -> Result<(), CorpusError> {
    let rights = &example.rights;
    if rights.license_expression.trim().is_empty()
        || rights.license_expression.len() > MAX_CORPUS_STRING_BYTES
        || !rights.evidence_artifact.is_valid()
        || rights
            .withdrawal_policy_artifact
            .as_ref()
            .is_some_and(|digest| !digest.is_valid())
    {
        return Err(CorpusError::InvalidRights(
            "license or evidence identity is invalid",
        ));
    }
    let required_use = match example.split {
        CorpusSplit::Adaptation => (rights.permitted_uses.adaptation, "adaptation"),
        CorpusSplit::Calibration => (rights.permitted_uses.calibration, "calibration"),
        CorpusSplit::Validation | CorpusSplit::Test => {
            (rights.permitted_uses.evaluation, "evaluation")
        }
    };
    if required_use.0 != Permission::Allowed {
        return Err(CorpusError::UseNotPermitted {
            example: example.example_id.clone(),
            usage: required_use.1,
        });
    }
    if example.collection == CollectionClass::ProjectSynthetic
        && (rights.consent_basis != ConsentBasis::ProjectGenerated
            || rights.sensitivity_review != SensitivityReview::SyntheticNoPersonalData)
    {
        return Err(CorpusError::InvalidRights(
            "project-synthetic records require generated/no-personal-data evidence",
        ));
    }
    if example.collection != CollectionClass::ProjectSynthetic
        && rights.sensitivity_review == SensitivityReview::SyntheticNoPersonalData
    {
        return Err(CorpusError::InvalidRights(
            "real records require a human sensitivity review",
        ));
    }
    if example.collection != CollectionClass::ProjectSynthetic
        && rights.withdrawal_policy_artifact.is_none()
    {
        return Err(CorpusError::InvalidRights(
            "retained real records require a withdrawal policy",
        ));
    }
    if example.collection == CollectionClass::PrivateAuthenticated
        && !matches!(
            rights.consent_basis,
            ConsentBasis::ExplicitOptIn | ConsentBasis::ContractualAuthorization
        )
    {
        return Err(CorpusError::InvalidRights(
            "private/authenticated records require explicit authorization",
        ));
    }
    Ok(())
}

fn validate_groups(groups: &LeakageGroups, metadata_bytes: &mut usize) -> Result<(), CorpusError> {
    if !bounded_identifier(&groups.origin)
        || groups
            .template
            .as_ref()
            .is_some_and(|value| !bounded_identifier(value))
        || groups.near_duplicates.is_empty()
    {
        return Err(CorpusError::InvalidGroups(
            "origin, template, or near-duplicate identity is invalid",
        ));
    }
    for (name, values) in [
        ("component", &groups.components),
        ("font", &groups.fonts),
        ("resource", &groups.resources),
        ("generator", &groups.generators),
        ("near_duplicate", &groups.near_duplicates),
    ] {
        if values.len() > MAX_GROUPS_PER_DIMENSION
            || values.iter().any(|value| !bounded_identifier(value))
        {
            return Err(CorpusError::InvalidGroups(name));
        }
    }
    for (dimension, value) in leakage_identities(groups) {
        add_metadata_bytes(metadata_bytes, dimension.len())?;
        add_metadata_bytes(metadata_bytes, value.len())?;
    }
    Ok(())
}

fn leakage_identities(groups: &LeakageGroups) -> impl Iterator<Item = (&'static str, &str)> {
    std::iter::once(("origin", groups.origin.as_str()))
        .chain(
            groups
                .template
                .iter()
                .map(|value| ("template", value.as_str())),
        )
        .chain(
            groups
                .components
                .iter()
                .map(|value| ("component", value.as_str())),
        )
        .chain(groups.fonts.iter().map(|value| ("font", value.as_str())))
        .chain(
            groups
                .resources
                .iter()
                .map(|value| ("resource", value.as_str())),
        )
        .chain(
            groups
                .generators
                .iter()
                .map(|value| ("generator", value.as_str())),
        )
        .chain(
            groups
                .near_duplicates
                .iter()
                .map(|value| ("near_duplicate", value.as_str())),
        )
}

fn bounded_identifier(value: &str) -> bool {
    value.len() <= MAX_CORPUS_STRING_BYTES && is_identifier(value)
}

fn add_metadata_bytes(total: &mut usize, bytes: usize) -> Result<(), CorpusError> {
    *total = total
        .checked_add(bytes)
        .ok_or(CorpusError::ResourceLimit("metadata bytes"))?;
    if *total > MAX_CORPUS_METADATA_BYTES {
        Err(CorpusError::ResourceLimit("metadata bytes"))
    } else {
        Ok(())
    }
}

fn increment<K: Ord>(counts: &mut BTreeMap<K, u64>, key: K) {
    let count = counts.entry(key).or_default();
    *count = count.saturating_add(1);
}

fn insert_split_identity<K: Ord>(
    identities: &mut BTreeMap<K, CorpusSplit>,
    identity: K,
    split: CorpusSplit,
    error: impl FnOnce(CorpusSplit) -> CorpusError,
) -> Result<(), CorpusError> {
    if let Some(first) = identities.get(&identity).copied()
        && first != split
    {
        return Err(error(first));
    }
    identities.insert(identity, split);
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum CorpusError {
    #[error("reconstruction corpus manifest is invalid: {0}")]
    InvalidManifest(&'static str),
    #[error("reconstruction corpus example is invalid: {0}")]
    InvalidExample(&'static str),
    #[error("reconstruction corpus rights record is invalid: {0}")]
    InvalidRights(&'static str),
    #[error("reconstruction corpus leakage groups are invalid: {0}")]
    InvalidGroups(&'static str),
    #[error("reconstruction corpus evidence suite is invalid: {0}")]
    SuiteEvidence(&'static str),
    #[error("reconstruction corpus resource limit exceeded for {0}")]
    ResourceLimit(&'static str),
    #[error("duplicate reconstruction corpus example {0}")]
    DuplicateExample(String),
    #[error("example {example} repeats artifact {digest}")]
    DuplicateArtifact {
        example: String,
        digest: ResourceDigest,
    },
    #[error("artifact {digest} leaks from {first:?} into {second:?}")]
    ArtifactLeakage {
        digest: ResourceDigest,
        first: CorpusSplit,
        second: CorpusSplit,
    },
    #[error("{dimension} group {group} leaks from {first:?} into {second:?}")]
    GroupLeakage {
        dimension: String,
        group: String,
        first: CorpusSplit,
        second: CorpusSplit,
    },
    #[error("example {example} does not permit {usage}")]
    UseNotPermitted {
        example: String,
        usage: &'static str,
    },
    #[error("decoded reconstruction corpus audit differs from the manifest")]
    AuditDrift,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(value: u8) -> ResourceDigest {
        ResourceDigest::from_sha256_hex(format!("{value:02x}").repeat(32))
    }

    fn example(id: &str, split: CorpusSplit, seed: u8) -> CorpusExample {
        CorpusExample {
            example_id: id.to_owned(),
            split,
            suite: EvaluationSuite::SyntheticExact,
            collection: CollectionClass::ProjectSynthetic,
            inputs: vec![CorpusArtifact {
                kind: ArtifactKind::Screenshot,
                digest: digest(seed),
                disclosure: ArtifactDisclosure::Public,
            }],
            targets: vec![CorpusArtifact {
                kind: ArtifactKind::TargetDocument,
                digest: digest(seed.saturating_add(1)),
                disclosure: ArtifactDisclosure::Restricted,
            }],
            rights: RightsRecord {
                license_expression: "Apache-2.0 OR MIT".to_owned(),
                evidence_artifact: digest(seed.saturating_add(2)),
                consent_basis: ConsentBasis::ProjectGenerated,
                permitted_uses: PermittedUses {
                    evaluation: Permission::Allowed,
                    calibration: Permission::Allowed,
                    adaptation: Permission::Allowed,
                    redistribution: Permission::Allowed,
                },
                sensitivity_review: SensitivityReview::SyntheticNoPersonalData,
                withdrawal_policy_artifact: None,
            },
            leakage: LeakageGroups {
                origin: format!("origin-{seed}"),
                template: Some(format!("template-{seed}")),
                components: BTreeSet::from([format!("component-{seed}")]),
                fonts: BTreeSet::from([format!("font-{seed}")]),
                resources: BTreeSet::from([format!("resource-{seed}")]),
                generators: BTreeSet::from([format!("generator-{seed}")]),
                near_duplicates: BTreeSet::from([format!("near-{seed}")]),
            },
        }
    }

    fn manifest() -> CorpusManifest {
        CorpusManifest {
            schema_version: 1,
            profile: CORPUS_MANIFEST_PROFILE.to_owned(),
            corpus_id: "synthetic-corpus-1".to_owned(),
            snapshot: digest(240),
            dataset_card: digest(241),
            evaluator_artifact: digest(242),
            examples: vec![
                example("adaptation-1", CorpusSplit::Adaptation, 10),
                example("test-1", CorpusSplit::Test, 20),
            ],
        }
    }

    #[test]
    fn audit_is_derived_and_round_trips() {
        let manifest = manifest();
        let audit = manifest.audit().unwrap();
        assert_eq!(audit.examples, 2);
        assert_eq!(audit.split_examples[&CorpusSplit::Test], 1);
        assert_eq!(audit.leakage_groups["origin"], 2);
        let bytes = serde_json::to_vec(&audit).unwrap();
        let decoded: CorpusAudit = serde_json::from_slice(&bytes).unwrap();
        decoded.validate_against(&manifest).unwrap();
    }

    #[test]
    fn every_leakage_dimension_is_split_isolated() {
        for dimension in [
            "origin",
            "template",
            "component",
            "font",
            "resource",
            "generator",
            "near_duplicate",
        ] {
            let mut manifest = manifest();
            let shared = "shared-family".to_owned();
            match dimension {
                "origin" => shared.clone_into(&mut manifest.examples[1].leakage.origin),
                "template" => manifest.examples[1].leakage.template = Some(shared.clone()),
                "component" => {
                    manifest.examples[1]
                        .leakage
                        .components
                        .insert(shared.clone());
                }
                "font" => {
                    manifest.examples[1].leakage.fonts.insert(shared.clone());
                }
                "resource" => {
                    manifest.examples[1]
                        .leakage
                        .resources
                        .insert(shared.clone());
                }
                "generator" => {
                    manifest.examples[1]
                        .leakage
                        .generators
                        .insert(shared.clone());
                }
                "near_duplicate" => {
                    manifest.examples[1]
                        .leakage
                        .near_duplicates
                        .insert(shared.clone());
                }
                _ => unreachable!(),
            }
            match dimension {
                "origin" => shared.clone_into(&mut manifest.examples[0].leakage.origin),
                "template" => manifest.examples[0].leakage.template = Some(shared),
                "component" => {
                    manifest.examples[0].leakage.components.insert(shared);
                }
                "font" => {
                    manifest.examples[0].leakage.fonts.insert(shared);
                }
                "resource" => {
                    manifest.examples[0].leakage.resources.insert(shared);
                }
                "generator" => {
                    manifest.examples[0].leakage.generators.insert(shared);
                }
                "near_duplicate" => {
                    manifest.examples[0].leakage.near_duplicates.insert(shared);
                }
                _ => unreachable!(),
            }
            assert!(matches!(
                manifest.audit(),
                Err(CorpusError::GroupLeakage {
                    dimension: leaked,
                    ..
                }) if leaked == dimension
            ));
        }
    }

    #[test]
    fn artifacts_and_permissions_cannot_cross_boundaries() {
        let mut leaked = manifest();
        leaked.examples[1].inputs[0].digest = leaked.examples[0].inputs[0].digest.clone();
        assert!(matches!(
            leaked.audit(),
            Err(CorpusError::ArtifactLeakage { .. })
        ));

        let mut forbidden = manifest();
        forbidden.examples[0].rights.permitted_uses.adaptation = Permission::Prohibited;
        assert!(matches!(
            forbidden.audit(),
            Err(CorpusError::UseNotPermitted {
                usage: "adaptation",
                ..
            })
        ));

        let mut uncalibrated = manifest();
        uncalibrated.examples[0].split = CorpusSplit::Calibration;
        uncalibrated.examples[0].rights.permitted_uses.calibration = Permission::Prohibited;
        assert!(matches!(
            uncalibrated.audit(),
            Err(CorpusError::UseNotPermitted {
                usage: "calibration",
                ..
            })
        ));
    }

    #[test]
    fn suite_and_private_capture_claims_are_constrained() {
        let mut screenshot = example("real-1", CorpusSplit::Test, 30);
        screenshot.suite = EvaluationSuite::RealScreenshot;
        screenshot.collection = CollectionClass::PublicWeb;
        screenshot.rights.consent_basis = ConsentBasis::ContractualAuthorization;
        screenshot.rights.sensitivity_review = SensitivityReview::HumanReviewedNoSensitiveData;
        screenshot.inputs.push(CorpusArtifact {
            kind: ArtifactKind::SourceDocument,
            digest: digest(40),
            disclosure: ArtifactDisclosure::Restricted,
        });
        let mut corpus = manifest();
        corpus.examples = vec![screenshot];
        assert!(matches!(corpus.audit(), Err(CorpusError::SuiteEvidence(_))));

        let mut private = example("private-1", CorpusSplit::Test, 50);
        private.suite = EvaluationSuite::RealScreenshot;
        private.collection = CollectionClass::PrivateAuthenticated;
        private.rights.consent_basis = ConsentBasis::ExplicitOptIn;
        private.rights.sensitivity_review = SensitivityReview::HumanReviewedRestricted;
        private.rights.withdrawal_policy_artifact = None;
        corpus.examples = vec![private];
        assert!(matches!(corpus.audit(), Err(CorpusError::InvalidRights(_))));
    }
}
