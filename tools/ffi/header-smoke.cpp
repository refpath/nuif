#include "bindings/nuif_ffi.h"

#include <cstddef>
#include <cstdint>
#include <type_traits>

static_assert(std::is_standard_layout_v<NuifFfiBuffer>);
static_assert(std::is_trivially_copyable_v<NuifFfiBuffer>);
static_assert(std::is_standard_layout_v<NuifFfiError>);

int main() {
    NuifFfiDocument *document = nullptr;
    NuifFfiBuffer buffer{};
    NuifFfiError error =
        nuif_ffi_document_load(nullptr, 0, NUIF_FFI_ENCODING_TEXT, &document);
    (void)nuif_ffi_document_load_package(nullptr, 0, &document);
    (void)nuif_ffi_document_validate(document, &buffer);
    (void)nuif_ffi_document_canonical_hash(document, &buffer);
    (void)nuif_ffi_document_export(document, NUIF_FFI_ENCODING_CBOR, &buffer);
    (void)nuif_ffi_document_export_package(
        document, NUIF_FFI_PACKAGE_PORTABLE, &buffer);
    (void)nuif_ffi_document_package_capability_report(
        document, nullptr, 0, &buffer);
    (void)nuif_ffi_document_require_package_capabilities(document, nullptr, 0);
    (void)nuif_ffi_document_snapshot_report(document, 640.0, 96.0, &buffer);
    (void)nuif_ffi_document_apply_patch(document, nullptr, 0, &buffer);
    (void)nuif_ffi_capabilities(&buffer);
    nuif_ffi_error_free(error);
    nuif_ffi_document_free(document);
    return 0;
}
