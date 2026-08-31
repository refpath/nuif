# Native binding contracts

`nuif_ffi.h` describes the experimental `nuif-ffi-0` byte-oriented C ABI. The
ABI owns opaque document handles and returned byte buffers; callers release
them with the matching library functions. It exposes no Rust model structs and
grants no filesystem, network or host-product authority. Bare documents and
packages use the same handle: callers explicitly negotiate required package
capabilities before mutation or snapshots, and snapshot JSON is the same
transport-neutral report used by the other SDK surfaces. One thread may access
a handle at a time; independent handles and returned buffers may be used on
other threads.

Gate FFI compiles the header and links/runs a C consumer against the release
library on POSIX targets. That consumer exercises an exact variable-font
package fixpoint, denial before capability authorization, and full snapshot
equality with the independently invoked CLI report. The same consumer repeats
under AddressSanitizer and UndefinedBehaviorSanitizer; a C++17 translation unit
checks layout and call compatibility, and `nuif_ffi.symbols` pins the exact
experimental exported-symbol set. Windows checks the header and library build
but does not claim the POSIX runtime, sanitizer or `nm` evidence.

The profile is deliberately not stable. Before promotion it still needs pinned
header generation and source-level compatibility policy, a linked C++ runtime
consumer, Swift/Kotlin consumers, complete target-matrix sanitizer evidence and
real XCFramework/AAR packages. The release workflow now emits an experimental
`nuif-ffi-<version>-<platform>-<architecture>` archive containing the header,
symbol baseline, native library artifacts, conformance evidence and checksums;
this does not promote the ABI to stable. Until those gates pass, the WASM
package or CLI remains
the supported foreign-language integration path.
