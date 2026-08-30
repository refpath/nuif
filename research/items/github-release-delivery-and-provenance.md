---
id: nuif:research:github-release-delivery-and-provenance
kind: synthesis
status: verified
title: GitHub prerelease delivery, artifact provenance, and desktop signing boundaries
source:
  url: https://docs.github.com/en/code-security/concepts/supply-chain-security/immutable-releases
  repository: https://github.com/actions
  authors: [GitHub, Cargo team, Apple, Microsoft, axodotdev]
  published_at: "GitHub, Cargo, Apple, Microsoft, and cargo-dist documentation retrieved 2026-08-30"
  license: "Documentation terms vary by publisher; referenced Actions are MIT"
retrieved_at: 2026-08-30
tags: [release, github-actions, semver, provenance, signing, macos, windows, packaging, supply-chain]
confidence: 0.96
claims: []
relations:
  - type: extends
    target: nuif:research:cargo-workspace-xtask-and-ci-layout
    note: The repository now requires native editor release artifacts rather than CI-only package artifacts.
  - type: related_to
    target: nuif:research:macos-metal-block-future-incompatibility
    note: The release graph includes the macOS Metal dependency and native-window verification boundary.
links:
  spec: []
  adr: [adrs/0007-editor-release-delivery.md]
  rfc: []
  code: [.github/workflows/ci.yml, .github/workflows/release.yml, xtask/src/main.rs, apps/editor/PACKAGING.md, docs/VERSIONING.md]
  experiments: []
---

# Summary

Cargo accepts Semantic Versioning prerelease identifiers such as
`0.1.0-alpha.1` and orders numeric prerelease components numerically. GitHub
Releases attach binary assets to a tag and distinguish prereleases from stable
releases. GitHub's immutable-release procedure creates a draft, attaches all
assets, and publishes the draft; publication then prevents tag and asset
mutation when repository immutability is enabled. GitHub artifact attestations
bind an artifact digest to the workflow identity and source revision through
OpenID Connect.

Direct desktop distribution has a separate trust boundary. Apple notarization
requires Developer ID signing, the hardened runtime, a secure timestamp, and a
notary-service submission. Microsoft documents Artifact Signing as its
recommended signing service for non-Store distribution and states that
unsigned applications receive stronger SmartScreen warnings. Repository-hosted
checksums and GitHub attestations establish origin and integrity, but they do
not replace operating-system code signing.

NUIF uses tag-driven GitHub prereleases for the reference editor. The first tag
is `v0.1.0-alpha.1`. Five native-host jobs build versioned archives, record
package manifests, and attest the archives. A final job creates checksums, a
separate editor and MCP CycloneDX software bills of materials, and a release
manifest, uploads all files to a draft, and publishes the prerelease. Every external workflow action is
pinned to the full commit of a verified release, checkout credentials are not
persisted, and a pinned zizmor audit rejects regressions. The alpha artifacts
remain explicitly unsigned until platform credentials are configured and
reviewed.

## Evidence

- Cargo requires three numeric version components and permits a hyphenated
  prerelease whose period-separated numeric components compare numerically.
  Locator: Cargo Book, *The Manifest Format*, `version` field, lines 88–98,
  retrieved 2026-08-30:
  https://doc.rust-lang.org/cargo/reference/manifest.html#the-version-field.
- GitHub recommends creating a draft, attaching all assets, and publishing the
  draft for immutable releases. Published immutable releases prevent tag and
  asset mutation and receive a release attestation. Locator: *Immutable
  releases*, "What immutable releases protect" and "Best practices for
  publishing immutable releases", retrieved 2026-08-30:
  https://docs.github.com/en/code-security/concepts/supply-chain-security/immutable-releases.
- GitHub artifact attestations require `id-token: write`, `contents: read`, and
  `attestations: write`; `actions/attest@v4` accepts a `subject-path` for binary
  provenance. Locator: *Using artifact attestations to establish provenance
  for builds*, binary example, retrieved 2026-08-30:
  https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations.
- GitHub's public runner table lists Ubuntu 24.04 on x86-64 and Arm64, Windows
  2025 on x86-64, macOS 15 on Arm64, and `macos-15-intel` on x86-64. Locator:
  *GitHub-hosted runners reference*, public repository table, lines 40–51,
  retrieved 2026-08-30:
  https://docs.github.com/en/actions/reference/runners/github-hosted-runners.
- Apple requires a Developer ID certificate, hardened runtime, secure
  timestamp, and valid executable signatures before notarization. `notarytool`
  and `stapler` support scripted distribution. Locator: *Notarizing macOS
  software before distribution*, "Prepare your software for notarization" and
  "Add a notarization step to your build scripts", retrieved 2026-08-30:
  https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution.
- Microsoft states that unsigned applications cannot transfer publisher
  reputation between releases and identifies Artifact Signing as the
  recommended non-Store signing service. Locator: *SmartScreen reputation for
  Windows app developers*, "Certificate options" and "Minimizing SmartScreen
  warnings", updated 2026-05-06 and retrieved 2026-08-30:
  https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation.
- cargo-dist 0.32.0 generates release workflows and general Rust archives. The
  NUIF editor already has native application layouts, package-local manifests,
  semantic trials, and platform-specific smoke tests in `cargo xtask
  editor-package`. Replacing that path would duplicate package policy without
  removing signing credentials or host verification. Locator: cargo-dist
  release `v0.32.0`, published 2026-05-21 and retrieved 2026-08-30:
  https://github.com/axodotdev/cargo-dist/releases/tag/v0.32.0; NUIF
  `xtask/src/main.rs`, `build_editor_package` and `verify_editor_package`.
- cargo-cyclonedx 0.5.9 generates per-crate or per-binary CycloneDX documents
  from Cargo metadata and the lock file. It honors `SOURCE_DATE_EPOCH` and
  omits the random serial number for reproducible output. Locator:
  cargo-cyclonedx release `0.5.9`, published 2026-03-19 and retrieved
  2026-08-30:
  https://github.com/CycloneDX/cyclonedx-rust-cargo/releases/tag/cargo-cyclonedx-0.5.9.
- GitHub states that pinning an action to a full-length commit SHA is the only
  immutable way to use an action. NUIF resolved the selected release tags
  through the GitHub Git data API and records both the SHA and human-readable
  release beside each `uses` entry. Locator: GitHub Docs, *Secure use
  reference*, "Using third-party actions", retrieved 2026-08-30:
  https://docs.github.com/en/actions/reference/security/secure-use.
- zizmor 1.29.0 identifies mutable action references, persisted checkout
  credentials, expression-to-shell interpolation, undocumented permissions,
  and absent concurrency controls. After remediation, `zizmor 1.29.0
  --pedantic .` reports no findings; CI runs the same version through
  `zizmor-action` 0.6.2 with its action itself pinned by full SHA. Locator:
  zizmor audit documentation and action release, retrieved 2026-08-30:
  https://docs.zizmor.sh/audits/ and
  https://github.com/zizmorcore/zizmor-action/releases/tag/v0.6.2.

## Mechanism

The editor version in `apps/editor/Cargo.toml` determines the only accepted
release tag: `v` followed by that exact version. `cargo xtask release-check`
rejects a mismatched tag, SemVer build metadata, or an uncommitted source tree.
The tag checkout runs the complete verification harness before native package
jobs start.

Each native job builds and tests on the target operating system and processor
architecture. `cargo xtask editor-package` produces a versioned archive and a
platform manifest containing the source revision, binary digest, archive
digest, smoke-test result, and signing status. `actions/attest@v4` records
provenance for both files. Five additional native jobs build and exercise the
separately versioned stateless MCP binary, then attest its archive and manifest.
The publication job downloads all editor, binding and MCP artifacts, requires
five editor archives/manifests and five MCP archives/manifests, and runs the attested
cargo-cyclonedx 0.5.9 binary with the tagged commit time as
`SOURCE_DATE_EPOCH`. It replaces the checkout path with `/src`, writes
`SHA256SUMS`, and combines editor packages, browser bindings and MCP services
into `release-manifest.json`. It attests both software bills of materials and both
index files before using GitHub CLI to create or resume a draft, upload the
assets, and publish the prerelease.

Workflow dependencies are resolved separately from Cargo dependencies. All
`uses` references are full commit SHAs with release-version comments, checkout
sets `persist-credentials: false`, and shell steps receive GitHub context values
through environment variables rather than source interpolation. Default token
access is read-only; only the package and publication jobs receive the OIDC,
attestation, and release-write permissions they require. CI cancels superseded
runs and executes a pinned pedantic zizmor audit, making these constraints
enforceable rather than review conventions.

The macOS package separates the SemVer version from the bundle build number.
`CFBundleShortVersionString` receives the three-component base version, while
`CFBundleVersion` receives the numeric GitHub workflow run number. This avoids
placing the `alpha.1` suffix in Apple's numeric bundle fields. The package
manifest and archive name retain the full SemVer version.

## NUIF relevance

**Borrow** GitHub's draft-attach-publish sequence, artifact attestations, and
native runner matrix. These mechanisms bind release assets to a reviewed tag
without introducing a release-specific build service.

**Adapt** SemVer at the application boundary. The editor uses
`0.1.0-alpha.1`, while draft specification profiles and unpublished library
crates retain their existing version namespaces.

**Reject** cargo-dist for the first alpha. Its generic archive generation does
not replace the existing macOS bundle, Linux desktop layout, Windows GUI
wrapper, package manifest, semantic trial, or signing boundary.

**Reject** describing unsigned alpha artifacts as trusted desktop
installations. Checksums and attestations provide integrity and provenance;
Developer ID notarization and Windows publisher signing remain separate
credentialed release stages.

## Open questions

- Which organization identity and credential store will sign macOS and Windows
  artifacts after the alpha review?
- Whether immutable releases are enabled is a repository setting that requires
  user review; the workflow remains compatible by using a draft before
  publication.
- Whether the stable editor distribution uses direct archives, the Microsoft
  Store, a macOS disk image, or installer packages depends on signing and update
  policy that the alpha does not establish.
