---
id: nuif:adr:0009
status: accepted
---

# ADR 0009: Source-built developer installation

Decision delegated to research on 2026-08-30. Evidence:
`nuif:research:developer-source-installation-and-os-trust` and
`nuif:research:github-release-delivery-and-provenance`.

## Context

NUIF Editor will remain a developer-facing reference and conformance tool.
Unsigned native archives are useful build evidence, but making them the normal
installation path couples routine research use to Apple and Microsoft
publisher-reputation systems. Marketplace publication and paid signing
identities are not architectural requirements for a developer tool.

A different path must still preserve source identity, dependency locking,
platform integration, updates, rollback and safe removal. It must not claim
that local compilation bypasses every managed-device policy or weaken an
operating system's security controls.

## Decision

1. The primary editor installation is a local build from a clean, pinned source
   revision using the checked-in `Cargo.lock` and repository Rust toolchain.
2. `cargo xtask` owns user-scoped install, update, doctor, rollback and
   uninstall commands. There is no system-wide installation mode.
3. Source installs record the version, source revision and cleanliness,
   lockfile digest, toolchain, platform, architecture and installed binary
   digest in a machine-readable receipt.
4. The `alpha` channel resolves only published prereleases. Its release
   manifest must pass GitHub attestation verification for the NUIF release
   workflow, tag and source revision before the revision is built.
5. Updates are explicit. The active and previous immutable installations are
   retained so activation can be rolled back without rebuilding.
6. macOS installs a locally built, ad-hoc-signed application under the user's
   Applications directory. Windows installs under the user's local application
   directories and may create a user Start-menu shortcut. Linux installs under
   user XDG and `~/.local` paths.
7. The lifecycle never disables Gatekeeper, System Integrity Protection,
   Defender, SmartScreen or Smart App Control and never adds a certificate to a
   trust store. Managed-device exceptions require organization approval.
8. GitHub release archives remain CI evidence, reproducibility material and an
   expert opt-in path. They are not described as trusted publisher-signed
   applications.
9. Homebrew source formulae and Nix flakes are compatible future convenience
   layers. A Scoop bucket downloads the unsigned Windows archive and therefore
   cannot replace the source-built trust path.

## Consequences

- A developer can install and update the editor without Apple Developer Program
  membership, Microsoft Store publication or administrator access on an
  ordinary development machine.
- The default path takes compilation time and requires the platform Rust/native
  build prerequisites.
- A locked-down machine may still reject locally built unsigned code. Its
  administrator must supply an approved certificate or policy; NUIF does not
  work around that control.
- Release automation and source installation have separate responsibilities:
  releases attest the channel input, while the local lifecycle owns the
  executable installed on the developer's machine.
