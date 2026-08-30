use nuif_codec::canonical_hash;
use nuif_core::{
    Align, AuthoredProperties, Document, Edges, Entity, EntityId, EntityKind, Fidelity,
    FlowDirection, LayoutFamily, LayoutStyle, SizeIntent, TextContent,
};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::{Node, Parser};

use crate::profile::profile_issues;
use crate::syntax::{parse_id, parse_number, unescape};
use crate::{
    AdapterError, AdapterReport, CorrespondenceRecord, CorrespondenceTarget, FidelityEntry,
    ImportedSource, MAX_ELEMENT_DEPTH, MAX_SOURCE_BYTES, MAX_SYNTAX_NODES, PROFILE_NAME,
    RetentiveSource, SourceSpan, entity_pointer,
};

#[derive(Clone, Copy)]
struct Attribute {
    span: SourceSpan,
}

#[derive(Clone, Debug)]
struct StyleEntry {
    value: String,
    span: SourceSpan,
}

struct ParseState {
    document: Document,
    correspondences: Vec<CorrespondenceRecord>,
    fidelity: Vec<FidelityEntry>,
}

/// Imports one statically marked Svelte subtree without executing JavaScript.
///
/// # Errors
///
/// Rejects syntax errors, executable template constructs, source limits,
/// duplicate identities and any model state outside the declared profile.
pub fn import_source(source: &str) -> Result<ImportedSource, AdapterError> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(AdapterError::SourceTooLarge);
    }
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_svelte_next::LANGUAGE.into())
        .map_err(|error| AdapterError::SvelteSyntax(error.to_string()))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| AdapterError::SvelteSyntax("parser returned no tree".to_owned()))?;
    let root = tree.root_node();
    if root.has_error() {
        return Err(AdapterError::SvelteSyntax(first_error(root)));
    }
    if count_nodes(root) > MAX_SYNTAX_NODES {
        return Err(AdapterError::NodeLimit);
    }
    let roots = marked_roots(root, source)?;
    if roots.len() != 1 {
        return Err(AdapterError::ProfileMarker(format!(
            "expected one data-nuif-profile root, observed {}",
            roots.len()
        )));
    }
    validate_component_root(root, roots[0], source)?;
    let open = start_tag(roots[0])?;
    let attributes = attributes(open, source)?;
    let document_attr = text_attribute(&attributes, "data-nuif-document", source)?;
    let document_id = parse_id(&document_attr.0, "/id")?;
    let mut state = ParseState {
        document: Document::empty(document_id),
        correspondences: Vec::new(),
        fidelity: Vec::new(),
    };
    record(
        &mut state,
        CorrespondenceTarget::Document { id: document_id },
        "/id".to_owned(),
        document_attr.1,
    );
    let root_id = parse_element(roots[0], source, None, 0, true, &mut state)?;
    state.document.roots.push(root_id);
    let issues = profile_issues(&state.document);
    if let Some(issue) = issues.first() {
        return Err(AdapterError::InvalidValue {
            pointer: issue.pointer.clone(),
            reason: match &issue.status {
                Fidelity::Unsupported { reason } => reason.clone(),
                _ => "imported model is outside the profile".to_owned(),
            },
        });
    }
    state
        .correspondences
        .sort_by_key(|record| record.span.start);
    state
        .fidelity
        .sort_by(|left, right| (&left.target, &left.pointer).cmp(&(&right.target, &right.pointer)));
    let report = AdapterReport {
        schema_version: 1,
        source_format: PROFILE_NAME.to_owned(),
        canonical_hash: Some(
            canonical_hash(&state.document)
                .map_err(|error| AdapterError::Canonical(error.to_string()))?,
        ),
        fidelity: state.fidelity,
        correspondences: state.correspondences,
        unmapped_source_preserved: true,
    };
    let document = state.document;
    Ok(ImportedSource {
        document: document.clone(),
        retentive: RetentiveSource {
            source: source.to_owned(),
            document,
            report,
        },
    })
}

fn parse_element(
    node: Node<'_>,
    source: &str,
    parent: Option<EntityId>,
    depth: usize,
    is_root: bool,
    state: &mut ParseState,
) -> Result<EntityId, AdapterError> {
    if depth > MAX_ELEMENT_DEPTH {
        return Err(AdapterError::DepthLimit);
    }
    if node.kind() != "element" {
        return Err(AdapterError::InvalidValue {
            pointer: "/entities".to_owned(),
            reason: "mapped nodes must be paired regular Svelte elements".to_owned(),
        });
    }
    let open = start_tag(node)?;
    let close = end_tag(node)?;
    let tag = tag_name(open, source)?;
    if tag != tag_name(close, source)? {
        return Err(AdapterError::SvelteSyntax(
            "mapped opening and closing tags differ".to_owned(),
        ));
    }
    let attributes = attributes(open, source)?;
    let (id_text, id_span) = text_attribute(&attributes, "data-nuif-id", source)?;
    let id = parse_id(&id_text, "/entities/*/id")?;
    if state.document.entities.contains_key(&id) {
        return Err(AdapterError::InvalidValue {
            pointer: entity_pointer(id, "/id"),
            reason: "entity identifier is duplicated".to_owned(),
        });
    }
    let (kind, kind_span) = text_attribute(&attributes, "data-nuif-kind", source)?;
    let (name, name_span) = text_attribute(&attributes, "data-nuif-name", source)?;
    let target = CorrespondenceTarget::Entity { id };
    record(state, target.clone(), entity_pointer(id, "/id"), id_span);
    record(
        state,
        target.clone(),
        entity_pointer(id, "/kind"),
        kind_span,
    );
    record(state, target, entity_pointer(id, "/name"), name_span);
    if is_root {
        require_root_attributes(&attributes, source)?;
    } else if attributes.contains_key("data-nuif-profile")
        || attributes.contains_key("data-nuif-document")
    {
        return Err(AdapterError::ProfileMarker(
            "profile and document markers are allowed only on the mapped root".to_owned(),
        ));
    }

    let style_span = attributes
        .get("style")
        .ok_or_else(|| AdapterError::InvalidValue {
            pointer: entity_pointer(id, "/authored"),
            reason: "required literal style attribute is missing".to_owned(),
        })?
        .span;
    let style = parse_inline_style(style_span, source)?;
    let mut entity = match kind.as_str() {
        "container" => parse_container(id, &tag, &attributes, &style, state)?,
        "text" => parse_text(
            id,
            &tag,
            node,
            open,
            close,
            source,
            &attributes,
            &style,
            state,
        )?,
        _ => {
            return Err(AdapterError::InvalidValue {
                pointer: entity_pointer(id, "/kind"),
                reason: "kind must be container or text".to_owned(),
            });
        }
    };
    entity.name = Some(name);
    if let Some(parent) = parent {
        state
            .document
            .entities
            .get_mut(&parent)
            .expect("parent is inserted before children")
            .children
            .push(id);
    }
    let is_container = entity.kind == EntityKind::Container;
    state.document.entities.insert(id, entity);
    if is_container {
        parse_container_children(node, source, id, depth, state)?;
    }
    Ok(id)
}

#[expect(
    clippy::too_many_lines,
    reason = "the exact CSS vocabulary and correspondence mapping stay adjacent for audit"
)]
fn parse_container(
    id: EntityId,
    tag: &str,
    attributes: &BTreeMap<String, Attribute>,
    style: &BTreeMap<String, StyleEntry>,
    state: &mut ParseState,
) -> Result<Entity, AdapterError> {
    const KEYS: &[&str] = &[
        "align-items",
        "box-sizing",
        "display",
        "flex-direction",
        "gap",
        "height",
        "padding-bottom",
        "padding-left",
        "padding-right",
        "padding-top",
        "width",
    ];
    require_exact_keys(
        attributes.keys(),
        if attributes.contains_key("data-nuif-profile") {
            &[
                "data-nuif-document",
                "data-nuif-id",
                "data-nuif-kind",
                "data-nuif-name",
                "data-nuif-profile",
                "style",
            ][..]
        } else {
            &["data-nuif-id", "data-nuif-kind", "data-nuif-name", "style"][..]
        },
        &entity_pointer(id, ""),
        "attribute",
    )?;
    if tag != "div" {
        return invalid(id, "/kind", "containers must use the regular div tag");
    }
    require_exact_keys(
        style.keys(),
        KEYS,
        &entity_pointer(id, "/authored"),
        "style property",
    )?;
    require_text(style, "box-sizing", "border-box", id)?;
    require_text(style, "display", "flex", id)?;
    let direction = match style_value(style, "flex-direction", id)? {
        "row" => FlowDirection::Row,
        "column" => FlowDirection::Column,
        _ => {
            return invalid(
                id,
                "/authored/layout/direction",
                "flex-direction must be row or column",
            );
        }
    };
    let align = match style_value(style, "align-items", id)? {
        "flex-start" => Align::Start,
        "center" => Align::Center,
        "flex-end" => Align::End,
        "stretch" => Align::Stretch,
        _ => {
            return invalid(
                id,
                "/authored/layout/align",
                "align-items is outside the profile",
            );
        }
    };
    let width = px_style(style, "width", id)?;
    let height = px_style(style, "height", id)?;
    let gap = px_style(style, "gap", id)?;
    let top = px_style(style, "padding-top", id)?;
    let right = px_style(style, "padding-right", id)?;
    let bottom = px_style(style, "padding-bottom", id)?;
    let left = px_style(style, "padding-left", id)?;
    for (key, pointer) in [
        ("width", "/authored/width"),
        ("height", "/authored/height"),
        ("flex-direction", "/authored/layout/direction"),
        ("gap", "/authored/layout/gap"),
        ("padding-top", "/authored/layout/padding/top"),
        ("padding-right", "/authored/layout/padding/right"),
        ("padding-bottom", "/authored/layout/padding/bottom"),
        ("padding-left", "/authored/layout/padding/left"),
        ("align-items", "/authored/layout/align"),
    ] {
        record(
            state,
            CorrespondenceTarget::Entity { id },
            entity_pointer(id, pointer),
            style[key].span,
        );
    }
    let mut entity = Entity::new(id, EntityKind::Container);
    entity.authored = AuthoredProperties {
        width: SizeIntent::Fixed(width),
        height: SizeIntent::Fixed(height),
        layout: LayoutStyle {
            family: LayoutFamily::Stack,
            direction,
            gap,
            padding: Edges {
                top,
                right,
                bottom,
                left,
            },
            align,
            ..LayoutStyle::default()
        },
        ..AuthoredProperties::default()
    };
    Ok(entity)
}

#[expect(
    clippy::too_many_arguments,
    reason = "text validation needs paired Svelte nodes and retained source in one atomic parser step"
)]
fn parse_text(
    id: EntityId,
    tag: &str,
    _node: Node<'_>,
    open: Node<'_>,
    close: Node<'_>,
    source: &str,
    attributes: &BTreeMap<String, Attribute>,
    style: &BTreeMap<String, StyleEntry>,
    state: &mut ParseState,
) -> Result<Entity, AdapterError> {
    require_exact_keys(
        attributes.keys(),
        &[
            "data-nuif-font-sha256",
            "data-nuif-id",
            "data-nuif-kind",
            "data-nuif-name",
            "style",
        ],
        &entity_pointer(id, ""),
        "attribute",
    )?;
    if tag != "span" {
        return invalid(id, "/kind", "text entities must use the regular span tag");
    }
    require_exact_keys(
        style.keys(),
        &["font-family", "font-size", "line-height", "width"],
        &entity_pointer(id, "/authored/text"),
        "style property",
    )?;
    require_text(style, "width", "100%", id)?;
    let font = style_value(style, "font-family", id)?.to_owned();
    let size = px_style(style, "font-size", id)?;
    let line_height = px_style(style, "line-height", id)?;
    let (font_sha256, font_sha_span) = text_attribute(attributes, "data-nuif-font-sha256", source)?;
    let text_span = SourceSpan {
        start: open.end_byte(),
        end: close.start_byte(),
    };
    let raw = &source[text_span.start..text_span.end];
    if raw.contains(['<', '{', '}']) || raw.contains(['\n', '\r', '\t']) {
        return invalid(
            id,
            "/authored/text/content",
            "text must be one literal Svelte text run using canonical entity escapes",
        );
    }
    let content = unescape(raw, &entity_pointer(id, "/authored/text/content"))?;
    for (key, pointer) in [
        ("font-family", "/authored/text/font"),
        ("font-size", "/authored/text/size"),
        ("line-height", "/authored/text/line_height"),
    ] {
        record(
            state,
            CorrespondenceTarget::Entity { id },
            entity_pointer(id, pointer),
            style[key].span,
        );
    }
    record(
        state,
        CorrespondenceTarget::Entity { id },
        entity_pointer(id, "/authored/text/font_sha256"),
        font_sha_span,
    );
    record(
        state,
        CorrespondenceTarget::Entity { id },
        entity_pointer(id, "/authored/text/content"),
        text_span,
    );
    let mut entity = Entity::new(id, EntityKind::Text);
    entity.authored.width = SizeIntent::Fill;
    entity.authored.height = SizeIntent::Intrinsic;
    entity.authored.text = Some(TextContent {
        content,
        font,
        font_sha256,
        font_asset: None,
        size,
        line_height,
    });
    Ok(entity)
}

fn parse_container_children(
    node: Node<'_>,
    source: &str,
    parent: EntityId,
    depth: usize,
    state: &mut ParseState,
) -> Result<(), AdapterError> {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "start_tag" | "end_tag" | "comment" => {}
            "text" | "svelte_raw_text" if source[child.byte_range()].trim().is_empty() => {}
            "element" => {
                parse_element(child, source, Some(parent), depth + 1, false, state)?;
            }
            _ => {
                return invalid(
                    parent,
                    "/children",
                    "container children must be mapped regular elements separated only by whitespace or comments",
                );
            }
        }
    }
    Ok(())
}

fn marked_roots<'tree>(root: Node<'tree>, source: &str) -> Result<Vec<Node<'tree>>, AdapterError> {
    let mut output = Vec::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if node.kind() == "element"
            && let Ok(open) = start_tag(node)
            && source[open.byte_range()].contains("data-nuif-profile")
        {
            let attributes = attributes(open, source)?;
            if let Some(attribute) = attributes.get("data-nuif-profile") {
                let value = text_value(*attribute, "data-nuif-profile", source)?.0;
                if value != PROFILE_NAME {
                    return Err(AdapterError::ProfileMarker(format!(
                        "unsupported profile {value:?}"
                    )));
                }
                output.push(node);
            }
        }
        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
    }
    Ok(output)
}

fn validate_component_root(
    document: Node<'_>,
    mapped_root: Node<'_>,
    source: &str,
) -> Result<(), AdapterError> {
    if mapped_root
        .parent()
        .is_none_or(|parent| parent.id() != document.id())
    {
        return Err(AdapterError::ProfileMarker(
            "mapped root must be a direct component child".to_owned(),
        ));
    }
    let mut cursor = document.walk();
    for child in document.named_children(&mut cursor) {
        if child.id() == mapped_root.id() || child.kind() == "comment" {
            continue;
        }
        if matches!(child.kind(), "text" | "svelte_raw_text")
            && source[child.byte_range()].trim().is_empty()
        {
            continue;
        }
        return Err(AdapterError::ProfileMarker(format!(
            "top-level {} is outside the static component profile",
            child.kind()
        )));
    }
    Ok(())
}

fn attributes(open: Node<'_>, source: &str) -> Result<BTreeMap<String, Attribute>, AdapterError> {
    let mut output = BTreeMap::new();
    let mut cursor = open.walk();
    for child in open.named_children(&mut cursor) {
        match child.kind() {
            "tag_name" => continue,
            "attribute" => {}
            _ => {
                return Err(AdapterError::InvalidValue {
                    pointer: "/entities/*".to_owned(),
                    reason: "directives, spreads and expression attributes are excluded".to_owned(),
                });
            }
        }
        let mut attribute_cursor = child.walk();
        let parts = child
            .named_children(&mut attribute_cursor)
            .collect::<Vec<_>>();
        if parts.len() != 2
            || parts[0].kind() != "attribute_name"
            || parts[1].kind() != "quoted_attribute_value"
        {
            return Err(AdapterError::InvalidValue {
                pointer: "/entities/*".to_owned(),
                reason: "attributes require one unnamespaced name and quoted literal value"
                    .to_owned(),
            });
        }
        let name = source[parts[0].byte_range()].to_owned();
        let raw = &source[parts[1].byte_range()];
        if raw.len() < 2 || !raw.starts_with('"') || !raw.ends_with('"') {
            return Err(AdapterError::InvalidValue {
                pointer: format!("/attributes/{name}"),
                reason: "literal Svelte attributes must use double quotes".to_owned(),
            });
        }
        let span = SourceSpan {
            start: parts[1].start_byte() + 1,
            end: parts[1].end_byte() - 1,
        };
        if source[span.start..span.end].contains(['{', '}']) {
            return Err(AdapterError::InvalidValue {
                pointer: format!("/attributes/{name}"),
                reason: "attribute expressions are excluded".to_owned(),
            });
        }
        if output.insert(name.clone(), Attribute { span }).is_some() {
            return Err(AdapterError::InvalidValue {
                pointer: format!("/attributes/{name}"),
                reason: "attribute is duplicated".to_owned(),
            });
        }
    }
    Ok(output)
}

fn parse_inline_style(
    span: SourceSpan,
    source: &str,
) -> Result<BTreeMap<String, StyleEntry>, AdapterError> {
    let raw = &source[span.start..span.end];
    if raw.contains(['&', '{', '}', '<', '>']) {
        return Err(AdapterError::InvalidValue {
            pointer: "/entities/*/authored".to_owned(),
            reason: "style must be a literal declaration list without entities or expressions"
                .to_owned(),
        });
    }
    let mut output = BTreeMap::new();
    let mut offset = 0;
    for segment in raw.split(';') {
        let segment_start = offset;
        offset += segment.len() + 1;
        let (trim_start, trim_end) = trimmed_range(segment);
        if trim_start == trim_end {
            if segment_start + segment.len() == raw.len() {
                continue;
            }
            return Err(AdapterError::InvalidValue {
                pointer: "/entities/*/authored".to_owned(),
                reason: "style contains an empty declaration".to_owned(),
            });
        }
        let trimmed = &segment[trim_start..trim_end];
        let colon = trimmed
            .find(':')
            .ok_or_else(|| AdapterError::InvalidValue {
                pointer: "/entities/*/authored".to_owned(),
                reason: "style declaration lacks a colon".to_owned(),
            })?;
        if trimmed[colon + 1..].contains(':') {
            return Err(AdapterError::InvalidValue {
                pointer: "/entities/*/authored".to_owned(),
                reason: "style value is outside the scalar profile".to_owned(),
            });
        }
        let key_raw = &trimmed[..colon];
        let value_raw = &trimmed[colon + 1..];
        let (key_start, key_end) = trimmed_range(key_raw);
        let (value_start, value_end) = trimmed_range(value_raw);
        let key = &key_raw[key_start..key_end];
        let value = &value_raw[value_start..value_end];
        if key.is_empty()
            || value.is_empty()
            || !key
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte == b'-')
        {
            return Err(AdapterError::InvalidValue {
                pointer: "/entities/*/authored".to_owned(),
                reason: "style keys and values must be non-empty profile literals".to_owned(),
            });
        }
        let absolute_value_start =
            span.start + segment_start + trim_start + colon + 1 + value_start;
        let entry = StyleEntry {
            value: value.to_owned(),
            span: SourceSpan {
                start: absolute_value_start,
                end: absolute_value_start + value.len(),
            },
        };
        if output.insert(key.to_owned(), entry).is_some() {
            return Err(AdapterError::InvalidValue {
                pointer: format!("/style/{key}"),
                reason: "style property is duplicated".to_owned(),
            });
        }
    }
    Ok(output)
}

fn trimmed_range(value: &str) -> (usize, usize) {
    let start = value.len() - value.trim_start_matches(char::is_whitespace).len();
    let end = value.trim_end_matches(char::is_whitespace).len();
    (start, end.max(start))
}

fn start_tag(node: Node<'_>) -> Result<Node<'_>, AdapterError> {
    named_child(node, "start_tag")
        .ok_or_else(|| AdapterError::SvelteSyntax("mapped element lacks start tag".to_owned()))
}

fn end_tag(node: Node<'_>) -> Result<Node<'_>, AdapterError> {
    named_child(node, "end_tag")
        .ok_or_else(|| AdapterError::SvelteSyntax("mapped element lacks end tag".to_owned()))
}

fn named_child<'tree>(node: Node<'tree>, kind: &str) -> Option<Node<'tree>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find(|child| child.kind() == kind)
}

fn tag_name(node: Node<'_>, source: &str) -> Result<String, AdapterError> {
    let name = named_child(node, "tag_name").ok_or_else(|| AdapterError::InvalidValue {
        pointer: "/entities/*/kind".to_owned(),
        reason: "mapped tags require a regular tag name".to_owned(),
    })?;
    let name = source[name.byte_range()].to_owned();
    if name.bytes().all(|byte| byte.is_ascii_lowercase()) {
        Ok(name)
    } else {
        Err(AdapterError::InvalidValue {
            pointer: "/entities/*/kind".to_owned(),
            reason: "mapped tags must be lowercase regular elements".to_owned(),
        })
    }
}

fn require_root_attributes(
    attributes: &BTreeMap<String, Attribute>,
    source: &str,
) -> Result<(), AdapterError> {
    let (profile, _) = text_attribute(attributes, "data-nuif-profile", source)?;
    if profile != PROFILE_NAME {
        return Err(AdapterError::ProfileMarker(format!(
            "expected {PROFILE_NAME}, observed {profile:?}"
        )));
    }
    text_attribute(attributes, "data-nuif-document", source).map(|_| ())
}

fn text_attribute(
    attributes: &BTreeMap<String, Attribute>,
    name: &str,
    source: &str,
) -> Result<(String, SourceSpan), AdapterError> {
    let attribute = attributes
        .get(name)
        .ok_or_else(|| AdapterError::InvalidValue {
            pointer: format!("/attributes/{name}"),
            reason: "required literal attribute is missing".to_owned(),
        })?;
    text_value(*attribute, name, source)
}

fn text_value(
    attribute: Attribute,
    name: &str,
    source: &str,
) -> Result<(String, SourceSpan), AdapterError> {
    Ok((
        unescape(
            &source[attribute.span.start..attribute.span.end],
            &format!("/attributes/{name}"),
        )?,
        attribute.span,
    ))
}

fn require_exact_keys<'a>(
    observed: impl Iterator<Item = &'a String>,
    required: &[&str],
    pointer: &str,
    label: &str,
) -> Result<(), AdapterError> {
    let observed = observed.map(String::as_str).collect::<BTreeSet<_>>();
    let required = required.iter().copied().collect::<BTreeSet<_>>();
    if observed == required {
        Ok(())
    } else {
        Err(AdapterError::InvalidValue {
            pointer: pointer.to_owned(),
            reason: format!("{label} set differs from the declared static profile"),
        })
    }
}

fn px_style(
    style: &BTreeMap<String, StyleEntry>,
    key: &str,
    id: EntityId,
) -> Result<f64, AdapterError> {
    let value = style_value(style, key, id)?;
    let number = value
        .strip_suffix("px")
        .ok_or_else(|| AdapterError::InvalidValue {
            pointer: entity_pointer(id, "/authored"),
            reason: format!("style {key} must be a literal px length"),
        })?;
    parse_number(number, &entity_pointer(id, "/authored"))
}

fn style_value<'a>(
    style: &'a BTreeMap<String, StyleEntry>,
    key: &str,
    id: EntityId,
) -> Result<&'a str, AdapterError> {
    style
        .get(key)
        .map(|entry| entry.value.as_str())
        .ok_or_else(|| AdapterError::InvalidValue {
            pointer: entity_pointer(id, "/authored"),
            reason: format!("style {key} is required"),
        })
}

fn require_text(
    style: &BTreeMap<String, StyleEntry>,
    key: &str,
    expected: &str,
    id: EntityId,
) -> Result<(), AdapterError> {
    if style_value(style, key, id)? == expected {
        Ok(())
    } else {
        invalid(
            id,
            "/authored",
            &format!("style {key} must equal {expected:?}"),
        )
    }
}

fn invalid<T>(id: EntityId, suffix: &str, reason: &str) -> Result<T, AdapterError> {
    Err(AdapterError::InvalidValue {
        pointer: entity_pointer(id, suffix),
        reason: reason.to_owned(),
    })
}

fn record(state: &mut ParseState, target: CorrespondenceTarget, pointer: String, span: SourceSpan) {
    state.correspondences.push(CorrespondenceRecord {
        target: target.clone(),
        pointer: pointer.clone(),
        span,
    });
    state.fidelity.push(FidelityEntry {
        target,
        pointer,
        status: Fidelity::Lossless,
    });
}

fn count_nodes(root: Node<'_>) -> usize {
    let mut count = 0_usize;
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        count = count.saturating_add(1);
        let mut cursor = node.walk();
        stack.extend(node.children(&mut cursor));
    }
    count
}

fn first_error(root: Node<'_>) -> String {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if node.is_error() || node.is_missing() {
            return format!(
                "{} at byte range {}..{}",
                node.kind(),
                node.start_byte(),
                node.end_byte()
            );
        }
        let mut cursor = node.walk();
        stack.extend(node.children(&mut cursor));
    }
    "syntax tree contains an unclassified error".to_owned()
}
