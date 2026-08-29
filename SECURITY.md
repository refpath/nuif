# Security

NUIF treats documents, assets, extensions and adapter inputs as untrusted.

Security-sensitive areas include binary/text decoders, archive decompression, fonts/images, vector path complexity, shader/effect extensions, GPU allocations and collaboration inputs.

Do not publish exploitable parser/renderer details in a public issue before a coordinated fix is available. Maintainers should establish a private vulnerability-reporting channel before the first public binary release.

The normative threat model lives in `spec/11-security.md`.
