use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::{build_editor_package, command_text, editor_version};

const PRODUCT: &str = "org.nuif.editor.dev";
const SCHEMA_VERSION: u64 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Platform {
    Macos,
    Windows,
    Linux,
}

impl Platform {
    fn host() -> Result<Self, String> {
        match env::consts::OS {
            "macos" => Ok(Self::Macos),
            "windows" => Ok(Self::Windows),
            "linux" => Ok(Self::Linux),
            platform => Err(format!(
                "developer installation is unsupported on {platform}"
            )),
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Macos => "macos",
            Self::Windows => "windows",
            Self::Linux => "linux",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Channel {
    Source,
    Alpha,
}

impl Channel {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "source" => Ok(Self::Source),
            "alpha" => Ok(Self::Alpha),
            _ => Err(format!(
                "unsupported editor channel {value:?}; expected source or alpha"
            )),
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Alpha => "alpha",
        }
    }
}

#[derive(Debug)]
struct InstallOptions {
    root: Option<PathBuf>,
    channel: Channel,
    allow_dirty: bool,
    expected_tag: Option<String>,
    expected_revision: Option<String>,
}

#[derive(Debug)]
struct LifecycleOptions {
    root: Option<PathBuf>,
}

#[derive(Debug)]
struct InstallPaths {
    state_root: PathBuf,
    versions: PathBuf,
    marker: PathBuf,
    state: PathBuf,
    integration: IntegrationPaths,
    sandboxed: bool,
}

#[derive(Debug)]
enum IntegrationPaths {
    Macos {
        app: PathBuf,
    },
    Windows {
        program: PathBuf,
        shortcut: PathBuf,
    },
    Linux {
        binary: PathBuf,
        desktop: PathBuf,
        icon: PathBuf,
    },
}

#[derive(Debug)]
struct SourceIdentity {
    version: String,
    revision: String,
    dirty: bool,
    tag: Option<String>,
    repository: Option<String>,
    lock_sha256: String,
    toolchain: String,
    channel: Channel,
}

#[derive(Debug)]
struct InstalledVersion {
    id: String,
    root: PathBuf,
    binary: PathBuf,
    app_bundle: Option<PathBuf>,
    binary_sha256: String,
    signing: String,
}

pub(crate) fn install(arguments: &[String]) -> Result<(), String> {
    let options = parse_install_options(arguments)?;
    let platform = Platform::host()?;
    let paths = install_paths(platform, options.root.as_deref())?;
    preflight_install(platform, &paths)?;
    let source = source_identity(&options)?;
    let package = build_editor_package()?;
    let installed = stage_install(platform, &paths, &package.package_root, &source)?;
    activate_install(platform, &paths, &installed, &source)?;
    doctor_paths(platform, &paths)?;
    println!(
        "installed NUIF Editor {} from {} ({})",
        source.version,
        source.revision,
        installed.root.display()
    );
    Ok(())
}

pub(crate) fn doctor(arguments: &[String]) -> Result<(), String> {
    let options = parse_lifecycle_options("editor-doctor", arguments)?;
    let platform = Platform::host()?;
    let paths = install_paths(platform, options.root.as_deref())?;
    doctor_paths(platform, &paths)
}

pub(crate) fn rollback(arguments: &[String]) -> Result<(), String> {
    let options = parse_lifecycle_options("editor-rollback", arguments)?;
    let platform = Platform::host()?;
    let paths = install_paths(platform, options.root.as_deref())?;
    validate_managed_root(&paths)?;
    let mut state = read_state(&paths)?;
    let active = state_string(&state, "active")?;
    let previous = state_string(&state, "previous")?;
    if active == previous {
        return Err("active and previous installations are identical".to_owned());
    }
    let previous_install = installed_version_from_receipt(platform, &paths, &previous)?;
    set_integration(platform, &paths, &previous_install)?;
    state["active"] = json!(previous);
    state["previous"] = json!(active);
    if let Err(error) = write_json_atomic(&paths.state, &state) {
        if let Ok(active_install) = installed_version_from_receipt(platform, &paths, &active) {
            let _ = set_integration(platform, &paths, &active_install);
        }
        return Err(error);
    }
    doctor_paths(platform, &paths)?;
    println!("rolled back NUIF Editor to {}", previous_install.id);
    Ok(())
}

pub(crate) fn uninstall(arguments: &[String]) -> Result<(), String> {
    let options = parse_lifecycle_options("editor-uninstall", arguments)?;
    let platform = Platform::host()?;
    let paths = install_paths(platform, options.root.as_deref())?;
    if !paths.state_root.exists() {
        println!("NUIF Editor is not installed for this user");
        return Ok(());
    }
    validate_managed_root(&paths)?;
    remove_integration(platform, &paths)?;
    remove_managed_state_root(&paths)?;
    println!("uninstalled the user-scoped NUIF Editor developer build");
    Ok(())
}

fn parse_install_options(arguments: &[String]) -> Result<InstallOptions, String> {
    let mut root = None;
    let mut channel = Channel::Source;
    let mut allow_dirty = false;
    let mut expected_tag = None;
    let mut expected_revision = None;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--user" => {}
            "--allow-dirty" => allow_dirty = true,
            "--root" => root = Some(next_path(arguments, &mut index, "--root")?),
            "--channel" => {
                channel = Channel::parse(next_value(arguments, &mut index, "--channel")?)?;
            }
            "--expected-tag" => {
                expected_tag =
                    Some(next_value(arguments, &mut index, "--expected-tag")?.to_owned());
            }
            "--expected-revision" => {
                expected_revision =
                    Some(next_value(arguments, &mut index, "--expected-revision")?.to_owned());
            }
            "--system" | "--global" => {
                return Err("system-wide editor installation is not supported".to_owned());
            }
            value => return Err(format!("unknown editor-install option {value:?}")),
        }
        index += 1;
    }
    if channel == Channel::Alpha && allow_dirty {
        return Err("the alpha channel cannot install a dirty source tree".to_owned());
    }
    validate_root(root.as_deref())?;
    if let Some(revision) = expected_revision.as_deref() {
        validate_revision(revision)?;
    }
    Ok(InstallOptions {
        root,
        channel,
        allow_dirty,
        expected_tag,
        expected_revision,
    })
}

fn parse_lifecycle_options(
    command: &str,
    arguments: &[String],
) -> Result<LifecycleOptions, String> {
    let mut root = None;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--user" => {}
            "--root" => root = Some(next_path(arguments, &mut index, "--root")?),
            value => return Err(format!("unknown {command} option {value:?}")),
        }
        index += 1;
    }
    validate_root(root.as_deref())?;
    Ok(LifecycleOptions { root })
}

fn next_value<'a>(
    arguments: &'a [String],
    index: &mut usize,
    option: &str,
) -> Result<&'a str, String> {
    *index += 1;
    arguments
        .get(*index)
        .map(String::as_str)
        .ok_or_else(|| format!("{option} requires a value"))
}

fn next_path(arguments: &[String], index: &mut usize, option: &str) -> Result<PathBuf, String> {
    Ok(PathBuf::from(next_value(arguments, index, option)?))
}

fn validate_root(root: Option<&Path>) -> Result<(), String> {
    if let Some(root) = root
        && !root.is_absolute()
    {
        return Err(format!(
            "installation root must be absolute: {}",
            root.display()
        ));
    }
    if let Some(root) = root
        && root.parent().is_none()
    {
        return Err(format!(
            "installation root must not be a filesystem root: {}",
            root.display()
        ));
    }
    Ok(())
}

fn preflight_install(platform: Platform, paths: &InstallPaths) -> Result<(), String> {
    if paths.state_root.exists() {
        validate_managed_root(paths)?;
    }
    match (&paths.integration, platform) {
        (IntegrationPaths::Macos { app }, Platform::Macos) => {
            validate_existing_managed_symlink(app, &paths.versions)
        }
        (IntegrationPaths::Windows { program, shortcut }, Platform::Windows) => {
            if program.exists() {
                validate_windows_program(program)?;
            }
            if shortcut.exists() && !paths.state.exists() {
                return Err(format!(
                    "refusing to replace an unmanaged Windows shortcut: {}",
                    shortcut.display()
                ));
            }
            Ok(())
        }
        (
            IntegrationPaths::Linux {
                binary,
                desktop,
                icon,
            },
            Platform::Linux,
        ) => {
            validate_existing_managed_symlink(binary, &paths.versions)?;
            validate_existing_managed_symlink(icon, &paths.versions)?;
            if desktop.exists() {
                let contents = fs::read_to_string(desktop).map_err(|error| error.to_string())?;
                if !contents.contains("X-NUIF-Managed=true") {
                    return Err(format!(
                        "refusing to replace an unmanaged desktop entry: {}",
                        desktop.display()
                    ));
                }
            }
            Ok(())
        }
        _ => Err("installation paths do not match the host platform".to_owned()),
    }
}

fn install_paths(platform: Platform, root: Option<&Path>) -> Result<InstallPaths, String> {
    if let Some(root) = root {
        let state_root = root.join("state");
        let integration_root = root.join("integration");
        let integration = match platform {
            Platform::Macos => IntegrationPaths::Macos {
                app: integration_root
                    .join("Applications")
                    .join("NUIF Editor Dev.app"),
            },
            Platform::Windows => IntegrationPaths::Windows {
                program: integration_root.join("Programs").join("NUIF Editor Dev"),
                shortcut: integration_root
                    .join("Start Menu")
                    .join("NUIF Editor Dev.lnk"),
            },
            Platform::Linux => IntegrationPaths::Linux {
                binary: integration_root.join("bin").join("nuif-editor-dev"),
                desktop: integration_root
                    .join("applications")
                    .join("org.nuif.Editor.Dev.desktop"),
                icon: integration_root.join("icons").join("nuif-editor-dev.svg"),
            },
        };
        return Ok(InstallPaths {
            versions: state_root.join("versions"),
            marker: state_root.join(".nuif-editor-managed.json"),
            state: state_root.join("state.json"),
            state_root,
            integration,
            sandboxed: true,
        });
    }

    let (state_root, integration) = match platform {
        Platform::Macos => {
            let home = required_environment_path("HOME")?;
            (
                home.join("Library/Application Support/org.nuif.Editor/dev"),
                IntegrationPaths::Macos {
                    app: home.join("Applications/NUIF Editor Dev.app"),
                },
            )
        }
        Platform::Windows => {
            let local = required_environment_path("LOCALAPPDATA")?;
            let roaming = required_environment_path("APPDATA")?;
            (
                local.join("NUIF Editor Dev"),
                IntegrationPaths::Windows {
                    program: local.join("Programs/NUIF Editor Dev"),
                    shortcut: roaming
                        .join("Microsoft/Windows/Start Menu/Programs/NUIF Editor Dev.lnk"),
                },
            )
        }
        Platform::Linux => {
            let home = required_environment_path("HOME")?;
            let data = env::var_os("XDG_DATA_HOME")
                .filter(|value| !value.is_empty())
                .map_or_else(|| home.join(".local/share"), PathBuf::from);
            let binary_root = env::var_os("XDG_BIN_HOME")
                .filter(|value| !value.is_empty())
                .map_or_else(|| home.join(".local/bin"), PathBuf::from);
            (
                data.join("nuif-editor-dev"),
                IntegrationPaths::Linux {
                    binary: binary_root.join("nuif-editor-dev"),
                    desktop: data
                        .join("applications")
                        .join("org.nuif.Editor.Dev.desktop"),
                    icon: data
                        .join("icons/hicolor/scalable/apps")
                        .join("nuif-editor-dev.svg"),
                },
            )
        }
    };
    Ok(InstallPaths {
        versions: state_root.join("versions"),
        marker: state_root.join(".nuif-editor-managed.json"),
        state: state_root.join("state.json"),
        state_root,
        integration,
        sandboxed: false,
    })
}

fn required_environment_path(name: &str) -> Result<PathBuf, String> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| format!("{name} is required for user-scoped installation"))
}

fn source_identity(options: &InstallOptions) -> Result<SourceIdentity, String> {
    let version = editor_version()?;
    let revision = required_command_text("git", &["rev-parse", "HEAD"])?;
    validate_revision(&revision)?;
    if let Some(expected) = options.expected_revision.as_deref()
        && revision != expected
    {
        return Err(format!(
            "source revision {revision} does not match attested revision {expected}"
        ));
    }
    let dirty = required_command_text("git", &["status", "--porcelain"])?;
    let dirty = !dirty.is_empty();
    if dirty && !options.allow_dirty {
        return Err(
            "source tree is dirty; commit the source or pass --allow-dirty for an explicit local experiment"
                .to_owned(),
        );
    }
    let expected_release_tag = format!("v{version}");
    let tags = required_command_text("git", &["tag", "--points-at", "HEAD"])?;
    let tag = tags
        .lines()
        .find(|candidate| *candidate == expected_release_tag)
        .map(str::to_owned);
    if options.channel == Channel::Alpha && tag.is_none() {
        return Err(format!(
            "alpha installation requires exact tag {expected_release_tag} at HEAD"
        ));
    }
    if let Some(expected) = options.expected_tag.as_deref()
        && tag.as_deref() != Some(expected)
    {
        return Err(format!(
            "source tag {:?} does not match attested tag {expected:?}",
            tag.as_deref()
        ));
    }
    let lock_sha256 = sha256_file(Path::new("Cargo.lock"))?;
    let toolchain = required_command_text("rustc", &["--version"])?;
    Ok(SourceIdentity {
        version,
        revision,
        dirty,
        tag,
        repository: command_text("git", &["config", "--get", "remote.origin.url"]),
        lock_sha256,
        toolchain,
        channel: options.channel,
    })
}

fn validate_revision(revision: &str) -> Result<(), String> {
    if revision.len() != 40 || !revision.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("invalid full Git source revision {revision:?}"));
    }
    Ok(())
}

fn validate_sha256(value: &str, context: &str) -> Result<(), String> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("{context} is not a SHA-256 value"));
    }
    Ok(())
}

fn stage_install(
    platform: Platform,
    paths: &InstallPaths,
    package_root: &Path,
    source: &SourceIdentity,
) -> Result<InstalledVersion, String> {
    ensure_managed_root(paths)?;
    fs::create_dir_all(&paths.versions).map_err(|error| error.to_string())?;
    reject_symlink(&paths.versions)?;
    let staging = unique_staging_directory(&paths.versions)?;
    if let Err(error) = copy_tree_contents(package_root, &staging) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }

    let (binary_relative, app_relative, signing) = match platform {
        Platform::Macos => {
            let app = PathBuf::from("NUIF Editor.app");
            let installed_app = staging.join(&app);
            if let Err(error) = ad_hoc_sign_macos(&installed_app) {
                let _ = fs::remove_dir_all(&staging);
                return Err(error);
            }
            (
                PathBuf::from("NUIF Editor.app/Contents/MacOS/NUIF Editor"),
                Some(app),
                "adhoc-local",
            )
        }
        Platform::Windows => (PathBuf::from("NUIF Editor.exe"), None, "unsigned-local"),
        Platform::Linux => (PathBuf::from("bin/nuif-editor"), None, "unsigned-local"),
    };
    let staging_binary = staging.join(&binary_relative);
    if !staging_binary.is_file() {
        let _ = fs::remove_dir_all(&staging);
        return Err(format!(
            "installed editor binary is absent: {}",
            staging_binary.display()
        ));
    }
    let binary_sha256 = sha256_file(&staging_binary)?;
    let id = install_id(&source.version, &source.revision, &binary_sha256)?;
    let final_root = paths.versions.join(&id);
    let receipt = install_receipt(
        platform,
        paths,
        source,
        &id,
        &binary_relative,
        &binary_sha256,
        signing,
    );
    write_json_atomic(&staging.join("receipt.json"), &receipt)?;

    if final_root.exists() {
        validate_version_directory(&paths.versions, &final_root)?;
        let existing = read_json(&final_root.join("receipt.json"))?;
        if existing["binary_sha256"] != binary_sha256 {
            let _ = fs::remove_dir_all(&staging);
            return Err(format!(
                "existing install {} has an unexpected binary digest",
                final_root.display()
            ));
        }
        fs::remove_dir_all(&staging).map_err(|error| error.to_string())?;
    } else {
        fs::rename(&staging, &final_root).map_err(|error| error.to_string())?;
    }
    Ok(InstalledVersion {
        id,
        binary: final_root.join(binary_relative),
        app_bundle: app_relative.map(|relative| final_root.join(relative)),
        root: final_root,
        binary_sha256,
        signing: signing.to_owned(),
    })
}

fn install_receipt(
    platform: Platform,
    paths: &InstallPaths,
    source: &SourceIdentity,
    id: &str,
    binary_relative: &Path,
    binary_sha256: &str,
    signing: &str,
) -> Value {
    json!({
        "schema_version": SCHEMA_VERSION,
        "product": PRODUCT,
        "status": "installed",
        "install_id": id,
        "version": source.version,
        "channel": source.channel.name(),
        "platform": platform.name(),
        "architecture": env::consts::ARCH,
        "source": {
            "repository": source.repository,
            "tag": source.tag,
            "revision": source.revision,
            "dirty": source.dirty,
            "cargo_lock_sha256": source.lock_sha256,
            "toolchain": source.toolchain,
        },
        "binary_relative_path": binary_relative,
        "binary_sha256": binary_sha256,
        "signing": {
            "status": signing,
            "publisher_identity": false,
        },
        "user_scoped": true,
        "sandboxed": paths.sandboxed,
    })
}

fn install_id(version: &str, revision: &str, binary_sha256: &str) -> Result<String, String> {
    if !version
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
    {
        return Err(format!("editor version is not path-safe: {version:?}"));
    }
    validate_revision(revision)?;
    validate_sha256(binary_sha256, "installed binary digest")?;
    Ok(format!(
        "{version}-{}-{}",
        &revision[..12],
        &binary_sha256[..12]
    ))
}

fn activate_install(
    platform: Platform,
    paths: &InstallPaths,
    installed: &InstalledVersion,
    source: &SourceIdentity,
) -> Result<(), String> {
    let old_state = if paths.state.exists() {
        Some(read_state(paths)?)
    } else {
        None
    };
    let old_active = old_state
        .as_ref()
        .and_then(|state| state["active"].as_str())
        .map(str::to_owned);
    let previous = old_active
        .as_deref()
        .filter(|active| *active != installed.id)
        .map(str::to_owned)
        .or_else(|| {
            old_state
                .as_ref()
                .and_then(|state| state["previous"].as_str())
                .map(str::to_owned)
        });
    set_integration(platform, paths, installed)?;
    let state = json!({
        "schema_version": SCHEMA_VERSION,
        "product": PRODUCT,
        "channel": source.channel.name(),
        "active": installed.id,
        "previous": previous,
    });
    if let Err(error) = write_json_atomic(&paths.state, &state) {
        if let Some(old_active) = old_active {
            if let Ok(old_install) = installed_version_from_receipt(platform, paths, &old_active) {
                let _ = set_integration(platform, paths, &old_install);
            }
        } else {
            let _ = remove_integration(platform, paths);
        }
        return Err(error);
    }
    prune_versions(paths, &installed.id, previous.as_deref())
}

fn ensure_managed_root(paths: &InstallPaths) -> Result<(), String> {
    if paths.state_root.exists() {
        return validate_managed_root(paths);
    }
    fs::create_dir_all(&paths.state_root).map_err(|error| error.to_string())?;
    reject_symlink(&paths.state_root)?;
    write_json_atomic(
        &paths.marker,
        &json!({"schema_version": SCHEMA_VERSION, "product": PRODUCT}),
    )
}

fn validate_managed_root(paths: &InstallPaths) -> Result<(), String> {
    reject_symlink(&paths.state_root)?;
    let marker = read_json(&paths.marker).map_err(|error| {
        format!(
            "refusing to manage unmarked install root {}: {error}",
            paths.state_root.display()
        )
    })?;
    if marker["schema_version"] != SCHEMA_VERSION || marker["product"] != PRODUCT {
        return Err(format!(
            "refusing to manage install root with an unknown marker: {}",
            paths.state_root.display()
        ));
    }
    Ok(())
}

fn reject_symlink(path: &Path) -> Result<(), String> {
    if fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err(format!(
            "managed path must not be a symlink: {}",
            path.display()
        ));
    }
    Ok(())
}

fn validate_version_directory(versions: &Path, version: &Path) -> Result<(), String> {
    if version.parent() != Some(versions) {
        return Err(format!(
            "version path is outside the managed versions directory: {}",
            version.display()
        ));
    }
    reject_symlink(version)
}

fn unique_staging_directory(versions: &Path) -> Result<PathBuf, String> {
    for suffix in 0..100_u32 {
        let staging = versions.join(format!(".staging-{}-{suffix}", std::process::id()));
        match fs::create_dir(&staging) {
            Ok(()) => return Ok(staging),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    Err("could not reserve an installation staging directory".to_owned())
}

fn copy_tree_contents(source: &Path, destination: &Path) -> Result<(), String> {
    if !source.is_dir() {
        return Err(format!(
            "package root is not a directory: {}",
            source.display()
        ));
    }
    for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        copy_entry(&entry.path(), &destination.join(entry.file_name()))?;
    }
    Ok(())
}

fn copy_entry(source: &Path, destination: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(source).map_err(|error| error.to_string())?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "package contains an unsupported symlink: {}",
            source.display()
        ));
    }
    if metadata.is_dir() {
        fs::create_dir(destination).map_err(|error| error.to_string())?;
        for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            copy_entry(&entry.path(), &destination.join(entry.file_name()))?;
        }
        return Ok(());
    }
    if metadata.is_file() {
        fs::copy(source, destination).map_err(|error| error.to_string())?;
        return Ok(());
    }
    Err(format!(
        "package contains an unsupported filesystem entry: {}",
        source.display()
    ))
}

fn ad_hoc_sign_macos(app: &Path) -> Result<(), String> {
    if !cfg!(target_os = "macos") {
        return Err("macOS signing requested on a non-macOS host".to_owned());
    }
    run(
        "codesign",
        &[
            OsStr::new("--force"),
            OsStr::new("--deep"),
            OsStr::new("--sign"),
            OsStr::new("-"),
            app.as_os_str(),
        ],
    )?;
    run(
        "codesign",
        &[
            OsStr::new("--verify"),
            OsStr::new("--deep"),
            OsStr::new("--strict"),
            app.as_os_str(),
        ],
    )
}

fn set_integration(
    platform: Platform,
    paths: &InstallPaths,
    installed: &InstalledVersion,
) -> Result<(), String> {
    match (&paths.integration, platform) {
        (IntegrationPaths::Macos { app }, Platform::Macos) => {
            let target = installed
                .app_bundle
                .as_ref()
                .ok_or_else(|| "macOS install does not contain an application bundle".to_owned())?;
            replace_symlink(target, app, &paths.versions)
        }
        (IntegrationPaths::Windows { program, shortcut }, Platform::Windows) => {
            set_windows_integration(paths, installed, program, shortcut)
        }
        (
            IntegrationPaths::Linux {
                binary,
                desktop,
                icon,
            },
            Platform::Linux,
        ) => {
            replace_symlink(&installed.binary, binary, &paths.versions)?;
            let source_icon = installed
                .root
                .join("share/icons/hicolor/scalable/apps/nuif-editor.svg");
            replace_symlink(&source_icon, icon, &paths.versions)?;
            write_linux_desktop(desktop, binary, icon)
        }
        _ => Err("installation paths do not match the host platform".to_owned()),
    }
}

#[cfg(unix)]
fn replace_symlink(target: &Path, link: &Path, managed_root: &Path) -> Result<(), String> {
    use std::os::unix::fs::symlink;

    if let Some(parent) = link.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    validate_existing_managed_symlink(link, managed_root)?;
    let temporary = link.with_extension(format!("nuif-new-{}", std::process::id()));
    if fs::symlink_metadata(&temporary).is_ok() {
        fs::remove_file(&temporary).map_err(|error| error.to_string())?;
    }
    symlink(target, &temporary).map_err(|error| error.to_string())?;
    fs::rename(&temporary, link).map_err(|error| error.to_string())
}

#[cfg(not(unix))]
fn replace_symlink(_target: &Path, _link: &Path, _managed_root: &Path) -> Result<(), String> {
    Err("symbolic-link integration is unsupported on this host".to_owned())
}

fn validate_existing_managed_symlink(link: &Path, managed_root: &Path) -> Result<(), String> {
    let Ok(metadata) = fs::symlink_metadata(link) else {
        return Ok(());
    };
    if !metadata.file_type().is_symlink() {
        return Err(format!(
            "refusing to replace an unmanaged integration path: {}",
            link.display()
        ));
    }
    let target = fs::read_link(link).map_err(|error| error.to_string())?;
    if !target.starts_with(managed_root) {
        return Err(format!(
            "refusing to replace integration link {} outside {}",
            link.display(),
            managed_root.display()
        ));
    }
    Ok(())
}

fn write_linux_desktop(desktop: &Path, binary: &Path, icon: &Path) -> Result<(), String> {
    if let Some(parent) = desktop.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    if desktop.exists() {
        let existing = fs::read_to_string(desktop).map_err(|error| error.to_string())?;
        if !existing.contains("X-NUIF-Managed=true") {
            return Err(format!(
                "refusing to replace an unmanaged desktop entry: {}",
                desktop.display()
            ));
        }
    }
    let contents = format!(
        "[Desktop Entry]\nType=Application\nName=NUIF Editor Dev\nComment=Native NUIF reference and conformance editor\nExec={} %f\nIcon={}\nTerminal=false\nCategories=Graphics;Development;\nMimeType=application/octet-stream;\nX-NUIF-Managed=true\n",
        desktop_quote(binary),
        desktop_quote(icon),
    );
    write_bytes_atomic(desktop, contents.as_bytes())
}

fn desktop_quote(path: &Path) -> String {
    let value = path.to_string_lossy();
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for character in value.chars() {
        if matches!(character, '\\' | '"' | '`' | '$') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped.push('"');
    escaped
}

fn set_windows_integration(
    paths: &InstallPaths,
    installed: &InstalledVersion,
    program: &Path,
    shortcut: &Path,
) -> Result<(), String> {
    if program.exists() {
        validate_windows_program(program)?;
        fs::remove_dir_all(program).map_err(|error| {
            format!(
                "could not replace {}; close NUIF Editor and retry: {error}",
                program.display()
            )
        })?;
    }
    fs::create_dir_all(program).map_err(|error| error.to_string())?;
    write_json_atomic(
        &program.join(".nuif-editor-managed.json"),
        &json!({"schema_version": SCHEMA_VERSION, "product": PRODUCT}),
    )?;
    let target = program.join("NUIF Editor Dev.exe");
    fs::copy(&installed.binary, &target).map_err(|error| error.to_string())?;
    if let Some(parent) = shortcut.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    if cfg!(target_os = "windows") {
        let script = "$shell = New-Object -ComObject WScript.Shell; $link = $shell.CreateShortcut($env:NUIF_SHORTCUT_PATH); $link.TargetPath = $env:NUIF_TARGET_PATH; $link.WorkingDirectory = $env:NUIF_WORKING_DIRECTORY; $link.IconLocation = $env:NUIF_TARGET_PATH; $link.Save()";
        let status = Command::new("powershell.exe")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                script,
            ])
            .env("NUIF_SHORTCUT_PATH", shortcut)
            .env("NUIF_TARGET_PATH", &target)
            .env("NUIF_WORKING_DIRECTORY", program)
            .status()
            .map_err(|error| error.to_string())?;
        check_status(status, "powershell.exe")?;
    } else if paths.sandboxed {
        write_bytes_atomic(shortcut, b"NUIF managed Windows shortcut fixture\n")?;
    } else {
        return Err("Windows integration requested on a non-Windows host".to_owned());
    }
    Ok(())
}

fn validate_windows_program(program: &Path) -> Result<(), String> {
    reject_symlink(program)?;
    let marker = read_json(&program.join(".nuif-editor-managed.json"))?;
    if marker["schema_version"] != SCHEMA_VERSION || marker["product"] != PRODUCT {
        return Err(format!(
            "refusing to replace an unmanaged Windows program: {}",
            program.display()
        ));
    }
    Ok(())
}

fn read_state(paths: &InstallPaths) -> Result<Value, String> {
    let state = read_json(&paths.state)?;
    if state["schema_version"] != SCHEMA_VERSION || state["product"] != PRODUCT {
        return Err("installed editor state has an unknown schema or product".to_owned());
    }
    Ok(state)
}

fn state_string(state: &Value, field: &str) -> Result<String, String> {
    state[field]
        .as_str()
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| format!("installed editor state has no {field} version"))
}

fn installed_version_from_receipt(
    platform: Platform,
    paths: &InstallPaths,
    id: &str,
) -> Result<InstalledVersion, String> {
    if id.contains('/') || id.contains('\\') || id.starts_with('.') {
        return Err(format!("invalid installed version identifier {id:?}"));
    }
    let root = paths.versions.join(id);
    validate_version_directory(&paths.versions, &root)?;
    let receipt = read_json(&root.join("receipt.json"))?;
    if receipt["schema_version"] != SCHEMA_VERSION
        || receipt["product"] != PRODUCT
        || receipt["install_id"] != id
        || receipt["platform"] != platform.name()
        || receipt["architecture"] != env::consts::ARCH
        || receipt["user_scoped"] != true
        || receipt["sandboxed"] != paths.sandboxed
    {
        return Err(format!("invalid install receipt for {id}"));
    }
    let version = receipt["version"]
        .as_str()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("install receipt {id} has no version"))?;
    let channel = receipt["channel"]
        .as_str()
        .ok_or_else(|| format!("install receipt {id} has no channel"))?;
    let channel = Channel::parse(channel)?;
    let revision = receipt["source"]["revision"]
        .as_str()
        .ok_or_else(|| format!("install receipt {id} has no source revision"))?;
    validate_revision(revision)?;
    let lock_sha256 = receipt["source"]["cargo_lock_sha256"]
        .as_str()
        .ok_or_else(|| format!("install receipt {id} has no lockfile digest"))?;
    validate_sha256(lock_sha256, "install receipt lockfile digest")?;
    let dirty = receipt["source"]["dirty"]
        .as_bool()
        .ok_or_else(|| format!("install receipt {id} has no source cleanliness state"))?;
    let expected_tag = format!("v{version}");
    if channel == Channel::Alpha
        && (dirty || receipt["source"]["tag"].as_str() != Some(expected_tag.as_str()))
    {
        return Err(format!(
            "alpha install receipt {id} does not identify a clean exact release tag"
        ));
    }
    let binary_relative = receipt["binary_relative_path"]
        .as_str()
        .ok_or_else(|| format!("install receipt {id} has no binary path"))?;
    let expected_binary = match platform {
        Platform::Macos => "NUIF Editor.app/Contents/MacOS/NUIF Editor",
        Platform::Windows => "NUIF Editor.exe",
        Platform::Linux => "bin/nuif-editor",
    };
    if binary_relative != expected_binary {
        return Err(format!(
            "install receipt {id} has an unexpected binary path"
        ));
    }
    let binary_sha256 = receipt["binary_sha256"]
        .as_str()
        .ok_or_else(|| format!("install receipt {id} has no binary digest"))?
        .to_owned();
    validate_sha256(&binary_sha256, "install receipt binary digest")?;
    if install_id(version, revision, &binary_sha256)? != id {
        return Err(format!(
            "install receipt {id} is not bound to its version, revision and binary"
        ));
    }
    Ok(InstalledVersion {
        id: id.to_owned(),
        binary: root.join(binary_relative),
        app_bundle: (platform == Platform::Macos).then(|| root.join("NUIF Editor.app")),
        root,
        binary_sha256,
        signing: receipt["signing"]["status"]
            .as_str()
            .unwrap_or("unknown")
            .to_owned(),
    })
}

fn doctor_paths(platform: Platform, paths: &InstallPaths) -> Result<(), String> {
    validate_managed_root(paths)?;
    let state = read_state(paths)?;
    let active = state_string(&state, "active")?;
    let installed = installed_version_from_receipt(platform, paths, &active)?;
    if !installed.binary.is_file() {
        return Err(format!(
            "installed editor binary is absent: {}",
            installed.binary.display()
        ));
    }
    let observed_sha256 = sha256_file(&installed.binary)?;
    if observed_sha256 != installed.binary_sha256 {
        return Err(format!(
            "installed editor binary digest mismatch for {}",
            installed.binary.display()
        ));
    }
    let receipt = read_json(&installed.root.join("receipt.json"))?;
    let version = receipt["version"]
        .as_str()
        .ok_or_else(|| "install receipt has no editor version".to_owned())?;
    let output = Command::new(&installed.binary)
        .arg("--version")
        .output()
        .map_err(|error| error.to_string())?;
    check_status(output.status, &installed.binary.display().to_string())?;
    let observed_version = String::from_utf8_lossy(&output.stdout);
    if observed_version.trim() != format!("NUIF Editor {version}") {
        return Err("installed editor returned an unexpected version".to_owned());
    }
    verify_integration(platform, paths, &installed)?;
    if platform == Platform::Macos {
        let app = installed
            .app_bundle
            .as_ref()
            .ok_or_else(|| "macOS install has no application bundle".to_owned())?;
        run(
            "codesign",
            &[
                OsStr::new("--verify"),
                OsStr::new("--deep"),
                OsStr::new("--strict"),
                app.as_os_str(),
            ],
        )?;
    }
    let report = json!({
        "schema_version": SCHEMA_VERSION,
        "status": "passed",
        "product": PRODUCT,
        "active": installed.id,
        "version": version,
        "platform": platform.name(),
        "architecture": env::consts::ARCH,
        "binary_sha256": observed_sha256,
        "signing": installed.signing,
        "state_root": paths.state_root,
        "sandboxed": paths.sandboxed,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn verify_integration(
    platform: Platform,
    paths: &InstallPaths,
    installed: &InstalledVersion,
) -> Result<(), String> {
    match (&paths.integration, platform) {
        (IntegrationPaths::Macos { app }, Platform::Macos) => verify_symlink(
            app,
            installed.app_bundle.as_deref().unwrap_or(Path::new("")),
        ),
        (IntegrationPaths::Windows { program, shortcut }, Platform::Windows) => {
            validate_windows_program(program)?;
            let active = program.join("NUIF Editor Dev.exe");
            if sha256_file(&active)? != installed.binary_sha256 {
                return Err("active Windows program digest does not match its receipt".to_owned());
            }
            if !shortcut.is_file() {
                return Err(format!(
                    "Windows shortcut is absent: {}",
                    shortcut.display()
                ));
            }
            Ok(())
        }
        (
            IntegrationPaths::Linux {
                binary,
                desktop,
                icon,
            },
            Platform::Linux,
        ) => {
            verify_symlink(binary, &installed.binary)?;
            verify_symlink(
                icon,
                &installed
                    .root
                    .join("share/icons/hicolor/scalable/apps/nuif-editor.svg"),
            )?;
            let desktop_contents =
                fs::read_to_string(desktop).map_err(|error| error.to_string())?;
            if !desktop_contents.contains("X-NUIF-Managed=true")
                || !desktop_contents.contains(&format!("Exec={} %f", desktop_quote(binary)))
            {
                return Err("Linux desktop entry does not select the active editor".to_owned());
            }
            Ok(())
        }
        _ => Err("installation paths do not match the host platform".to_owned()),
    }
}

#[cfg(unix)]
fn verify_symlink(link: &Path, target: &Path) -> Result<(), String> {
    let observed = fs::read_link(link).map_err(|error| {
        format!(
            "managed integration link {} is absent or invalid: {error}",
            link.display()
        )
    })?;
    if observed != target {
        return Err(format!(
            "managed integration link {} points to {} instead of {}",
            link.display(),
            observed.display(),
            target.display()
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn verify_symlink(_link: &Path, _target: &Path) -> Result<(), String> {
    Err("symbolic-link integration is unsupported on this host".to_owned())
}

fn remove_integration(platform: Platform, paths: &InstallPaths) -> Result<(), String> {
    match (&paths.integration, platform) {
        (IntegrationPaths::Macos { app }, Platform::Macos) => {
            remove_managed_symlink(app, &paths.versions)
        }
        (IntegrationPaths::Windows { program, shortcut }, Platform::Windows) => {
            if program.exists() {
                validate_windows_program(program)?;
                fs::remove_dir_all(program).map_err(|error| error.to_string())?;
            }
            if shortcut.exists() {
                fs::remove_file(shortcut).map_err(|error| error.to_string())?;
            }
            Ok(())
        }
        (
            IntegrationPaths::Linux {
                binary,
                desktop,
                icon,
            },
            Platform::Linux,
        ) => {
            remove_managed_symlink(binary, &paths.versions)?;
            remove_managed_symlink(icon, &paths.versions)?;
            if desktop.exists() {
                let contents = fs::read_to_string(desktop).map_err(|error| error.to_string())?;
                if !contents.contains("X-NUIF-Managed=true") {
                    return Err(format!(
                        "refusing to remove an unmanaged desktop entry: {}",
                        desktop.display()
                    ));
                }
                fs::remove_file(desktop).map_err(|error| error.to_string())?;
            }
            Ok(())
        }
        _ => Err("installation paths do not match the host platform".to_owned()),
    }
}

fn remove_managed_symlink(link: &Path, managed_root: &Path) -> Result<(), String> {
    let Ok(metadata) = fs::symlink_metadata(link) else {
        return Ok(());
    };
    if !metadata.file_type().is_symlink() {
        return Err(format!(
            "refusing to remove an unmanaged integration path: {}",
            link.display()
        ));
    }
    let target = fs::read_link(link).map_err(|error| error.to_string())?;
    if !target.starts_with(managed_root) {
        return Err(format!(
            "refusing to remove integration link {} outside {}",
            link.display(),
            managed_root.display()
        ));
    }
    fs::remove_file(link).map_err(|error| error.to_string())
}

fn remove_managed_state_root(paths: &InstallPaths) -> Result<(), String> {
    validate_managed_root(paths)?;
    let parent = paths
        .state_root
        .parent()
        .ok_or_else(|| "managed state root has no parent".to_owned())?;
    if paths.state_root == parent || paths.state_root.as_os_str().is_empty() {
        return Err("refusing to remove an unsafe state root".to_owned());
    }
    fs::remove_dir_all(&paths.state_root).map_err(|error| error.to_string())
}

fn prune_versions(
    paths: &InstallPaths,
    active: &str,
    previous: Option<&str>,
) -> Result<(), String> {
    for entry in fs::read_dir(&paths.versions).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            return Err("installed version name is not valid UTF-8".to_owned());
        };
        if name == active || previous == Some(name) {
            continue;
        }
        let path = entry.path();
        validate_version_directory(&paths.versions, &path)?;
        fs::remove_dir_all(path).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn read_json(path: &Path) -> Result<Value, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    write_bytes_atomic(path, &bytes)
}

fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let temporary = path.with_extension(format!("nuif-new-{}", std::process::id()));
    fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    match fs::rename(&temporary, path) {
        Ok(()) => Ok(()),
        Err(error)
            if path.exists()
                && matches!(
                    error.kind(),
                    std::io::ErrorKind::AlreadyExists | std::io::ErrorKind::PermissionDenied
                ) =>
        {
            fs::remove_file(path).map_err(|remove_error| remove_error.to_string())?;
            fs::rename(&temporary, path).map_err(|rename_error| rename_error.to_string())
        }
        Err(error) => Err(error.to_string()),
    }
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn required_command_text(program: &str, arguments: &[&str]) -> Result<String, String> {
    command_text(program, arguments)
        .ok_or_else(|| format!("{program} {} failed", arguments.join(" ")))
}

fn run(program: &str, arguments: &[&OsStr]) -> Result<(), String> {
    let status = Command::new(program)
        .args(arguments)
        .status()
        .map_err(|error| error.to_string())?;
    check_status(status, program)
}

fn check_status(status: ExitStatus, program: &str) -> Result<(), String> {
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} failed with {status}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_identifier_binds_version_revision_and_binary() {
        let revision = "0123456789abcdef0123456789abcdef01234567";
        let digest = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
        assert_eq!(
            install_id("0.1.0-alpha.2", revision, digest).unwrap(),
            "0.1.0-alpha.2-0123456789ab-abcdef012345"
        );
    }

    #[test]
    fn install_options_reject_system_scope_and_dirty_alpha() {
        assert!(parse_install_options(&["--system".to_owned()]).is_err());
        assert!(
            parse_install_options(&[
                "--channel".to_owned(),
                "alpha".to_owned(),
                "--allow-dirty".to_owned(),
            ])
            .is_err()
        );
        let filesystem_root = if cfg!(windows) { "C:\\" } else { "/" };
        assert!(parse_install_options(&["--root".to_owned(), filesystem_root.to_owned()]).is_err());
    }

    #[test]
    fn sandbox_layout_keeps_every_path_under_the_requested_root() {
        let root = env::temp_dir().join("nuif-editor-layout-test");
        for platform in [Platform::Macos, Platform::Windows, Platform::Linux] {
            let paths = install_paths(platform, Some(&root)).unwrap();
            assert!(paths.state_root.starts_with(&root));
            match paths.integration {
                IntegrationPaths::Macos { app } => assert!(app.starts_with(&root)),
                IntegrationPaths::Windows { program, shortcut } => {
                    assert!(program.starts_with(&root));
                    assert!(shortcut.starts_with(&root));
                }
                IntegrationPaths::Linux {
                    binary,
                    desktop,
                    icon,
                } => {
                    assert!(binary.starts_with(&root));
                    assert!(desktop.starts_with(&root));
                    assert!(icon.starts_with(&root));
                }
            }
        }
    }

    #[test]
    fn desktop_paths_escape_reserved_characters() {
        assert_eq!(
            desktop_quote(Path::new("/tmp/NUIF $dev/editor")),
            "\"/tmp/NUIF \\$dev/editor\""
        );
    }

    #[cfg(unix)]
    #[test]
    fn unmanaged_integration_symlink_is_never_claimed() {
        use std::os::unix::fs::symlink;

        let root = env::temp_dir().join(format!(
            "nuif-editor-unmanaged-link-test-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        let paths = install_paths(Platform::Macos, Some(&root)).unwrap();
        let IntegrationPaths::Macos { app } = &paths.integration else {
            unreachable!();
        };
        fs::create_dir_all(app.parent().unwrap()).unwrap();
        let unrelated = root.join("unrelated.app");
        fs::create_dir(&unrelated).unwrap();
        symlink(&unrelated, app).unwrap();
        assert!(preflight_install(Platform::Macos, &paths).is_err());
        fs::remove_dir_all(&root).unwrap();
    }
}
