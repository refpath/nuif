# Versioning and release operation

The native editor uses Semantic Versioning independently from the draft
specification profiles and unpublished library crates. The first editor release
is `0.1.0-alpha.1`; the corresponding Git tag is `v0.1.0-alpha.1`. The decision
and source evidence are recorded in ADR 0007 and
`nuif:research:github-release-delivery-and-provenance`.

## Version contract

- `apps/editor/Cargo.toml` contains the editor version.
- A release tag equals `v` followed by the exact editor version.
- Prerelease corrections increment the numeric suffix. A published tag is not
  moved or reused.
- The draft specification's profile-zero identifier is not an application
  version and does not change when the editor prerelease increments.
- Editor alpha maturity does not transfer to proposed package, resource,
  capture, reconstruction or model artifacts. Each of those requires its own
  profile identifier, conformance evidence and release record.
- Library crate versions remain independent until a crate publication policy
  is adopted.

## Release sequence

1. Update the editor version and `docs/releases/<version>.md` in one reviewed
   commit.
2. Run `cargo xtask all`, the native editor tests, `cargo deny check`, and
   `cargo xtask editor-package` on a clean tree.
3. Create and push the exact version tag.
4. `.github/workflows/release.yml` checks out the tag and runs `cargo xtask
   release-check <tag>` followed by the complete verification harness.
5. Native jobs build, test, package, and attest five host architectures.
6. The publication job writes checksums and a combined release manifest,
   creates a draft release, uploads all assets, and publishes the prerelease.

The workflow can be rerun manually for an existing unpublished tag. It refuses
to replace a published release. GitHub's immutable-release setting is compatible
with the draft-attach-publish sequence but remains a user-managed repository
setting. Locator: ADR 0007; `.github/workflows/release.yml`; GitHub *Immutable
releases*, retrieved 2026-08-30:
https://docs.github.com/en/code-security/concepts/supply-chain-security/immutable-releases.

## Artifact contract

Release archives use this form:

```text
nuif-editor-<version>-<os>-<architecture>.<tar.gz|zip>
```

Each archive has a sibling `.manifest.json`. The release also contains
`SHA256SUMS`, `nuif-editor-<version>.cdx.json`, and
`release-manifest.json`. The CycloneDX document inventories the editor's Cargo
dependency graph for all release targets. GitHub provenance can be checked with:

```sh
gh attestation verify <archive> --repo refpath/nuif
```

SHA-256 verification on Linux uses `sha256sum -c SHA256SUMS`. macOS uses
`shasum -a 256 -c SHA256SUMS`. PowerShell users can compare
`Get-FileHash -Algorithm SHA256` with the corresponding line in
`SHA256SUMS`.

## Developer channel contract

Release archives are evidence and an expert opt-in path. The primary developer
installation builds locally from a retained source checkout according to ADR
0009. `source` installs the current clean revision. `alpha` additionally
requires the exact release tag and rejects a dirty tree.

An explicit `editor-update --channel alpha` operation queries published
prereleases and selects the greatest numeric `MAJOR.MINOR.PATCH-alpha.N`
version. It accepts a release only when `release-manifest.json` exists, its five
package records passed from one clean source revision, and its GitHub
attestation verifies the repository, release workflow, tag, source digest and
GitHub-hosted runner. The updater fetches the attested tag with Git hooks
disabled and requires the checked-out commit to equal that digest before
building with `Cargo.lock`.

The lifecycle retains an active and previous immutable install. It never moves
a release tag, installs from a mutable branch, performs a silent update or
changes an operating-system trust policy. See `apps/editor/INSTALLING.md` for
commands and paths.

## Signing boundary

The alpha artifacts are unsigned and record that status in each manifest.
Checksums and GitHub attestations verify bytes and build origin; they do not
provide an operating-system publisher identity. Apple distribution requires a
Developer ID signature, hardened runtime, timestamp, notarization, and ticket
stapling. Windows direct distribution requires a trusted publisher signature
for an identified publisher and reduced SmartScreen friction. Locator:
`nuif:research:github-release-delivery-and-provenance`, Evidence.

Signing credentials are never stored in the repository. Adding a signing stage
requires a separate review of identity ownership, secret storage, rotation,
fork behavior, and release recovery.
