import Foundation

var output = NuifFfiBuffer(ptr: nil, len: 0, capacity: 0)
let error = nuif_ffi_capabilities(&output)
guard error.code == NUIF_FFI_OK else {
    nuif_ffi_error_free(error)
    fatalError("nuif_ffi_capabilities failed")
}
defer { nuif_ffi_buffer_free(output) }

guard let pointer = output.ptr, output.len > 0 else {
    fatalError("nuif_ffi_capabilities returned no bytes")
}
let data = Data(bytes: pointer, count: output.len)
let value = try JSONSerialization.jsonObject(with: data)
guard let object = value as? [String: Any],
      object["api_profile"] as? String == "nuif-ffi-0" else {
    fatalError("nuif_ffi_capabilities returned the wrong profile")
}
