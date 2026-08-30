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
const RELEASE_REPOSITORY: &str = "refpath/nuif";
const RELEASE_WORKFLOW: &str = "refpath/nuif/.github/workflows/release.yml";
const SOURCE_REPOSITORY: &str = "https://github.com/refpath/nuif.git";

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
struct UpdateOptions {
    root: Option<PathBuf>,
    check_only: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AlphaRelease {
    version: String,
    tag: String,
    revision: String,
    release_url: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReleaseCandidate {
    version: String,
    tag: String,
    release_url: String,
}

#[derive(Debug)]
struct ManagedTempDir {
    path: PathBuf,
    marker: PathBuf,
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
    tree_sha256: String,
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

pub(crate) fn update(arguments: &[String]) -> Result<(), String> {
    let options = parse_update_options(arguments)?;
    let platform = Platform::host()?;
    let paths = install_paths(platform, options.root.as_deref())?;
    require_update_tools()?;
    let release = resolve_alpha_release()?;
    let current = current_install_revision(platform, &paths)?;
    let report = json!({
        "schema_version": SCHEMA_VERSION,
        "status": "resolved",
        "channel": "alpha",
        "available": {
            "version": release.version,
            "tag": release.tag,
            "revision": release.revision,
            "release_url": release.release_url,
        },
        "current_revision": current,
        "update_required": current.as_deref() != Some(release.revision.as_str()),
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
    );
    if options.check_only || current.as_deref() == Some(release.revision.as_str()) {
        return Ok(());
    }
    install_alpha_release(&release, options.root.as_deref())?;
    let refreshed_paths = install_paths(platform, options.root.as_deref())?;
    doctor_paths(platform, &refreshed_paths)
}

pub(crate) fn trial(arguments: &[String]) -> Result<(), String> {
    let mut channel = Channel::Source;
    let mut allow_dirty = false;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--allow-dirty" => allow_dirty = true,
            "--channel" => {
                channel = Channel::parse(next_value(arguments, &mut index, "--channel")?)?;
            }
            value => return Err(format!("unknown editor-install-trial option {value:?}")),
        }
        index += 1;
    }
    if channel == Channel::Alpha && allow_dirty {
        return Err("the alpha install trial cannot use a dirty source tree".to_owned());
    }
    let temporary = ManagedTempDir::new("install-trial")?;
    let root = temporary.path.join("user");
    let mut install_arguments = vec![
        "--user".to_owned(),
        "--channel".to_owned(),
        channel.name().to_owned(),
        "--root".to_owned(),
        root.display().to_string(),
    ];
    if allow_dirty {
        install_arguments.push("--allow-dirty".to_owned());
    }
    install(&install_arguments)?;
    doctor(&["--root".to_owned(), root.display().to_string()])?;

    let platform = Platform::host()?;
    let paths = install_paths(platform, Some(&root))?;
    let state = read_state(&paths)?;
    let active = state_string(&state, "active")?;
    let installed = installed_version_from_receipt(platform, &paths, &active)?;
    let receipt = read_json(&installed.root.join("receipt.json"))?;
    uninstall(&["--root".to_owned(), root.display().to_string()])?;
    if paths.state_root.exists() {
        return Err("install trial left managed state after uninstall".to_owned());
    }
    let report = json!({
        "schema_version": SCHEMA_VERSION,
        "status": "passed",
        "channel": channel.name(),
        "platform": platform.name(),
        "architecture": env::consts::ARCH,
        "active_install": active,
        "version": receipt["version"],
        "source": receipt["source"],
        "binary_sha256": receipt["binary_sha256"],
        "signing": receipt["signing"],
        "checks": {
            "install": "passed",
            "doctor": "passed",
            "uninstall": "passed",
            "managed_state_removed": true,
        },
    });
    write_json_atomic(Path::new("target/editor-install-trial.json"), &report)?;
    println!("user-scoped editor install trial passed");
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

fn parse_update_options(arguments: &[String]) -> Result<UpdateOptions, String> {
    let mut root = None;
    let mut check_only = false;
    let mut channel = None;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--user" => {}
            "--check" => check_only = true,
            "--root" => root = Some(next_path(arguments, &mut index, "--root")?),
            "--channel" => {
                channel = Some(next_value(arguments, &mut index, "--channel")?.to_owned());
            }
            value => return Err(format!("unknown editor-update option {value:?}")),
        }
        index += 1;
    }
    if channel.as_deref().is_some_and(|value| value != "alpha") {
        return Err("editor-update currently supports only the alpha channel".to_owned());
    }
    validate_root(root.as_deref())?;
    Ok(UpdateOptions { root, check_only })
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
    let tree_sha256 = source_tree_sha256()?;
    let toolchain = required_command_text("rustc", &["--version"])?;
    Ok(SourceIdentity {
        version,
        revision,
        dirty,
        tag,
        repository: command_text("git", &["config", "--get", "remote.origin.url"]),
        lock_sha256,
        tree_sha256,
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
    let id = install_id(
        &source.version,
        &source.revision,
        &source.tree_sha256,
        &binary_sha256,
    )?;
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
            "working_tree_sha256": source.tree_sha256,
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

fn install_id(
    version: &str,
    revision: &str,
    tree_sha256: &str,
    binary_sha256: &str,
) -> Result<String, String> {
    if !version
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
    {
        return Err(format!("editor version is not path-safe: {version:?}"));
    }
    validate_revision(revision)?;
    validate_sha256(tree_sha256, "working-tree digest")?;
    validate_sha256(binary_sha256, "installed binary digest")?;
    Ok(format!(
        "{version}-{}-{}-{}",
        &revision[..12],
        &tree_sha256[..12],
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
    let tree_sha256 = receipt["source"]["working_tree_sha256"]
        .as_str()
        .ok_or_else(|| format!("install receipt {id} has no working-tree digest"))?;
    validate_sha256(tree_sha256, "install receipt working-tree digest")?;
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
    if install_id(version, revision, tree_sha256, &binary_sha256)? != id {
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
    let tools = json!({
        "git": tool_identity("git", &["--version"]),
        "gh": tool_identity("gh", &["--version"]),
        "cargo": tool_identity("cargo", &["--version"]),
        "rustc": tool_identity("rustc", &["--version"]),
    });
    let update_ready = ["git", "gh", "cargo", "rustc"]
        .iter()
        .all(|tool| tools[*tool].is_string());
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
        "source_update": {
            "ready": update_ready,
            "tools": tools,
        },
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

fn source_tree_sha256() -> Result<String, String> {
    let output = Command::new("git")
        .args([
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ])
        .output()
        .map_err(|error| error.to_string())?;
    check_status(output.status, "git ls-files")?;
    let mut files = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| {
            String::from_utf8(path.to_vec())
                .map_err(|_| "source path is not valid UTF-8".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    files.sort();
    let mut digest = Sha256::new();
    for relative in files {
        let path = Path::new(&relative);
        if path.is_absolute()
            || path
                .components()
                .any(|component| component == std::path::Component::ParentDir)
        {
            return Err(format!("Git reported an unsafe source path {relative:?}"));
        }
        let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
        digest.update((relative.len() as u64).to_le_bytes());
        digest.update(relative.as_bytes());
        if metadata.file_type().is_symlink() {
            digest.update(b"symlink");
            let target = fs::read_link(path).map_err(|error| error.to_string())?;
            let target = target
                .to_str()
                .ok_or_else(|| format!("symlink target is not valid UTF-8: {}", path.display()))?;
            digest.update((target.len() as u64).to_le_bytes());
            digest.update(target.as_bytes());
        } else if metadata.is_file() {
            digest.update(b"file");
            let bytes = fs::read(path).map_err(|error| error.to_string())?;
            digest.update((bytes.len() as u64).to_le_bytes());
            digest.update(bytes);
        } else {
            return Err(format!(
                "Git source entry is neither a file nor a symlink: {}",
                path.display()
            ));
        }
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn require_update_tools() -> Result<(), String> {
    for (program, arguments) in [
        ("git", &["--version"][..]),
        ("gh", &["--version"][..]),
        ("cargo", &["--version"][..]),
        ("rustc", &["--version"][..]),
    ] {
        let output = Command::new(program)
            .args(arguments)
            .output()
            .map_err(|error| format!("{program} is required for editor updates: {error}"))?;
        check_status(output.status, program)?;
    }
    Ok(())
}

fn current_install_revision(
    platform: Platform,
    paths: &InstallPaths,
) -> Result<Option<String>, String> {
    if !paths.state_root.exists() {
        return Ok(None);
    }
    validate_managed_root(paths)?;
    if !paths.state.exists() {
        return Ok(None);
    }
    let state = read_state(paths)?;
    let active = state_string(&state, "active")?;
    let installed = installed_version_from_receipt(platform, paths, &active)?;
    let receipt = read_json(&installed.root.join("receipt.json"))?;
    let revision = receipt["source"]["revision"]
        .as_str()
        .ok_or_else(|| "active install receipt has no source revision".to_owned())?;
    validate_revision(revision)?;
    Ok(Some(revision.to_owned()))
}

fn resolve_alpha_release() -> Result<AlphaRelease, String> {
    let mut command = Command::new("gh");
    command.args([
        "api",
        "-H",
        "Accept: application/vnd.github+json",
        "-H",
        "X-GitHub-Api-Version: 2022-11-28",
        "repos/refpath/nuif/releases?per_page=100",
    ]);
    let releases: Value = command_json(&mut command, "GitHub release query")?;
    let candidate = select_alpha_release(&releases)?;
    let temporary = ManagedTempDir::new("manifest")?;
    let manifest = temporary.path.join("release-manifest.json");
    let mut download = Command::new("gh");
    download.args(["release", "download"]);
    download.arg(&candidate.tag);
    download.args([
        "--repo",
        RELEASE_REPOSITORY,
        "--pattern",
        "release-manifest.json",
        "--dir",
    ]);
    download.arg(&temporary.path);
    run_command(&mut download, "GitHub release manifest download")?;
    let manifest_value = read_json(&manifest)?;
    let revision = validate_release_manifest(&candidate, &manifest_value)?;
    verify_release_manifest_attestation(&manifest, &candidate.tag, &revision)?;
    Ok(AlphaRelease {
        version: candidate.version,
        tag: candidate.tag,
        revision,
        release_url: candidate.release_url,
    })
}

fn select_alpha_release(releases: &Value) -> Result<ReleaseCandidate, String> {
    let releases = releases
        .as_array()
        .ok_or_else(|| "GitHub release query did not return an array".to_owned())?;
    releases
        .iter()
        .filter(|release| release["draft"] == false && release["prerelease"] == true)
        .filter_map(|release| {
            let tag = release["tag_name"].as_str()?;
            let version = tag.strip_prefix('v')?;
            let order = parse_alpha_version(version)?;
            let has_manifest = release["assets"].as_array().is_some_and(|assets| {
                assets.iter().any(|asset| {
                    asset["name"] == "release-manifest.json" && asset["state"] == "uploaded"
                })
            });
            if !has_manifest {
                return None;
            }
            Some((
                order,
                ReleaseCandidate {
                    version: version.to_owned(),
                    tag: tag.to_owned(),
                    release_url: release["html_url"].as_str().unwrap_or_default().to_owned(),
                },
            ))
        })
        .max_by_key(|(order, _)| *order)
        .map(|(_, release)| release)
        .ok_or_else(|| "no published alpha release with a release manifest was found".to_owned())
}

fn parse_alpha_version(version: &str) -> Option<(u64, u64, u64, u64)> {
    let (base, alpha) = version.split_once("-alpha.")?;
    if alpha.is_empty() || alpha.starts_with('0') && alpha != "0" {
        return None;
    }
    let mut base = base.split('.');
    let major = semver_number(base.next()?)?;
    let minor = semver_number(base.next()?)?;
    let patch = semver_number(base.next()?)?;
    if base.next().is_some() {
        return None;
    }
    Some((major, minor, patch, alpha.parse().ok()?))
}

fn semver_number(value: &str) -> Option<u64> {
    if value.is_empty() || value.starts_with('0') && value != "0" {
        return None;
    }
    value.parse().ok()
}

fn validate_release_manifest(
    candidate: &ReleaseCandidate,
    manifest: &Value,
) -> Result<String, String> {
    if manifest["schema_version"] != SCHEMA_VERSION
        || manifest["tag"] != candidate.tag
        || manifest["version"] != candidate.version
        || manifest["packages"].as_array().map(Vec::len) != Some(5)
        || manifest["sbom"] != format!("nuif-editor-{}.cdx.json", candidate.version)
    {
        return Err(format!(
            "release manifest for {} failed its channel contract",
            candidate.tag
        ));
    }
    let revision = manifest["source_revision"]
        .as_str()
        .ok_or_else(|| "release manifest has no source revision".to_owned())?;
    validate_revision(revision)?;
    for package in manifest["packages"].as_array().into_iter().flatten() {
        if package["status"] != "passed"
            || package["source_dirty"] != false
            || package["source_revision"] != revision
            || package["version"] != candidate.version
            || package["smoke_test"]["status"] != "passed"
            || package["version_test"]["status"] != "passed"
        {
            return Err(format!(
                "release manifest contains an invalid package for {}",
                candidate.tag
            ));
        }
    }
    Ok(revision.to_owned())
}

fn verify_release_manifest_attestation(
    manifest: &Path,
    tag: &str,
    revision: &str,
) -> Result<(), String> {
    let source_ref = format!("refs/tags/{tag}");
    let mut command = Command::new("gh");
    command.args(["attestation", "verify"]);
    command.arg(manifest);
    command.args([
        "--repo",
        RELEASE_REPOSITORY,
        "--signer-workflow",
        RELEASE_WORKFLOW,
        "--source-ref",
        &source_ref,
        "--source-digest",
        revision,
        "--deny-self-hosted-runners",
        "--format",
        "json",
    ]);
    command.stdout(std::process::Stdio::null());
    run_command(&mut command, "GitHub release manifest attestation")
}

fn install_alpha_release(release: &AlphaRelease, root: Option<&Path>) -> Result<(), String> {
    let temporary = ManagedTempDir::new("source")?;
    let source = temporary.path.join("source");
    checkout_release_source(release, SOURCE_REPOSITORY, &source)?;

    let mut install = Command::new("cargo");
    install.current_dir(&source).args([
        "xtask",
        "editor-install",
        "--user",
        "--channel",
        "alpha",
        "--expected-tag",
        &release.tag,
        "--expected-revision",
        &release.revision,
    ]);
    if let Some(root) = root {
        install.arg("--root").arg(root);
    }
    run_command(&mut install, "attested alpha source installation")
}

fn checkout_release_source(
    release: &AlphaRelease,
    repository: &str,
    source: &Path,
) -> Result<(), String> {
    let mut init = Command::new("git");
    init.args(["init", "--quiet"]);
    init.arg(source);
    init.env("GIT_TERMINAL_PROMPT", "0");
    run_command(&mut init, "Git source initialization")?;

    let mut remote = git_source_command(source);
    remote.args(["remote", "add", "origin", repository]);
    run_command(&mut remote, "Git source remote configuration")?;
    let tag_ref = format!("refs/tags/{0}:refs/tags/{0}", release.tag);
    let mut fetch = git_source_command(source);
    fetch.args(["fetch", "--quiet", "--depth", "1", "origin", &tag_ref]);
    run_command(&mut fetch, "Git tagged source fetch")?;
    let mut checkout = git_source_command(source);
    checkout.args(["checkout", "--quiet", "--detach", &release.tag]);
    run_command(&mut checkout, "Git tagged source checkout")?;

    let mut head = git_source_command(source);
    head.args(["rev-parse", "HEAD"]);
    let observed_revision = command_output(&mut head, "Git source revision")?;
    if observed_revision != release.revision {
        return Err(format!(
            "fetched revision {observed_revision} does not match attested revision {}",
            release.revision
        ));
    }
    let mut origin = git_source_command(source);
    origin.args(["remote", "get-url", "origin"]);
    if command_output(&mut origin, "Git source remote")? != repository {
        return Err("fetched source remote changed unexpectedly".to_owned());
    }
    let mut status = git_source_command(source);
    status.args(["status", "--porcelain"]);
    if !command_output(&mut status, "Git source cleanliness")?.is_empty() {
        return Err("fetched alpha source is not clean".to_owned());
    }
    Ok(())
}

fn git_source_command(source: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(source)
        .args(["-c", "core.hooksPath=/dev/null"])
        .env("GIT_TERMINAL_PROMPT", "0");
    command
}

fn command_json(command: &mut Command, context: &str) -> Result<Value, String> {
    let output = command
        .output()
        .map_err(|error| format!("{context}: {error}"))?;
    check_status(output.status, context)?;
    serde_json::from_slice(&output.stdout).map_err(|error| format!("{context}: {error}"))
}

fn command_output(command: &mut Command, context: &str) -> Result<String, String> {
    let output = command
        .output()
        .map_err(|error| format!("{context}: {error}"))?;
    check_status(output.status, context)?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn run_command(command: &mut Command, context: &str) -> Result<(), String> {
    let status = command
        .status()
        .map_err(|error| format!("{context}: {error}"))?;
    check_status(status, context)
}

impl ManagedTempDir {
    fn new(purpose: &str) -> Result<Self, String> {
        if purpose.is_empty()
            || !purpose
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte == b'-')
        {
            return Err("temporary directory purpose is not path-safe".to_owned());
        }
        let temporary_root = env::temp_dir();
        for suffix in 0..100_u32 {
            let path = temporary_root.join(format!(
                "nuif-editor-update-{purpose}-{}-{suffix}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => {
                    let marker = path.join(".nuif-editor-update-temp");
                    fs::write(&marker, PRODUCT).map_err(|error| error.to_string())?;
                    return Ok(Self { path, marker });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error.to_string()),
            }
        }
        Err("could not reserve an update temporary directory".to_owned())
    }
}

impl Drop for ManagedTempDir {
    fn drop(&mut self) {
        let safe_parent = self.path.parent() == Some(env::temp_dir().as_path());
        let safe_name = self
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("nuif-editor-update-"));
        let marked = fs::read_to_string(&self.marker).is_ok_and(|value| value == PRODUCT);
        if safe_parent && safe_name && marked {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

fn required_command_text(program: &str, arguments: &[&str]) -> Result<String, String> {
    command_text(program, arguments)
        .ok_or_else(|| format!("{program} {} failed", arguments.join(" ")))
}

fn tool_identity(program: &str, arguments: &[&str]) -> Option<String> {
    let output = Command::new(program).args(arguments).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    (!stdout.is_empty())
        .then_some(stdout)
        .or_else(|| (!stderr.is_empty()).then_some(stderr))
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
        let tree = "123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0";
        let digest = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
        assert_eq!(
            install_id("0.1.0-alpha.2", revision, tree, digest).unwrap(),
            "0.1.0-alpha.2-0123456789ab-123456789abc-abcdef012345"
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

    #[test]
    fn alpha_release_selection_uses_semver_order_and_requires_manifest() {
        let releases = json!([
            {
                "draft": false,
                "prerelease": true,
                "tag_name": "v0.1.0-alpha.9",
                "html_url": "https://example.invalid/9",
                "assets": [{"name": "release-manifest.json", "state": "uploaded"}],
            },
            {
                "draft": false,
                "prerelease": true,
                "tag_name": "v0.1.0-alpha.10",
                "html_url": "https://example.invalid/10",
                "assets": [{"name": "release-manifest.json", "state": "uploaded"}],
            },
            {
                "draft": false,
                "prerelease": true,
                "tag_name": "v0.2.0-alpha.1",
                "html_url": "https://example.invalid/incomplete",
                "assets": [],
            },
            {
                "draft": true,
                "prerelease": true,
                "tag_name": "v9.0.0-alpha.1",
                "html_url": "https://example.invalid/draft",
                "assets": [{"name": "release-manifest.json", "state": "uploaded"}],
            }
        ]);
        let selected = select_alpha_release(&releases).unwrap();
        assert_eq!(selected.version, "0.1.0-alpha.10");
        assert_eq!(selected.tag, "v0.1.0-alpha.10");
    }

    #[test]
    fn release_manifest_binds_all_packages_to_one_clean_revision() {
        let revision = "0123456789abcdef0123456789abcdef01234567";
        let candidate = ReleaseCandidate {
            version: "0.1.0-alpha.2".to_owned(),
            tag: "v0.1.0-alpha.2".to_owned(),
            release_url: "https://example.invalid/release".to_owned(),
        };
        let package = json!({
            "status": "passed",
            "source_dirty": false,
            "source_revision": revision,
            "version": "0.1.0-alpha.2",
            "smoke_test": {"status": "passed"},
            "version_test": {"status": "passed"},
        });
        let manifest = json!({
            "schema_version": 1,
            "tag": "v0.1.0-alpha.2",
            "version": "0.1.0-alpha.2",
            "source_revision": revision,
            "sbom": "nuif-editor-0.1.0-alpha.2.cdx.json",
            "packages": [package.clone(), package.clone(), package.clone(), package.clone(), package],
        });
        assert_eq!(
            validate_release_manifest(&candidate, &manifest).unwrap(),
            revision
        );
        let mut dirty = manifest;
        dirty["packages"][2]["source_dirty"] = json!(true);
        assert!(validate_release_manifest(&candidate, &dirty).is_err());
    }

    #[test]
    fn alpha_version_parser_rejects_mutable_or_ambiguous_versions() {
        assert_eq!(parse_alpha_version("0.1.0-alpha.12"), Some((0, 1, 0, 12)));
        assert!(parse_alpha_version("0.1.0-alpha.01").is_none());
        assert!(parse_alpha_version("0.1.0-beta.1").is_none());
        assert!(parse_alpha_version("main").is_none());
    }

    #[test]
    fn attested_source_checkout_uses_the_exact_tag_revision() {
        let origin = ManagedTempDir::new("fixture").unwrap();
        let repository = origin.path.join("repository");
        let mut init = Command::new("git");
        init.args(["init", "--quiet"]).arg(&repository);
        run_command(&mut init, "fixture Git initialization").unwrap();
        for (key, value) in [
            ("user.name", "NUIF Test"),
            ("user.email", "test@nuif.invalid"),
            ("commit.gpgsign", "false"),
            ("tag.gpgsign", "false"),
        ] {
            let mut config = Command::new("git");
            config
                .arg("-C")
                .arg(&repository)
                .args(["config", key, value]);
            run_command(&mut config, "fixture Git configuration").unwrap();
        }
        fs::write(repository.join("source.txt"), "pinned source\n").unwrap();
        let mut add = Command::new("git");
        add.arg("-C").arg(&repository).args(["add", "source.txt"]);
        run_command(&mut add, "fixture Git add").unwrap();
        let mut commit = Command::new("git");
        commit
            .arg("-C")
            .arg(&repository)
            .args(["commit", "--quiet", "-m", "fixture source"]);
        run_command(&mut commit, "fixture Git commit").unwrap();
        let mut tag = Command::new("git");
        tag.arg("-C").arg(&repository).args([
            "tag",
            "--annotate",
            "--message",
            "fixture tag",
            "v0.1.0-alpha.2",
        ]);
        run_command(&mut tag, "fixture Git tag").unwrap();
        let mut revision_command = Command::new("git");
        revision_command
            .arg("-C")
            .arg(&repository)
            .args(["rev-parse", "HEAD"]);
        let revision = command_output(&mut revision_command, "fixture revision").unwrap();
        let release = AlphaRelease {
            version: "0.1.0-alpha.2".to_owned(),
            tag: "v0.1.0-alpha.2".to_owned(),
            revision,
            release_url: "https://example.invalid/release".to_owned(),
        };
        let checkout = ManagedTempDir::new("checkout").unwrap();
        let destination = checkout.path.join("source");
        checkout_release_source(&release, repository.to_str().unwrap(), &destination).unwrap();
        assert_eq!(
            fs::read_to_string(destination.join("source.txt")).unwrap(),
            "pinned source\n"
        );
    }
}
