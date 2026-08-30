# Native editor packaging

`cargo xtask editor-package` builds the release application wrapper, creates the native package for the host platform, runs the packaged executable with `--help` and `--version`, hashes the executable and archive, and writes a package-specific manifest plus `target/dist/editor-package-manifest.json`.

| Host | Package | Archive | Application entry point |
|---|---|---|---|
| macOS | `target/dist/nuif-editor-<version>-macos-<arch>/NUIF Editor.app` | `.tar.gz` | Finder, `open`, or the executable under `Contents/MacOS` |
| Windows | `target/dist/nuif-editor-<version>-windows-<arch>/` | `.zip` | `NUIF Editor.exe` |
| Linux | `target/dist/nuif-editor-<version>-linux-<arch>/` | `.tar.gz` | `bin/nuif-editor`; freedesktop entry and scalable icon are under `share/` |

`cargo xtask editor-launch` rebuilds and verifies the host package and then opens it. On macOS this calls `open -n` on the `.app`; on Windows and Linux it starts the packaged application executable.

Packages contain the Apache-2.0 and MIT license files, a scope notice and a package-local manifest. Versioned archives preserve the executable layout and are CI artifacts. Tag builds also become GitHub prerelease assets with package manifests, `SHA256SUMS`, a CycloneDX software bill of materials, `release-manifest.json`, and GitHub artifact attestations. The tag and publication contract is defined in `docs/VERSIONING.md` and ADR 0007.

Downloaded packages are not the canonical developer installation. `cargo
xtask editor-install --user` builds the same package from the retained source
checkout and installs an immutable version with a source/build receipt. The
explicit update, doctor, rollback and uninstall lifecycle is documented in
`INSTALLING.md` and ADR 0009.

Current alpha archives are unsigned. Code signing, notarization and installer or store publication require platform credentials and remain separate release operations. The archive manifest records the unsigned status. A locally built macOS developer installation is ad-hoc signed after copying and records `adhoc-local`; that is neither Developer ID nor notarization. Apple and Windows signing requirements are recorded in `nuif:research:github-release-delivery-and-provenance` and the source-install boundary in `nuif:research:developer-source-installation-and-os-trust`.

The Linux package expects a graphical Wayland or X11 session, Fontconfig at runtime, and a graphics driver supported by wgpu. Building it on Debian or Ubuntu requires `libfontconfig1-dev`; CI installs that package before every workspace or editor build. The archive is a relocatable application directory, not a distribution-specific system package. The Windows application wrapper uses the GUI subsystem so a console window is not opened; the separate `nuif-editor` binary retains its headless/JSONL interface.

The `native-editor` CI matrix runs the editor check, tests, semantic and visual trial, complete sandboxed install/doctor/uninstall lifecycle, and packaging command on GitHub-hosted macOS, Windows and Linux systems. Each job uploads its archive, package manifest, install receipt evidence, semantic-node inventory and shell screenshot. This is native-host evidence; a successful cross-compilation alone is not treated as platform verification.

The release workflow adds Linux Arm64 and separate macOS Arm64 and x86-64 hosts. It requires five archives and five package manifests before publication. Each release job builds at the tagged source revision, runs the sandboxed lifecycle in strict `alpha` mode and uses the pinned Rust 1.98.0 toolchain.
