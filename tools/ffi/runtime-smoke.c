#include "bindings/nuif_ffi.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int report_error(const char *operation, NuifFfiError error) {
    fprintf(stderr, "%s failed (%u): ", operation, error.code);
    if (error.message.ptr != NULL) {
        (void)fwrite(error.message.ptr, 1, error.message.len, stderr);
    }
    fputc('\n', stderr);
    nuif_ffi_error_free(error);
    return 1;
}

static int read_file(const char *path, uint8_t **bytes, size_t *len) {
    FILE *file = fopen(path, "rb");
    long size;
    if (file == NULL || fseek(file, 0, SEEK_END) != 0 ||
        (size = ftell(file)) < 0 || fseek(file, 0, SEEK_SET) != 0) {
        if (file != NULL) {
            fclose(file);
        }
        return 1;
    }
    *len = (size_t)size;
    *bytes = malloc(*len == 0 ? 1 : *len);
    if (*bytes == NULL || fread(*bytes, 1, *len, file) != *len) {
        free(*bytes);
        *bytes = NULL;
        fclose(file);
        return 1;
    }
    return fclose(file) != 0;
}

static int write_file(const char *path, const uint8_t *bytes, size_t len) {
    FILE *file = fopen(path, "wb");
    if (file == NULL) {
        return 1;
    }
    int failed = fwrite(bytes, 1, len, file) != len;
    return fclose(file) != 0 || failed;
}

int main(int argc, char **argv) {
    static const uint8_t supported[] =
        "[\"nuif-opentype-variable-truetype-single-0\"]";
    static const uint8_t unsupported[] = "[]";
    uint8_t *package = NULL;
    size_t package_len = 0;
    NuifFfiDocument *document = NULL;
    NuifFfiBuffer output = {0};
    NuifFfiError error;
    int result = 1;

    if (argc != 3 || read_file(argv[1], &package, &package_len) != 0) {
        fprintf(stderr, "usage: ffi-runtime-smoke <package.nuif> <snapshot.json>\n");
        goto cleanup;
    }

    error = nuif_ffi_capabilities(&output);
    if (error.code != NUIF_FFI_OK || output.ptr == NULL || output.len == 0) {
        result = report_error("capabilities", error);
        goto cleanup;
    }
    nuif_ffi_buffer_free(output);
    output = (NuifFfiBuffer){0};

    error = nuif_ffi_document_load_package(package, package_len, &document);
    if (error.code != NUIF_FFI_OK || document == NULL) {
        result = report_error("load package", error);
        goto cleanup;
    }

    error = nuif_ffi_document_package_capability_report(
        document, unsupported, sizeof(unsupported) - 1, &output);
    if (error.code != NUIF_FFI_OK || output.ptr == NULL || output.len == 0) {
        result = report_error("unsupported capability report", error);
        goto cleanup;
    }
    nuif_ffi_buffer_free(output);
    output = (NuifFfiBuffer){0};

    error = nuif_ffi_document_snapshot_report(document, 640.0, 96.0, &output);
    if (error.code != NUIF_FFI_OPERATION || output.ptr != NULL) {
        result = report_error("unauthorized snapshot rejection", error);
        goto cleanup;
    }
    nuif_ffi_error_free(error);

    error = nuif_ffi_document_require_package_capabilities(
        document, supported, sizeof(supported) - 1);
    if (error.code != NUIF_FFI_OK) {
        result = report_error("authorize package", error);
        goto cleanup;
    }

    error = nuif_ffi_document_package_capability_report(
        document, supported, sizeof(supported) - 1, &output);
    if (error.code != NUIF_FFI_OK || output.ptr == NULL || output.len == 0) {
        result = report_error("supported capability report", error);
        goto cleanup;
    }
    nuif_ffi_buffer_free(output);
    output = (NuifFfiBuffer){0};

    error = nuif_ffi_document_snapshot_report(document, 640.0, 96.0, &output);
    if (error.code != NUIF_FFI_OK || output.ptr == NULL || output.len == 0) {
        result = report_error("snapshot", error);
        goto cleanup;
    }
    if (write_file(argv[2], output.ptr, output.len) != 0) {
        fprintf(stderr, "could not write snapshot report\n");
        goto cleanup;
    }
    nuif_ffi_buffer_free(output);
    output = (NuifFfiBuffer){0};

    error = nuif_ffi_document_export_package(
        document, NUIF_FFI_PACKAGE_PORTABLE, &output);
    if (error.code != NUIF_FFI_OK || output.len != package_len ||
        memcmp(output.ptr, package, package_len) != 0) {
        result = report_error("package byte fixpoint", error);
        goto cleanup;
    }

    result = 0;

cleanup:
    if (output.ptr != NULL) {
        nuif_ffi_buffer_free(output);
    }
    nuif_ffi_document_free(document);
    free(package);
    return result;
}
