#![doc = "Experimental byte-oriented C ABI over the authoritative NUIF SDK."]

use nuif_api::{DocumentEncoding, EngineError, NuifDocument, profile_zero_context};
use nuif_codec::MAX_INPUT_BYTES;
use nuif_core::is_identifier;
use nuif_font::OPENTYPE_VARIABLE_TRUETYPE_PROFILE;
use nuif_package::{
    MAX_CAPABILITY_BYTES, MAX_PACKAGE_BYTES, MAX_REQUIRED_CAPABILITIES, PackageMode,
};
use nuif_protocol::{Patch, PatchLimits, enforce_patch_limits};
use std::collections::BTreeSet;
use std::mem::ManuallyDrop;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::slice;

/// ABI profile identifier. This profile is experimental and is not a stable
/// promise about symbol or ownership compatibility.
pub const API_PROFILE: &str = "nuif-ffi-0";
/// Maximum patch bytes accepted at the ABI boundary.
pub const MAX_PATCH_BYTES: usize = 4 * 1024 * 1024;
/// Maximum JSON capability-set transport accepted at the ABI boundary.
pub const MAX_CAPABILITY_SET_BYTES: usize = 64 * 1024;
/// Maximum serialized snapshot report returned across the ABI boundary.
pub const MAX_SNAPSHOT_REPORT_BYTES: usize = 3 * 1024 * 1024;
const MAX_PATCH_TRANSACTIONS: usize = 1_024;
const MAX_PATCH_OPERATIONS: usize = 16_384;

/// Stable error classes returned by every fallible ABI function.
pub mod error_code {
    pub const OK: u32 = 0;
    pub const INVALID_ARGUMENT: u32 = 1;
    pub const INPUT_LIMIT: u32 = 2;
    pub const DECODE: u32 = 3;
    pub const VALIDATION: u32 = 4;
    pub const OPERATION: u32 = 5;
    pub const OUTPUT_LIMIT: u32 = 6;
    pub const PANIC: u32 = 255;
}

/// A Rust-owned byte buffer returned by the ABI.
///
/// Call [`nuif_ffi_buffer_free`] exactly once for a non-empty buffer. A null
/// pointer with zero length and capacity represents an empty buffer.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct NuifFfiBuffer {
    pub ptr: *mut u8,
    pub len: usize,
    pub capacity: usize,
}

impl NuifFfiBuffer {
    const EMPTY: Self = Self {
        ptr: ptr::null_mut(),
        len: 0,
        capacity: 0,
    };

    fn from_vec(bytes: Vec<u8>) -> Self {
        if bytes.is_empty() {
            return Self::EMPTY;
        }
        let mut bytes = ManuallyDrop::new(bytes);
        Self {
            ptr: bytes.as_mut_ptr(),
            len: bytes.len(),
            capacity: bytes.capacity(),
        }
    }
}

/// Returned error value. The message is UTF-8 and must be released with
/// [`nuif_ffi_error_free`].
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct NuifFfiError {
    pub code: u32,
    pub message: NuifFfiBuffer,
}

impl NuifFfiError {
    const OK: Self = Self {
        code: error_code::OK,
        message: NuifFfiBuffer::EMPTY,
    };

    fn failure(failure: Failure) -> Self {
        Self {
            code: failure.code,
            message: NuifFfiBuffer::from_vec(failure.message.into_bytes()),
        }
    }
}

#[derive(Debug)]
struct Failure {
    code: u32,
    message: String,
}

impl Failure {
    fn new(code: u32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// Opaque document handle. Its fields are never part of the C contract.
#[repr(C)]
pub struct NuifFfiDocument {
    inner: NuifDocument,
}

fn protect<T>(operation: impl FnOnce() -> Result<T, Failure>) -> Result<T, Failure> {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(result) => result,
        Err(_) => Err(Failure::new(
            error_code::PANIC,
            "panic was contained at the NUIF C ABI boundary",
        )),
    }
}

fn encoding(value: u32) -> Result<DocumentEncoding, Failure> {
    match value {
        0 => Ok(DocumentEncoding::CanonicalText),
        1 => Ok(DocumentEncoding::DeterministicCbor),
        _ => Err(Failure::new(
            error_code::INVALID_ARGUMENT,
            format!("unknown document encoding value {value}"),
        )),
    }
}

fn package_mode(value: u32) -> Result<PackageMode, Failure> {
    match value {
        0 => Ok(PackageMode::Portable),
        1 => Ok(PackageMode::Authoring),
        _ => Err(Failure::new(
            error_code::INVALID_ARGUMENT,
            format!("unknown package mode value {value}"),
        )),
    }
}

fn engine_failure(error: &EngineError, code: u32) -> Failure {
    Failure::new(code, error.to_string())
}

unsafe fn input<'a>(ptr: *const u8, len: usize, limit: usize) -> Result<&'a [u8], Failure> {
    if len > limit {
        return Err(Failure::new(
            error_code::INPUT_LIMIT,
            format!("input exceeds {limit} bytes (observed {len})"),
        ));
    }
    if len == 0 {
        return Ok(&[]);
    }
    if ptr.is_null() {
        return Err(Failure::new(
            error_code::INVALID_ARGUMENT,
            "input pointer is null for a non-empty buffer",
        ));
    }
    // SAFETY: the caller's safety contract requires `ptr` to reference `len`
    // readable bytes for the duration of this call; null and limit cases were
    // checked above.
    Ok(unsafe { slice::from_raw_parts(ptr, len) })
}

unsafe fn output_slot<T>(ptr: *mut T) -> Result<(), Failure> {
    if ptr.is_null() {
        Err(Failure::new(
            error_code::INVALID_ARGUMENT,
            "output pointer is null",
        ))
    } else {
        Ok(())
    }
}

fn report_bytes(document: &NuifDocument) -> Result<Vec<u8>, Failure> {
    let report = document
        .validate()
        .map_err(|error| engine_failure(&error, error_code::VALIDATION))?;
    serde_json::to_vec(&report)
        .map_err(|error| Failure::new(error_code::OUTPUT_LIMIT, error.to_string()))
}

fn patch_from_bytes(bytes: &[u8]) -> Result<Patch, Failure> {
    let patch: Patch = serde_json::from_slice(bytes)
        .map_err(|error| Failure::new(error_code::DECODE, error.to_string()))?;
    enforce_patch_limits(
        &patch,
        PatchLimits {
            transactions: MAX_PATCH_TRANSACTIONS,
            operations: MAX_PATCH_OPERATIONS,
        },
    )
    .map_err(|error| Failure::new(error_code::INPUT_LIMIT, error.to_string()))?;
    Ok(patch)
}

fn capability_set_from_bytes(bytes: &[u8]) -> Result<BTreeSet<String>, Failure> {
    let values: Vec<String> = serde_json::from_slice(bytes)
        .map_err(|error| Failure::new(error_code::DECODE, error.to_string()))?;
    if values.len() > MAX_REQUIRED_CAPABILITIES {
        return Err(Failure::new(
            error_code::INPUT_LIMIT,
            format!(
                "capability set exceeds {MAX_REQUIRED_CAPABILITIES} entries (observed {})",
                values.len()
            ),
        ));
    }
    if let Some(capability) = values
        .iter()
        .find(|value| value.len() > MAX_CAPABILITY_BYTES || !is_identifier(value))
    {
        return Err(Failure::new(
            error_code::INVALID_ARGUMENT,
            format!("capability {capability:?} is not a bounded identifier"),
        ));
    }
    let observed = values.len();
    let capabilities = values.into_iter().collect::<BTreeSet<_>>();
    if capabilities.len() != observed {
        return Err(Failure::new(
            error_code::INVALID_ARGUMENT,
            "duplicate capability declarations are not canonical",
        ));
    }
    Ok(capabilities)
}

fn serialize_report<T: serde::Serialize>(report: &T, limit: usize) -> Result<Vec<u8>, Failure> {
    let bytes = serde_json::to_vec(report)
        .map_err(|error| Failure::new(error_code::OUTPUT_LIMIT, error.to_string()))?;
    if bytes.len() > limit {
        return Err(Failure::new(
            error_code::OUTPUT_LIMIT,
            format!("output exceeds {limit} bytes (observed {})", bytes.len()),
        ));
    }
    Ok(bytes)
}

/// Frees a buffer returned by this ABI. Passing an empty buffer is a no-op.
///
/// # Safety
/// `buffer` must have been returned by this library and must not have been
/// freed previously.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nuif_ffi_buffer_free(buffer: NuifFfiBuffer) {
    if buffer.ptr.is_null() {
        return;
    }
    // SAFETY: the caller promises this is an owned buffer returned by this
    // library with its original length and capacity.
    drop(unsafe { Vec::from_raw_parts(buffer.ptr, buffer.len, buffer.capacity) });
}

/// Frees the message owned by an error value. Passing a success value is safe.
///
/// # Safety
/// `error` must not be used after this call; its message must have come from
/// this library and must not have been released previously.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nuif_ffi_error_free(error: NuifFfiError) {
    // SAFETY: the message follows the same ownership contract as any returned
    // buffer.
    unsafe { nuif_ffi_buffer_free(error.message) };
}

/// Loads a canonical bare document into an opaque handle.
///
/// Encoding values are `0` for canonical text and `1` for deterministic CBOR.
///
/// # Safety
/// `bytes` must reference `len` readable bytes (unless `len` is zero), and
/// `out` must point to writable handle storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nuif_ffi_document_load(
    bytes: *const u8,
    len: usize,
    encoding_value: u32,
    out: *mut *mut NuifFfiDocument,
) -> NuifFfiError {
    if let Err(error) = unsafe { output_slot(out) } {
        return NuifFfiError::failure(error);
    }
    // SAFETY: `out` was checked above and is only written after successful
    // construction.
    unsafe { *out = ptr::null_mut() };
    let result = protect(|| {
        let encoding = encoding(encoding_value)?;
        // SAFETY: forwarded directly from this function's caller contract.
        let bytes = unsafe { input(bytes, len, MAX_INPUT_BYTES) }?;
        let document = NuifDocument::load(bytes, encoding)
            .map_err(|error| engine_failure(&error, error_code::DECODE))?;
        Ok(Box::into_raw(Box::new(NuifFfiDocument { inner: document })))
    });
    match result {
        Ok(handle) => {
            // SAFETY: `out` was checked and the handle is uniquely owned.
            unsafe { *out = handle };
            NuifFfiError::OK
        }
        Err(error) => NuifFfiError::failure(error),
    }
}

/// Structurally loads a deterministic NUIF package into an opaque handle.
///
/// A package that declares required capabilities remains available for inert
/// inspection and same-mode export. Evaluation and mutation stay disabled
/// until [`nuif_ffi_document_require_package_capabilities`] succeeds.
///
/// # Safety
/// `bytes` must reference `len` readable bytes (unless `len` is zero), and
/// `out` must point to writable handle storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nuif_ffi_document_load_package(
    bytes: *const u8,
    len: usize,
    out: *mut *mut NuifFfiDocument,
) -> NuifFfiError {
    if let Err(error) = unsafe { output_slot(out) } {
        return NuifFfiError::failure(error);
    }
    // SAFETY: `out` was checked above and is only replaced on success.
    unsafe { *out = ptr::null_mut() };
    let result = protect(|| {
        // SAFETY: forwarded directly from this function's caller contract.
        let bytes = unsafe { input(bytes, len, MAX_PACKAGE_BYTES) }?;
        let document = NuifDocument::load_package(bytes)
            .map_err(|error| engine_failure(&error, error_code::DECODE))?;
        Ok(Box::into_raw(Box::new(NuifFfiDocument { inner: document })))
    });
    match result {
        Ok(handle) => {
            // SAFETY: `out` was checked and the handle is uniquely owned.
            unsafe { *out = handle };
            NuifFfiError::OK
        }
        Err(error) => NuifFfiError::failure(error),
    }
}

/// Releases a document handle. A null handle is a no-op.
///
/// # Safety
/// `document` must be null or a handle returned by this library that has not
/// already been released.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nuif_ffi_document_free(document: *mut NuifFfiDocument) {
    if document.is_null() {
        return;
    }
    // SAFETY: the caller promises unique ownership of the handle.
    drop(unsafe { Box::from_raw(document) });
}

/// Serializes structural validation diagnostics as UTF-8 JSON.
///
/// # Safety
/// `document` must be a live handle and `out` must point to writable buffer
/// storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nuif_ffi_document_validate(
    document: *const NuifFfiDocument,
    out: *mut NuifFfiBuffer,
) -> NuifFfiError {
    if let Err(error) = unsafe { output_slot(out) } {
        return NuifFfiError::failure(error);
    }
    // SAFETY: output slot was checked and is initialized only on success.
    unsafe { *out = NuifFfiBuffer::EMPTY };
    if document.is_null() {
        return NuifFfiError::failure(Failure::new(
            error_code::INVALID_ARGUMENT,
            "document handle is null",
        ));
    }
    let result = protect(|| {
        // SAFETY: the caller contract requires a live handle.
        let document = unsafe { &(*document).inner };
        report_bytes(document)
    });
    match result {
        Ok(bytes) => {
            // SAFETY: output slot was checked above.
            unsafe { *out = NuifFfiBuffer::from_vec(bytes) };
            NuifFfiError::OK
        }
        Err(error) => NuifFfiError::failure(error),
    }
}

/// Returns the canonical semantic document hash as UTF-8 bytes.
///
/// # Safety
/// `document` must be a live handle and `out` must point to writable buffer
/// storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nuif_ffi_document_canonical_hash(
    document: *const NuifFfiDocument,
    out: *mut NuifFfiBuffer,
) -> NuifFfiError {
    if let Err(error) = unsafe { output_slot(out) } {
        return NuifFfiError::failure(error);
    }
    unsafe { *out = NuifFfiBuffer::EMPTY };
    if document.is_null() {
        return NuifFfiError::failure(Failure::new(
            error_code::INVALID_ARGUMENT,
            "document handle is null",
        ));
    }
    let result = protect(|| {
        // SAFETY: the caller contract requires a live handle.
        let document = unsafe { &(*document).inner };
        document
            .canonical_hash()
            .map(String::into_bytes)
            .map_err(|error| engine_failure(&error, error_code::VALIDATION))
    });
    match result {
        Ok(bytes) => {
            unsafe { *out = NuifFfiBuffer::from_vec(bytes) };
            NuifFfiError::OK
        }
        Err(error) => NuifFfiError::failure(error),
    }
}

/// Exports a canonical bare document into a caller-owned buffer.
///
/// Encoding values are `0` for canonical text and `1` for deterministic CBOR.
///
/// # Safety
/// `document` must be a live handle and `out` must point to writable buffer
/// storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nuif_ffi_document_export(
    document: *const NuifFfiDocument,
    encoding_value: u32,
    out: *mut NuifFfiBuffer,
) -> NuifFfiError {
    if let Err(error) = unsafe { output_slot(out) } {
        return NuifFfiError::failure(error);
    }
    unsafe { *out = NuifFfiBuffer::EMPTY };
    if document.is_null() {
        return NuifFfiError::failure(Failure::new(
            error_code::INVALID_ARGUMENT,
            "document handle is null",
        ));
    }
    let result = protect(|| {
        let encoding = encoding(encoding_value)?;
        // SAFETY: the caller contract requires a live handle.
        let document = unsafe { &(*document).inner };
        document
            .export(encoding)
            .map_err(|error| engine_failure(&error, error_code::OPERATION))
    });
    match result {
        Ok(bytes) => {
            unsafe { *out = NuifFfiBuffer::from_vec(bytes) };
            NuifFfiError::OK
        }
        Err(error) => NuifFfiError::failure(error),
    }
}

/// Exports a deterministic package while retaining loaded resource bytes.
///
/// Package mode values are `0` for portable and `1` for authoring.
///
/// # Safety
/// `document` must be a live handle and `out` must point to writable buffer
/// storage. The same handle must not be accessed concurrently.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nuif_ffi_document_export_package(
    document: *const NuifFfiDocument,
    mode_value: u32,
    out: *mut NuifFfiBuffer,
) -> NuifFfiError {
    if let Err(error) = unsafe { output_slot(out) } {
        return NuifFfiError::failure(error);
    }
    // SAFETY: output slot was checked and is initialized before any failure.
    unsafe { *out = NuifFfiBuffer::EMPTY };
    if document.is_null() {
        return NuifFfiError::failure(Failure::new(
            error_code::INVALID_ARGUMENT,
            "document handle is null",
        ));
    }
    let result = protect(|| {
        let mode = package_mode(mode_value)?;
        // SAFETY: the caller contract requires a live, non-concurrent handle.
        let document = unsafe { &(*document).inner };
        document
            .export_package(mode)
            .map_err(|error| engine_failure(&error, error_code::OPERATION))
    });
    match result {
        Ok(bytes) => {
            // SAFETY: output slot was checked above.
            unsafe { *out = NuifFfiBuffer::from_vec(bytes) };
            NuifFfiError::OK
        }
        Err(error) => NuifFfiError::failure(error),
    }
}

/// Reports a package's exact required, supported-required and missing sets.
/// A bare document returns JSON `null`.
///
/// # Safety
/// `document` must be a live, non-concurrently accessed handle;
/// `capabilities_json` must reference a bounded UTF-8 JSON string array; and
/// `out` must point to writable buffer storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nuif_ffi_document_package_capability_report(
    document: *const NuifFfiDocument,
    capabilities_json: *const u8,
    capabilities_len: usize,
    out: *mut NuifFfiBuffer,
) -> NuifFfiError {
    if let Err(error) = unsafe { output_slot(out) } {
        return NuifFfiError::failure(error);
    }
    // SAFETY: output slot was checked and is initialized before any failure.
    unsafe { *out = NuifFfiBuffer::EMPTY };
    if document.is_null() {
        return NuifFfiError::failure(Failure::new(
            error_code::INVALID_ARGUMENT,
            "document handle is null",
        ));
    }
    let result = protect(|| {
        // SAFETY: forwarded directly from this function's caller contract.
        let bytes = unsafe {
            input(
                capabilities_json,
                capabilities_len,
                MAX_CAPABILITY_SET_BYTES,
            )
        }?;
        let capabilities = capability_set_from_bytes(bytes)?;
        // SAFETY: the caller contract requires a live, non-concurrent handle.
        let document = unsafe { &(*document).inner };
        serialize_report(
            &document.package_capability_report(&capabilities),
            MAX_CAPABILITY_SET_BYTES,
        )
    });
    match result {
        Ok(bytes) => {
            // SAFETY: output slot was checked above.
            unsafe { *out = NuifFfiBuffer::from_vec(bytes) };
            NuifFfiError::OK
        }
        Err(error) => NuifFfiError::failure(error),
    }
}

/// Authorizes a loaded package when the supplied capability set covers every
/// manifest requirement. A failed call leaves the document unauthorized.
///
/// # Safety
/// `document` must be a live, uniquely owned handle and
/// `capabilities_json` must reference a bounded UTF-8 JSON string array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nuif_ffi_document_require_package_capabilities(
    document: *mut NuifFfiDocument,
    capabilities_json: *const u8,
    capabilities_len: usize,
) -> NuifFfiError {
    if document.is_null() {
        return NuifFfiError::failure(Failure::new(
            error_code::INVALID_ARGUMENT,
            "document handle is null",
        ));
    }
    let result = protect(|| {
        // SAFETY: forwarded directly from this function's caller contract.
        let bytes = unsafe {
            input(
                capabilities_json,
                capabilities_len,
                MAX_CAPABILITY_SET_BYTES,
            )
        }?;
        let capabilities = capability_set_from_bytes(bytes)?;
        // SAFETY: the caller contract requires unique ownership of a live
        // handle for mutation.
        let document = unsafe { &mut (*document).inner };
        document
            .require_package_capabilities(&capabilities)
            .map_err(|error| engine_failure(&error, error_code::OPERATION))
    });
    match result {
        Ok(()) => NuifFfiError::OK,
        Err(error) => NuifFfiError::failure(error),
    }
}

/// Evaluates and rasterizes a document or authorized package and returns the
/// transport-neutral snapshot report as UTF-8 JSON.
///
/// The report contains a SHA-256 commitment to the exact RGBA bytes rather
/// than transferring the potentially large raster across this call.
///
/// # Safety
/// `document` must be a live, non-concurrently accessed handle and `out` must
/// point to writable buffer storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nuif_ffi_document_snapshot_report(
    document: *const NuifFfiDocument,
    width: f64,
    height: f64,
    out: *mut NuifFfiBuffer,
) -> NuifFfiError {
    if let Err(error) = unsafe { output_slot(out) } {
        return NuifFfiError::failure(error);
    }
    // SAFETY: output slot was checked and is initialized before any failure.
    unsafe { *out = NuifFfiBuffer::EMPTY };
    if document.is_null() {
        return NuifFfiError::failure(Failure::new(
            error_code::INVALID_ARGUMENT,
            "document handle is null",
        ));
    }
    let result = protect(|| {
        // SAFETY: the caller contract requires a live, non-concurrent handle.
        let document = unsafe { &(*document).inner };
        let snapshot = document
            .snapshot(&profile_zero_context(width, height))
            .map_err(|error| engine_failure(&error, error_code::OPERATION))?;
        serialize_report(&snapshot.report(), MAX_SNAPSHOT_REPORT_BYTES)
    });
    match result {
        Ok(bytes) => {
            // SAFETY: output slot was checked above.
            unsafe { *out = NuifFfiBuffer::from_vec(bytes) };
            NuifFfiError::OK
        }
        Err(error) => NuifFfiError::failure(error),
    }
}

/// Applies a bounded JSON patch atomically and returns the new canonical hash.
///
/// # Safety
/// `document` must be a live, uniquely owned handle; `patch` must reference
/// `len` readable bytes (unless zero); and `out` must point to writable buffer
/// storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nuif_ffi_document_apply_patch(
    document: *mut NuifFfiDocument,
    patch: *const u8,
    len: usize,
    out: *mut NuifFfiBuffer,
) -> NuifFfiError {
    if let Err(error) = unsafe { output_slot(out) } {
        return NuifFfiError::failure(error);
    }
    unsafe { *out = NuifFfiBuffer::EMPTY };
    if document.is_null() {
        return NuifFfiError::failure(Failure::new(
            error_code::INVALID_ARGUMENT,
            "document handle is null",
        ));
    }
    let result = protect(|| {
        // SAFETY: forwarded directly from this function's caller contract.
        let bytes = unsafe { input(patch, len, MAX_PATCH_BYTES) }?;
        let patch = patch_from_bytes(bytes)?;
        // SAFETY: the caller contract requires unique ownership of a live
        // handle for mutation.
        let document = unsafe { &mut (*document).inner };
        document
            .apply_patch(&patch)
            .map_err(|error| engine_failure(&error, error_code::OPERATION))?;
        document
            .canonical_hash()
            .map(String::into_bytes)
            .map_err(|error| engine_failure(&error, error_code::VALIDATION))
    });
    match result {
        Ok(bytes) => {
            unsafe { *out = NuifFfiBuffer::from_vec(bytes) };
            NuifFfiError::OK
        }
        Err(error) => NuifFfiError::failure(error),
    }
}

/// Returns the ABI profile/version record as UTF-8 JSON.
///
/// # Safety
/// `out` must point to writable buffer storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nuif_ffi_capabilities(out: *mut NuifFfiBuffer) -> NuifFfiError {
    if let Err(error) = unsafe { output_slot(out) } {
        return NuifFfiError::failure(error);
    }
    let result = protect(|| {
        serde_json::to_vec(&serde_json::json!({
            "schema_version": 1,
            "api_profile": API_PROFILE,
            "binding_version": env!("CARGO_PKG_VERSION"),
            "encodings": ["nuif-text-0", "nuif-cbor-0"],
            "package_profile": "nuif-package-0",
            "package_capabilities": [OPENTYPE_VARIABLE_TRUETYPE_PROFILE],
            "operations": ["load", "load_package", "validate", "canonical_hash", "export", "export_package", "package_capability_report", "require_package_capabilities", "snapshot_report", "apply_patch"],
            "limits": {
                "document_bytes": MAX_INPUT_BYTES,
                "package_bytes": MAX_PACKAGE_BYTES,
                "capability_set_bytes": MAX_CAPABILITY_SET_BYTES,
                "required_capabilities": MAX_REQUIRED_CAPABILITIES,
                "capability_bytes": MAX_CAPABILITY_BYTES,
                "snapshot_report_bytes": MAX_SNAPSHOT_REPORT_BYTES,
                "patch_bytes": MAX_PATCH_BYTES,
                "patch_transactions": MAX_PATCH_TRANSACTIONS,
                "patch_operations": MAX_PATCH_OPERATIONS,
            },
            "stable": false,
            "authorities": [],
        }))
        .map_err(|error| Failure::new(error_code::OUTPUT_LIMIT, error.to_string()))
    });
    match result {
        Ok(bytes) => {
            unsafe { *out = NuifFfiBuffer::from_vec(bytes) };
            NuifFfiError::OK
        }
        Err(error) => NuifFfiError::failure(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nuif_codec::{CanonicalText, Encoder};
    use nuif_core::{Document, Entity, EntityId, EntityKind};
    use nuif_testing::{VariableFontFixtureLocation, variable_font_package_fixture};

    fn document() -> Document {
        let mut document = Document::empty(EntityId::new(1));
        let root = Entity::new(EntityId::new(2), EntityKind::Container);
        document.roots.push(root.id);
        document.entities.insert(root.id, root);
        document
    }

    fn take_buffer(buffer: NuifFfiBuffer) -> Vec<u8> {
        if buffer.ptr.is_null() {
            return Vec::new();
        }
        // SAFETY: test buffers come directly from this crate's ABI functions.
        unsafe { Vec::from_raw_parts(buffer.ptr, buffer.len, buffer.capacity) }
    }

    #[test]
    fn byte_surface_round_trips_through_the_core() {
        let input = CanonicalText.encode(&document()).unwrap();
        let mut handle = ptr::null_mut();
        let error =
            unsafe { nuif_ffi_document_load(input.as_ptr(), input.len(), 0, &raw mut handle) };
        assert_eq!(error.code, error_code::OK);
        assert!(!handle.is_null());

        let mut hash = NuifFfiBuffer::EMPTY;
        let error = unsafe { nuif_ffi_document_canonical_hash(handle, &raw mut hash) };
        assert_eq!(error.code, error_code::OK);
        assert!(
            String::from_utf8(take_buffer(hash))
                .unwrap()
                .starts_with("nuif-cbor-0:")
        );

        let mut exported = NuifFfiBuffer::EMPTY;
        let error = unsafe { nuif_ffi_document_export(handle, 0, &raw mut exported) };
        assert_eq!(error.code, error_code::OK);
        assert_eq!(take_buffer(exported), input);
        unsafe { nuif_ffi_document_free(handle) };
    }

    #[test]
    fn invalid_arguments_fail_without_mutation_or_allocation() {
        let mut output = NuifFfiBuffer::EMPTY;
        let error = unsafe { nuif_ffi_document_validate(ptr::null(), &raw mut output) };
        assert_eq!(error.code, error_code::INVALID_ARGUMENT);
        assert!(output.ptr.is_null());
        unsafe { nuif_ffi_error_free(error) };

        let mut handle = ptr::null_mut();
        let error = unsafe { nuif_ffi_document_load(ptr::null(), 1, 0, &raw mut handle) };
        assert_eq!(error.code, error_code::INVALID_ARGUMENT);
        assert!(handle.is_null());
        unsafe { nuif_ffi_error_free(error) };
    }

    #[test]
    fn capabilities_are_explicitly_experimental() {
        let mut output = NuifFfiBuffer::EMPTY;
        let error = unsafe { nuif_ffi_capabilities(&raw mut output) };
        assert_eq!(error.code, error_code::OK);
        let capabilities: serde_json::Value = serde_json::from_slice(&take_buffer(output)).unwrap();
        assert_eq!(capabilities["api_profile"], API_PROFILE);
        assert_eq!(capabilities["stable"], false);
        assert_eq!(
            capabilities["package_capabilities"][0],
            OPENTYPE_VARIABLE_TRUETYPE_PROFILE
        );
    }

    #[test]
    fn variable_font_package_crosses_the_ffi_without_semantic_forks() {
        let package = variable_font_package_fixture(VariableFontFixtureLocation::Interior);
        let bytes = package.encode().unwrap();
        let mut handle = ptr::null_mut();
        let error =
            unsafe { nuif_ffi_document_load_package(bytes.as_ptr(), bytes.len(), &raw mut handle) };
        assert_eq!(error.code, error_code::OK);

        let unsupported = b"[]";
        let mut report = NuifFfiBuffer::EMPTY;
        let error = unsafe {
            nuif_ffi_document_package_capability_report(
                handle,
                unsupported.as_ptr(),
                unsupported.len(),
                &raw mut report,
            )
        };
        assert_eq!(error.code, error_code::OK);
        let report: serde_json::Value = serde_json::from_slice(&take_buffer(report)).unwrap();
        assert_eq!(report["fully_supported"], false);

        let mut snapshot = NuifFfiBuffer::EMPTY;
        let error =
            unsafe { nuif_ffi_document_snapshot_report(handle, 640.0, 96.0, &raw mut snapshot) };
        assert_eq!(error.code, error_code::OPERATION);
        assert!(snapshot.ptr.is_null());
        unsafe { nuif_ffi_error_free(error) };

        let supported = format!("[\"{OPENTYPE_VARIABLE_TRUETYPE_PROFILE}\"]");
        let error = unsafe {
            nuif_ffi_document_require_package_capabilities(
                handle,
                supported.as_ptr(),
                supported.len(),
            )
        };
        assert_eq!(error.code, error_code::OK);

        let direct = NuifDocument::load_package_with_capabilities(
            &bytes,
            &BTreeSet::from([OPENTYPE_VARIABLE_TRUETYPE_PROFILE.to_owned()]),
        )
        .unwrap()
        .snapshot(&profile_zero_context(640.0, 96.0))
        .unwrap()
        .report();
        let error =
            unsafe { nuif_ffi_document_snapshot_report(handle, 640.0, 96.0, &raw mut snapshot) };
        assert_eq!(error.code, error_code::OK);
        let observed: serde_json::Value = serde_json::from_slice(&take_buffer(snapshot)).unwrap();
        assert_eq!(observed, serde_json::to_value(direct).unwrap());

        let mut exported = NuifFfiBuffer::EMPTY;
        let error = unsafe { nuif_ffi_document_export_package(handle, 0, &raw mut exported) };
        assert_eq!(error.code, error_code::OK);
        assert_eq!(take_buffer(exported), bytes);
        unsafe { nuif_ffi_document_free(handle) };
    }

    #[test]
    fn capability_transport_is_bounded_and_canonical() {
        assert!(matches!(
            capability_set_from_bytes(br#"["feature.example","feature.example"]"#),
            Err(Failure {
                code: error_code::INVALID_ARGUMENT,
                ..
            })
        ));
        assert!(matches!(
            capability_set_from_bytes(br#"["not valid"]"#),
            Err(Failure {
                code: error_code::INVALID_ARGUMENT,
                ..
            })
        ));
        let mut handle = ptr::null_mut();
        let error = unsafe {
            nuif_ffi_document_load_package(ptr::null(), MAX_PACKAGE_BYTES + 1, &raw mut handle)
        };
        assert_eq!(error.code, error_code::INPUT_LIMIT);
        assert!(handle.is_null());
        unsafe { nuif_ffi_error_free(error) };
    }
}
