#include "bindings/nuif_ffi.h"

int main(void) {
    NuifFfiBuffer capabilities = {0};
    NuifFfiError error = nuif_ffi_capabilities(&capabilities);
    if (error.code != NUIF_FFI_OK || capabilities.ptr == 0 || capabilities.len == 0) {
        nuif_ffi_error_free(error);
        return 1;
    }
    nuif_ffi_buffer_free(capabilities);
    return 0;
}
