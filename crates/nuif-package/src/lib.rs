#![doc = "Deterministic, bounded NUIF packages and explicit resource resolution."]

use nuif_codec::{
    CodecError, Decoder, DeterministicCbor, Encoder, canonical_hash, decode_canonical_record,
    encode_canonical_record,
};
use nuif_core::{
    AssetId, AssetPortability, Document, ResourceDerivation, ResourceDescriptor, ResourceDigest,
    ResourceLocator, ResourceRole, Severity, validate,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use thiserror::Error;

pub const PROFILE: &str = "nuif-package-0";
pub const MIME_TYPE: &[u8] = b"application/nuif+zip";
pub const DOCUMENT_MEDIA_TYPE: &str = "application/nuif+cbor";
pub const MAX_PACKAGE_BYTES: usize = 80 * 1024 * 1024;
pub const MAX_MANIFEST_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_RESOURCE_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_TOTAL_RESOURCE_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_RESOURCES: usize = 8_192;
pub const MAX_MEMBERS: usize = MAX_RESOURCES + 3;
pub const MAX_PATH_BYTES: usize = 96;

const MIME_PATH: &str = "mimetype";
const MANIFEST_PATH: &str = "manifest.cbor";
const DOCUMENT_PATH: &str = "document.cbor";
const BLOB_PREFIX: &str = "blobs/sha256/";
const ZIP_LOCAL_SIGNATURE: u32 = 0x0403_4b50;
const ZIP_CENTRAL_SIGNATURE: u32 = 0x0201_4b50;
const ZIP_END_SIGNATURE: u32 = 0x0605_4b50;
const ZIP_VERSION_NEEDED: u16 = 10;
const ZIP_VERSION_MADE_BY: u16 = 0x030a;
const ZIP_DOS_TIME: u16 = 0;
const ZIP_DOS_DATE: u16 = 33;
const ZIP_EXTERNAL_ATTRIBUTES: u32 = 0x81a4_0000;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageMode {
    Portable,
    Authoring,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentDescriptor {
    pub media_type: String,
    pub size: u64,
    pub canonical_hash: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageManifest {
    pub schema_version: u32,
    pub profile: String,
    pub mode: PackageMode,
    pub document: DocumentDescriptor,
    #[serde(default)]
    pub required_capabilities: BTreeSet<String>,
    #[serde(default)]
    pub assets: BTreeMap<AssetId, ResourceDigest>,
    #[serde(default)]
    pub resources: BTreeMap<ResourceDigest, ResourceDescriptor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NuifPackage {
    pub document: Document,
    pub mode: PackageMode,
    pub required_capabilities: BTreeSet<String>,
    pub resources: BTreeMap<ResourceDigest, ResourceDescriptor>,
    embedded: BTreeMap<ResourceDigest, Vec<u8>>,
}

impl NuifPackage {
    #[must_use]
    pub fn new(document: Document, mode: PackageMode) -> Self {
        Self {
            document,
            mode,
            required_capabilities: BTreeSet::new(),
            resources: BTreeMap::new(),
            embedded: BTreeMap::new(),
        }
    }

    /// Adds exact embedded bytes and returns their immutable identity.
    ///
    /// # Errors
    ///
    /// Rejects invalid media types, derivation metadata, conflicting
    /// descriptors, and profile resource limits.
    pub fn add_embedded(
        &mut self,
        bytes: Vec<u8>,
        media_type: impl Into<String>,
        role: ResourceRole,
        derivation: Option<ResourceDerivation>,
    ) -> Result<ResourceDigest, PackageError> {
        check_resource_size(bytes.len())?;
        let digest = digest(&bytes);
        let descriptor = ResourceDescriptor {
            digest: digest.clone(),
            size: u64::try_from(bytes.len()).map_err(|_| PackageError::ResourceLimit {
                resource: "resource bytes",
                limit: MAX_RESOURCE_BYTES,
                observed: usize::MAX,
            })?,
            media_type: media_type.into(),
            role,
            locator: ResourceLocator::Embedded {
                path: blob_path(&digest)?,
            },
            derivation,
        };
        validate_descriptor(&descriptor)?;
        if let Some(existing) = self.resources.get(&digest)
            && existing != &descriptor
        {
            return Err(PackageError::DescriptorConflict { digest });
        }
        self.resources.insert(digest.clone(), descriptor);
        self.embedded.insert(digest.clone(), bytes);
        self.check_resource_collection()?;
        Ok(digest)
    }

    /// Adds a digest-pinned linked resource without resolving it.
    ///
    /// # Errors
    ///
    /// Rejects invalid or credential-bearing locators, descriptors, and
    /// resource-count overflow.
    pub fn add_linked(
        &mut self,
        digest: ResourceDigest,
        size: u64,
        media_type: impl Into<String>,
        role: ResourceRole,
        uri: impl Into<String>,
        derivation: Option<ResourceDerivation>,
    ) -> Result<(), PackageError> {
        let descriptor = ResourceDescriptor {
            digest: digest.clone(),
            size,
            media_type: media_type.into(),
            role,
            locator: ResourceLocator::Linked { uri: uri.into() },
            derivation,
        };
        validate_descriptor(&descriptor)?;
        if let Some(existing) = self.resources.get(&digest)
            && existing != &descriptor
        {
            return Err(PackageError::DescriptorConflict { digest });
        }
        self.resources.insert(digest, descriptor);
        self.check_resource_collection()
    }

    #[must_use]
    pub fn embedded(&self, digest: &ResourceDigest) -> Option<&[u8]> {
        self.embedded.get(digest).map(Vec::as_slice)
    }

    /// Resolves a resource without granting the package network authority.
    /// Embedded resources are returned directly. Linked resources require an
    /// explicit caller-supplied resolver and are size/digest checked.
    ///
    /// # Errors
    ///
    /// Returns a typed availability, resolver, size, or digest error.
    pub fn resolve_resource(
        &self,
        digest: &ResourceDigest,
        resolver: Option<&mut dyn ResourceResolver>,
    ) -> Result<Vec<u8>, PackageError> {
        let descriptor =
            self.resources
                .get(digest)
                .ok_or_else(|| PackageError::ResourceUndeclared {
                    digest: digest.clone(),
                })?;
        if let Some(bytes) = self.embedded.get(digest) {
            verify_resource(descriptor, bytes)?;
            return Ok(bytes.clone());
        }
        let resolver = resolver.ok_or_else(|| PackageError::ResolutionRequired {
            digest: digest.clone(),
        })?;
        let bytes = resolver
            .resolve(descriptor)
            .map_err(PackageError::Resolver)?;
        verify_resource(descriptor, &bytes)?;
        Ok(bytes)
    }

    /// Produces the canonical package manifest for the current document and
    /// resources.
    ///
    /// # Errors
    ///
    /// Rejects invalid documents, descriptors, asset bindings, and policy.
    pub fn manifest(&self) -> Result<PackageManifest, PackageError> {
        validate_document(&self.document)?;
        self.check_resource_collection()?;
        let document_bytes = DeterministicCbor.encode(&self.document)?;
        let assets = self
            .document
            .assets
            .iter()
            .filter_map(|(id, asset)| asset.resource.clone().map(|digest| (*id, digest)))
            .collect::<BTreeMap<_, _>>();
        let manifest = PackageManifest {
            schema_version: 1,
            profile: PROFILE.to_owned(),
            mode: self.mode,
            document: DocumentDescriptor {
                media_type: DOCUMENT_MEDIA_TYPE.to_owned(),
                size: u64::try_from(document_bytes.len()).map_err(|_| {
                    PackageError::ResourceLimit {
                        resource: "document bytes",
                        limit: usize::MAX,
                        observed: document_bytes.len(),
                    }
                })?,
                canonical_hash: canonical_hash(&self.document)?,
            },
            required_capabilities: self.required_capabilities.clone(),
            assets,
            resources: self.resources.clone(),
        };
        validate_manifest(&manifest, &self.document, &self.embedded)?;
        Ok(manifest)
    }

    /// Encodes a byte-deterministic `nuif-package-0` ZIP archive.
    ///
    /// # Errors
    ///
    /// Rejects invalid package state or an encoded package beyond its budget.
    pub fn encode(&self) -> Result<Vec<u8>, PackageError> {
        let document = DeterministicCbor.encode(&self.document)?;
        let manifest = encode_canonical_record(&self.manifest()?)?;
        if manifest.len() > MAX_MANIFEST_BYTES {
            return Err(PackageError::ResourceLimit {
                resource: "manifest bytes",
                limit: MAX_MANIFEST_BYTES,
                observed: manifest.len(),
            });
        }
        let mut members = vec![Member::new(MIME_PATH, MIME_TYPE.to_vec())];
        for (digest, bytes) in &self.embedded {
            members.push(Member::new(&blob_path(digest)?, bytes.clone()));
        }
        members.push(Member::new(DOCUMENT_PATH, document));
        members.push(Member::new(MANIFEST_PATH, manifest));
        members[1..].sort_by(|left, right| left.name.as_bytes().cmp(right.name.as_bytes()));
        write_zip(&members)
    }

    /// Decodes and fully validates an untrusted package in memory.
    ///
    /// # Errors
    ///
    /// Rejects malformed ZIP structure, non-canonical records, undeclared or
    /// missing resources, policy violations, and all configured limits.
    pub fn decode(bytes: &[u8]) -> Result<Self, PackageError> {
        let members = read_zip(bytes)?;
        if members.get(MIME_PATH).map(Vec::as_slice) != Some(MIME_TYPE) {
            return Err(PackageError::InvalidArchive {
                reason: "mimetype member is absent or not the exact provisional media type"
                    .to_owned(),
            });
        }
        let manifest_bytes = required_member(&members, MANIFEST_PATH)?;
        if manifest_bytes.len() > MAX_MANIFEST_BYTES {
            return Err(PackageError::ResourceLimit {
                resource: "manifest bytes",
                limit: MAX_MANIFEST_BYTES,
                observed: manifest_bytes.len(),
            });
        }
        let manifest: PackageManifest = decode_canonical_record(manifest_bytes)?;
        let document_bytes = required_member(&members, DOCUMENT_PATH)?;
        let document = DeterministicCbor.decode(document_bytes)?;
        let mut embedded = BTreeMap::new();
        for (name, value) in &members {
            if let Some(hex) = name.strip_prefix(BLOB_PREFIX) {
                embedded.insert(ResourceDigest::from_sha256_hex(hex), value.clone());
            }
        }
        validate_manifest(&manifest, &document, &embedded)?;
        Ok(Self {
            document,
            mode: manifest.mode,
            required_capabilities: manifest.required_capabilities,
            resources: manifest.resources,
            embedded,
        })
    }

    /// Hashes the exact deterministic package bytes.
    ///
    /// # Errors
    ///
    /// Returns any package-encoding error.
    pub fn package_hash(&self) -> Result<String, PackageError> {
        let bytes = self.encode()?;
        Ok(format!("{PROFILE}:{}", digest(&bytes)))
    }

    fn check_resource_collection(&self) -> Result<(), PackageError> {
        if self.resources.len() > MAX_RESOURCES {
            return Err(PackageError::ResourceLimit {
                resource: "resource descriptors",
                limit: MAX_RESOURCES,
                observed: self.resources.len(),
            });
        }
        let mut total = 0_usize;
        for (digest, descriptor) in &self.resources {
            if digest != &descriptor.digest {
                return Err(PackageError::InvalidManifest {
                    reason: format!(
                        "resource map key {digest} differs from descriptor {}",
                        descriptor.digest
                    ),
                });
            }
            validate_descriptor(descriptor)?;
            let size = usize::try_from(descriptor.size).unwrap_or(usize::MAX);
            check_resource_size(size)?;
            if matches!(descriptor.locator, ResourceLocator::Embedded { .. }) {
                total = total.saturating_add(size);
            }
        }
        if total > MAX_TOTAL_RESOURCE_BYTES {
            return Err(PackageError::ResourceLimit {
                resource: "total embedded resource bytes",
                limit: MAX_TOTAL_RESOURCE_BYTES,
                observed: total,
            });
        }
        Ok(())
    }
}

pub trait ResourceResolver {
    /// Retrieves candidate bytes for an explicitly linked descriptor.
    ///
    /// # Errors
    ///
    /// Returns a policy-, transport-, authentication-, or availability-level
    /// explanation. The package layer independently verifies returned bytes.
    fn resolve(&mut self, descriptor: &ResourceDescriptor) -> Result<Vec<u8>, String>;
}

#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum PackageError {
    #[error(transparent)]
    Codec(#[from] CodecError),
    #[error("document is invalid: {codes:?}")]
    InvalidDocument { codes: Vec<String> },
    #[error("invalid package manifest: {reason}")]
    InvalidManifest { reason: String },
    #[error("invalid ZIP package: {reason}")]
    InvalidArchive { reason: String },
    #[error("required package member {name} is missing")]
    MissingMember { name: String },
    #[error("package resource limit exceeded for {resource}: limit {limit}, observed {observed}")]
    ResourceLimit {
        resource: &'static str,
        limit: usize,
        observed: usize,
    },
    #[error("resource {digest} has conflicting descriptors")]
    DescriptorConflict { digest: ResourceDigest },
    #[error("resource {digest} is not declared")]
    ResourceUndeclared { digest: ResourceDigest },
    #[error("resource {digest} requires an explicit resolver")]
    ResolutionRequired { digest: ResourceDigest },
    #[error("resource resolver failed: {0}")]
    Resolver(String),
    #[error("resource {digest} size mismatch: expected {expected}, observed {observed}")]
    SizeMismatch {
        digest: ResourceDigest,
        expected: u64,
        observed: usize,
    },
    #[error("resource digest mismatch: expected {expected}, observed {observed}")]
    DigestMismatch {
        expected: ResourceDigest,
        observed: ResourceDigest,
    },
}

fn validate_document(document: &Document) -> Result<(), PackageError> {
    let codes = validate(document)
        .into_iter()
        .filter(|item| item.severity == Severity::Error)
        .map(|item| item.code)
        .collect::<Vec<_>>();
    if codes.is_empty() {
        Ok(())
    } else {
        Err(PackageError::InvalidDocument { codes })
    }
}

fn validate_manifest(
    manifest: &PackageManifest,
    document: &Document,
    embedded: &BTreeMap<ResourceDigest, Vec<u8>>,
) -> Result<(), PackageError> {
    if manifest.schema_version != 1
        || manifest.profile != PROFILE
        || manifest.document.media_type != DOCUMENT_MEDIA_TYPE
    {
        return Err(PackageError::InvalidManifest {
            reason: "unsupported manifest version, profile, or document media type".to_owned(),
        });
    }
    validate_document(document)?;
    let document_bytes = DeterministicCbor.encode(document)?;
    if manifest.document.size != u64::try_from(document_bytes.len()).unwrap_or(u64::MAX)
        || manifest.document.canonical_hash != canonical_hash(document)?
    {
        return Err(PackageError::InvalidManifest {
            reason: "document size or canonical hash does not match document.cbor".to_owned(),
        });
    }
    let assets = document
        .assets
        .iter()
        .filter_map(|(id, asset)| asset.resource.clone().map(|digest| (*id, digest)))
        .collect::<BTreeMap<_, _>>();
    if manifest.assets != assets {
        return Err(PackageError::InvalidManifest {
            reason: "manifest asset bindings differ from the semantic document".to_owned(),
        });
    }
    validate_resources(manifest, document, embedded)
}

fn validate_resources(
    manifest: &PackageManifest,
    document: &Document,
    embedded: &BTreeMap<ResourceDigest, Vec<u8>>,
) -> Result<(), PackageError> {
    if manifest.resources.len() > MAX_RESOURCES {
        return Err(PackageError::ResourceLimit {
            resource: "resource descriptors",
            limit: MAX_RESOURCES,
            observed: manifest.resources.len(),
        });
    }
    for (digest, descriptor) in &manifest.resources {
        if digest != &descriptor.digest {
            return Err(PackageError::InvalidManifest {
                reason: format!("descriptor key {digest} does not match its digest"),
            });
        }
        validate_descriptor(descriptor)?;
        match &descriptor.locator {
            ResourceLocator::Embedded { path } => {
                if path != &blob_path(digest)? {
                    return Err(PackageError::InvalidManifest {
                        reason: format!("resource {digest} has a non-canonical embedded path"),
                    });
                }
                verify_resource(
                    descriptor,
                    embedded
                        .get(digest)
                        .ok_or_else(|| PackageError::MissingMember { name: path.clone() })?,
                )?;
            }
            ResourceLocator::Linked { .. } => {
                if embedded.contains_key(digest) {
                    return Err(PackageError::InvalidManifest {
                        reason: format!("linked resource {digest} has undeclared embedded bytes"),
                    });
                }
            }
        }
    }
    for digest in embedded.keys() {
        if !manifest.resources.contains_key(digest) {
            return Err(PackageError::ResourceUndeclared {
                digest: digest.clone(),
            });
        }
    }
    for (id, asset) in &document.assets {
        let Some(digest) = &asset.resource else {
            if manifest.mode == PackageMode::Portable
                && !matches!(asset.portability, AssetPortability::Unavailable)
            {
                return Err(PackageError::InvalidManifest {
                    reason: format!("portable package asset {id} has no exact resource"),
                });
            }
            continue;
        };
        let descriptor =
            manifest
                .resources
                .get(digest)
                .ok_or_else(|| PackageError::InvalidManifest {
                    reason: format!("asset {id} binds undeclared resource {digest}"),
                })?;
        if manifest.mode == PackageMode::Portable
            && (!matches!(
                asset.portability,
                AssetPortability::Portable | AssetPortability::Substituted
            ) || !matches!(descriptor.locator, ResourceLocator::Embedded { .. }))
        {
            return Err(PackageError::InvalidManifest {
                reason: format!("asset {id} is not portable with embedded exact bytes"),
            });
        }
    }
    Ok(())
}

fn validate_descriptor(descriptor: &ResourceDescriptor) -> Result<(), PackageError> {
    if !descriptor.digest.is_valid() {
        return Err(PackageError::InvalidManifest {
            reason: format!("resource digest {} is invalid", descriptor.digest),
        });
    }
    let size = usize::try_from(descriptor.size).unwrap_or(usize::MAX);
    check_resource_size(size)?;
    if !valid_media_type(&descriptor.media_type) {
        return Err(PackageError::InvalidManifest {
            reason: format!("resource {} has invalid media type", descriptor.digest),
        });
    }
    match (descriptor.role, &descriptor.derivation) {
        (ResourceRole::Derived, None) => {
            return Err(PackageError::InvalidManifest {
                reason: format!("derived resource {} lacks derivation", descriptor.digest),
            });
        }
        (ResourceRole::Derived, Some(derivation)) => {
            if derivation.inputs.is_empty()
                || !nuif_core::is_identifier(&derivation.profile)
                || derivation.inputs.iter().any(|digest| !digest.is_valid())
            {
                return Err(PackageError::InvalidManifest {
                    reason: format!("resource {} has invalid derivation", descriptor.digest),
                });
            }
        }
        (_, Some(_)) => {
            return Err(PackageError::InvalidManifest {
                reason: format!("non-derived resource {} has derivation", descriptor.digest),
            });
        }
        (_, None) => {}
    }
    match &descriptor.locator {
        ResourceLocator::Embedded { path } => validate_path(path),
        ResourceLocator::Linked { uri } => validate_linked_uri(uri),
    }
}

fn valid_media_type(value: &str) -> bool {
    let Some((kind, subtype)) = value.split_once('/') else {
        return false;
    };
    let valid = |part: &str| {
        !part.is_empty()
            && part.bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(
                        byte,
                        b'!' | b'#' | b'$' | b'&' | b'^' | b'_' | b'.' | b'+' | b'-'
                    )
            })
    };
    !subtype.contains('/') && valid(kind) && valid(subtype)
}

fn validate_linked_uri(uri: &str) -> Result<(), PackageError> {
    let lower = uri.to_ascii_lowercase();
    let authority_has_credentials = lower
        .split_once("://")
        .and_then(|(_, remainder)| remainder.split('/').next())
        .is_some_and(|authority| authority.contains('@'));
    let credential_hint = [
        "token=",
        "password=",
        "passwd=",
        "authorization=",
        "api_key=",
    ]
    .iter()
    .any(|needle| lower.contains(needle));
    if uri.is_empty()
        || uri.len() > 2_048
        || !uri.is_ascii()
        || uri.bytes().any(|byte| byte.is_ascii_control())
        || authority_has_credentials
        || credential_hint
    {
        return Err(PackageError::InvalidManifest {
            reason: "linked resource locator is invalid or appears to contain credentials"
                .to_owned(),
        });
    }
    Ok(())
}

fn check_resource_size(size: usize) -> Result<(), PackageError> {
    if size > MAX_RESOURCE_BYTES {
        Err(PackageError::ResourceLimit {
            resource: "resource bytes",
            limit: MAX_RESOURCE_BYTES,
            observed: size,
        })
    } else {
        Ok(())
    }
}

fn verify_resource(descriptor: &ResourceDescriptor, bytes: &[u8]) -> Result<(), PackageError> {
    if descriptor.size != u64::try_from(bytes.len()).unwrap_or(u64::MAX) {
        return Err(PackageError::SizeMismatch {
            digest: descriptor.digest.clone(),
            expected: descriptor.size,
            observed: bytes.len(),
        });
    }
    let observed = digest(bytes);
    if observed != descriptor.digest {
        return Err(PackageError::DigestMismatch {
            expected: descriptor.digest.clone(),
            observed,
        });
    }
    Ok(())
}

#[must_use]
pub fn digest(bytes: &[u8]) -> ResourceDigest {
    let hash = Sha256::digest(bytes);
    let mut hex = String::with_capacity(64);
    for byte in hash {
        write!(hex, "{byte:02x}").expect("writing to a string cannot fail");
    }
    ResourceDigest::from_sha256_hex(hex)
}

fn blob_path(digest: &ResourceDigest) -> Result<String, PackageError> {
    digest
        .sha256_hex()
        .map(|hex| format!("{BLOB_PREFIX}{hex}"))
        .ok_or_else(|| PackageError::InvalidManifest {
            reason: format!("resource digest {digest} is invalid"),
        })
}

fn required_member<'a>(
    members: &'a BTreeMap<String, Vec<u8>>,
    name: &str,
) -> Result<&'a [u8], PackageError> {
    members
        .get(name)
        .map(Vec::as_slice)
        .ok_or_else(|| PackageError::MissingMember {
            name: name.to_owned(),
        })
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Member {
    name: String,
    bytes: Vec<u8>,
}

impl Member {
    fn new(name: &str, bytes: Vec<u8>) -> Self {
        Self {
            name: name.to_owned(),
            bytes,
        }
    }
}

#[derive(Clone, Debug)]
struct CentralEntry {
    name: String,
    crc32: u32,
    size: u32,
    offset: u32,
}

fn write_zip(members: &[Member]) -> Result<Vec<u8>, PackageError> {
    validate_member_sequence(members)?;
    let mut output = Vec::new();
    let mut entries = Vec::with_capacity(members.len());
    for member in members {
        let offset = as_u32(output.len(), "package bytes")?;
        let size = as_u32(member.bytes.len(), "member bytes")?;
        let name_len = as_u16(member.name.len(), "path bytes")?;
        let crc32 = crc32(&member.bytes);
        push_u32(&mut output, ZIP_LOCAL_SIGNATURE);
        push_u16(&mut output, ZIP_VERSION_NEEDED);
        push_u16(&mut output, 0);
        push_u16(&mut output, 0);
        push_u16(&mut output, ZIP_DOS_TIME);
        push_u16(&mut output, ZIP_DOS_DATE);
        push_u32(&mut output, crc32);
        push_u32(&mut output, size);
        push_u32(&mut output, size);
        push_u16(&mut output, name_len);
        push_u16(&mut output, 0);
        output.extend_from_slice(member.name.as_bytes());
        output.extend_from_slice(&member.bytes);
        entries.push(CentralEntry {
            name: member.name.clone(),
            crc32,
            size,
            offset,
        });
    }
    let central_offset = as_u32(output.len(), "package bytes")?;
    for entry in &entries {
        push_u32(&mut output, ZIP_CENTRAL_SIGNATURE);
        push_u16(&mut output, ZIP_VERSION_MADE_BY);
        push_u16(&mut output, ZIP_VERSION_NEEDED);
        push_u16(&mut output, 0);
        push_u16(&mut output, 0);
        push_u16(&mut output, ZIP_DOS_TIME);
        push_u16(&mut output, ZIP_DOS_DATE);
        push_u32(&mut output, entry.crc32);
        push_u32(&mut output, entry.size);
        push_u32(&mut output, entry.size);
        push_u16(&mut output, as_u16(entry.name.len(), "path bytes")?);
        push_u16(&mut output, 0);
        push_u16(&mut output, 0);
        push_u16(&mut output, 0);
        push_u16(&mut output, 0);
        push_u32(&mut output, ZIP_EXTERNAL_ATTRIBUTES);
        push_u32(&mut output, entry.offset);
        output.extend_from_slice(entry.name.as_bytes());
    }
    let central_size = as_u32(output.len(), "package bytes")?.saturating_sub(central_offset);
    push_u32(&mut output, ZIP_END_SIGNATURE);
    push_u16(&mut output, 0);
    push_u16(&mut output, 0);
    let count = as_u16(entries.len(), "member count")?;
    push_u16(&mut output, count);
    push_u16(&mut output, count);
    push_u32(&mut output, central_size);
    push_u32(&mut output, central_offset);
    push_u16(&mut output, 0);
    if output.len() > MAX_PACKAGE_BYTES {
        return Err(PackageError::ResourceLimit {
            resource: "package bytes",
            limit: MAX_PACKAGE_BYTES,
            observed: output.len(),
        });
    }
    Ok(output)
}

fn validate_member_sequence(members: &[Member]) -> Result<(), PackageError> {
    if members.len() > MAX_MEMBERS {
        return Err(PackageError::ResourceLimit {
            resource: "package members",
            limit: MAX_MEMBERS,
            observed: members.len(),
        });
    }
    if members.first().map(|member| member.name.as_str()) != Some(MIME_PATH) {
        return Err(PackageError::InvalidArchive {
            reason: "mimetype must be the first local and central member".to_owned(),
        });
    }
    let mut previous = None;
    let mut names = BTreeSet::new();
    for (index, member) in members.iter().enumerate() {
        validate_path(&member.name)?;
        if !names.insert(&member.name) {
            return Err(PackageError::InvalidArchive {
                reason: format!("duplicate member {}", member.name),
            });
        }
        if index > 1 && previous.is_some_and(|name: &str| name.as_bytes() >= member.name.as_bytes())
        {
            return Err(PackageError::InvalidArchive {
                reason: "members after mimetype are not in strict bytewise path order".to_owned(),
            });
        }
        if member.bytes.len() > MAX_RESOURCE_BYTES && member.name.starts_with(BLOB_PREFIX) {
            check_resource_size(member.bytes.len())?;
        }
        if index > 0 {
            previous = Some(&member.name);
        }
    }
    Ok(())
}

fn read_zip(bytes: &[u8]) -> Result<BTreeMap<String, Vec<u8>>, PackageError> {
    if bytes.len() > MAX_PACKAGE_BYTES {
        return Err(PackageError::ResourceLimit {
            resource: "package bytes",
            limit: MAX_PACKAGE_BYTES,
            observed: bytes.len(),
        });
    }
    if bytes.len() < 22 {
        return invalid_archive("archive is shorter than the end record");
    }
    let end_offset = bytes.len() - 22;
    let mut end = SliceReader::new(&bytes[end_offset..]);
    expect_u32(&mut end, ZIP_END_SIGNATURE, "end signature")?;
    if end.u16()? != 0 || end.u16()? != 0 {
        return invalid_archive("split or spanned archives are forbidden");
    }
    let disk_entries = usize::from(end.u16()?);
    let entries = usize::from(end.u16()?);
    let central_size = usize::try_from(end.u32()?).unwrap_or(usize::MAX);
    let central_offset = usize::try_from(end.u32()?).unwrap_or(usize::MAX);
    if end.u16()? != 0 || !end.finished() || entries != disk_entries {
        return invalid_archive("archive comments or inconsistent entry counts are forbidden");
    }
    if entries > MAX_MEMBERS {
        return Err(PackageError::ResourceLimit {
            resource: "package members",
            limit: MAX_MEMBERS,
            observed: entries,
        });
    }
    if central_offset.checked_add(central_size) != Some(end_offset) {
        return invalid_archive("central directory bounds are inconsistent");
    }
    let central_bytes =
        bytes
            .get(central_offset..end_offset)
            .ok_or_else(|| PackageError::InvalidArchive {
                reason: "central directory is outside the package".to_owned(),
            })?;
    let entries = read_central_entries(central_bytes, entries)?;
    read_local_entries(bytes, central_offset, &entries)
}

fn read_central_entries(bytes: &[u8], count: usize) -> Result<Vec<CentralEntry>, PackageError> {
    let mut reader = SliceReader::new(bytes);
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        expect_u32(&mut reader, ZIP_CENTRAL_SIGNATURE, "central signature")?;
        if reader.u16()? != ZIP_VERSION_MADE_BY
            || reader.u16()? != ZIP_VERSION_NEEDED
            || reader.u16()? != 0
            || reader.u16()? != 0
            || reader.u16()? != ZIP_DOS_TIME
            || reader.u16()? != ZIP_DOS_DATE
        {
            return invalid_archive("central header uses unsupported flags, method, or metadata");
        }
        let crc32 = reader.u32()?;
        let compressed = reader.u32()?;
        let size = reader.u32()?;
        let name_len = usize::from(reader.u16()?);
        let extra_len = reader.u16()?;
        let comment_len = reader.u16()?;
        let disk = reader.u16()?;
        let internal = reader.u16()?;
        let external = reader.u32()?;
        let offset = reader.u32()?;
        if compressed != size
            || extra_len != 0
            || comment_len != 0
            || disk != 0
            || internal != 0
            || external != ZIP_EXTERNAL_ATTRIBUTES
        {
            return invalid_archive("central header violates the stored deterministic profile");
        }
        let name = read_name(&mut reader, name_len)?;
        entries.push(CentralEntry {
            name,
            crc32,
            size,
            offset,
        });
    }
    if !reader.finished() {
        return invalid_archive("central directory contains trailing bytes");
    }
    let sequence = entries
        .iter()
        .map(|entry| Member::new(&entry.name, Vec::new()))
        .collect::<Vec<_>>();
    validate_member_sequence(&sequence)?;
    Ok(entries)
}

fn read_local_entries(
    bytes: &[u8],
    central_offset: usize,
    entries: &[CentralEntry],
) -> Result<BTreeMap<String, Vec<u8>>, PackageError> {
    let mut expected_offset = 0_usize;
    let mut members = BTreeMap::new();
    for entry in entries {
        let offset = usize::try_from(entry.offset).unwrap_or(usize::MAX);
        if offset != expected_offset || offset >= central_offset {
            return invalid_archive("local members are not contiguous in central-directory order");
        }
        let mut reader = SliceReader::new(&bytes[offset..central_offset]);
        expect_u32(&mut reader, ZIP_LOCAL_SIGNATURE, "local signature")?;
        if reader.u16()? != ZIP_VERSION_NEEDED
            || reader.u16()? != 0
            || reader.u16()? != 0
            || reader.u16()? != ZIP_DOS_TIME
            || reader.u16()? != ZIP_DOS_DATE
        {
            return invalid_archive("local header uses unsupported flags, method, or metadata");
        }
        if reader.u32()? != entry.crc32
            || reader.u32()? != entry.size
            || reader.u32()? != entry.size
        {
            return invalid_archive("local and central sizes or CRC do not match");
        }
        let name_len = usize::from(reader.u16()?);
        if reader.u16()? != 0 {
            return invalid_archive("local header extra fields are forbidden");
        }
        let name = read_name(&mut reader, name_len)?;
        if name != entry.name {
            return invalid_archive("local and central member names differ");
        }
        let size = usize::try_from(entry.size).unwrap_or(usize::MAX);
        if name.starts_with(BLOB_PREFIX) {
            check_resource_size(size)?;
        }
        let value = reader.take(size)?.to_vec();
        if crc32(&value) != entry.crc32 {
            return invalid_archive("member CRC does not match its bytes");
        }
        expected_offset =
            offset
                .checked_add(reader.position())
                .ok_or_else(|| PackageError::InvalidArchive {
                    reason: "local member offset overflow".to_owned(),
                })?;
        members.insert(name, value);
    }
    if expected_offset != central_offset {
        return invalid_archive("data exists between local members and central directory");
    }
    Ok(members)
}

fn validate_path(path: &str) -> Result<(), PackageError> {
    let registered = matches!(path, MIME_PATH | MANIFEST_PATH | DOCUMENT_PATH)
        || path.strip_prefix(BLOB_PREFIX).is_some_and(|hex| {
            hex.len() == 64
                && hex
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        });
    if !registered
        || path.len() > MAX_PATH_BYTES
        || !path.is_ascii()
        || path.starts_with('/')
        || path.ends_with('/')
        || path.contains('\\')
        || path
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return Err(PackageError::InvalidArchive {
            reason: format!("member path {path:?} is not registered and canonical"),
        });
    }
    Ok(())
}

fn read_name(reader: &mut SliceReader<'_>, length: usize) -> Result<String, PackageError> {
    if length > MAX_PATH_BYTES {
        return Err(PackageError::ResourceLimit {
            resource: "path bytes",
            limit: MAX_PATH_BYTES,
            observed: length,
        });
    }
    let bytes = reader.take(length)?;
    if !bytes.is_ascii() {
        return invalid_archive("member names must be ASCII");
    }
    let name = std::str::from_utf8(bytes)
        .expect("ASCII is UTF-8")
        .to_owned();
    validate_path(&name)?;
    Ok(name)
}

#[derive(Clone, Copy)]
struct SliceReader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> SliceReader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], PackageError> {
        let end =
            self.position
                .checked_add(length)
                .ok_or_else(|| PackageError::InvalidArchive {
                    reason: "ZIP field offset overflow".to_owned(),
                })?;
        let value =
            self.bytes
                .get(self.position..end)
                .ok_or_else(|| PackageError::InvalidArchive {
                    reason: "truncated ZIP field".to_owned(),
                })?;
        self.position = end;
        Ok(value)
    }

    fn u16(&mut self) -> Result<u16, PackageError> {
        let bytes: [u8; 2] = self
            .take(2)?
            .try_into()
            .expect("the bounded slice has two bytes");
        Ok(u16::from_le_bytes(bytes))
    }

    fn u32(&mut self) -> Result<u32, PackageError> {
        let bytes: [u8; 4] = self
            .take(4)?
            .try_into()
            .expect("the bounded slice has four bytes");
        Ok(u32::from_le_bytes(bytes))
    }

    const fn position(self) -> usize {
        self.position
    }

    fn finished(self) -> bool {
        self.position == self.bytes.len()
    }
}

fn expect_u32(
    reader: &mut SliceReader<'_>,
    expected: u32,
    label: &str,
) -> Result<(), PackageError> {
    let observed = reader.u32()?;
    if observed == expected {
        Ok(())
    } else {
        invalid_archive(&format!("invalid {label}"))
    }
}

fn invalid_archive<T>(reason: &str) -> Result<T, PackageError> {
    Err(PackageError::InvalidArchive {
        reason: reason.to_owned(),
    })
}

fn as_u16(value: usize, resource: &'static str) -> Result<u16, PackageError> {
    u16::try_from(value).map_err(|_| PackageError::ResourceLimit {
        resource,
        limit: usize::from(u16::MAX),
        observed: value,
    })
}

fn as_u32(value: usize, resource: &'static str) -> Result<u32, PackageError> {
    u32::try_from(value).map_err(|_| PackageError::ResourceLimit {
        resource,
        limit: usize::try_from(u32::MAX).unwrap_or(usize::MAX),
        observed: value,
    })
}

fn push_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & 0_u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;
    use nuif_core::{Asset, AssetKind, EntityId, ImageAsset};
    use std::io::{Cursor, Write as _};
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    fn package() -> NuifPackage {
        let mut document = Document::empty(EntityId::new(1));
        let asset_id = AssetId::new(0xa0);
        let mut package = NuifPackage::new(document.clone(), PackageMode::Portable);
        let digest = package
            .add_embedded(
                b"exact resource".to_vec(),
                "image/png",
                ResourceRole::Authoring,
                None,
            )
            .unwrap();
        document.assets.insert(
            asset_id,
            Asset {
                schema_version: 1,
                id: asset_id,
                name: Some("hero".to_owned()),
                resource: Some(digest),
                portability: AssetPortability::Portable,
                kind: AssetKind::Image(ImageAsset {
                    width: 1,
                    height: 1,
                    decoder_profile: "nuif-png-0".to_owned(),
                }),
            },
        );
        package.document = document;
        package
    }

    #[test]
    fn package_roundtrip_reaches_a_byte_fixpoint() {
        let package = package();
        let bytes = package.encode().unwrap();
        let decoded = NuifPackage::decode(&bytes).unwrap();
        assert_eq!(decoded.document, package.document);
        assert_eq!(decoded.encode().unwrap(), bytes);
        assert!(package.package_hash().unwrap().starts_with(PROFILE));
    }

    #[test]
    fn corruption_and_extra_members_fail_atomically() {
        let bytes = package().encode().unwrap();
        let mut corrupted = bytes.clone();
        let resource_offset = corrupted
            .windows(b"exact resource".len())
            .position(|window| window == b"exact resource")
            .unwrap();
        corrupted[resource_offset] ^= 1;
        assert!(NuifPackage::decode(&corrupted).is_err());

        let mut members = vec![Member::new(MIME_PATH, MIME_TYPE.to_vec())];
        members.push(Member::new("unregistered", Vec::new()));
        assert!(write_zip(&members).is_err());
    }

    #[test]
    fn linked_resolution_is_explicit_and_verified() {
        struct Resolver(Vec<u8>);
        impl ResourceResolver for Resolver {
            fn resolve(&mut self, _: &ResourceDescriptor) -> Result<Vec<u8>, String> {
                Ok(self.0.clone())
            }
        }

        let bytes = b"linked".to_vec();
        let digest = digest(&bytes);
        let mut package =
            NuifPackage::new(Document::empty(EntityId::new(1)), PackageMode::Authoring);
        package
            .add_linked(
                digest.clone(),
                bytes.len() as u64,
                "image/png",
                ResourceRole::Authoring,
                "https://example.invalid/resource",
                None,
            )
            .unwrap();
        assert!(matches!(
            package.resolve_resource(&digest, None),
            Err(PackageError::ResolutionRequired { .. })
        ));
        assert_eq!(
            package
                .resolve_resource(&digest, Some(&mut Resolver(bytes.clone())))
                .unwrap(),
            bytes
        );
        assert!(
            package
                .add_linked(
                    digest,
                    6,
                    "image/png",
                    ResourceRole::Authoring,
                    "https://secret@example.invalid/resource",
                    None,
                )
                .is_err()
        );
    }

    #[test]
    fn independent_zip_writer_matches_normative_headers() {
        let package = package();
        let expected = package.encode().unwrap();
        let document = DeterministicCbor.encode(&package.document).unwrap();
        let manifest = encode_canonical_record(&package.manifest().unwrap()).unwrap();
        let mut members = vec![Member::new(MIME_PATH, MIME_TYPE.to_vec())];
        for (digest, bytes) in &package.embedded {
            members.push(Member::new(&blob_path(digest).unwrap(), bytes.clone()));
        }
        members.push(Member::new(DOCUMENT_PATH, document));
        members.push(Member::new(MANIFEST_PATH, manifest));
        members[1..].sort_by(|left, right| left.name.cmp(&right.name));

        let cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(cursor);
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .last_modified_time(zip::DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0).unwrap())
            .unix_permissions(0o644);
        for member in members {
            writer.start_file(member.name, options).unwrap();
            writer.write_all(&member.bytes).unwrap();
        }
        let independent = writer.finish().unwrap().into_inner();
        assert_eq!(independent, expected);
    }
}
