---
id: nuif:research:developer-source-installation-and-os-trust
kind: synthesis
status: verified
title: Source-built developer installation and operating-system trust boundaries
source:
  url: https://doc.rust-lang.org/cargo/commands/cargo-install.html
  repository: https://github.com/refpath/nuif
  authors: [Cargo team, GitHub, Apple, Microsoft, Homebrew maintainers, Scoop maintainers, Nix maintainers]
  published_at: "Cargo, GitHub, Apple, Microsoft, Homebrew, Scoop, and Nix documentation retrieved 2026-08-30"
  license: "Documentation terms vary by publisher; NUIF is Apache-2.0 OR MIT"
retrieved_at: 2026-08-30
tags: [editor, installation, developer-tool, cargo, provenance, macos, windows, linux, homebrew, scoop, nix]
confidence: 0.96
claims: []
relations:
  - type: extends
    target: nuif:research:github-release-delivery-and-provenance
    note: Release evidence remains useful while installation moves from downloaded unsigned binaries to verified local source builds.
  - type: depends_on
    target: nuif:research:cargo-workspace-xtask-and-ci-layout
    note: The existing xtask and pinned workspace toolchain own the cross-platform developer lifecycle.
links:
  spec: []
  adr: [adrs/0009-developer-source-installation.md]
  rfc: []
  code: [xtask/src/main.rs, apps/editor/PACKAGING.md, docs/VERSIONING.md]
  experiments: []
---

# Summary

NUIF Editor is a reference, conformance and research tool rather than a
consumer desktop product. Its primary installation path therefore builds a
reviewed revision locally with the checked-in Cargo lock file and installs it
inside the current user's account. Tagged GitHub archives remain durable CI
evidence, reproducibility inputs and an expert download path; they are not the
default trust mechanism for running the editor.

Local compilation does not create a universal security-policy exemption.
Apple Gatekeeper evaluates software downloaded from outside the App Store, and
Microsoft Smart App Control can require trusted signatures for all executable
files. The installer must not disable Gatekeeper, System Integrity Protection,
Defender, SmartScreen or Smart App Control. A managed device whose policy
rejects the local build requires an organization-approved signing identity or
device policy. That identity may be internal and does not require marketplace
publication.

The source installer resolves a named alpha channel through the published,
attested release manifest; pins its tag and source revision; checks out that
revision; builds with `Cargo.lock`; and records the resulting binary digest,
toolchain and source identity. Installation is user-scoped, explicit updates
retain one rollback version, and uninstall removes only directories carrying a
NUIF-owned marker.

## Evidence

- Cargo builds installable binaries from a Git repository and accepts an exact
  tag or revision, a selected binary and `--locked` to use the checked-in lock
  file. Locator: Cargo Book, `cargo install`, "Install Options" and "Dealing
  with the Lockfile", retrieved 2026-08-30:
  https://doc.rust-lang.org/cargo/commands/cargo-install.html.
- GitHub verifies binary attestations with `gh attestation verify`; repository,
  signer workflow, source ref and source digest can be constrained by the
  verifier. Locator: GitHub Docs, *Using artifact attestations to establish
  provenance for builds*, binary verification, retrieved 2026-08-30:
  https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations.
- Gatekeeper verifies downloaded applications from outside the App Store for
  an identified developer, notarization and alteration, and requests first-run
  approval. Locator: Apple Platform Security, *Gatekeeper and runtime
  protection in macOS*, retrieved 2026-08-30:
  https://support.apple.com/guide/security/gatekeeper-and-runtime-protection-sec5599b66df/web.
- Microsoft documents publisher and file-hash reputation, states that an
  unsigned version starts without transferable publisher reputation, and
  notes that Smart App Control signature checks apply to all executable files.
  Locator: Microsoft Learn, *SmartScreen reputation for Windows app
  developers*, retrieved 2026-08-30:
  https://learn.microsoft.com/windows/apps/package-and-deploy/smartscreen-reputation.
- Homebrew taps distribute external formulae through Git repositories and
  support source builds; its Cargo formula guidance separates dependency fetch
  from offline installation. Locator: Homebrew Documentation, *How to Create
  and Maintain a Tap* and *Formula Cookbook*, retrieved 2026-08-30:
  https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap and
  https://docs.brew.sh/Formula-Cookbook.
- Scoop buckets are Git repositories of JSON manifests. Their archive URL and
  SHA-256 fields provide convenient user-scoped installation but still execute
  the downloaded Windows artifact, so Scoop does not remove SmartScreen's
  publisher boundary. Locator: Scoop Wiki, *Buckets* and *App Manifests*,
  retrieved 2026-08-30:
  https://github.com/ScoopInstaller/Scoop/wiki/Buckets and
  https://github.com/ScoopInstaller/Scoop/wiki/App-Manifests.
- Nix flakes name source inputs and lock their resolved references. They are a
  useful optional reproducible environment for macOS and Linux, but are not a
  native Windows installation path. Locator: Nix Reference Manual, `nix
  flake`, retrieved 2026-08-30:
  https://releases.nixos.org/nix/nix-2.25.5/manual/command-ref/new-cli/nix3-flake.html.

## Mechanism

The local installer has two trust modes. `source` installs the current clean
checkout and records its revision. `alpha` first resolves the newest published
NUIF prerelease, downloads `release-manifest.json`, verifies its GitHub
attestation against the NUIF release workflow and tag, and then requires the
checked-out source revision to equal the attested revision. Both modes build
with the repository lock file.

Each installation lives in an immutable version directory identified by the
application version, source revision and installed binary digest. A small state
document selects the active and previous installations. Platform integration
points reference only the active version: a user Applications bundle on macOS,
a per-user program and Start-menu shortcut on Windows, and XDG executable,
desktop and icon entries on Linux. Changing the active version is separate from
building it, allowing rollback without a rebuild.

macOS applies a local ad-hoc signature after copying the locally built bundle
and verifies that signature. This provides a structurally valid local code
signature, not a Developer ID or notarization claim. Windows never modifies a
certificate store or security policy. Linux never writes outside user-owned
XDG paths. A marker and receipt constrain doctor, rollback and uninstall to
the directories created by the installer.

## NUIF relevance

**Adopt** a source-built, user-scoped developer channel as the normal way to
run the reference editor. It matches the project's research role and makes the
reviewed source, lock file and toolchain part of the install receipt.

**Retain** GitHub packages, checksums, the SBOM and attestations as independent
build evidence and recovery material. Their existence does not imply an
operating-system publisher identity.

**Offer later** a source-building Homebrew tap and a Nix flake. A Scoop bucket
can be a convenience for explicitly opted-in Windows users, but cannot be
described as a trust bypass.

**Reject** silent self-updates, mutable branch installation, piping an
uninspected network script into a shell, automatic trust-store changes, and
instructions that disable platform security controls.

## Open questions

- Whether an organization wants to publish and trust an internal macOS or
  Windows signing identity for managed development machines.
- Whether sufficient demand exists to maintain a Homebrew source formula, a
  Nix flake and a Scoop convenience bucket in addition to the built-in source
  lifecycle.
- Whether the explicit alpha update resolver should later support stable and
  nightly channels after those channels have separate publication policies.
