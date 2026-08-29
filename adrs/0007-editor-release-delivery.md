---
id: nuif:adr:0007
status: accepted
---

# ADR 0007: Tag-driven native editor prereleases

Decision delegated to research on 2026-08-30. Evidence:
`nuif:research:github-release-delivery-and-provenance` and
`nuif:research:cargo-workspace-xtask-and-ci-layout`.

## Context

The native editor packaging task produces host-specific archives and verifies
their executable entry points. CI retained those archives as temporary workflow
artifacts. It did not provide a version contract, a durable download location,
cross-platform checksums, or build provenance. Application delivery requires a
tag-to-version invariant and a publication transaction that does not expose a
partially assembled release.

## Decision

1. The editor uses Semantic Versioning independently from unpublished library
   crates. The first application version is `0.1.0-alpha.1` and its release tag
   is `v0.1.0-alpha.1`.
2. A release tag identifies one immutable source revision. Tags are not moved;
   a correction increments the prerelease number.
3. `.github/workflows/release.yml` validates that the tag equals `v` followed
   by the editor package version and that the checkout is clean.
4. The release workflow runs the complete repository harness before building
   native packages on Linux x86-64, Linux Arm64, Windows x86-64, macOS Arm64,
   and macOS x86-64 hosts.
5. Each archive name contains the editor version, operating system, and
   architecture. Each package manifest records its source revision, version,
   platform, architecture, executable and archive SHA-256 digests, smoke tests,
   and signing status.
6. GitHub artifact attestations cover every archive and platform manifest. The
   publication job creates and attests `SHA256SUMS`, a CycloneDX software bill
   of materials, and `release-manifest.json`.
7. Publication creates a draft, attaches every asset, and then publishes the
   prerelease. A failed build or upload leaves no published partial release.
8. Alpha packages are unsigned. macOS Developer ID signing and notarization,
   Windows publisher signing, installer publication, and automatic updates are
   credentialed stages that require separate review.
9. cargo-dist is not introduced for the alpha because the existing xtask owns
   native application layouts, manifests, trials, and smoke tests.

## Consequences

- GitHub Releases becomes the durable download surface for tagged editor
  builds. CI artifacts remain diagnostic evidence for ordinary commits.
- Application consumers can verify SHA-256 checksums and GitHub provenance
  without building the repository.
- Unsigned alpha packages may trigger Gatekeeper or SmartScreen warnings. The
  release notes state this limitation.
- The stable-release gate remains open until macOS and Windows signing
  identities, credential rotation, and update policy are approved.
