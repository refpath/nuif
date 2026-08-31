#![doc = "Model-neutral observation, reconstruction, calibration, and correction contracts."]

pub mod evaluation;
pub mod layout_inference;

use nuif_codec::{
    CodecError, Encoder, canonical_hash, decode_canonical_record, encode_canonical_record,
};
use nuif_core::{AffineTransform, AssetId, Document, EntityId, Fidelity, ResourceDigest};
use nuif_protocol::{ApplyError, Operation, Patch, apply_patch};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};
use thiserror::Error;

pub const OBSERVATION_PROFILE: &str = "nuif-observations-0";
pub const MAX_OBSERVATIONS: usize = 100_000;
pub const MAX_OMISSIONS: usize = 16_384;
pub const MAX_CONTEXT_PROPERTIES: usize = 64;
pub const MAX_OBSERVATION_STRING_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_OBSERVATION_SINGLE_STRING_BYTES: usize = 1024 * 1024;
pub const MAX_PROPOSAL_OPERATIONS: usize = 16_384;
pub const MAX_PROPOSAL_TRANSACTIONS: usize = 1_024;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ObservationId(pub String);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    AuthoredSource,
    ResolvedSource,
    ObservedPixels,
    Inferred,
    UserConfirmed,
    Derived,
    Unavailable,
}

impl EvidenceClass {
    #[must_use]
    pub fn fidelity_ceiling(self) -> Fidelity {
        match self {
            Self::AuthoredSource | Self::ResolvedSource => Fidelity::Lossless,
            Self::UserConfirmed => Fidelity::Representable,
            Self::ObservedPixels | Self::Inferred | Self::Derived => Fidelity::Approximated {
                reason: "evidence does not prove original authored semantics".to_owned(),
            },
            Self::Unavailable => Fidelity::Unsupported {
                reason: "required evidence is unavailable".to_owned(),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "space")]
pub enum CoordinateSpace {
    SourcePixels { source: String },
    DevicePixels { scale_factor: f64 },
    ViewportCssPixels { viewport: String },
    CropLocalPixels { resource: ResourceDigest },
    NuifLogical { document: EntityId },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoordinateTransform {
    pub from: CoordinateSpace,
    pub to: CoordinateSpace,
    pub affine: AffineTransform,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Subject {
    Document,
    Entity { entity: EntityId },
    Property { entity: EntityId, pointer: String },
    Asset { asset: AssetId },
    Resource { digest: ResourceDigest },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ObservationValue {
    Text {
        content: String,
        bounds: Option<Bounds>,
    },
    Geometry {
        bounds: Bounds,
    },
    Color {
        rgba: [f32; 4],
    },
    Hierarchy {
        parent: Option<ObservationId>,
        order: u32,
    },
    Accessibility {
        role: String,
        name: Option<String>,
    },
    FontUse {
        family: String,
        postscript_name: String,
        glyph_count: u32,
        custom: bool,
    },
    Resource {
        digest: Option<ResourceDigest>,
        media_type: Option<String>,
        size: Option<u64>,
    },
    SourceSpan {
        uri: String,
        start: u64,
        end: u64,
    },
    Boolean {
        value: bool,
    },
    Number {
        value: f64,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Confidence {
    pub raw: f64,
    pub calibrated: Option<f64>,
    pub calibration_artifact: Option<ResourceDigest>,
}

impl Confidence {
    #[must_use]
    pub const fn raw(value: f64) -> Self {
        Self {
            raw: value,
            calibrated: None,
            calibration_artifact: None,
        }
    }

    #[must_use]
    pub fn decision_value(&self) -> Option<f64> {
        self.calibrated
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub id: ObservationId,
    pub evidence: EvidenceClass,
    pub subject: Option<Subject>,
    pub coordinate_space: Option<CoordinateSpace>,
    pub transform: Option<CoordinateTransform>,
    pub value: ObservationValue,
    pub confidence: Option<Confidence>,
    pub source: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Omission {
    pub category: String,
    pub reason: String,
    pub affected: Option<Subject>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureContext {
    pub profile: String,
    pub properties: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationBundle {
    pub schema_version: u32,
    pub profile: String,
    pub capture_id: String,
    pub adapter: String,
    pub adapter_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<CaptureContext>,
    pub observations: Vec<Observation>,
    #[serde(default)]
    pub omissions: Vec<Omission>,
}

impl ObservationBundle {
    /// Validates evidence, coordinate, confidence, identity, and resource
    /// budgets without interpreting observations as document semantics.
    ///
    /// # Errors
    ///
    /// Returns every retained validation issue up to the bundle bounds.
    pub fn validate(&self) -> Result<(), ObservationError> {
        if self.schema_version != 1 || self.profile != OBSERVATION_PROFILE {
            return Err(ObservationError::InvalidBundle(
                "unsupported observation schema or profile".to_owned(),
            ));
        }
        if !identifier(&self.capture_id)
            || !identifier(&self.adapter)
            || self.adapter_version.is_empty()
        {
            return Err(ObservationError::InvalidBundle(
                "capture, adapter, and adapter-version identities are invalid".to_owned(),
            ));
        }
        if self.observations.len() > MAX_OBSERVATIONS {
            return Err(ObservationError::ResourceLimit {
                resource: "observations",
                limit: MAX_OBSERVATIONS,
                observed: self.observations.len(),
            });
        }
        if self.omissions.len() > MAX_OMISSIONS {
            return Err(ObservationError::ResourceLimit {
                resource: "omissions",
                limit: MAX_OMISSIONS,
                observed: self.omissions.len(),
            });
        }
        if let Some(context) = &self.context {
            if !identifier(&context.profile) {
                return Err(ObservationError::InvalidBundle(
                    "capture context profile is not an identifier".to_owned(),
                ));
            }
            if context.properties.len() > MAX_CONTEXT_PROPERTIES {
                return Err(ObservationError::ResourceLimit {
                    resource: "capture context properties",
                    limit: MAX_CONTEXT_PROPERTIES,
                    observed: context.properties.len(),
                });
            }
        }
        let mut ids = BTreeSet::new();
        let mut string_bytes = 0_usize;
        add_string(&mut string_bytes, &self.capture_id)?;
        add_string(&mut string_bytes, &self.adapter)?;
        add_string(&mut string_bytes, &self.adapter_version)?;
        if let Some(context) = &self.context {
            add_string(&mut string_bytes, &context.profile)?;
            for (name, value) in &context.properties {
                if !identifier(name) {
                    return Err(ObservationError::InvalidBundle(format!(
                        "capture context property {name:?} is not an identifier"
                    )));
                }
                add_string(&mut string_bytes, name)?;
                add_string(&mut string_bytes, value)?;
            }
        }
        for observation in &self.observations {
            validate_observation(observation, &mut ids, &mut string_bytes)?;
        }
        for omission in &self.omissions {
            if !identifier(&omission.category) {
                return Err(ObservationError::InvalidBundle(format!(
                    "omission category {:?} is not an identifier",
                    omission.category
                )));
            }
            add_string(&mut string_bytes, &omission.category)?;
            add_string(&mut string_bytes, &omission.reason)?;
        }
        for observation in &self.observations {
            if let ObservationValue::Hierarchy {
                parent: Some(parent),
                ..
            } = &observation.value
                && !ids.contains(parent)
            {
                return Err(ObservationError::InvalidBundle(format!(
                    "observation {} references missing parent {}",
                    observation.id.0, parent.0
                )));
            }
        }
        Ok(())
    }

    /// Encodes a validated bundle as canonical deterministic CBOR.
    ///
    /// # Errors
    ///
    /// Returns a bundle-validation or canonical-codec error.
    pub fn encode(&self) -> Result<Vec<u8>, ObservationError> {
        self.validate()?;
        encode_canonical_record(self).map_err(Into::into)
    }

    /// Decodes canonical CBOR and validates the complete bundle.
    ///
    /// # Errors
    ///
    /// Returns a canonical-codec or bundle-validation error.
    pub fn decode(bytes: &[u8]) -> Result<Self, ObservationError> {
        let bundle: Self = decode_canonical_record(bytes)?;
        bundle.validate()?;
        Ok(bundle)
    }

    #[must_use]
    pub fn ids(&self) -> BTreeSet<ObservationId> {
        self.observations
            .iter()
            .map(|observation| observation.id.clone())
            .collect()
    }

    #[must_use]
    pub fn observed_resource_digests(&self) -> BTreeSet<ResourceDigest> {
        self.observations
            .iter()
            .filter_map(|observation| match &observation.value {
                ObservationValue::Resource {
                    digest: Some(digest),
                    ..
                } => Some(digest.clone()),
                _ => None,
            })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Error)]
pub enum ObservationError {
    #[error(transparent)]
    Codec(#[from] CodecError),
    #[error("invalid observation bundle: {0}")]
    InvalidBundle(String),
    #[error(
        "observation resource limit exceeded for {resource}: limit {limit}, observed {observed}"
    )]
    ResourceLimit {
        resource: &'static str,
        limit: usize,
        observed: usize,
    },
}

fn validate_observation(
    observation: &Observation,
    ids: &mut BTreeSet<ObservationId>,
    string_bytes: &mut usize,
) -> Result<(), ObservationError> {
    if !identifier(&observation.id.0) || !ids.insert(observation.id.clone()) {
        return Err(ObservationError::InvalidBundle(format!(
            "observation identity {:?} is invalid or duplicated",
            observation.id.0
        )));
    }
    add_string(string_bytes, &observation.id.0)?;
    add_string(string_bytes, &observation.source)?;
    if let Some(confidence) = &observation.confidence {
        validate_confidence(confidence)?;
        if matches!(
            observation.evidence,
            EvidenceClass::AuthoredSource
                | EvidenceClass::ResolvedSource
                | EvidenceClass::Unavailable
        ) {
            return Err(ObservationError::InvalidBundle(format!(
                "observation {} attaches inference confidence to non-inferred evidence",
                observation.id.0
            )));
        }
    }
    if let Some(space) = &observation.coordinate_space {
        validate_coordinate_space(space, string_bytes)?;
    }
    if let Some(transform) = &observation.transform {
        validate_coordinate_space(&transform.from, string_bytes)?;
        validate_coordinate_space(&transform.to, string_bytes)?;
        if !affine_finite(transform.affine) {
            return Err(ObservationError::InvalidBundle(format!(
                "observation {} has a non-finite coordinate transform",
                observation.id.0
            )));
        }
    }
    validate_value(&observation.value, string_bytes)
}

fn validate_confidence(confidence: &Confidence) -> Result<(), ObservationError> {
    if !probability(confidence.raw)
        || confidence
            .calibrated
            .is_some_and(|value| !probability(value))
        || confidence.calibrated.is_some() != confidence.calibration_artifact.is_some()
        || confidence
            .calibration_artifact
            .as_ref()
            .is_some_and(|digest| !digest.is_valid())
    {
        return Err(ObservationError::InvalidBundle(
            "raw and calibrated confidence must be separate probabilities, and calibrated values require a valid artifact digest"
                .to_owned(),
        ));
    }
    Ok(())
}

fn validate_coordinate_space(
    space: &CoordinateSpace,
    string_bytes: &mut usize,
) -> Result<(), ObservationError> {
    match space {
        CoordinateSpace::SourcePixels { source } => add_string(string_bytes, source),
        CoordinateSpace::DevicePixels { scale_factor } => {
            if scale_factor.is_finite() && *scale_factor > 0.0 {
                Ok(())
            } else {
                Err(ObservationError::InvalidBundle(
                    "device-pixel scale factor must be finite and positive".to_owned(),
                ))
            }
        }
        CoordinateSpace::ViewportCssPixels { viewport } => add_string(string_bytes, viewport),
        CoordinateSpace::CropLocalPixels { resource } => {
            if resource.is_valid() {
                Ok(())
            } else {
                Err(ObservationError::InvalidBundle(
                    "crop-local coordinate space has an invalid resource digest".to_owned(),
                ))
            }
        }
        CoordinateSpace::NuifLogical { .. } => Ok(()),
    }
}

fn validate_value(
    value: &ObservationValue,
    string_bytes: &mut usize,
) -> Result<(), ObservationError> {
    match value {
        ObservationValue::Text { content, bounds } => {
            add_string(string_bytes, content)?;
            bounds.map_or(Ok(()), validate_bounds)
        }
        ObservationValue::Geometry { bounds } => validate_bounds(*bounds),
        ObservationValue::Color { rgba } => {
            if rgba
                .iter()
                .all(|channel| channel.is_finite() && (0.0..=1.0).contains(channel))
            {
                Ok(())
            } else {
                Err(ObservationError::InvalidBundle(
                    "observed color channels must be finite and within 0..=1".to_owned(),
                ))
            }
        }
        ObservationValue::Hierarchy { .. } | ObservationValue::Boolean { .. } => Ok(()),
        ObservationValue::Accessibility { role, name } => {
            add_string(string_bytes, role)?;
            if let Some(name) = name {
                add_string(string_bytes, name)?;
            }
            Ok(())
        }
        ObservationValue::FontUse {
            family,
            postscript_name,
            ..
        } => {
            add_string(string_bytes, family)?;
            add_string(string_bytes, postscript_name)
        }
        ObservationValue::Resource {
            digest, media_type, ..
        } => {
            if digest.as_ref().is_some_and(|digest| !digest.is_valid()) {
                return Err(ObservationError::InvalidBundle(
                    "observed resource digest is invalid".to_owned(),
                ));
            }
            if let Some(media_type) = media_type {
                add_string(string_bytes, media_type)?;
            }
            Ok(())
        }
        ObservationValue::SourceSpan { uri, start, end } => {
            add_string(string_bytes, uri)?;
            if start <= end {
                Ok(())
            } else {
                Err(ObservationError::InvalidBundle(
                    "source span start exceeds end".to_owned(),
                ))
            }
        }
        ObservationValue::Number { value } => {
            if value.is_finite() {
                Ok(())
            } else {
                Err(ObservationError::InvalidBundle(
                    "observed number must be finite".to_owned(),
                ))
            }
        }
    }
}

fn validate_bounds(bounds: Bounds) -> Result<(), ObservationError> {
    if [bounds.x, bounds.y, bounds.width, bounds.height]
        .into_iter()
        .all(f64::is_finite)
        && bounds.width >= 0.0
        && bounds.height >= 0.0
    {
        Ok(())
    } else {
        Err(ObservationError::InvalidBundle(
            "observation bounds must be finite with non-negative dimensions".to_owned(),
        ))
    }
}

fn add_string(total: &mut usize, value: &str) -> Result<(), ObservationError> {
    if value.len() > MAX_OBSERVATION_SINGLE_STRING_BYTES {
        return Err(ObservationError::ResourceLimit {
            resource: "single observation string bytes",
            limit: MAX_OBSERVATION_SINGLE_STRING_BYTES,
            observed: value.len(),
        });
    }
    *total = total.saturating_add(value.len());
    if *total > MAX_OBSERVATION_STRING_BYTES {
        Err(ObservationError::ResourceLimit {
            resource: "observation string bytes",
            limit: MAX_OBSERVATION_STRING_BYTES,
            observed: *total,
        })
    } else {
        Ok(())
    }
}

fn affine_finite(affine: AffineTransform) -> bool {
    [affine.a, affine.b, affine.c, affine.d, affine.tx, affine.ty]
        .into_iter()
        .all(f64::is_finite)
}

fn probability(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn identifier(value: &str) -> bool {
    nuif_core::is_identifier(value)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InferenceProvenance {
    pub method: String,
    pub artifact: Option<ResourceDigest>,
    pub observations: BTreeSet<ObservationId>,
    pub confidence: Confidence,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    pub schema_version: u32,
    pub provenance: InferenceProvenance,
    pub patch: Patch,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProposalPolicy {
    pub max_transactions: usize,
    pub max_operations: usize,
    pub allow_remove: bool,
    pub allow_flattened_screenshot: bool,
    pub protected_entities: BTreeSet<EntityId>,
}

impl Default for ProposalPolicy {
    fn default() -> Self {
        Self {
            max_transactions: MAX_PROPOSAL_TRANSACTIONS,
            max_operations: MAX_PROPOSAL_OPERATIONS,
            allow_remove: false,
            allow_flattened_screenshot: false,
            protected_entities: BTreeSet::new(),
        }
    }
}

/// Validates and atomically applies one reconstruction proposal through the
/// core operation path.
///
/// # Errors
///
/// Rejects ungrounded evidence, unobserved resource bindings, forbidden
/// operations, budget overflow, stale revisions, or invalid results.
pub fn apply_proposal(
    document: &mut Document,
    observations: &ObservationBundle,
    proposal: &Proposal,
    policy: &ProposalPolicy,
) -> Result<(), ReconstructionError> {
    observations.validate()?;
    validate_proposal(document, observations, proposal, policy)?;
    apply_patch(document, &proposal.patch)?;
    Ok(())
}

fn validate_proposal(
    document: &Document,
    observations: &ObservationBundle,
    proposal: &Proposal,
    policy: &ProposalPolicy,
) -> Result<(), ReconstructionError> {
    if proposal.schema_version != 1 || !identifier(&proposal.provenance.method) {
        return Err(ReconstructionError::InvalidProposal(
            "unsupported proposal version or invalid method identifier".to_owned(),
        ));
    }
    validate_confidence(&proposal.provenance.confidence)?;
    if proposal.patch.base_revision.as_deref() != Some(canonical_hash(document)?.as_str()) {
        return Err(ReconstructionError::InvalidProposal(
            "proposal must pin the current canonical base revision".to_owned(),
        ));
    }
    let ids = observations.ids();
    if proposal.provenance.observations.is_empty()
        || !proposal
            .provenance
            .observations
            .iter()
            .all(|id| ids.contains(id))
    {
        return Err(ReconstructionError::InvalidProposal(
            "proposal evidence is empty or references unknown observations".to_owned(),
        ));
    }
    if proposal.patch.transactions.len() > policy.max_transactions {
        return Err(ReconstructionError::BudgetExceeded("transactions"));
    }
    let operation_count = proposal
        .patch
        .transactions
        .iter()
        .map(|transaction| transaction.operations.len())
        .sum::<usize>();
    if operation_count > policy.max_operations {
        return Err(ReconstructionError::BudgetExceeded("operations"));
    }
    let observed_resources = observations.observed_resource_digests();
    let screenshot_resources = observations
        .observations
        .iter()
        .filter(|observation| observation.evidence == EvidenceClass::ObservedPixels)
        .filter_map(|observation| match &observation.value {
            ObservationValue::Resource {
                digest: Some(digest),
                ..
            } => Some(digest.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    for operation in proposal
        .patch
        .transactions
        .iter()
        .flat_map(|transaction| &transaction.operations)
    {
        validate_proposed_operation(
            operation,
            document,
            policy,
            &observed_resources,
            &screenshot_resources,
        )?;
    }
    Ok(())
}

fn validate_proposed_operation(
    operation: &Operation,
    document: &Document,
    policy: &ProposalPolicy,
    observed_resources: &BTreeSet<ResourceDigest>,
    screenshot_resources: &BTreeSet<ResourceDigest>,
) -> Result<(), ReconstructionError> {
    match operation {
        Operation::Insert { entity, .. } => {
            if !entity.extensions.0.is_empty() {
                return Err(ReconstructionError::ForbiddenOperation(
                    "insert_with_extensions",
                ));
            }
        }
        Operation::Remove { entity } => {
            if !policy.allow_remove || policy.protected_entities.contains(entity) {
                return Err(ReconstructionError::ForbiddenOperation("remove"));
            }
        }
        Operation::Move { entity, .. } => {
            if policy.protected_entities.contains(entity) {
                return Err(ReconstructionError::ForbiddenOperation("move_protected"));
            }
        }
        Operation::SetAsset { asset } => {
            if asset
                .resource
                .as_ref()
                .is_some_and(|digest| !observed_resources.contains(digest))
            {
                return Err(ReconstructionError::UnobservedResource);
            }
            if !policy.allow_flattened_screenshot
                && asset
                    .resource
                    .as_ref()
                    .is_some_and(|digest| screenshot_resources.contains(digest))
            {
                return Err(ReconstructionError::ForbiddenOperation(
                    "flattened_screenshot_asset",
                ));
            }
        }
        Operation::BindAssetResource {
            digest: Some(digest),
            ..
        } => {
            if !observed_resources.contains(digest) {
                return Err(ReconstructionError::UnobservedResource);
            }
            if !policy.allow_flattened_screenshot && screenshot_resources.contains(digest) {
                return Err(ReconstructionError::ForbiddenOperation(
                    "flattened_screenshot_binding",
                ));
            }
        }
        Operation::SetExtensionDeclarations { .. }
        | Operation::SetExtension { .. }
        | Operation::RemoveExtension { .. }
        | Operation::SetUnknownPayload { .. }
        | Operation::RestoreSubtree { .. } => {
            return Err(ReconstructionError::ForbiddenOperation(
                "extension_or_internal",
            ));
        }
        Operation::Rename { .. }
        | Operation::SetSize { .. }
        | Operation::SetLayout { .. }
        | Operation::SetGridPlacement { .. }
        | Operation::SetPosition { .. }
        | Operation::SetFill { .. }
        | Operation::SetText { .. }
        | Operation::SetImage { .. }
        | Operation::RemoveAsset { .. }
        | Operation::SetToken { .. }
        | Operation::RemoveToken { .. }
        | Operation::SetValue { .. }
        | Operation::RemoveValue { .. }
        | Operation::BindAssetResource { digest: None, .. } => {}
    }
    if let Operation::SetImage {
        value: Some(image), ..
    } = operation
        && !policy.allow_flattened_screenshot
        && document
            .assets
            .get(&image.asset)
            .and_then(|asset| asset.resource.as_ref())
            .is_some_and(|digest| screenshot_resources.contains(digest))
    {
        return Err(ReconstructionError::ForbiddenOperation(
            "flattened_screenshot_image",
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Error)]
pub enum ReconstructionError {
    #[error(transparent)]
    Observation(#[from] ObservationError),
    #[error(transparent)]
    Codec(#[from] CodecError),
    #[error(transparent)]
    Apply(#[from] ApplyError),
    #[error("invalid proposal: {0}")]
    InvalidProposal(String),
    #[error("proposal budget exceeded for {0}")]
    BudgetExceeded(&'static str),
    #[error("proposal operation {0} is forbidden by the reconstruction grammar")]
    ForbiddenOperation(&'static str),
    #[error("proposal binds a resource digest absent from its observations")]
    UnobservedResource,
    #[error("candidate evaluator failed: {0}")]
    Evaluator(String),
    #[error("candidate proposer failed: {0}")]
    Proposer(String),
    #[error("candidate score is non-finite or lacks a protected metric")]
    InvalidScore,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateScore {
    pub objective: f64,
    #[serde(default)]
    pub metrics: BTreeMap<String, f64>,
}

pub trait CandidateEvaluator {
    /// Evaluates semantic and rendered candidate quality; lower is better.
    ///
    /// # Errors
    ///
    /// Returns an implementation-specific bounded evaluation failure.
    fn evaluate(&mut self, document: &Document) -> Result<CandidateScore, String>;
}

pub trait CorrectionProvider {
    /// Proposes the next typed transaction or returns `None` when exhausted.
    ///
    /// # Errors
    ///
    /// Returns a provider/tool failure. Implementations receive no implicit
    /// filesystem, network, or mutation authority from this interface.
    fn propose(
        &mut self,
        document: &Document,
        observations: &ObservationBundle,
        iteration: usize,
    ) -> Result<Option<Proposal>, String>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProtectedMetric {
    pub name: String,
    pub max_regression: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoopBudget {
    pub max_iterations: usize,
    pub max_provider_calls: usize,
    pub max_millis: u64,
    pub max_estimated_bytes: usize,
    pub proposal_policy: ProposalPolicy,
    pub protected_metrics: Vec<ProtectedMetric>,
}

impl Default for LoopBudget {
    fn default() -> Self {
        Self {
            max_iterations: 8,
            max_provider_calls: 8,
            max_millis: 30_000,
            max_estimated_bytes: 32 * 1024 * 1024,
            proposal_policy: ProposalPolicy::default(),
            protected_metrics: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopStatus {
    NoProposal,
    RepeatedState,
    IterationBudget,
    ProviderCallBudget,
    TimeBudget,
    MemoryBudget,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttemptReport {
    pub iteration: usize,
    pub accepted: bool,
    pub canonical_hash: String,
    pub score: CandidateScore,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReconstructionReport {
    pub status: LoopStatus,
    pub provider_calls: usize,
    pub accepted: usize,
    pub initial_score: CandidateScore,
    pub final_score: CandidateScore,
    pub attempts: Vec<AttemptReport>,
    pub final_hash: String,
}

/// Runs a finite corrective reconstruction loop over typed transactions.
/// Candidate state is accepted only when the objective improves and every
/// declared protected metric remains inside its regression bound.
///
/// # Errors
///
/// Returns observation, proposal, provider, evaluator, codec, or score errors.
pub fn run_loop(
    document: &mut Document,
    observations: &ObservationBundle,
    provider: &mut dyn CorrectionProvider,
    evaluator: &mut dyn CandidateEvaluator,
    budget: &LoopBudget,
) -> Result<ReconstructionReport, ReconstructionError> {
    observations.validate()?;
    let started = Instant::now();
    let initial_score = evaluator
        .evaluate(document)
        .map_err(ReconstructionError::Evaluator)?;
    validate_score(&initial_score, &budget.protected_metrics)?;
    let mut score = initial_score.clone();
    let mut seen = BTreeSet::from([canonical_hash(document)?]);
    let mut attempts = Vec::new();
    let mut provider_calls = 0_usize;
    let mut accepted = 0_usize;
    let mut status = LoopStatus::IterationBudget;
    for iteration in 0..budget.max_iterations {
        if provider_calls >= budget.max_provider_calls {
            status = LoopStatus::ProviderCallBudget;
            break;
        }
        if started.elapsed() >= Duration::from_millis(budget.max_millis) {
            status = LoopStatus::TimeBudget;
            break;
        }
        if estimated_bytes(document, observations)? > budget.max_estimated_bytes {
            status = LoopStatus::MemoryBudget;
            break;
        }
        provider_calls += 1;
        let Some(proposal) = provider
            .propose(document, observations, iteration)
            .map_err(ReconstructionError::Proposer)?
        else {
            status = LoopStatus::NoProposal;
            break;
        };
        let mut candidate = document.clone();
        apply_proposal(
            &mut candidate,
            observations,
            &proposal,
            &budget.proposal_policy,
        )?;
        let hash = canonical_hash(&candidate)?;
        if !seen.insert(hash.clone()) {
            status = LoopStatus::RepeatedState;
            break;
        }
        let candidate_score = evaluator
            .evaluate(&candidate)
            .map_err(ReconstructionError::Evaluator)?;
        validate_score(&candidate_score, &budget.protected_metrics)?;
        let accept = candidate_score.objective < score.objective
            && protected_metrics_hold(&score, &candidate_score, &budget.protected_metrics);
        attempts.push(AttemptReport {
            iteration,
            accepted: accept,
            canonical_hash: hash,
            score: candidate_score.clone(),
        });
        if accept {
            *document = candidate;
            score = candidate_score;
            accepted += 1;
        }
    }
    Ok(ReconstructionReport {
        status,
        provider_calls,
        accepted,
        initial_score,
        final_score: score,
        attempts,
        final_hash: canonical_hash(document)?,
    })
}

fn estimated_bytes(
    document: &Document,
    observations: &ObservationBundle,
) -> Result<usize, ReconstructionError> {
    let document = nuif_codec::DeterministicCbor.encode(document)?.len();
    let observations = observations.encode()?.len();
    Ok(document.saturating_add(observations))
}

fn validate_score(
    score: &CandidateScore,
    protected: &[ProtectedMetric],
) -> Result<(), ReconstructionError> {
    if !score.objective.is_finite()
        || score.metrics.values().any(|value| !value.is_finite())
        || protected.iter().any(|metric| {
            !metric.max_regression.is_finite()
                || metric.max_regression < 0.0
                || !score.metrics.contains_key(&metric.name)
        })
    {
        Err(ReconstructionError::InvalidScore)
    } else {
        Ok(())
    }
}

fn protected_metrics_hold(
    current: &CandidateScore,
    candidate: &CandidateScore,
    protected: &[ProtectedMetric],
) -> bool {
    protected.iter().all(|metric| {
        candidate.metrics[&metric.name] <= current.metrics[&metric.name] + metric.max_regression
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationPoint {
    pub raw: f64,
    pub calibrated: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationTable {
    pub artifact: ResourceDigest,
    pub points: Vec<CalibrationPoint>,
}

impl CalibrationTable {
    /// Applies monotonic piecewise-linear calibration.
    ///
    /// # Errors
    ///
    /// Rejects an invalid artifact, fewer than two points, non-probabilities,
    /// duplicate/decreasing raw values, or decreasing calibrated values.
    pub fn calibrate(&self, raw: f64) -> Result<Confidence, ReconstructionError> {
        if !self.artifact.is_valid()
            || self.points.len() < 2
            || !probability(raw)
            || self
                .points
                .iter()
                .any(|point| !probability(point.raw) || !probability(point.calibrated))
            || self
                .points
                .windows(2)
                .any(|pair| pair[0].raw >= pair[1].raw || pair[0].calibrated > pair[1].calibrated)
        {
            return Err(ReconstructionError::InvalidProposal(
                "calibration table is not a monotonic probability mapping".to_owned(),
            ));
        }
        let calibrated = if raw <= self.points[0].raw {
            self.points[0].calibrated
        } else if raw >= self.points[self.points.len() - 1].raw {
            self.points[self.points.len() - 1].calibrated
        } else {
            let pair = self
                .points
                .windows(2)
                .find(|pair| raw >= pair[0].raw && raw <= pair[1].raw)
                .ok_or_else(|| {
                    ReconstructionError::InvalidProposal(
                        "calibration interval lookup failed".to_owned(),
                    )
                })?;
            let ratio = (raw - pair[0].raw) / (pair[1].raw - pair[0].raw);
            pair[0].calibrated + ratio * (pair[1].calibrated - pair[0].calibrated)
        };
        Ok(Confidence {
            raw,
            calibrated: Some(calibrated),
            calibration_artifact: Some(self.artifact.clone()),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewDecision {
    Accept,
    Review,
    Reject,
}

/// Makes a selective decision from calibrated confidence only.
///
/// # Errors
///
/// Rejects absent calibration and invalid or unordered thresholds.
pub fn selective_decision(
    confidence: &Confidence,
    accept_at: f64,
    review_at: f64,
) -> Result<ReviewDecision, ReconstructionError> {
    if !probability(accept_at) || !probability(review_at) || review_at > accept_at {
        return Err(ReconstructionError::InvalidProposal(
            "selective thresholds must be ordered probabilities".to_owned(),
        ));
    }
    validate_confidence(confidence)?;
    let value = confidence.calibrated.ok_or_else(|| {
        ReconstructionError::InvalidProposal(
            "selective decisions require calibrated confidence".to_owned(),
        )
    })?;
    Ok(if value >= accept_at {
        ReviewDecision::Accept
    } else if value >= review_at {
        ReviewDecision::Review
    } else {
        ReviewDecision::Reject
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use nuif_core::{Asset, AssetId, AssetKind, AssetPortability, Entity, EntityKind, ImageAsset};
    use nuif_protocol::{Anchor, Transaction};

    fn observations() -> ObservationBundle {
        ObservationBundle {
            schema_version: 1,
            profile: OBSERVATION_PROFILE.to_owned(),
            capture_id: "capture-1".to_owned(),
            adapter: "screenshot-baseline".to_owned(),
            adapter_version: "1".to_owned(),
            context: Some(CaptureContext {
                profile: "test-context-0".to_owned(),
                properties: BTreeMap::from([("viewport".to_owned(), "100x100".to_owned())]),
            }),
            observations: vec![Observation {
                id: ObservationId("root-geometry".to_owned()),
                evidence: EvidenceClass::ObservedPixels,
                subject: None,
                coordinate_space: Some(CoordinateSpace::ViewportCssPixels {
                    viewport: "primary".to_owned(),
                }),
                transform: None,
                value: ObservationValue::Geometry {
                    bounds: Bounds {
                        x: 0.0,
                        y: 0.0,
                        width: 100.0,
                        height: 100.0,
                    },
                },
                confidence: Some(Confidence::raw(0.8)),
                source: "pixels".to_owned(),
            }],
            omissions: Vec::new(),
        }
    }

    #[test]
    fn observation_bundle_is_canonical_and_keeps_evidence_ceiling() {
        let bundle = observations();
        let bytes = bundle.encode().unwrap();
        assert_eq!(ObservationBundle::decode(&bytes).unwrap(), bundle);
        assert!(matches!(
            EvidenceClass::ObservedPixels.fidelity_ceiling(),
            Fidelity::Approximated { .. }
        ));

        let mut invalid = bundle;
        invalid.observations[0]
            .confidence
            .as_mut()
            .unwrap()
            .calibrated = Some(0.7);
        assert!(invalid.validate().is_err());

        let mut excessive = observations();
        excessive.context.as_mut().unwrap().properties = (0..=MAX_CONTEXT_PROPERTIES)
            .map(|index| (format!("property-{index}"), index.to_string()))
            .collect();
        assert!(matches!(
            excessive.validate(),
            Err(ObservationError::ResourceLimit {
                resource: "capture context properties",
                ..
            })
        ));
    }

    #[test]
    fn proposal_uses_core_atomicity_and_pinned_evidence() {
        let mut document = Document::empty(EntityId::new(1));
        let proposal = Proposal {
            schema_version: 1,
            provenance: InferenceProvenance {
                method: "deterministic-baseline".to_owned(),
                artifact: None,
                observations: BTreeSet::from([ObservationId("root-geometry".to_owned())]),
                confidence: Confidence::raw(0.6),
            },
            patch: Patch {
                base_revision: Some(canonical_hash(&document).unwrap()),
                transactions: vec![Transaction {
                    id: 1,
                    operations: vec![Operation::Insert {
                        parent: None,
                        anchor: Anchor::Start,
                        entity: Box::new(Entity::new(EntityId::new(2), EntityKind::Surface)),
                    }],
                }],
            },
        };
        apply_proposal(
            &mut document,
            &observations(),
            &proposal,
            &ProposalPolicy::default(),
        )
        .unwrap();
        assert_eq!(document.roots, [EntityId::new(2)]);
    }

    #[test]
    fn editable_policy_rejects_flattened_screenshot_resources() {
        let digest = ResourceDigest::from_sha256_hex("a".repeat(64));
        let mut evidence = observations();
        evidence.observations.push(Observation {
            id: ObservationId("screenshot-resource".to_owned()),
            evidence: EvidenceClass::ObservedPixels,
            subject: Some(Subject::Resource {
                digest: digest.clone(),
            }),
            coordinate_space: None,
            transform: None,
            value: ObservationValue::Resource {
                digest: Some(digest.clone()),
                media_type: Some("image/png".to_owned()),
                size: Some(4),
            },
            confidence: Some(Confidence::raw(1.0)),
            source: "screenshot-bytes".to_owned(),
        });
        let asset = Asset {
            schema_version: 1,
            id: AssetId::new(1),
            name: Some("flat screenshot".to_owned()),
            resource: Some(digest),
            portability: AssetPortability::PrivateAuthoring,
            kind: AssetKind::Image(ImageAsset {
                width: 1,
                height: 1,
                decoder_profile: "nuif-png-rgba8-0".to_owned(),
            }),
        };
        let mut document = Document::empty(EntityId::new(1));
        let proposal = Proposal {
            schema_version: 1,
            provenance: InferenceProvenance {
                method: "flat-copy".to_owned(),
                artifact: None,
                observations: BTreeSet::from([ObservationId("screenshot-resource".to_owned())]),
                confidence: Confidence::raw(1.0),
            },
            patch: Patch {
                base_revision: Some(canonical_hash(&document).unwrap()),
                transactions: vec![Transaction {
                    id: 1,
                    operations: vec![Operation::SetAsset {
                        asset: asset.clone(),
                    }],
                }],
            },
        };

        assert!(matches!(
            apply_proposal(
                &mut document,
                &evidence,
                &proposal,
                &ProposalPolicy::default()
            ),
            Err(ReconstructionError::ForbiddenOperation(
                "flattened_screenshot_asset"
            ))
        ));
        assert!(document.assets.is_empty());

        apply_proposal(
            &mut document,
            &evidence,
            &proposal,
            &ProposalPolicy {
                allow_flattened_screenshot: true,
                ..ProposalPolicy::default()
            },
        )
        .unwrap();
        assert_eq!(document.assets[&asset.id], asset);
    }

    #[test]
    fn calibration_and_selective_review_are_explicit() {
        let table = CalibrationTable {
            artifact: ResourceDigest::from_sha256_hex("a".repeat(64)),
            points: vec![
                CalibrationPoint {
                    raw: 0.0,
                    calibrated: 0.1,
                },
                CalibrationPoint {
                    raw: 1.0,
                    calibrated: 0.9,
                },
            ],
        };
        let confidence = table.calibrate(0.5).unwrap();
        assert_eq!(confidence.calibrated, Some(0.5));
        assert_eq!(
            selective_decision(&confidence, 0.8, 0.4).unwrap(),
            ReviewDecision::Review
        );
    }

    #[test]
    fn correction_loop_accepts_improvement_and_stops_on_repeated_state() {
        struct Provider;
        impl CorrectionProvider for Provider {
            fn propose(
                &mut self,
                document: &Document,
                _: &ObservationBundle,
                iteration: usize,
            ) -> Result<Option<Proposal>, String> {
                Ok(Some(Proposal {
                    schema_version: 1,
                    provenance: InferenceProvenance {
                        method: "test-provider".to_owned(),
                        artifact: None,
                        observations: BTreeSet::from([ObservationId("root-geometry".to_owned())]),
                        confidence: Confidence::raw(0.5),
                    },
                    patch: Patch {
                        base_revision: Some(canonical_hash(document).unwrap()),
                        transactions: vec![Transaction {
                            id: u128::try_from(iteration + 1).unwrap(),
                            operations: vec![Operation::Rename {
                                entity: EntityId::new(2),
                                name: (iteration == 0).then(|| "improved".to_owned()),
                            }],
                        }],
                    },
                }))
            }
        }
        struct Evaluator;
        impl CandidateEvaluator for Evaluator {
            fn evaluate(&mut self, document: &Document) -> Result<CandidateScore, String> {
                Ok(CandidateScore {
                    objective: if document.entities[&EntityId::new(2)].name.as_deref()
                        == Some("improved")
                    {
                        0.0
                    } else {
                        1.0
                    },
                    metrics: BTreeMap::from([("validity".to_owned(), 0.0)]),
                })
            }
        }

        let mut document = Document::empty(EntityId::new(1));
        let entity = Entity::new(EntityId::new(2), EntityKind::Surface);
        document.roots.push(entity.id);
        document.entities.insert(entity.id, entity);
        let report = run_loop(
            &mut document,
            &observations(),
            &mut Provider,
            &mut Evaluator,
            &LoopBudget {
                protected_metrics: vec![ProtectedMetric {
                    name: "validity".to_owned(),
                    max_regression: 0.0,
                }],
                ..LoopBudget::default()
            },
        )
        .unwrap();
        assert_eq!(report.status, LoopStatus::RepeatedState);
        assert_eq!(report.accepted, 1);
        assert_eq!(
            document.entities[&EntityId::new(2)].name.as_deref(),
            Some("improved")
        );
    }
}
