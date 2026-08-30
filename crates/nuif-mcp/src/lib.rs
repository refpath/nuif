#![doc = "Stateless Model Context Protocol tools over the authoritative NUIF core API."]

use nuif_api::Session;
use nuif_codec::{CanonicalText, Canonicalizer, Decoder, Encoder, canonical_hash};
use nuif_core::{Diagnostic, Document, Fidelity, Severity, validate};
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

const INSTRUCTIONS: &str = "Stateless NUIF profile nuif-mcp-tools-0. Every tool is pure, accepts inline canonical NUIF text, returns a new value, and has no filesystem, network, host-document, credential, or hidden session authority.";

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
        Ok(Json(validation_output(&document)))
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
        let diagnostics = validate(&document);
        let errors = error_count(&diagnostics);
        Ok(Json(InspectionOutput {
            schema_version: 1,
            status: status(errors).to_owned(),
            document_id: document.id.to_string(),
            canonical_hash: canonical_hash(&document).ok(),
            entities: document.entities.len(),
            roots: document.roots.iter().map(ToString::to_string).collect(),
            tokens: document.tokens.len(),
            relations: document.relations.len(),
            assets: document.assets.len(),
            extensions_used: document
                .extension_declarations
                .used
                .iter()
                .cloned()
                .collect(),
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
        enforce_document_bytes(&input.document)?;
        let canonical = CanonicalText
            .canonicalize(input.document.as_bytes())
            .map_err(|error| coded("NUIF_DOCUMENT_CANONICALIZE_FAILED", error))?;
        let document = CanonicalText
            .decode(&canonical)
            .map_err(|error| coded("NUIF_DOCUMENT_DECODE_FAILED", error))?;
        Ok(Json(CanonicalDocumentOutput {
            schema_version: 1,
            status: "passed".to_owned(),
            canonical_hash: canonical_hash(&document)
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
        let document = decode_document(&input.document)?;
        let (patch, usage) = decode_patch(&input.patch)?;
        let mut session = Session::new(document);
        session
            .apply(&patch)
            .map_err(|error| coded("NUIF_PATCH_APPLY_FAILED", error))?;
        let bytes = CanonicalText
            .encode(session.document())
            .map_err(|error| coded("NUIF_DOCUMENT_ENCODE_FAILED", error))?;
        Ok(Json(ApplyPatchOutput {
            schema_version: 1,
            status: "passed".to_owned(),
            canonical_hash: canonical_hash(session.document())
                .map_err(|error| coded("NUIF_CANONICAL_HASH_FAILED", error))?,
            document: String::from_utf8(bytes)
                .map_err(|error| coded("NUIF_DOCUMENT_UTF8_FAILED", error))?,
            transactions: usage.transactions,
            operations: usage.operations,
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

fn decode_document(input: &str) -> Result<Document, String> {
    enforce_document_bytes(input)?;
    CanonicalText
        .decode_for_validation(input.as_bytes())
        .map_err(|error| coded("NUIF_DOCUMENT_DECODE_FAILED", error))
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

fn validation_output(document: &Document) -> ValidationOutput {
    let diagnostics = validate(document);
    let errors = error_count(&diagnostics);
    ValidationOutput {
        schema_version: 1,
        status: status(errors).to_owned(),
        canonical_hash: canonical_hash(document).ok(),
        errors,
        diagnostics: diagnostics.into_iter().map(Into::into).collect(),
    }
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
    use nuif_core::{Entity, EntityId, EntityKind};
    use tokio::io::AsyncReadExt;

    fn document_text() -> String {
        let mut document = Document::empty(EntityId::new(1));
        let root = Entity::new(EntityId::new(2), EntityKind::Container);
        document.roots.push(root.id);
        document.entities.insert(root.id, root);
        String::from_utf8(CanonicalText.encode(&document).unwrap()).unwrap()
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
