# Native binding contracts

`nuif_ffi.h` describes the experimental `nuif-ffi-0` byte-oriented C ABI. The
ABI owns opaque document handles and returned byte buffers; callers release
them with the matching library functions. It exposes no Rust model structs and
grants no filesystem, network or host-product authority.

The profile is deliberately not stable. Before promotion it needs a pinned
header/symbol compatibility check, panic and allocator tests under sanitizers,
consumer fixtures in C/C++/Swift/Kotlin, a declared threading contract and
real target packages. Until those gates pass, the WASM package or CLI remains
the supported foreign-language integration path.
