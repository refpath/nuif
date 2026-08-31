use nuif_codec::{CodecError, decode_canonical_record, encode_canonical_record};
use nuif_core::{ResourceDigest, is_identifier};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use thiserror::Error;

pub const PROVIDER_MANIFEST_PROFILE: &str = "nuif-reconstruction-provider-manifest-0";
pub const MAX_PROVIDER_ARTIFACTS: usize = 256;
pub const MAX_PROVIDER_MANIFESTS: usize = 256;
pub const MAX_PROVIDER_PROFILES: usize = 256;
pub const MAX_PROVIDER_STRING_BYTES: usize = 4_096;
pub const MAX_PROVIDER_METADATA_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderIdentity {
    pub kind: String,
    pub manifest: ResourceDigest,
}

impl ProviderIdentity {
    /// Validates the provider kind and exact manifest-byte identity.
    ///
    /// # Errors
    ///
    /// Rejects an invalid provider identifier or SHA-256 digest.
    pub fn validate(&self) -> Result<(), ProviderError> {
        if bounded_identifier(&self.kind) && self.manifest.is_valid() {
            Ok(())
        } else {
            Err(ProviderError::InvalidIdentity)
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderCapability {
    Ocr,
    RegionDetection,
    UiGrounding,
    LayoutInference,
    Proposal,
    Correction,
    Evaluation,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    Local,
    Remote,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderMaturity {
    Development,
    Released,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderArtifactRole {
    Implementation,
    ModelWeights,
    Processor,
    Adapter,
    Quantization,
    PromptTemplate,
    ToolConfiguration,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderArtifact {
    pub id: String,
    pub role: ProviderArtifactRole,
    pub digest: ResourceDigest,
    pub format: String,
    pub version: String,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum InventoryFormat {
    #[serde(rename = "spdx-3.0.1")]
    Spdx301,
    #[serde(rename = "cyclonedx-1.7")]
    CycloneDx17,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupplyChainInventory {
    pub format: InventoryFormat,
    pub artifact: ResourceDigest,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderManifest {
    pub schema_version: u32,
    pub profile: String,
    pub provider_id: String,
    pub kind: String,
    pub maturity: ProviderMaturity,
    pub capabilities: BTreeSet<ProviderCapability>,
    pub execution_modes: BTreeSet<ExecutionMode>,
    pub input_profiles: BTreeSet<String>,
    pub output_profiles: BTreeSet<String>,
    pub artifacts: Vec<ProviderArtifact>,
    pub model_card: Option<ResourceDigest>,
    pub inventory: Option<SupplyChainInventory>,
}

impl ProviderManifest {
    /// Validates one provider capability wrapper.
    ///
    /// The external SPDX/CycloneDX inventory remains authoritative for supply
    /// chain detail. This wrapper only binds NUIF capabilities and wire profiles
    /// to exact operational artifacts.
    ///
    /// # Errors
    ///
    /// Rejects malformed identities, missing capabilities/profiles, duplicate
    /// artifacts, ambiguous implementation identity, undocumented learned
    /// artifacts, invalid digests, and bounded-metadata excess.
    pub fn validate(&self) -> Result<(), ProviderError> {
        if self.schema_version != 1
            || self.profile != PROVIDER_MANIFEST_PROFILE
            || !bounded_identifier(&self.provider_id)
            || !bounded_identifier(&self.kind)
            || self.capabilities.is_empty()
            || self.execution_modes.is_empty()
            || self.input_profiles.is_empty()
            || self.output_profiles.is_empty()
            || self.artifacts.is_empty()
        {
            return Err(ProviderError::InvalidManifest(
                "schema, identity, capabilities, execution, or profiles are invalid",
            ));
        }
        if self.input_profiles.len() > MAX_PROVIDER_PROFILES
            || self.output_profiles.len() > MAX_PROVIDER_PROFILES
            || self.artifacts.len() > MAX_PROVIDER_ARTIFACTS
        {
            return Err(ProviderError::ResourceLimit);
        }
        if self
            .input_profiles
            .iter()
            .chain(&self.output_profiles)
            .any(|profile| !bounded_identifier(profile))
            || self
                .inventory
                .as_ref()
                .is_some_and(|inventory| !inventory.artifact.is_valid())
            || self
                .model_card
                .as_ref()
                .is_some_and(|digest| !digest.is_valid())
        {
            return Err(ProviderError::InvalidManifest(
                "profile, inventory, or model-card identity is invalid",
            ));
        }
        let mut artifact_ids = BTreeSet::new();
        let mut implementation_count = 0_usize;
        let mut learned_artifact = false;
        let mut metadata_bytes = self
            .provider_id
            .len()
            .checked_add(self.kind.len())
            .ok_or(ProviderError::ResourceLimit)?;
        for profile in self.input_profiles.iter().chain(&self.output_profiles) {
            add_metadata(&mut metadata_bytes, profile.len())?;
        }
        for artifact in &self.artifacts {
            if !bounded_identifier(&artifact.id)
                || !artifact_ids.insert(&artifact.id)
                || !artifact.digest.is_valid()
                || !bounded_text(&artifact.format)
                || !bounded_text(&artifact.version)
            {
                return Err(ProviderError::InvalidArtifact);
            }
            add_metadata(&mut metadata_bytes, artifact.id.len())?;
            add_metadata(&mut metadata_bytes, artifact.format.len())?;
            add_metadata(&mut metadata_bytes, artifact.version.len())?;
            if artifact.role == ProviderArtifactRole::Implementation {
                implementation_count = implementation_count.saturating_add(1);
            }
            learned_artifact |= matches!(
                artifact.role,
                ProviderArtifactRole::ModelWeights
                    | ProviderArtifactRole::Processor
                    | ProviderArtifactRole::Adapter
                    | ProviderArtifactRole::Quantization
            );
        }
        if implementation_count != 1 {
            return Err(ProviderError::InvalidManifest(
                "exactly one implementation artifact is required",
            ));
        }
        if learned_artifact && self.model_card.is_none() {
            return Err(ProviderError::InvalidManifest(
                "learned artifacts require a model card",
            ));
        }
        if (self.maturity == ProviderMaturity::Released || learned_artifact)
            && self.inventory.is_none()
        {
            return Err(ProviderError::InvalidManifest(
                "released or learned providers require a supply-chain inventory",
            ));
        }
        Ok(())
    }

    /// Encodes the validated wrapper with deterministic CBOR.
    ///
    /// # Errors
    ///
    /// Returns validation or canonical codec failure.
    pub fn encode(&self) -> Result<Vec<u8>, ProviderError> {
        self.validate()?;
        Ok(encode_canonical_record(self)?)
    }

    /// Decodes canonical CBOR and validates the wrapper.
    ///
    /// # Errors
    ///
    /// Rejects malformed, noncanonical, invalid, or excessive input.
    pub fn decode(bytes: &[u8]) -> Result<Self, ProviderError> {
        let manifest: Self = decode_canonical_record(bytes)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Returns the provider identity bound to the exact canonical manifest.
    ///
    /// # Errors
    ///
    /// Returns validation or canonical codec failure.
    pub fn identity(&self) -> Result<ProviderIdentity, ProviderError> {
        let bytes = self.encode()?;
        Ok(ProviderIdentity {
            kind: self.kind.clone(),
            manifest: ResourceDigest::from_sha256_hex(format!("{:x}", Sha256::digest(bytes))),
        })
    }
}

fn bounded_identifier(value: &str) -> bool {
    value.len() <= MAX_PROVIDER_STRING_BYTES && is_identifier(value)
}

fn bounded_text(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_PROVIDER_STRING_BYTES
        && !value.chars().any(char::is_control)
}

fn add_metadata(total: &mut usize, bytes: usize) -> Result<(), ProviderError> {
    *total = total
        .checked_add(bytes)
        .ok_or(ProviderError::ResourceLimit)?;
    if *total > MAX_PROVIDER_METADATA_BYTES {
        Err(ProviderError::ResourceLimit)
    } else {
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum ProviderError {
    #[error(transparent)]
    Codec(#[from] CodecError),
    #[error("provider identity is invalid")]
    InvalidIdentity,
    #[error("provider manifest is invalid: {0}")]
    InvalidManifest(&'static str),
    #[error("provider artifact is invalid or duplicated")]
    InvalidArtifact,
    #[error("provider manifest resource limit exceeded")]
    ResourceLimit,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(value: char) -> ResourceDigest {
        ResourceDigest::from_sha256_hex(value.to_string().repeat(64))
    }

    fn manifest() -> ProviderManifest {
        ProviderManifest {
            schema_version: 1,
            profile: PROVIDER_MANIFEST_PROFILE.to_owned(),
            provider_id: "deterministic-layout-provider-1".to_owned(),
            kind: "layout-inference".to_owned(),
            maturity: ProviderMaturity::Development,
            capabilities: BTreeSet::from([ProviderCapability::LayoutInference]),
            execution_modes: BTreeSet::from([ExecutionMode::Local]),
            input_profiles: BTreeSet::from(["nuif-observations-0".to_owned()]),
            output_profiles: BTreeSet::from(["nuif-proposal-0".to_owned()]),
            artifacts: vec![ProviderArtifact {
                id: "implementation".to_owned(),
                role: ProviderArtifactRole::Implementation,
                digest: digest('a'),
                format: "rust-library".to_owned(),
                version: "0.0.1".to_owned(),
            }],
            model_card: None,
            inventory: Some(SupplyChainInventory {
                format: InventoryFormat::Spdx301,
                artifact: digest('b'),
            }),
        }
    }

    #[test]
    fn manifest_reaches_a_canonical_identity_fixpoint() {
        let manifest = manifest();
        let bytes = manifest.encode().unwrap();
        let decoded = ProviderManifest::decode(&bytes).unwrap();
        assert_eq!(decoded, manifest);
        assert_eq!(decoded.identity().unwrap(), manifest.identity().unwrap());
    }

    #[test]
    fn learned_artifacts_require_cards_and_one_implementation() {
        let mut learned = manifest();
        learned.artifacts.push(ProviderArtifact {
            id: "model".to_owned(),
            role: ProviderArtifactRole::ModelWeights,
            digest: digest('c'),
            format: "safetensors".to_owned(),
            version: "1".to_owned(),
        });
        assert!(matches!(
            learned.validate(),
            Err(ProviderError::InvalidManifest(
                "learned artifacts require a model card"
            ))
        ));
        learned.model_card = Some(digest('d'));
        learned.artifacts[1].role = ProviderArtifactRole::Implementation;
        assert!(matches!(
            learned.validate(),
            Err(ProviderError::InvalidManifest(
                "exactly one implementation artifact is required"
            ))
        ));
    }
}
