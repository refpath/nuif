use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use serde::Deserialize;
use sha2::{Digest, Sha256};

const CONFIG_PATH: &str = "docs/catalog.json";
const BOOK_ROOT: &str = "target/docs-book";
const BOOK_SOURCE: &str = "target/docs-book/src";
const SITE_OUTPUT: &str = "target/docs-site";
const CATALOG_OUTPUT: &str = "target/documentation-catalog.json";
const REPORT_OUTPUT: &str = "target/documentation-report.json";
const MDBOOK_VERSION: &str = "0.5.4";

const DOCUMENT_KINDS: &[&str] = &[
    "adr",
    "article",
    "dataset",
    "implementation",
    "paper",
    "repository",
    "rfc",
    "specification",
    "standard",
    "synthesis",
    "whitepaper",
];

const DOCUMENT_STATUSES: &[&str] = &[
    "accepted",
    "draft",
    "executable",
    "exploratory",
    "informational",
    "pre-draft",
    "proposed",
    "provisional",
    "rejected",
    "reviewed",
    "seed",
    "superseded",
    "verified",
];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogConfig {
    #[serde(rename = "$schema")]
    schema: Option<String>,
    schema_version: u64,
    site: SiteConfig,
    limits: LimitConfig,
    frontmatter_required: Vec<PathBuf>,
    exclude: Vec<PathBuf>,
    manuscript: ManuscriptConfig,
    sections: Vec<SectionConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SiteConfig {
    title: String,
    description: String,
    repository_url: String,
    canonical_url: String,
    maturity: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LimitConfig {
    document_bytes: u64,
    frontmatter_bytes: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SectionConfig {
    id: String,
    title: String,
    paths: Vec<PathBuf>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManuscriptConfig {
    title: String,
    subtitle: String,
    authors: Vec<String>,
    version: String,
    date: String,
    status: String,
    #[serde(rename = "abstract")]
    abstract_text: String,
    sources: Vec<PathBuf>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct Frontmatter {
    id: Option<String>,
    kind: Option<String>,
    status: Option<String>,
    title: Option<String>,
}

#[derive(Debug)]
struct Document {
    source_path: PathBuf,
    section_id: String,
    section_title: String,
    id: String,
    kind: String,
    status: String,
    title: String,
    frontmatter: Option<Frontmatter>,
    body: String,
    source_bytes: Vec<u8>,
}

#[derive(Debug)]
struct Compilation {
    config: CatalogConfig,
    documents: Vec<Document>,
    source_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LinkDestination {
    start: usize,
    end: usize,
    value: String,
    image: bool,
}

pub(crate) fn check() -> Result<(), String> {
    match compile(false) {
        Ok(compilation) => {
            write_outputs(&compilation, "passed", &[])?;
            println!(
                "{} documents, {} sections, 0 errors",
                compilation.documents.len(),
                compilation.config.sections.len()
            );
            Ok(())
        }
        Err(error) => {
            write_failure_report(&error)?;
            Err(error)
        }
    }
}

pub(crate) fn build() -> Result<(), String> {
    let compilation = compile(true).inspect_err(|error| {
        let _ = write_failure_report(error);
    })?;
    write_outputs(&compilation, "passed", &[])?;
    require_mdbook()?;
    remove_exact_directory(Path::new(SITE_OUTPUT))?;
    run_mdbook(&["build", BOOK_ROOT])?;
    verify_site(&compilation)?;
    println!(
        "built {} documents into {SITE_OUTPUT}",
        compilation.documents.len()
    );
    Ok(())
}

pub(crate) fn paper() -> Result<(), String> {
    build()?;
    let source = Path::new(SITE_OUTPUT).join("generated/research-manuscript.html");
    let html = fs::read_to_string(&source)
        .map_err(|error| format!("cannot read {}: {error}", source.display()))?;
    let printable_path = Path::new(SITE_OUTPUT).join("generated/research-manuscript-print.html");
    fs::write(&printable_path, html)
        .map_err(|error| format!("cannot write {}: {error}", printable_path.display()))?;
    let output = Path::new("target/research-manuscript.pdf");
    if output.exists() {
        fs::remove_file(output)
            .map_err(|error| format!("cannot replace {}: {error}", output.display()))?;
    }
    let source_url = format!(
        "file://{}",
        fs::canonicalize(&printable_path)
            .map_err(|error| format!("cannot resolve {}: {error}", printable_path.display()))?
            .to_string_lossy()
    );
    let output_argument = format!(
        "--print-to-pdf={}",
        absolute_path(output)?.to_string_lossy()
    );
    let status = Command::new(chrome_for_testing()?)
        .args([
            "--headless=new",
            "--disable-gpu",
            "--disable-dev-shm-usage",
            "--hide-scrollbars",
            "--no-pdf-header-footer",
            output_argument.as_str(),
            source_url.as_str(),
        ])
        .status()
        .map_err(|error| format!("could not start Chrome for Testing: {error}"))?;
    if !status.success() {
        return Err(format!(
            "Chrome for Testing PDF render failed with {status}"
        ));
    }
    verify_pdf(output)?;
    let downloads = Path::new(SITE_OUTPUT).join("downloads");
    fs::create_dir_all(&downloads)
        .map_err(|error| format!("cannot create {}: {error}", downloads.display()))?;
    fs::copy(output, downloads.join("nuif-research-manuscript.pdf"))
        .map_err(|error| format!("cannot add the manuscript to the site: {error}"))?;
    add_pdf_download_link(&source)?;
    println!("built technical manuscript at {}", output.display());
    Ok(())
}

pub(crate) fn serve() -> Result<(), String> {
    let compilation = compile(true).inspect_err(|error| {
        let _ = write_failure_report(error);
    })?;
    write_outputs(&compilation, "passed", &[])?;
    require_mdbook()?;
    run_mdbook(&["serve", BOOK_ROOT])
}

pub(crate) fn setup() -> Result<(), String> {
    let status = Command::new("cargo")
        .args(["install", "mdbook", "--version", MDBOOK_VERSION, "--locked"])
        .status()
        .map_err(|error| format!("could not start cargo install: {error}"))?;
    if status.success() {
        require_mdbook()
    } else {
        Err(format!("cargo install mdbook failed with {status}"))
    }
}

fn compile(stage: bool) -> Result<Compilation, String> {
    let config_bytes =
        fs::read(CONFIG_PATH).map_err(|error| format!("cannot read {CONFIG_PATH}: {error}"))?;
    let config: CatalogConfig = serde_json::from_slice(&config_bytes)
        .map_err(|error| format!("cannot parse {CONFIG_PATH}: {error}"))?;
    validate_config(&config)?;

    let excluded = config.exclude.iter().cloned().collect::<BTreeSet<_>>();
    let mut selected = BTreeSet::new();
    let mut located = Vec::new();
    for section in &config.sections {
        for configured_path in &section.paths {
            for source_path in expand_path(configured_path)? {
                if excluded.contains(&source_path) {
                    continue;
                }
                if !selected.insert(source_path.clone()) {
                    return Err(format!(
                        "documentation source appears more than once: {}",
                        source_path.display()
                    ));
                }
                located.push((section, source_path));
            }
        }
    }

    if located.is_empty() {
        return Err("documentation catalog selects no Markdown files".to_owned());
    }

    let mut documents = Vec::with_capacity(located.len());
    let mut identifiers = BTreeMap::<String, PathBuf>::new();
    let mut digest = Sha256::new();
    digest.update(&config_bytes);
    for (section, source_path) in located {
        let document = read_document(&config, section, &source_path)?;
        if let Some(previous) = identifiers.insert(document.id.clone(), source_path.clone()) {
            return Err(format!(
                "duplicate document identifier {} in {} and {}",
                document.id,
                previous.display(),
                source_path.display()
            ));
        }
        validate_links(&document.source_path, &document.body)?;
        digest.update(document.source_path.to_string_lossy().as_bytes());
        digest.update([0]);
        digest.update(&document.source_bytes);
        digest.update([0]);
        documents.push(document);
    }
    let source_digest = format!("sha256:{:x}", digest.finalize());
    let compilation = Compilation {
        config,
        documents,
        source_digest,
    };
    for source in &compilation.config.manuscript.sources {
        if !compilation
            .documents
            .iter()
            .any(|document| &document.source_path == source)
        {
            return Err(format!(
                "manuscript source is not selected for publication: {}",
                source.display()
            ));
        }
    }
    if stage {
        stage_book(&compilation)?;
    }
    Ok(compilation)
}

fn validate_config(config: &CatalogConfig) -> Result<(), String> {
    if config.schema_version != 1 {
        return Err(format!(
            "unsupported documentation catalog schema version {}",
            config.schema_version
        ));
    }
    if config.schema.as_deref() != Some("./schema/catalog.schema.json") {
        return Err("documentation catalog must identify its checked-in schema".to_owned());
    }
    if config.limits.document_bytes < 1_024
        || config.limits.frontmatter_bytes < 256
        || u64::try_from(config.limits.frontmatter_bytes)
            .is_ok_and(|limit| limit > config.limits.document_bytes)
    {
        return Err("documentation input limits are inconsistent".to_owned());
    }
    for (field, value) in [
        ("site.title", config.site.title.as_str()),
        ("site.description", config.site.description.as_str()),
        ("site.repository_url", config.site.repository_url.as_str()),
        ("site.canonical_url", config.site.canonical_url.as_str()),
        ("site.maturity", config.site.maturity.as_str()),
        ("manuscript.title", config.manuscript.title.as_str()),
        ("manuscript.subtitle", config.manuscript.subtitle.as_str()),
        ("manuscript.version", config.manuscript.version.as_str()),
        ("manuscript.date", config.manuscript.date.as_str()),
        ("manuscript.status", config.manuscript.status.as_str()),
        (
            "manuscript.abstract",
            config.manuscript.abstract_text.as_str(),
        ),
    ] {
        if value.trim().is_empty() {
            return Err(format!("{field} must not be empty"));
        }
    }
    if !config.site.repository_url.starts_with("https://")
        || !config.site.canonical_url.starts_with("https://")
    {
        return Err("documentation URLs must use HTTPS".to_owned());
    }

    let mut section_ids = BTreeSet::new();
    for section in &config.sections {
        if !valid_slug(&section.id) || !section_ids.insert(section.id.as_str()) {
            return Err(format!(
                "documentation section identifier is invalid or duplicated: {}",
                section.id
            ));
        }
        if section.title.trim().is_empty() || section.paths.is_empty() {
            return Err(format!(
                "documentation section {} needs a title and at least one path",
                section.id
            ));
        }
        for path in &section.paths {
            validate_relative_path(path)?;
        }
    }
    for path in config
        .frontmatter_required
        .iter()
        .chain(config.exclude.iter())
        .chain(config.manuscript.sources.iter())
    {
        validate_relative_path(path)?;
    }
    if config.manuscript.sources.is_empty() {
        return Err("documentation manuscript selects no source modules".to_owned());
    }
    if config.manuscript.authors.is_empty()
        || config
            .manuscript
            .authors
            .iter()
            .any(|author| author.trim().is_empty())
    {
        return Err("documentation manuscript requires non-empty authors".to_owned());
    }
    let unique_manuscript_sources = config.manuscript.sources.iter().collect::<BTreeSet<_>>();
    if unique_manuscript_sources.len() != config.manuscript.sources.len() {
        return Err("documentation manuscript contains duplicate source modules".to_owned());
    }
    Ok(())
}

fn validate_relative_path(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "documentation paths must be normalized repository-relative paths: {}",
            path.display()
        ));
    }
    Ok(())
}

fn expand_path(path: &Path) -> Result<Vec<PathBuf>, String> {
    validate_relative_path(path)?;
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("catalog path {} is unavailable: {error}", path.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "documentation catalog path must not be a symbolic link: {}",
            path.display()
        ));
    }
    if metadata.is_file() {
        if path.extension().and_then(|value| value.to_str()) != Some("md") {
            return Err(format!(
                "documentation source is not Markdown: {}",
                path.display()
            ));
        }
        return Ok(vec![path.to_owned()]);
    }
    if !metadata.is_dir() {
        return Err(format!(
            "documentation catalog path has an unsupported type: {}",
            path.display()
        ));
    }
    let mut output = Vec::new();
    collect_markdown(path, &mut output)?;
    output.sort_by_key(|entry| {
        let readme_rank =
            usize::from(entry.file_name().and_then(|value| value.to_str()) != Some("README.md"));
        (readme_rank, entry.clone())
    });
    Ok(output)
}

fn collect_markdown(directory: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("cannot read {}: {error}", directory.display()))?;
    let mut entries = entries
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("cannot enumerate {}: {error}", directory.display()))?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?;
        if file_type.is_symlink() {
            return Err(format!(
                "published documentation must not traverse symbolic links: {}",
                path.display()
            ));
        }
        if file_type.is_dir() {
            collect_markdown(&path, output)?;
        } else if file_type.is_file()
            && path.extension().and_then(|value| value.to_str()) == Some("md")
        {
            output.push(path);
        }
    }
    Ok(())
}

fn read_document(
    config: &CatalogConfig,
    section: &SectionConfig,
    source_path: &Path,
) -> Result<Document, String> {
    let metadata = fs::metadata(source_path)
        .map_err(|error| format!("cannot inspect {}: {error}", source_path.display()))?;
    if metadata.len() > config.limits.document_bytes {
        return Err(format!(
            "{} exceeds the {} byte documentation limit",
            source_path.display(),
            config.limits.document_bytes
        ));
    }
    let source_bytes = fs::read(source_path)
        .map_err(|error| format!("cannot read {}: {error}", source_path.display()))?;
    let source = std::str::from_utf8(&source_bytes)
        .map_err(|error| format!("{} is not UTF-8: {error}", source_path.display()))?;
    let (frontmatter_source, body) = split_frontmatter(source, config.limits.frontmatter_bytes)
        .map_err(|error| format!("{}: {error}", source_path.display()))?;
    let frontmatter = frontmatter_source
        .map(|yaml| {
            serde_saphyr::from_str::<Frontmatter>(yaml)
                .map_err(|error| format!("frontmatter is invalid: {error}"))
        })
        .transpose()
        .map_err(|error| format!("{}: {error}", source_path.display()))?;
    let required = config
        .frontmatter_required
        .iter()
        .any(|root| source_path == root || source_path.starts_with(root));
    if required && frontmatter.is_none() {
        return Err(format!(
            "{} requires YAML frontmatter",
            source_path.display()
        ));
    }

    if let Some(frontmatter) = &frontmatter {
        validate_frontmatter(source_path, frontmatter, required)?;
    }
    let title = frontmatter
        .as_ref()
        .and_then(|metadata| metadata.title.as_deref())
        .map(str::to_owned)
        .or_else(|| first_heading(body).map(str::to_owned))
        .ok_or_else(|| format!("{} has no level-one heading", source_path.display()))?;
    let id = frontmatter
        .as_ref()
        .and_then(|metadata| metadata.id.clone())
        .unwrap_or_else(|| derived_identifier(source_path));
    let kind = frontmatter
        .as_ref()
        .and_then(|metadata| metadata.kind.clone())
        .unwrap_or_else(|| inferred_kind(source_path).to_owned());
    let status = frontmatter
        .as_ref()
        .and_then(|metadata| metadata.status.clone())
        .unwrap_or_else(|| "informational".to_owned());

    Ok(Document {
        source_path: source_path.to_owned(),
        section_id: section.id.clone(),
        section_title: section.title.clone(),
        id,
        kind,
        status,
        title,
        frontmatter,
        body: body.to_owned(),
        source_bytes,
    })
}

fn split_frontmatter(source: &str, limit: usize) -> Result<(Option<&str>, &str), String> {
    let (prefix_len, rest) = if let Some(rest) = source.strip_prefix("---\n") {
        (4, rest)
    } else if let Some(rest) = source.strip_prefix("---\r\n") {
        (5, rest)
    } else {
        return Ok((None, source));
    };
    let mut consumed = 0;
    for line in rest.split_inclusive('\n') {
        let content = line.trim_end_matches(['\r', '\n']);
        if content == "---" {
            let metadata_end = consumed;
            let body_start = prefix_len + consumed + line.len();
            if metadata_end > limit {
                return Err(format!(
                    "frontmatter exceeds the configured {limit} byte limit"
                ));
            }
            return Ok((Some(&rest[..metadata_end]), &source[body_start..]));
        }
        consumed += line.len();
        if consumed > limit {
            return Err(format!(
                "frontmatter exceeds the configured {limit} byte limit"
            ));
        }
    }
    Err("opening frontmatter delimiter has no closing delimiter".to_owned())
}

fn validate_frontmatter(
    path: &Path,
    frontmatter: &Frontmatter,
    required: bool,
) -> Result<(), String> {
    if required
        && (frontmatter.id.is_none() || frontmatter.kind.is_none() || frontmatter.status.is_none())
    {
        return Err(format!(
            "{} frontmatter requires id, kind and status",
            path.display()
        ));
    }
    if let Some(id) = &frontmatter.id
        && !valid_identifier(id)
    {
        return Err(format!(
            "{} has invalid document identifier {id:?}",
            path.display()
        ));
    }
    if let Some(kind) = &frontmatter.kind
        && !DOCUMENT_KINDS.contains(&kind.as_str())
    {
        return Err(format!(
            "{} has undeclared document kind {kind:?}",
            path.display()
        ));
    }
    if let Some(status) = &frontmatter.status
        && !DOCUMENT_STATUSES.contains(&status.as_str())
    {
        return Err(format!(
            "{} has undeclared document status {status:?}",
            path.display()
        ));
    }
    if frontmatter
        .title
        .as_ref()
        .is_some_and(|title| title.trim().is_empty())
    {
        return Err(format!("{} has an empty metadata title", path.display()));
    }
    Ok(())
}

fn first_heading(body: &str) -> Option<&str> {
    body.lines()
        .find_map(|line| line.strip_prefix("# ").map(str::trim))
        .filter(|title| !title.is_empty())
}

fn derived_identifier(path: &Path) -> String {
    let stem = path
        .with_extension("")
        .to_string_lossy()
        .to_ascii_lowercase()
        .replace(['/', '_'], "-");
    format!("nuif:doc:{stem}")
}

fn inferred_kind(path: &Path) -> &'static str {
    if path.starts_with("spec") {
        "specification"
    } else if path.starts_with("rfcs") {
        "rfc"
    } else if path.starts_with("adrs") {
        "adr"
    } else if path.starts_with("research") {
        "synthesis"
    } else {
        "implementation"
    }
}

fn valid_identifier(value: &str) -> bool {
    let segments = value.split(':').collect::<Vec<_>>();
    segments.len() >= 3
        && segments[0] == "nuif"
        && segments.iter().skip(1).all(|segment| {
            !segment.is_empty()
                && segment.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'-' | b'_' | b'.')
                })
        })
}

fn valid_slug(value: &str) -> bool {
    value.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
        })
}

fn validate_links(source_path: &Path, body: &str) -> Result<(), String> {
    for destination in markdown_destinations(body) {
        if external_destination(&destination.value) || destination.value.starts_with('#') {
            continue;
        }
        let (path_value, _) = split_destination(&destination.value);
        if path_value.is_empty() {
            continue;
        }
        let resolved = resolve_link(source_path, path_value)?;
        if !resolved.exists() {
            return Err(format!(
                "{} links to missing repository path {}",
                source_path.display(),
                resolved.display()
            ));
        }
    }
    Ok(())
}

fn markdown_destinations(body: &str) -> Vec<LinkDestination> {
    let mut output = Vec::new();
    let mut offset = 0;
    let mut fence: Option<char> = None;
    for line in body.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            fence = if fence == Some('`') {
                None
            } else if fence.is_none() {
                Some('`')
            } else {
                fence
            };
            offset += line.len();
            continue;
        }
        if trimmed.starts_with("~~~") {
            fence = if fence == Some('~') {
                None
            } else if fence.is_none() {
                Some('~')
            } else {
                fence
            };
            offset += line.len();
            continue;
        }
        if fence.is_none() {
            collect_line_destinations(line, offset, &mut output);
        }
        offset += line.len();
    }
    output
}

fn collect_line_destinations(line: &str, offset: usize, output: &mut Vec<LinkDestination>) {
    let bytes = line.as_bytes();
    let mut index = 0;
    let mut inline_code = false;
    while index + 1 < bytes.len() {
        if bytes[index] == b'`' {
            inline_code = !inline_code;
            index += 1;
            continue;
        }
        if !inline_code && bytes[index] == b']' && bytes[index + 1] == b'(' {
            let image = line[..index]
                .rfind('[')
                .is_some_and(|opening| opening > 0 && bytes[opening - 1] == b'!');
            let mut cursor = index + 2;
            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            let angle = cursor < bytes.len() && bytes[cursor] == b'<';
            if angle {
                cursor += 1;
            }
            let start = cursor;
            let mut nesting = 0_u32;
            let mut escaped = false;
            while cursor < bytes.len() {
                let byte = bytes[cursor];
                if escaped {
                    escaped = false;
                    cursor += 1;
                    continue;
                }
                if byte == b'\\' {
                    escaped = true;
                    cursor += 1;
                    continue;
                }
                if angle && byte == b'>' {
                    break;
                }
                if !angle && nesting == 0 && (byte == b')' || byte.is_ascii_whitespace()) {
                    break;
                }
                if !angle && byte == b'(' {
                    nesting += 1;
                } else if !angle && byte == b')' && nesting > 0 {
                    nesting -= 1;
                }
                cursor += 1;
            }
            if cursor > start {
                output.push(LinkDestination {
                    start: offset + start,
                    end: offset + cursor,
                    value: line[start..cursor].to_owned(),
                    image,
                });
            }
            index = cursor;
        } else {
            index += 1;
        }
    }
}

fn external_destination(value: &str) -> bool {
    value.starts_with("https://") || value.starts_with("http://") || value.starts_with("mailto:")
}

fn split_destination(value: &str) -> (&str, &str) {
    value
        .find(['#', '?'])
        .map_or((value, ""), |index| (&value[..index], &value[index..]))
}

fn resolve_link(source_path: &Path, value: &str) -> Result<PathBuf, String> {
    if value.starts_with('/') {
        return Err(format!(
            "{} contains a repository-absolute link {value:?}",
            source_path.display()
        ));
    }
    let base = source_path.parent().unwrap_or_else(|| Path::new(""));
    let mut output = PathBuf::new();
    for component in base.join(value).components() {
        match component {
            Component::Normal(part) => output.push(part),
            Component::CurDir => {}
            Component::ParentDir => {
                if !output.pop() {
                    return Err(format!(
                        "{} link escapes the repository: {value}",
                        source_path.display()
                    ));
                }
            }
            Component::Prefix(_) | Component::RootDir => {
                return Err(format!(
                    "{} contains an unsupported link path: {value}",
                    source_path.display()
                ));
            }
        }
    }
    Ok(output)
}

fn stage_book(compilation: &Compilation) -> Result<(), String> {
    let book_root = Path::new(BOOK_ROOT);
    remove_exact_directory(book_root)?;
    fs::create_dir_all(BOOK_SOURCE)
        .map_err(|error| format!("cannot create {BOOK_SOURCE}: {error}"))?;

    let selected = compilation
        .documents
        .iter()
        .map(|document| document.source_path.clone())
        .collect::<BTreeSet<_>>();
    for document in &compilation.documents {
        let destination = Path::new(BOOK_SOURCE).join(&document.source_path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        }
        let staged = stage_document(document, &selected, &compilation.config.site.repository_url)?;
        fs::write(&destination, staged)
            .map_err(|error| format!("cannot write {}: {error}", destination.display()))?;
    }
    write_generated_pages(compilation)?;
    fs::write(
        Path::new(BOOK_SOURCE).join("SUMMARY.md"),
        build_summary(compilation),
    )
    .map_err(|error| format!("cannot write generated SUMMARY.md: {error}"))?;
    fs::write(book_root.join("book.toml"), build_book_config(compilation))
        .map_err(|error| format!("cannot write generated book.toml: {error}"))?;
    let theme = book_root.join("theme");
    fs::create_dir_all(&theme)
        .map_err(|error| format!("cannot create {}: {error}", theme.display()))?;
    fs::copy("docs/theme/nuif.css", theme.join("nuif.css"))
        .map_err(|error| format!("cannot stage documentation theme: {error}"))?;
    Ok(())
}

fn stage_document(
    document: &Document,
    selected: &BTreeSet<PathBuf>,
    repository_url: &str,
) -> Result<String, String> {
    let mut body = rewrite_unpublished_links(document, selected, repository_url)?;
    if document
        .frontmatter
        .as_ref()
        .and_then(|value| value.title.as_ref())
        .is_some()
        && first_heading(&body) == Some("Summary")
    {
        body = body.replacen(
            "# Summary",
            &format!("# {}\n\n## Summary", document.title),
            1,
        );
    }
    let source_url = format!(
        "{repository_url}/blob/main/{}",
        document.source_path.to_string_lossy()
    );
    if document.frontmatter.is_some() {
        let (without_legacy, visible_status) = remove_legacy_status(&body);
        body = inject_after_heading(
            &without_legacy,
            &format!(
                "> Document status: {}. [Canonical source]({source_url}).",
                visible_status.unwrap_or_else(|| format!("`{}`", document.status))
            ),
        );
    } else {
        let _ = write!(body, "\n\n---\n\n[Canonical source]({source_url}).\n");
    }
    Ok(body)
}

fn remove_legacy_status(body: &str) -> (String, Option<String>) {
    let mut output = String::with_capacity(body.len());
    let mut visible_status = None;
    for (index, line) in body.split_inclusive('\n').enumerate() {
        if index < 12
            && visible_status.is_none()
            && let Some(status) = line.trim_end_matches(['\r', '\n']).strip_prefix("Status:")
        {
            visible_status = Some(status.trim().trim_end_matches('.').to_owned());
            continue;
        }
        output.push_str(line);
    }
    (output, visible_status)
}

fn inject_after_heading(body: &str, banner: &str) -> String {
    let mut output = String::with_capacity(body.len() + banner.len() + 4);
    let mut inserted = false;
    for line in body.split_inclusive('\n') {
        output.push_str(line);
        if !inserted && line.starts_with("# ") {
            output.push('\n');
            output.push_str(banner);
            output.push_str("\n\n");
            inserted = true;
        }
    }
    if !inserted {
        output.insert_str(0, &format!("{banner}\n\n"));
    }
    output
}

fn rewrite_unpublished_links(
    document: &Document,
    selected: &BTreeSet<PathBuf>,
    repository_url: &str,
) -> Result<String, String> {
    let destinations = markdown_destinations(&document.body);
    let mut output = document.body.clone();
    for destination in destinations.into_iter().rev() {
        if external_destination(&destination.value) || destination.value.starts_with('#') {
            continue;
        }
        let (path_value, suffix) = split_destination(&destination.value);
        if path_value.is_empty() {
            continue;
        }
        let resolved = resolve_link(&document.source_path, path_value)?;
        if selected.contains(&resolved) {
            if resolved.file_name().and_then(|value| value.to_str()) == Some("README.md") {
                let directory = path_value
                    .strip_suffix("README.md")
                    .filter(|value| !value.is_empty())
                    .unwrap_or("./");
                output.replace_range(
                    destination.start..destination.end,
                    &format!("{directory}{suffix}"),
                );
            }
            continue;
        }
        let target = if destination.image {
            format!(
                "https://raw.githubusercontent.com/refpath/nuif/main/{}{}",
                resolved.to_string_lossy(),
                suffix
            )
        } else {
            let mode = if resolved.is_dir() { "tree" } else { "blob" };
            format!(
                "{repository_url}/{mode}/main/{}{}",
                resolved.to_string_lossy(),
                suffix
            )
        };
        output.replace_range(destination.start..destination.end, &target);
    }
    Ok(output)
}

fn write_generated_pages(compilation: &Compilation) -> Result<(), String> {
    let generated = Path::new(BOOK_SOURCE).join("generated");
    fs::create_dir_all(&generated)
        .map_err(|error| format!("cannot create {}: {error}", generated.display()))?;
    fs::write(
        generated.join("documentation-index.md"),
        generated_documentation_index(compilation),
    )
    .map_err(|error| format!("cannot write documentation index: {error}"))?;
    fs::write(
        generated.join("research-index.md"),
        generated_research_index(compilation),
    )
    .map_err(|error| format!("cannot write research index: {error}"))?;
    fs::write(
        generated.join("adapter-index.md"),
        generated_adapter_index()?,
    )
    .map_err(|error| format!("cannot write adapter index: {error}"))?;
    fs::write(
        generated.join("research-manuscript.md"),
        generated_research_manuscript(compilation)?,
    )
    .map_err(|error| format!("cannot write research manuscript: {error}"))?;
    Ok(())
}

fn generated_research_manuscript(compilation: &Compilation) -> Result<String, String> {
    let manuscript = &compilation.config.manuscript;
    let mut output = format!(
        "# {}\n\n{}\n\n{}  \nVersion `{}` · {}\n\n> Manuscript status: {}. This generated document does not change the status of any included specification module.\n\n## Abstract\n\n{}\n\n## Reproducibility\n\nThe body is compiled from {} canonical whitepaper modules at source digest `{}`. Editorial changes belong in those source modules.\n",
        manuscript.title,
        manuscript.subtitle,
        manuscript.authors.join(", "),
        manuscript.version,
        manuscript.date,
        manuscript.status,
        manuscript.abstract_text,
        manuscript.sources.len(),
        compilation.source_digest
    );
    for source in &manuscript.sources {
        let document = compilation
            .documents
            .iter()
            .find(|document| &document.source_path == source)
            .ok_or_else(|| format!("manuscript source is unavailable: {}", source.display()))?;
        let rewritten =
            rewrite_manuscript_links(document, &compilation.config.site.repository_url)?;
        let (without_status, _) = remove_legacy_status(&rewritten);
        output.push_str("\n\n---\n\n");
        output.push_str(&demote_headings(&without_status));
        let _ = write!(
            output,
            "\n\n<p class=\"canonical-module\"><a href=\"{}/blob/main/{}\">Canonical module.</a></p>\n",
            compilation.config.site.repository_url,
            source.to_string_lossy()
        );
    }
    Ok(output)
}

fn rewrite_manuscript_links(document: &Document, repository_url: &str) -> Result<String, String> {
    let destinations = markdown_destinations(&document.body);
    let mut output = document.body.clone();
    for destination in destinations.into_iter().rev() {
        if external_destination(&destination.value) || destination.value.starts_with('#') {
            continue;
        }
        let (path_value, suffix) = split_destination(&destination.value);
        if path_value.is_empty() {
            continue;
        }
        let resolved = resolve_link(&document.source_path, path_value)?;
        let target = if destination.image {
            format!(
                "https://raw.githubusercontent.com/refpath/nuif/main/{}{}",
                resolved.to_string_lossy(),
                suffix
            )
        } else {
            let mode = if resolved.is_dir() { "tree" } else { "blob" };
            format!(
                "{repository_url}/{mode}/main/{}{}",
                resolved.to_string_lossy(),
                suffix
            )
        };
        output.replace_range(destination.start..destination.end, &target);
    }
    Ok(output)
}

fn demote_headings(body: &str) -> String {
    let mut output = String::with_capacity(body.len());
    let mut fence: Option<char> = None;
    for line in body.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            fence = if fence == Some('`') {
                None
            } else if fence.is_none() {
                Some('`')
            } else {
                fence
            };
        } else if trimmed.starts_with("~~~") {
            fence = if fence == Some('~') {
                None
            } else if fence.is_none() {
                Some('~')
            } else {
                fence
            };
        }
        if fence.is_none() && line.starts_with('#') {
            output.push('#');
        }
        output.push_str(line);
    }
    output
}

fn generated_documentation_index(compilation: &Compilation) -> String {
    let mut by_status = BTreeMap::<&str, usize>::new();
    for document in &compilation.documents {
        *by_status.entry(&document.status).or_default() += 1;
    }
    let mut output = format!(
        "# Documentation index\n\nThis page is generated from `docs/catalog.json` and document frontmatter. The site describes a {}.\n\n- Source digest: `{}`\n- Published Markdown files: {}\n- Navigation sections: {}\n\n## Status inventory\n\n| Status | Documents |\n|---|---:|\n",
        compilation.config.site.maturity,
        compilation.source_digest,
        compilation.documents.len(),
        compilation.config.sections.len()
    );
    for (status, count) in by_status {
        let _ = writeln!(output, "| `{status}` | {count} |");
    }
    output
}

fn generated_research_index(compilation: &Compilation) -> String {
    let records = compilation
        .documents
        .iter()
        .filter(|document| document.source_path.starts_with("research/items"))
        .collect::<Vec<_>>();
    let mut by_status = BTreeMap::<&str, usize>::new();
    for document in &records {
        *by_status.entry(&document.status).or_default() += 1;
    }
    let mut output = format!(
        "# Research record index\n\nThis page is generated from {} registered source records.\n\n## Status inventory\n\n| Status | Records |\n|---|---:|\n",
        records.len()
    );
    for (status, count) in by_status {
        let _ = writeln!(output, "| `{status}` | {count} |");
    }
    output.push_str("\n## Records\n\n| Record | Status | Kind |\n|---|---|---|\n");
    for document in records {
        let _ = writeln!(
            output,
            "| [{}](../{}) | `{}` | `{}` |",
            document.title,
            document.source_path.to_string_lossy(),
            document.status,
            document.kind
        );
    }
    output
}

fn generated_adapter_index() -> Result<String, String> {
    let bytes = fs::read("adapters/index.json")
        .map_err(|error| format!("cannot read adapters/index.json: {error}"))?;
    let index: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("cannot parse adapters/index.json: {error}"))?;
    let targets = index["targets"]
        .as_array()
        .ok_or("adapters/index.json targets must be an array")?;
    let mut output = format!(
        "# Adapter inventory\n\nThis page is generated from `adapters/index.json`. The inventory contains {} host or format targets.\n\n| Target | Status | Surface | Profiles |\n|---|---|---|---:|\n",
        targets.len()
    );
    for target in targets {
        let id = target["id"].as_str().unwrap_or("invalid");
        let status = target["status"].as_str().unwrap_or("invalid");
        let surface = target["surface"].as_str().unwrap_or("invalid");
        let profiles = target["profiles"].as_array().map_or(0, Vec::len);
        let _ = writeln!(output, "| `{id}` | `{status}` | {surface} | {profiles} |");
    }
    Ok(output)
}

fn build_summary(compilation: &Compilation) -> String {
    let mut output =
        String::from("# Summary\n\n[Documentation index](generated/documentation-index.md)\n\n");
    let _ = writeln!(
        output,
        "[{}](generated/research-manuscript.md)\n",
        compilation.config.manuscript.title
    );
    for section in &compilation.config.sections {
        let _ = write!(output, "# {}\n\n", section.title);
        if section.id == "research" {
            output.push_str("- [Research record index](generated/research-index.md)\n");
        } else if section.id == "adapters" {
            output.push_str("- [Adapter inventory](generated/adapter-index.md)\n");
        }
        for document in compilation
            .documents
            .iter()
            .filter(|document| document.section_id == section.id)
        {
            let _ = writeln!(
                output,
                "- [{}]({})",
                summary_title(&document.title),
                document.source_path.to_string_lossy()
            );
        }
        output.push('\n');
    }
    output
}

fn summary_title(title: &str) -> String {
    title.replace(['[', ']'], "")
}

fn build_book_config(compilation: &Compilation) -> String {
    format!(
        "[book]\ntitle = {:?}\ndescription = {:?}\nauthors = [\"NUIF contributors\"]\nlanguage = \"en\"\nsrc = \"src\"\n\n[build]\nbuild-dir = \"../docs-site\"\ncreate-missing = false\n\n[output.html]\nsite-url = \"/nuif/\"\ngit-repository-url = {:?}\nadditional-css = [\"theme/nuif.css\"]\nno-section-label = true\ndefault-theme = \"light\"\npreferred-dark-theme = \"ayu\"\n\n[output.html.search]\nenable = true\nlimit-results = 30\nuse-boolean-and = true\n",
        compilation.config.site.title,
        compilation.config.site.description,
        compilation.config.site.repository_url
    )
}

fn write_outputs(
    compilation: &Compilation,
    status: &str,
    failures: &[String],
) -> Result<(), String> {
    fs::create_dir_all("target").map_err(|error| format!("cannot create target: {error}"))?;
    let documents = compilation
        .documents
        .iter()
        .map(|document| {
            serde_json::json!({
                "id": document.id,
                "kind": document.kind,
                "status": document.status,
                "title": document.title,
                "section": document.section_id,
                "section_title": document.section_title,
                "source_path": document.source_path,
            })
        })
        .collect::<Vec<_>>();
    let catalog = serde_json::json!({
        "schema_version": 1,
        "source_digest": compilation.source_digest,
        "site": {
            "title": compilation.config.site.title,
            "canonical_url": compilation.config.site.canonical_url,
            "repository_url": compilation.config.site.repository_url,
            "maturity": compilation.config.site.maturity,
        },
        "documents": documents,
    });
    write_json(Path::new(CATALOG_OUTPUT), &catalog)?;
    let report = serde_json::json!({
        "schema_version": 1,
        "status": status,
        "source_digest": compilation.source_digest,
        "summary": {
            "documents": compilation.documents.len(),
            "sections": compilation.config.sections.len(),
            "blocking_failures": failures.len(),
        },
        "failures": failures,
    });
    write_json(Path::new(REPORT_OUTPUT), &report)
}

fn write_failure_report(error: &str) -> Result<(), String> {
    fs::create_dir_all("target").map_err(|failure| format!("cannot create target: {failure}"))?;
    let report = serde_json::json!({
        "schema_version": 1,
        "status": "failed",
        "summary": {"blocking_failures": 1},
        "failures": [error],
    });
    write_json(Path::new(REPORT_OUTPUT), &report)
}

fn write_json(path: &Path, value: &serde_json::Value) -> Result<(), String> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    fs::write(path, bytes).map_err(|error| format!("cannot write {}: {error}", path.display()))
}

fn absolute_path(path: &Path) -> Result<PathBuf, String> {
    std::env::current_dir()
        .map(|directory| directory.join(path))
        .map_err(|error| format!("cannot resolve the working directory: {error}"))
}

fn chrome_for_testing() -> Result<PathBuf, String> {
    let candidates = [
        PathBuf::from(
            "target/chrome-for-testing/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
        ),
        PathBuf::from(
            "target/chrome-for-testing/chrome-mac-x64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
        ),
        PathBuf::from("target/chrome-for-testing/chrome-linux64/chrome"),
        PathBuf::from("target/chrome-for-testing/chrome-win64/chrome.exe"),
    ];
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(|| {
            "the pinned Chrome for Testing binary is absent; run cargo xtask browser-install"
                .to_owned()
        })
}

fn verify_pdf(path: &Path) -> Result<(), String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("cannot read generated PDF {}: {error}", path.display()))?;
    let eof_present = bytes
        .windows(5)
        .rposition(|window| window == b"%%EOF")
        .is_some_and(|position| position + 1_024 >= bytes.len());
    if bytes.len() < 10_000 || !bytes.starts_with(b"%PDF-") || !eof_present {
        return Err(format!(
            "generated manuscript is not a complete PDF artifact: {} bytes",
            bytes.len()
        ));
    }
    Ok(())
}

fn add_pdf_download_link(page: &Path) -> Result<(), String> {
    let html = fs::read_to_string(page)
        .map_err(|error| format!("cannot read {}: {error}", page.display()))?;
    let marker = "<main>";
    let replacement = concat!(
        "<main>\n",
        "<p><a href=\"../downloads/nuif-research-manuscript.pdf\" ",
        "download>Download the generated PDF manuscript</a></p>"
    );
    if !html.contains(marker) {
        return Err(format!(
            "generated manuscript page has no main element: {}",
            page.display()
        ));
    }
    fs::write(page, html.replacen(marker, replacement, 1))
        .map_err(|error| format!("cannot update {}: {error}", page.display()))
}

fn require_mdbook() -> Result<(), String> {
    let output = Command::new("mdbook")
        .arg("--version")
        .output()
        .map_err(|error| {
            format!("mdBook {MDBOOK_VERSION} is required: {error}; run cargo xtask docs-setup")
        })?;
    if !output.status.success() {
        return Err(format!("mdbook --version failed with {}", output.status));
    }
    let version = String::from_utf8_lossy(&output.stdout);
    if !version
        .split_whitespace()
        .any(|part| part.trim_start_matches('v') == MDBOOK_VERSION)
    {
        return Err(format!(
            "mdBook {MDBOOK_VERSION} is required, found {:?}; run cargo xtask docs-setup",
            version.trim()
        ));
    }
    Ok(())
}

fn run_mdbook(arguments: &[&str]) -> Result<(), String> {
    let status = Command::new("mdbook")
        .args(arguments)
        .status()
        .map_err(|error| format!("could not start mdbook: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "mdbook {} failed with {status}",
            arguments.join(" ")
        ))
    }
}

fn verify_site(compilation: &Compilation) -> Result<(), String> {
    let index = Path::new(SITE_OUTPUT).join("index.html");
    if !index.is_file() {
        return Err(format!("mdBook did not produce {}", index.display()));
    }
    let html_count = count_files_with_extension(Path::new(SITE_OUTPUT), "html")?;
    if html_count < compilation.documents.len() {
        return Err(format!(
            "mdBook produced {html_count} HTML files for {} source documents",
            compilation.documents.len()
        ));
    }
    for required in ["print.html", "generated/documentation-index.html"] {
        let path = Path::new(SITE_OUTPUT).join(required);
        if !path.is_file() {
            return Err(format!("mdBook output is missing {}", path.display()));
        }
    }
    if !directory_contains_prefix(Path::new(SITE_OUTPUT), "searchindex-", ".js")? {
        return Err("mdBook output is missing its content-addressed search index".to_owned());
    }
    Ok(())
}

fn directory_contains_prefix(directory: &Path, prefix: &str, suffix: &str) -> Result<bool, String> {
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("cannot inspect {}: {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        if entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_file()
            && entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.starts_with(prefix) && name.ends_with(suffix))
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn count_files_with_extension(directory: &Path, extension: &str) -> Result<usize, String> {
    let mut count = 0;
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("cannot inspect {}: {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_dir() {
            count += count_files_with_extension(&entry.path(), extension)?;
        } else if file_type.is_file()
            && entry.path().extension().and_then(|value| value.to_str()) == Some(extension)
        {
            count += 1;
        }
    }
    Ok(count)
}

fn remove_exact_directory(path: &Path) -> Result<(), String> {
    if !matches!(path.to_str(), Some("target/docs-book" | "target/docs-site")) {
        return Err(format!(
            "refusing to remove an undeclared documentation path: {}",
            path.display()
        ));
    }
    if path.exists() {
        fs::remove_dir_all(path)
            .map_err(|error| format!("cannot remove {}: {error}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontmatter_is_bounded_and_removed() {
        let source = "---\nid: nuif:spec:test\nkind: specification\nstatus: draft\n---\n# Test\n";
        let (metadata, body) = split_frontmatter(source, 128).expect("valid frontmatter");
        assert!(metadata.is_some_and(|value| value.contains("nuif:spec:test")));
        assert_eq!(body, "# Test\n");
        assert!(split_frontmatter(source, 8).is_err());
    }

    #[test]
    fn duplicate_yaml_keys_are_rejected() {
        let yaml = "id: nuif:spec:test\nid: nuif:spec:other\nkind: specification\nstatus: draft\n";
        assert!(serde_saphyr::from_str::<Frontmatter>(yaml).is_err());
    }

    #[test]
    fn markdown_links_skip_code_and_record_images() {
        let body = "[doc](../spec/01-model.md) ` [code](missing.md)` ![image](asset.png)\n```md\n[ignored](missing.md)\n```\n";
        let links = markdown_destinations(body);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].value, "../spec/01-model.md");
        assert!(!links[0].image);
        assert_eq!(links[1].value, "asset.png");
        assert!(links[1].image);
    }

    #[test]
    fn document_identifiers_have_bounded_segments() {
        assert!(valid_identifier("nuif:research:serde-saphyr-yaml-parser"));
        assert!(!valid_identifier("nuif:Research:parser"));
        assert!(!valid_identifier("nuif:spec:../escape"));
    }

    #[test]
    fn repository_links_cannot_escape_the_root() {
        assert_eq!(
            resolve_link(Path::new("docs/guide.md"), "../spec/01-model.md")
                .expect("repository link"),
            PathBuf::from("spec/01-model.md")
        );
        assert!(resolve_link(Path::new("README.md"), "../outside.md").is_err());
    }

    #[test]
    fn manuscript_headings_are_demoted_outside_code_fences() {
        let body = "# Chapter\n\n## Section\n\n```md\n# Example\n```\n";
        assert_eq!(
            demote_headings(body),
            "## Chapter\n\n### Section\n\n```md\n# Example\n```\n"
        );
    }
}
