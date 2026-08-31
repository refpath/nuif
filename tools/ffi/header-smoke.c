#include "bindings/nuif_ffi.h"

int main(void) {
    NuifFfiDocument *document = 0;
    NuifFfiBuffer buffer = {0};
    NuifFfiError error = nuif_ffi_document_load(0, 0, NUIF_FFI_ENCODING_TEXT, &document);
    (void)nuif_ffi_document_load_package(0, 0, &document);
    (void)nuif_ffi_document_validate(document, &buffer);
    (void)nuif_ffi_document_canonical_hash(document, &buffer);
    (void)nuif_ffi_document_export(document, NUIF_FFI_ENCODING_CBOR, &buffer);
    (void)nuif_ffi_document_export_package(document, NUIF_FFI_PACKAGE_PORTABLE, &buffer);
    (void)nuif_ffi_document_package_capability_report(document, 0, 0, &buffer);
    (void)nuif_ffi_document_require_package_capabilities(document, 0, 0);
    (void)nuif_ffi_document_snapshot_report(document, 640.0, 96.0, &buffer);
    (void)nuif_ffi_document_apply_patch(document, 0, 0, &buffer);
    (void)nuif_ffi_capabilities(&buffer);
    nuif_ffi_error_free(error);
    nuif_ffi_document_free(document);
    return 0;
}
