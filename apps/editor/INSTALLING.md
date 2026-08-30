# Developer installation

NUIF Editor is installed as a user-scoped developer tool built from reviewed
source. Apple notarization, Microsoft Store publication and administrator
access are not prerequisites for this path. The source checkout is the control
plane for install, update, diagnosis, rollback and removal; retain it after the
initial installation.

## Prerequisites

- Git, Rustup and the Rust toolchain selected by `rust-toolchain.toml`;
- GitHub CLI (`gh`) for verified alpha-channel updates;
- the native build prerequisites for the host;
- Linux Fontconfig development files when building on Linux.

On Debian or Ubuntu, install the Linux native dependency with:

```sh
sudo apt-get install libfontconfig1-dev
```

Do not pipe a remote installation script into a shell. Clone an exact release
tag, inspect it when required, and build through the checked-in xtask:

```sh
git clone --branch <release-tag> --depth 1 https://github.com/refpath/nuif.git
cd nuif
git rev-parse HEAD
cargo xtask editor-install --user --channel alpha
```

The `alpha` install rejects a dirty checkout, requires the exact
`v<editor-version>` tag at `HEAD`, uses `Cargo.lock`, builds the native package,
installs it, and immediately runs `editor-doctor`. A local research branch uses
the `source` channel:

```sh
cargo xtask editor-install --user --channel source
```

Dirty source is rejected by default. An intentional local experiment can opt
in with `--allow-dirty`; its receipt contains a deterministic working-tree
digest as well as the commit revision.

## Lifecycle

Run lifecycle commands from any retained NUIF checkout that contains ADR 0009
support:

```sh
cargo xtask editor-update --user --channel alpha --check
cargo xtask editor-update --user --channel alpha
cargo xtask editor-doctor --user
cargo xtask editor-rollback --user
cargo xtask editor-uninstall --user
```

`editor-update` selects the highest published numeric alpha version. Before it
executes release source, it downloads `release-manifest.json`, verifies the
GitHub attestation against `refpath/nuif`, the release workflow, the tag, the
source revision and a GitHub-hosted runner, then fetches that exact tag with Git
hooks disabled. The fetched commit must equal the attested revision and remain
clean. Updates are explicit; the editor never updates itself while it is
running.

The active and previous immutable installations are retained. Rollback only
changes the platform integration point and state file; it does not rebuild or
contact the network. A later rollback swaps the two versions again.

`editor-doctor` verifies the managed marker, state and receipt schemas, source
and lockfile identities, installed binary digest and reported version,
platform integration, and the local macOS ad-hoc signature. Its JSON report
also states whether Git, GitHub CLI, Cargo and Rustc are available for a source
update.

## User-owned paths

| Host | Immutable state | Active application integration |
|---|---|---|
| macOS | `~/Library/Application Support/org.nuif.Editor/dev/versions/` | `~/Applications/NUIF Editor Dev.app` |
| Windows | `%LOCALAPPDATA%\NUIF Editor Dev\versions\` | `%LOCALAPPDATA%\Programs\NUIF Editor Dev\` and a user Start-menu shortcut |
| Linux | `${XDG_DATA_HOME:-~/.local/share}/nuif-editor-dev/versions/` | `~/.local/bin/nuif-editor-dev` plus user XDG desktop and icon entries |

Every removable state root and Windows program directory carries a product
marker. Existing unrelated files, directories, symbolic links, desktop entries
or shortcuts are rejected rather than claimed. `--root <absolute-path>` places
state and integration below an isolated root for CI and disposable testing;
filesystem roots are rejected.

## Trust boundary

The macOS source install applies and verifies a free local ad-hoc signature.
That signature is not Developer ID, notarization or a publisher identity. The
Windows source install does not add a certificate or change Defender,
SmartScreen or Smart App Control. No lifecycle command disables Gatekeeper,
System Integrity Protection or another operating-system security control.

A managed machine may still require an organization-approved signing
certificate or device policy. That is an administrator-owned trust decision,
not an installation workaround and not a requirement to publish through an
Apple or Microsoft marketplace.

## Secondary distribution

GitHub archives, their manifests, checksums, SBOM and attestations remain
release evidence, reproducibility material and an expert opt-in download path.
They are intentionally not the primary developer installation.

A future Homebrew tap should use a source formula. A Nix flake can provide a
locked macOS/Linux development environment. A Scoop bucket may provide a
convenient user-scoped Windows archive installation, but it still runs the
downloaded unsigned executable and cannot be presented as a SmartScreen trust
solution.
