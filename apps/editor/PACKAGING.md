# Native editor packaging

`cargo xtask editor-package` builds the release application wrapper, creates the native package for the host platform, runs the packaged executable with `--help`, hashes the executable and archive, and writes `target/dist/editor-package-manifest.json`.

| Host | Package | Archive | Application entry point |
|---|---|---|---|
| macOS | `target/dist/nuif-editor-macos-<arch>/NUIF Editor.app` | `.tar.gz` | Finder, `open`, or the executable under `Contents/MacOS` |
| Windows | `target/dist/nuif-editor-windows-<arch>/` | `.zip` | `NUIF Editor.exe` |
| Linux | `target/dist/nuif-editor-linux-<arch>/` | `.tar.gz` | `bin/nuif-editor`; freedesktop entry and scalable icon are under `share/` |

`cargo xtask editor-launch` rebuilds and verifies the host package and then opens it. On macOS this calls `open -n` on the `.app`; on Windows and Linux it starts the packaged application executable.

Packages contain the Apache-2.0 and MIT license files, a scope notice and a package-local manifest. Archives preserve the executable layout and are the CI artifacts. Current development packages are unsigned. Code signing, notarization and installer-store publication require platform credentials and are deliberately separate release operations; the manifest records the unsigned status instead of implying trust that is not present.

The Linux package expects a graphical Wayland or X11 session and a graphics driver supported by wgpu. It is a relocatable application directory, not a distribution-specific system package. The Windows application wrapper uses the GUI subsystem so a console window is not opened; the separate `nuif-editor` binary retains its headless/JSONL interface.
