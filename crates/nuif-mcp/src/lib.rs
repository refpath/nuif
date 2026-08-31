#![doc = "Stateless Model Context Protocol tools over the authoritative NUIF core API."]

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use nuif_api::{DocumentEncoding, EngineError, NuifDocument, profile_zero_context};
use nuif_core::{Diagnostic, Fidelity, Severity, is_identifier};
use nuif_package::{MAX_CAPABILITY_BYTES, MAX_REQUIRED_CAPABILITIES, PackageError};
use nuif_protocol::{Patch, PatchLimits, enforce_patch_limits};
use rmcp::{
    Json, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{Implementation, ProtocolVersion, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    collections::BTreeSet,
    io,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, ReadBuf};

pub const API_PROFILE: &str = "nuif-mcp-tools-0";
pub const MAX_DOCUMENT_BYTES: usize = 1024 * 1024;
pub const MAX_PATCH_BYTES: usize = 1024 * 1024;
pub const MAX_PATCH_TRANSACTIONS: usize = 1024;
pub const MAX_PATCH_OPERATIONS: usize = 16_384;
pub const MAX_MESSAGE_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_PACKAGE_BYTES: usize = 1024 * 1024;
pub const MAX_SNAPSHOT_REPORT_BYTES: usize = 3 * 1024 * 1024;

const INSTRUCTIONS: &str = "Stateless NUIF profile nuif-mcp-tools-0. Every tool is pure and has no filesystem, network, host-document, credential, or hidden session authority. Document tools accept inline canonical NUIF text; the bounded snapshot tool accepts an inline base64 package plus explicit capabilities.";

#[derive(Clone, Debug)]
pub struct NuifMcp {
    tool_router: ToolRouter<Self>,
}

impl NuifMcp {
    #[must_use]
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

impl Default for NuifMcp {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DocumentInput {
    /// Canonical NUIF text (`nuif-text-0`) supplied inline.
    pub document: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplyPatchInput {
    /// Canonical NUIF text (`nuif-text-0`) supplied inline.
    pub document: String,
    /// A JSON-encoded NUIF semantic Patch.
    pub patch: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PackageSnapshotInput {
    /// Canonical base64 for one bounded deterministic `.nuif` package.
    pub package_base64: String,
    /// Complete package capabilities explicitly supported by the caller.
    pub capabilities: Vec<String>,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct DiagnosticOutput {
    pub code: String,
    pub severity: String,
    pub message: String,
    pub entity: Option<String>,
    pub pointer: Option<String>,
    pub fidelity: Option<FidelityOutput>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case", tag = "class")]
pub enum FidelityOutput {
    Lossless,
    Representable,
    Approximated { reason: String },
    PreservedUnrenderable { namespace: String },
    Unsupported { reason: String },
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ValidationOutput {
    pub schema_version: u32,
    pub status: String,
    pub canonical_hash: Option<String>,
    pub errors: usize,
    pub diagnostics: Vec<DiagnosticOutput>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct InspectionOutput {
    pub schema_version: u32,
    pub status: String,
    pub document_id: String,
    pub canonical_hash: Option<String>,
    pub entities: usize,
    pub roots: Vec<String>,
    pub tokens: usize,
    pub relations: usize,
    pub assets: usize,
    pub extensions_used: Vec<String>,
    pub errors: usize,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct CanonicalDocumentOutput {
    pub schema_version: u32,
    pub status: String,
    pub canonical_hash: String,
    pub document: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ApplyPatchOutput {
    pub schema_version: u32,
    pub status: String,
    pub canonical_hash: String,
    pub document: String,
    pub transactions: usize,
    pub operations: usize,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct PackageSnapshotOutput {
    pub schema_version: u32,
    pub status: String,
    pub canonical_hash: String,
    pub layout: serde_json::Value,
    pub scene: serde_json::Value,
    pub raster: serde_json::Value,
}

#[tool_router(router = tool_router)]
impl NuifMcp {
    /// Validates a document without requiring it to be canonicalizable.
    ///
    /// # Errors
    ///
    /// Returns a coded tool error when the inline document exceeds the profile
    /// limit or cannot be decoded.
    #[tool(
        name = "nuif_validate",
        description = "Validate an inline nuif-text-0 document and return bounded structural diagnostics without mutating any environment.",
        annotations(
            title = "Validate NUIF",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub fn validate(
        &self,
        Parameters(input): Parameters<DocumentInput>,
    ) -> Result<Json<ValidationOutput>, String> {
        let document = decode_document(&input.document)?;
        Ok(Json(validation_output(&document)?))
    }

    /// Reports bounded structural and identity information for a document.
    ///
    /// # Errors
    ///
    /// Returns a coded tool error when the inline document exceeds the profile
    /// limit or cannot be decoded.
    #[tool(
        name = "nuif_inspect",
        description = "Inspect the stable identity, cardinalities, extensions, validation state, and canonical hash of inline nuif-text-0.",
        annotations(
            title = "Inspect NUIF",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub fn inspect(
        &self,
        Parameters(input): Parameters<DocumentInput>,
    ) -> Result<Json<InspectionOutput>, String> {
        let document = decode_document(&input.document)?;
        let diagnostics = document
            .validate()
            .map_err(|error| coded("NUIF_VALIDATE_FAILED", error))?
            .diagnostics;
        let errors = error_count(&diagnostics);
        let model = document.document();
        Ok(Json(InspectionOutput {
            schema_version: 1,
            status: status(errors).to_owned(),
            document_id: model.id.to_string(),
            canonical_hash: document.canonical_hash().ok(),
            entities: model.entities.len(),
            roots: model.roots.iter().map(ToString::to_string).collect(),
            tokens: model.tokens.len(),
            relations: model.relations.len(),
            assets: model.assets.len(),
            extensions_used: model.extension_declarations.used.iter().cloned().collect(),
            errors,
        }))
    }

    /// Produces exact canonical text and its revision hash.
    ///
    /// # Errors
    ///
    /// Returns a coded tool error for excessive, malformed or structurally
    /// invalid NUIF text.
    #[tool(
        name = "nuif_canonicalize",
        description = "Canonicalize valid inline nuif-text-0 and return the exact canonical text and revision hash.",
        annotations(
            title = "Canonicalize NUIF",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub fn canonicalize(
        &self,
        Parameters(input): Parameters<DocumentInput>,
    ) -> Result<Json<CanonicalDocumentOutput>, String> {
        let document = decode_document(&input.document)?;
        let canonical = document
            .export(DocumentEncoding::CanonicalText)
            .map_err(|error| coded("NUIF_DOCUMENT_CANONICALIZE_FAILED", error))?;
        Ok(Json(CanonicalDocumentOutput {
            schema_version: 1,
            status: "passed".to_owned(),
            canonical_hash: document
                .canonical_hash()
                .map_err(|error| coded("NUIF_CANONICAL_HASH_FAILED", error))?,
            document: String::from_utf8(canonical)
                .map_err(|error| coded("NUIF_DOCUMENT_UTF8_FAILED", error))?,
        }))
    }

    /// Applies a semantic patch to a temporary session and returns a new value.
    ///
    /// # Errors
    ///
    /// Returns a coded tool error for excessive or malformed input, stale patch
    /// preconditions, failed operations, or an invalid resulting document.
    #[tool(
        name = "nuif_apply_patch",
        description = "Atomically apply a bounded semantic patch to inline nuif-text-0 and return a new canonical document; no external document is mutated.",
        annotations(
            title = "Apply NUIF patch",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub fn apply_patch(
        &self,
        Parameters(input): Parameters<ApplyPatchInput>,
    ) -> Result<Json<ApplyPatchOutput>, String> {
        let mut document = decode_document(&input.document)?;
        let (patch, usage) = decode_patch(&input.patch)?;
        document
            .apply_patch(&patch)
            .map_err(|error| coded("NUIF_PATCH_APPLY_FAILED", error))?;
        let bytes = document
            .export(DocumentEncoding::CanonicalText)
            .map_err(|error| coded("NUIF_DOCUMENT_ENCODE_FAILED", error))?;
        Ok(Json(ApplyPatchOutput {
            schema_version: 1,
            status: "passed".to_owned(),
            canonical_hash: document
                .canonical_hash()
                .map_err(|error| coded("NUIF_CANONICAL_HASH_FAILED", error))?,
            document: String::from_utf8(bytes)
                .map_err(|error| coded("NUIF_DOCUMENT_UTF8_FAILED", error))?,
            transactions: usage.transactions,
            operations: usage.operations,
        }))
    }

    /// Evaluates a bounded, capability-authorized package through the shared
    /// layout and render runtime without granting filesystem or network access.
    ///
    /// # Errors
    ///
    /// Returns coded failures for malformed transport, unsupported package
    /// requirements, invalid context, unresolved resources, or excessive
    /// output.
    #[tool(
        name = "nuif_snapshot_package",
        description = "Evaluate an inline base64 nuif-package-0 with explicit capabilities and return the shared canonical snapshot report.",
        annotations(
            title = "Snapshot NUIF package",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub fn snapshot_package(
        &self,
        Parameters(input): Parameters<PackageSnapshotInput>,
    ) -> Result<Json<PackageSnapshotOutput>, String> {
        let package = decode_package(&input.package_base64)?;
        let capabilities = decode_capabilities(input.capabilities)?;
        let document = NuifDocument::load_package_with_capabilities(&package, &capabilities)
            .map_err(|error| coded(package_load_error_code(&error), error))?;
        let snapshot = document
            .snapshot(&profile_zero_context(input.width, input.height))
            .map_err(|error| coded("NUIF_SNAPSHOT_FAILED", error))?;
        let report = snapshot.report();
        let encoded = serde_json::to_vec(&report)
            .map_err(|error| coded("NUIF_REPORT_ENCODE_FAILED", error))?;
        if encoded.len() > MAX_SNAPSHOT_REPORT_BYTES {
            return Err(format!(
                "NUIF_SNAPSHOT_REPORT_LIMIT_EXCEEDED: report exceeds {MAX_SNAPSHOT_REPORT_BYTES} bytes (observed {})",
                encoded.len()
            ));
        }
        Ok(Json(PackageSnapshotOutput {
            schema_version: report.schema_version,
            status: report.status,
            canonical_hash: report.canonical_hash,
            layout: serde_json::to_value(report.layout)
                .map_err(|error| coded("NUIF_REPORT_ENCODE_FAILED", error))?,
            scene: serde_json::to_value(report.scene)
                .map_err(|error| coded("NUIF_REPORT_ENCODE_FAILED", error))?,
            raster: serde_json::to_value(report.raster)
                .map_err(|error| coded("NUIF_REPORT_ENCODE_FAILED", error))?,
        }))
    }
}

#[allow(
    clippy::unused_async_trait_impl,
    reason = "the official rmcp tool-handler macro emits async forwarding methods"
)]
#[tool_handler(router = self.tool_router)]
impl ServerHandler for NuifMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("nuif-mcp", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }

    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        Cow::Borrowed(&[ProtocolVersion::V_2026_07_28])
    }
}

fn decode_document(input: &str) -> Result<NuifDocument, String> {
    enforce_document_bytes(input)?;
    NuifDocument::load(input.as_bytes(), DocumentEncoding::CanonicalText)
        .map_err(|error| coded("NUIF_DOCUMENT_DECODE_FAILED", error))
}

fn decode_package(input: &str) -> Result<Vec<u8>, String> {
    let max_base64_bytes = MAX_PACKAGE_BYTES.div_ceil(3).saturating_mul(4);
    if input.len() > max_base64_bytes {
        return Err(format!(
            "NUIF_PACKAGE_LIMIT_EXCEEDED: base64 package exceeds {max_base64_bytes} bytes (observed {})",
            input.len()
        ));
    }
    let bytes = BASE64
        .decode(input)
        .map_err(|error| coded("NUIF_PACKAGE_BASE64_INVALID", error))?;
    if bytes.len() > MAX_PACKAGE_BYTES {
        return Err(format!(
            "NUIF_PACKAGE_LIMIT_EXCEEDED: decoded package exceeds {MAX_PACKAGE_BYTES} bytes (observed {})",
            bytes.len()
        ));
    }
    Ok(bytes)
}

fn decode_capabilities(values: Vec<String>) -> Result<BTreeSet<String>, String> {
    if values.len() > MAX_REQUIRED_CAPABILITIES {
        return Err(format!(
            "NUIF_CAPABILITY_SET_LIMIT_EXCEEDED: capability count exceeds {MAX_REQUIRED_CAPABILITIES} (observed {})",
            values.len()
        ));
    }
    if let Some(value) = values
        .iter()
        .find(|value| value.len() > MAX_CAPABILITY_BYTES || !is_identifier(value))
    {
        return Err(format!(
            "NUIF_CAPABILITY_SET_INVALID: capability {value:?} is not a bounded identifier"
        ));
    }
    let observed = values.len();
    let capabilities = values.into_iter().collect::<BTreeSet<_>>();
    if capabilities.len() != observed {
        return Err(
            "NUIF_CAPABILITY_SET_INVALID: duplicate capability declarations are not canonical"
                .to_owned(),
        );
    }
    Ok(capabilities)
}

const fn package_load_error_code(error: &EngineError) -> &'static str {
    if matches!(
        error,
        EngineError::Package(PackageError::RequiredCapabilitiesUnavailable { .. })
    ) {
        "NUIF_PACKAGE_CAPABILITIES_UNAVAILABLE"
    } else {
        "NUIF_PACKAGE_DECODE_FAILED"
    }
}

fn enforce_document_bytes(input: &str) -> Result<(), String> {
    if input.len() > MAX_DOCUMENT_BYTES {
        Err(format!(
            "NUIF_DOCUMENT_LIMIT_EXCEEDED: document input exceeds {MAX_DOCUMENT_BYTES} bytes (observed {})",
            input.len()
        ))
    } else {
        Ok(())
    }
}

fn decode_patch(input: &str) -> Result<(Patch, nuif_protocol::PatchUsage), String> {
    if input.len() > MAX_PATCH_BYTES {
        return Err(format!(
            "NUIF_PATCH_LIMIT_EXCEEDED: patch input exceeds {MAX_PATCH_BYTES} bytes (observed {})",
            input.len()
        ));
    }
    let patch = serde_json::from_str::<Patch>(input)
        .map_err(|error| coded("NUIF_PATCH_DECODE_FAILED", error))?;
    let usage = enforce_patch_limits(
        &patch,
        PatchLimits {
            transactions: MAX_PATCH_TRANSACTIONS,
            operations: MAX_PATCH_OPERATIONS,
        },
    )
    .map_err(|error| coded("NUIF_PATCH_LIMIT_EXCEEDED", error))?;
    Ok((patch, usage))
}

fn validation_output(document: &NuifDocument) -> Result<ValidationOutput, String> {
    let diagnostics = document
        .validate()
        .map_err(|error| coded("NUIF_VALIDATE_FAILED", error))?
        .diagnostics;
    let errors = error_count(&diagnostics);
    Ok(ValidationOutput {
        schema_version: 1,
        status: status(errors).to_owned(),
        canonical_hash: document.canonical_hash().ok(),
        errors,
        diagnostics: diagnostics.into_iter().map(Into::into).collect(),
    })
}

fn error_count(diagnostics: &[Diagnostic]) -> usize {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .count()
}

const fn status(errors: usize) -> &'static str {
    if errors == 0 { "passed" } else { "failed" }
}

fn coded(code: &str, error: impl std::fmt::Display) -> String {
    format!("{code}: {error}")
}

impl From<Diagnostic> for DiagnosticOutput {
    fn from(diagnostic: Diagnostic) -> Self {
        Self {
            code: diagnostic.code,
            severity: match diagnostic.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Information => "information",
                Severity::Hint => "hint",
            }
            .to_owned(),
            message: diagnostic.message,
            entity: diagnostic.entity.map(|entity| entity.to_string()),
            pointer: diagnostic.pointer,
            fidelity: diagnostic.fidelity.map(Into::into),
        }
    }
}

impl From<Fidelity> for FidelityOutput {
    fn from(fidelity: Fidelity) -> Self {
        match fidelity {
            Fidelity::Lossless => Self::Lossless,
            Fidelity::Representable => Self::Representable,
            Fidelity::Approximated { reason } => Self::Approximated { reason },
            Fidelity::PreservedUnrenderable { namespace } => {
                Self::PreservedUnrenderable { namespace }
            }
            Fidelity::Unsupported { reason } => Self::Unsupported { reason },
        }
    }
}

/// Async reader that closes a stdio MCP connection before an unbounded line
/// can be accumulated by the upstream transport.
pub struct BoundedLineReader<R> {
    inner: R,
    max_line_bytes: usize,
    current_line_bytes: usize,
}

impl<R> BoundedLineReader<R> {
    #[must_use]
    pub const fn new(inner: R, max_line_bytes: usize) -> Self {
        Self {
            inner,
            max_line_bytes,
            current_line_bytes: 0,
        }
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for BoundedLineReader<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        output: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let allowed = self
            .max_line_bytes
            .saturating_sub(self.current_line_bytes)
            .saturating_add(1);
        let destination = output.initialize_unfilled();
        let requested = destination.len().min(allowed);
        if requested == 0 {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "MCP message line limit exceeded",
            )));
        }
        let mut limited = ReadBuf::new(&mut destination[..requested]);
        match Pin::new(&mut self.inner).poll_read(context, &mut limited) {
            Poll::Ready(Ok(())) => {
                let bytes = limited.filled();
                let bytes_read = bytes.len();
                let mut line_bytes = self.current_line_bytes;
                for byte in bytes {
                    if *byte == b'\n' {
                        line_bytes = 0;
                    } else {
                        line_bytes = line_bytes.saturating_add(1);
                        if line_bytes > self.max_line_bytes {
                            return Poll::Ready(Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "MCP message line limit exceeded",
                            )));
                        }
                    }
                }
                self.current_line_bytes = line_bytes;
                output.advance(bytes_read);
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Runs the bounded, stdio-only MCP server until its client closes stdin.
///
/// # Errors
///
/// Returns a startup, transport, or runtime error after writing no data to
/// stdout except protocol messages.
pub fn run_stdio() -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            NuifMcp::new()
                .serve((
                    BoundedLineReader::new(tokio::io::stdin(), MAX_MESSAGE_BYTES),
                    tokio::io::stdout(),
                ))
                .await?
                .waiting()
                .await?;
            Ok(())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use nuif_codec::{CanonicalText, Decoder, Encoder};
    use nuif_core::{Document, Entity, EntityId, EntityKind};
    use nuif_package::{NuifPackage, PackageMode};
    use tokio::io::AsyncReadExt;

    fn document_text() -> String {
        let mut document = Document::empty(EntityId::new(1));
        let root = Entity::new(EntityId::new(2), EntityKind::Container);
        document.roots.push(root.id);
        document.entities.insert(root.id, root);
        String::from_utf8(CanonicalText.encode(&document).unwrap()).unwrap()
    }

    fn package_bytes(required_capability: Option<&str>) -> Vec<u8> {
        let mut package = NuifPackage::new(
            CanonicalText.decode(document_text().as_bytes()).unwrap(),
            PackageMode::Portable,
        );
        if let Some(capability) = required_capability {
            package.required_capabilities.insert(capability.to_owned());
        }
        package.encode().unwrap()
    }

    #[test]
    fn pure_tools_share_the_core_hash_and_patch_semantics() {
        let server = NuifMcp::new();
        let document = document_text();
        let before = server
            .validate(Parameters(DocumentInput {
                document: document.clone(),
            }))
            .unwrap()
            .0;
        assert_eq!(before.status, "passed");
        let patch = serde_json::json!({
            "base_revision": before.canonical_hash,
            "transactions": [{
                "id": 1,
                "operations": [{
                    "op": "rename",
                    "entity": "00000000000000000000000000000002",
                    "name": "MCP edited"
                }]
            }]
        })
        .to_string();
        let applied = server
            .apply_patch(Parameters(ApplyPatchInput { document, patch }))
            .unwrap()
            .0;
        assert_eq!(applied.transactions, 1);
        assert_eq!(applied.operations, 1);
        assert!(applied.document.contains("MCP edited"));
        assert_eq!(
            server
                .canonicalize(Parameters(DocumentInput {
                    document: applied.document,
                }))
                .unwrap()
                .0
                .canonical_hash,
            applied.canonical_hash
        );
    }

    #[test]
    fn invalid_and_excessive_inputs_have_stable_tool_error_classes() {
        let server = NuifMcp::new();
        assert!(
            server
                .validate(Parameters(DocumentInput {
                    document: "{".to_owned(),
                }))
                .err()
                .unwrap()
                .starts_with("NUIF_DOCUMENT_DECODE_FAILED:")
        );
        assert!(
            server
                .validate(Parameters(DocumentInput {
                    document: " ".repeat(MAX_DOCUMENT_BYTES + 1),
                }))
                .err()
                .unwrap()
                .starts_with("NUIF_DOCUMENT_LIMIT_EXCEEDED:")
        );
        assert!(
            server
                .apply_patch(Parameters(ApplyPatchInput {
                    document: document_text(),
                    patch: " ".repeat(MAX_PATCH_BYTES + 1),
                }))
                .err()
                .unwrap()
                .starts_with("NUIF_PATCH_LIMIT_EXCEEDED:")
        );
        assert!(
            server
                .snapshot_package(Parameters(PackageSnapshotInput {
                    package_base64: "not base64".to_owned(),
                    capabilities: Vec::new(),
                    width: 10.0,
                    height: 12.0,
                }))
                .err()
                .unwrap()
                .starts_with("NUIF_PACKAGE_BASE64_INVALID:")
        );
        assert!(
            server
                .snapshot_package(Parameters(PackageSnapshotInput {
                    package_base64: BASE64.encode(package_bytes(None)),
                    capabilities: vec!["feature.example".to_owned(); 2],
                    width: 10.0,
                    height: 12.0,
                }))
                .err()
                .unwrap()
                .starts_with("NUIF_CAPABILITY_SET_INVALID:")
        );
    }

    #[test]
    fn package_snapshot_requires_capability_and_matches_direct_api() {
        let package = package_bytes(Some("feature.example"));
        let server = NuifMcp::new();
        let unavailable = server
            .snapshot_package(Parameters(PackageSnapshotInput {
                package_base64: BASE64.encode(&package),
                capabilities: Vec::new(),
                width: 10.0,
                height: 12.0,
            }))
            .err()
            .unwrap();
        assert!(unavailable.starts_with("NUIF_PACKAGE_CAPABILITIES_UNAVAILABLE:"));

        let supported = BTreeSet::from(["feature.example".to_owned()]);
        let direct = NuifDocument::load_package_with_capabilities(&package, &supported)
            .unwrap()
            .snapshot(&profile_zero_context(10.0, 12.0))
            .unwrap()
            .report();
        let mcp = server
            .snapshot_package(Parameters(PackageSnapshotInput {
                package_base64: BASE64.encode(&package),
                capabilities: supported.into_iter().collect(),
                width: 10.0,
                height: 12.0,
            }))
            .unwrap()
            .0;

        assert_eq!(mcp.schema_version, direct.schema_version);
        assert_eq!(mcp.status, direct.status);
        assert_eq!(mcp.canonical_hash, direct.canonical_hash);
        assert_eq!(mcp.layout, serde_json::to_value(direct.layout).unwrap());
        assert_eq!(mcp.scene, serde_json::to_value(direct.scene).unwrap());
        assert_eq!(mcp.raster, serde_json::to_value(direct.raster).unwrap());
    }

    #[tokio::test]
    async fn line_reader_accepts_the_boundary_and_rejects_one_over() {
        let mut accepted = BoundedLineReader::new(&b"1234\nnext\n"[..], 4);
        let mut bytes = Vec::new();
        accepted.read_to_end(&mut bytes).await.unwrap();
        assert_eq!(bytes, b"1234\nnext\n");

        let mut rejected = BoundedLineReader::new(&b"12345\n"[..], 4);
        let mut bytes = Vec::new();
        let error = rejected.read_to_end(&mut bytes).await.unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(bytes.len() <= 4);
    }
}
