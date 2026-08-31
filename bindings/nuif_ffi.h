#ifndef NUIF_FFI_H
#define NUIF_FFI_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* nuif-ffi-0; experimental, byte-oriented and not ABI-stable.
 * One thread may access a given NuifFfiDocument handle at a time. Independent
 * handles and Rust-owned result buffers may be used or freed on other threads.
 */
#define NUIF_FFI_API_PROFILE "nuif-ffi-0"
#define NUIF_FFI_ENCODING_TEXT 0u
#define NUIF_FFI_ENCODING_CBOR 1u
#define NUIF_FFI_PACKAGE_PORTABLE 0u
#define NUIF_FFI_PACKAGE_AUTHORING 1u

#define NUIF_FFI_OK 0u
#define NUIF_FFI_INVALID_ARGUMENT 1u
#define NUIF_FFI_INPUT_LIMIT 2u
#define NUIF_FFI_DECODE 3u
#define NUIF_FFI_VALIDATION 4u
#define NUIF_FFI_OPERATION 5u
#define NUIF_FFI_OUTPUT_LIMIT 6u
#define NUIF_FFI_PANIC 255u

typedef struct NuifFfiDocument NuifFfiDocument;

typedef struct NuifFfiBuffer {
    uint8_t *ptr;
    size_t len;
    size_t capacity;
} NuifFfiBuffer;

typedef struct NuifFfiError {
    uint32_t code;
    NuifFfiBuffer message;
} NuifFfiError;

void nuif_ffi_buffer_free(NuifFfiBuffer buffer);
void nuif_ffi_error_free(NuifFfiError error);

NuifFfiError nuif_ffi_document_load(
    const uint8_t *bytes,
    size_t len,
    uint32_t encoding,
    NuifFfiDocument **out);
NuifFfiError nuif_ffi_document_load_package(
    const uint8_t *bytes,
    size_t len,
    NuifFfiDocument **out);
void nuif_ffi_document_free(NuifFfiDocument *document);

NuifFfiError nuif_ffi_document_validate(
    const NuifFfiDocument *document,
    NuifFfiBuffer *out);
NuifFfiError nuif_ffi_document_canonical_hash(
    const NuifFfiDocument *document,
    NuifFfiBuffer *out);
NuifFfiError nuif_ffi_document_export(
    const NuifFfiDocument *document,
    uint32_t encoding,
    NuifFfiBuffer *out);
NuifFfiError nuif_ffi_document_export_package(
    const NuifFfiDocument *document,
    uint32_t mode,
    NuifFfiBuffer *out);
NuifFfiError nuif_ffi_document_package_capability_report(
    const NuifFfiDocument *document,
    const uint8_t *capabilities_json,
    size_t capabilities_len,
    NuifFfiBuffer *out);
NuifFfiError nuif_ffi_document_require_package_capabilities(
    NuifFfiDocument *document,
    const uint8_t *capabilities_json,
    size_t capabilities_len);
NuifFfiError nuif_ffi_document_snapshot_report(
    const NuifFfiDocument *document,
    double width,
    double height,
    NuifFfiBuffer *out);
NuifFfiError nuif_ffi_document_apply_patch(
    NuifFfiDocument *document,
    const uint8_t *patch,
    size_t len,
    NuifFfiBuffer *out);

NuifFfiError nuif_ffi_capabilities(NuifFfiBuffer *out);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* NUIF_FFI_H */
