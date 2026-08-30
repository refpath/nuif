use nuif_codec::canonical_hash;
use nuif_core::{
    Align, AuthoredProperties, Document, Edges, Entity, EntityId, EntityKind, Fidelity,
    FlowDirection, LayoutFamily, LayoutStyle, SizeIntent, TextContent,
};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::{Node, Parser};

use crate::profile::profile_issues;
use crate::syntax::{parse_id, parse_number, unescape_jsx};
use crate::{
    AdapterError, AdapterReport, CorrespondenceRecord, CorrespondenceTarget, FidelityEntry,
    ImportedSource, MAX_JSX_DEPTH, MAX_SOURCE_BYTES, MAX_SYNTAX_NODES, PROFILE_NAME,
    RetentiveSource, SourceSpan, entity_pointer,
};

#[derive(Clone, Copy)]
enum AttributeValue<'tree> {
    Text { span: SourceSpan },
    Expression(Node<'tree>),
}

struct Attribute<'tree> {
    value: AttributeValue<'tree>,
}

#[derive(Clone, Debug)]
enum StyleValue {
    Number(f64),
    Text(String),
}

#[derive(Clone, Debug)]
struct StyleEntry {
    value: StyleValue,
    span: SourceSpan,
}

struct ParseState {
    document: Document,
    correspondences: Vec<CorrespondenceRecord>,
    fidelity: Vec<FidelityEntry>,
}

/// Imports one statically marked intrinsic JSX subtree without executing JavaScript.
///
/// # Errors
///
/// Rejects syntax errors, dynamic expressions, components, source limits,
/// duplicate identities and any model state outside the declared profile.
pub fn import_source(source: &str) -> Result<ImportedSource, AdapterError> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(AdapterError::SourceTooLarge);
    }
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_javascript::LANGUAGE.into())
        .map_err(|error| AdapterError::JsxSyntax(error.to_string()))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| AdapterError::JsxSyntax("parser returned no tree".to_owned()))?;
    let root = tree.root_node();
    if root.has_error() {
        return Err(AdapterError::JsxSyntax(first_error(root)));
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
    validate_static_wrapper(roots[0], source)?;
    let open = roots[0]
        .child_by_field_name("open_tag")
        .ok_or_else(|| AdapterError::JsxSyntax("mapped root lacks opening tag".to_owned()))?;
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
    if depth > MAX_JSX_DEPTH {
        return Err(AdapterError::DepthLimit);
    }
    if node.kind() != "jsx_element" {
        return Err(AdapterError::InvalidValue {
            pointer: "/entities".to_owned(),
            reason: "mapped nodes must be paired intrinsic JSX elements".to_owned(),
        });
    }
    let open = node
        .child_by_field_name("open_tag")
        .ok_or_else(|| AdapterError::JsxSyntax("mapped element lacks opening tag".to_owned()))?;
    let close = node
        .child_by_field_name("close_tag")
        .ok_or_else(|| AdapterError::JsxSyntax("mapped element lacks closing tag".to_owned()))?;
    let tag = tag_name(open, source)?;
    if tag != tag_name(close, source)? {
        return Err(AdapterError::JsxSyntax(
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
    record(
        state,
        target.clone(),
        entity_pointer(id, "/name"),
        name_span,
    );
    if is_root {
        require_root_attributes(&attributes, source)?;
    } else if attributes.contains_key("data-nuif-profile")
        || attributes.contains_key("data-nuif-document")
    {
        return Err(AdapterError::ProfileMarker(
            "profile and document markers are allowed only on the mapped root".to_owned(),
        ));
    }

    let style_node = expression_attribute(&attributes, "style")?;
    let style = parse_style(style_node, source)?;
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
    reason = "the exact container style vocabulary and its correspondence mapping stay adjacent for audit"
)]
fn parse_container(
    id: EntityId,
    tag: &str,
    attributes: &BTreeMap<String, Attribute<'_>>,
    style: &BTreeMap<String, StyleEntry>,
    state: &mut ParseState,
) -> Result<Entity, AdapterError> {
    const KEYS: &[&str] = &[
        "alignItems",
        "boxSizing",
        "display",
        "flexDirection",
        "gap",
        "height",
        "paddingBottom",
        "paddingLeft",
        "paddingRight",
        "paddingTop",
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
        return invalid(id, "/kind", "containers must use the intrinsic div tag");
    }
    require_exact_keys(
        style.keys(),
        KEYS,
        &entity_pointer(id, "/authored"),
        "style property",
    )?;
    require_text(style, "boxSizing", "border-box", id)?;
    require_text(style, "display", "flex", id)?;
    let direction = match text_style(style, "flexDirection", id)?.as_str() {
        "row" => FlowDirection::Row,
        "column" => FlowDirection::Column,
        _ => {
            return invalid(
                id,
                "/authored/layout/direction",
                "flexDirection must be row or column",
            );
        }
    };
    let align = match text_style(style, "alignItems", id)?.as_str() {
        "flex-start" => Align::Start,
        "center" => Align::Center,
        "flex-end" => Align::End,
        "stretch" => Align::Stretch,
        _ => {
            return invalid(
                id,
                "/authored/layout/align",
                "alignItems is outside the profile",
            );
        }
    };
    let width = number_style(style, "width", id)?;
    let height = number_style(style, "height", id)?;
    let gap = number_style(style, "gap", id)?;
    let top = number_style(style, "paddingTop", id)?;
    let right = number_style(style, "paddingRight", id)?;
    let bottom = number_style(style, "paddingBottom", id)?;
    let left = number_style(style, "paddingLeft", id)?;
    for (key, pointer) in [
        ("width", "/authored/width"),
        ("height", "/authored/height"),
        ("flexDirection", "/authored/layout/direction"),
        ("gap", "/authored/layout/gap"),
        ("paddingTop", "/authored/layout/padding/top"),
        ("paddingRight", "/authored/layout/padding/right"),
        ("paddingBottom", "/authored/layout/padding/bottom"),
        ("paddingLeft", "/authored/layout/padding/left"),
        ("alignItems", "/authored/layout/align"),
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
    reason = "text validation needs the paired JSX nodes, retained source and report state in one atomic parser step"
)]
fn parse_text(
    id: EntityId,
    tag: &str,
    node: Node<'_>,
    open: Node<'_>,
    close: Node<'_>,
    source: &str,
    attributes: &BTreeMap<String, Attribute<'_>>,
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
        return invalid(id, "/kind", "text entities must use the intrinsic span tag");
    }
    require_exact_keys(
        style.keys(),
        &["fontFamily", "fontSize", "lineHeight", "width"],
        &entity_pointer(id, "/authored/text"),
        "style property",
    )?;
    require_text(style, "width", "100%", id)?;
    let font = text_style(style, "fontFamily", id)?;
    let size = number_style(style, "fontSize", id)?;
    let line_height = text_style(style, "lineHeight", id)?
        .strip_suffix("px")
        .ok_or_else(|| AdapterError::InvalidValue {
            pointer: entity_pointer(id, "/authored/text/line_height"),
            reason: "lineHeight must be a literal px string".to_owned(),
        })
        .and_then(|value| parse_number(value, &entity_pointer(id, "/authored/text/line_height")))?;
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
            "text must be one literal JSX text run using canonical entity escapes",
        );
    }
    let mut cursor = node.walk();
    if node.named_children(&mut cursor).any(|child| {
        !matches!(
            child.kind(),
            "jsx_opening_element" | "jsx_closing_element" | "jsx_text" | "html_character_reference"
        )
    }) {
        return invalid(
            id,
            "/authored/text/content",
            "text cannot contain JSX expressions or nested elements",
        );
    }
    let content = unescape_jsx(raw, &entity_pointer(id, "/authored/text/content"))?;
    for (key, pointer) in [
        ("fontFamily", "/authored/text/font"),
        ("fontSize", "/authored/text/size"),
        ("lineHeight", "/authored/text/line_height"),
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
            "jsx_opening_element" | "jsx_closing_element" => {}
            "jsx_text" if source[child.byte_range()].trim().is_empty() => {}
            "jsx_element" => {
                parse_element(child, source, Some(parent), depth + 1, false, state)?;
            }
            _ => {
                return invalid(
                    parent,
                    "/children",
                    "container children must be mapped paired intrinsic elements separated only by whitespace",
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
        if node.kind() == "jsx_element"
            && let Some(open) = node.child_by_field_name("open_tag")
            && source[open.byte_range()].contains("data-nuif-profile")
        {
            let attributes = attributes(open, source)?;
            if let Some(attribute) = attributes.get("data-nuif-profile") {
                let (value, _) = text_value(attribute, "data-nuif-profile", source)?;
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

fn validate_static_wrapper(node: Node<'_>, source: &str) -> Result<(), AdapterError> {
    let mut current = node;
    while let Some(parent) = current.parent() {
        match parent.kind() {
            "parenthesized_expression" => current = parent,
            "return_statement" => {
                let block = parent.parent().ok_or_else(|| {
                    AdapterError::ProfileMarker("mapped return lacks function body".to_owned())
                })?;
                if block.kind() != "statement_block" {
                    return Err(AdapterError::ProfileMarker(
                        "mapped JSX must be returned directly from a function".to_owned(),
                    ));
                }
                let mut cursor = block.walk();
                if block
                    .named_children(&mut cursor)
                    .filter(|child| child.kind() != "comment")
                    .any(|child| child.id() != parent.id())
                {
                    return Err(AdapterError::ProfileMarker(
                        "mapped component body may contain only its direct return".to_owned(),
                    ));
                }
                let function = block.parent().ok_or_else(|| {
                    AdapterError::ProfileMarker("mapped return lacks function".to_owned())
                })?;
                if function.kind() != "function_declaration"
                    || function
                        .child_by_field_name("parameters")
                        .is_none_or(|parameters| source[parameters.byte_range()].trim() != "()")
                    || function
                        .parent()
                        .is_none_or(|parent| parent.kind() != "export_statement")
                {
                    return Err(AdapterError::ProfileMarker(
                        "mapped JSX requires an exported zero-argument function declaration"
                            .to_owned(),
                    ));
                }
                let export = function.parent().expect("checked export parent");
                let mut export_cursor = export.walk();
                if !export
                    .children(&mut export_cursor)
                    .any(|child| child.kind() == "default")
                {
                    return Err(AdapterError::ProfileMarker(
                        "mapped function must be the default export".to_owned(),
                    ));
                }
                let mut function_cursor = function.walk();
                if function
                    .children(&mut function_cursor)
                    .any(|child| child.kind() == "async")
                {
                    return Err(AdapterError::ProfileMarker(
                        "mapped function must be synchronous".to_owned(),
                    ));
                }
                return Ok(());
            }
            _ => {
                return Err(AdapterError::ProfileMarker(
                    "mapped JSX must be the direct returned expression".to_owned(),
                ));
            }
        }
    }
    Err(AdapterError::ProfileMarker(
        "mapped JSX lacks an exported function wrapper".to_owned(),
    ))
}

fn attributes<'tree>(
    open: Node<'tree>,
    source: &str,
) -> Result<BTreeMap<String, Attribute<'tree>>, AdapterError> {
    let mut output = BTreeMap::new();
    let mut cursor = open.walk();
    for child in open.named_children(&mut cursor) {
        if child.kind() == "jsx_expression" {
            return Err(AdapterError::InvalidValue {
                pointer: "/entities/*".to_owned(),
                reason: "JSX attribute spreads are not executable in the static profile".to_owned(),
            });
        }
        if child.kind() != "jsx_attribute" {
            continue;
        }
        let mut attribute_cursor = child.walk();
        let parts = child
            .named_children(&mut attribute_cursor)
            .collect::<Vec<_>>();
        if parts.len() != 2 || parts[0].kind() != "property_identifier" {
            return Err(AdapterError::InvalidValue {
                pointer: "/entities/*".to_owned(),
                reason: "attributes require one unnamespaced name and literal value".to_owned(),
            });
        }
        let name = source[parts[0].byte_range()].to_owned();
        let value = match parts[1].kind() {
            "string" => {
                let raw = &source[parts[1].byte_range()];
                if !raw.starts_with('"') || !raw.ends_with('"') || raw.len() < 2 {
                    return Err(AdapterError::InvalidValue {
                        pointer: format!("/attributes/{name}"),
                        reason: "literal JSX attributes must use double quotes".to_owned(),
                    });
                }
                AttributeValue::Text {
                    span: SourceSpan {
                        start: parts[1].start_byte() + 1,
                        end: parts[1].end_byte() - 1,
                    },
                }
            }
            "jsx_expression" => AttributeValue::Expression(parts[1]),
            _ => {
                return Err(AdapterError::InvalidValue {
                    pointer: format!("/attributes/{name}"),
                    reason: "attribute value is outside the literal profile".to_owned(),
                });
            }
        };
        if output.insert(name.clone(), Attribute { value }).is_some() {
            return Err(AdapterError::InvalidValue {
                pointer: format!("/attributes/{name}"),
                reason: "attribute is duplicated".to_owned(),
            });
        }
    }
    Ok(output)
}

fn parse_style(
    expression: Node<'_>,
    source: &str,
) -> Result<BTreeMap<String, StyleEntry>, AdapterError> {
    let mut cursor = expression.walk();
    let children = expression.named_children(&mut cursor).collect::<Vec<_>>();
    if children.len() != 1 || children[0].kind() != "object" {
        return Err(AdapterError::InvalidValue {
            pointer: "/entities/*/authored".to_owned(),
            reason: "style must be one literal object expression".to_owned(),
        });
    }
    let object = children[0];
    let mut output = BTreeMap::new();
    let mut object_cursor = object.walk();
    for child in object.named_children(&mut object_cursor) {
        if child.kind() == "comment" {
            continue;
        }
        if child.kind() != "pair" {
            return Err(AdapterError::InvalidValue {
                pointer: "/entities/*/authored".to_owned(),
                reason: "style spreads, methods and shorthand properties are excluded".to_owned(),
            });
        }
        let key = child
            .child_by_field_name("key")
            .filter(|key| key.kind() == "property_identifier")
            .ok_or_else(|| AdapterError::InvalidValue {
                pointer: "/entities/*/authored".to_owned(),
                reason: "style keys must be uncomputed identifiers".to_owned(),
            })?;
        let value =
            child
                .child_by_field_name("value")
                .ok_or_else(|| AdapterError::InvalidValue {
                    pointer: "/entities/*/authored".to_owned(),
                    reason: "style property lacks a value".to_owned(),
                })?;
        let name = source[key.byte_range()].to_owned();
        let parsed = match value.kind() {
            "number" => StyleValue::Number(parse_number(
                &source[value.byte_range()],
                &format!("/style/{name}"),
            )?),
            "string" => {
                let raw = &source[value.byte_range()];
                if !raw.starts_with('"') || !raw.ends_with('"') {
                    return Err(AdapterError::InvalidValue {
                        pointer: format!("/style/{name}"),
                        reason: "style strings must use JSON-compatible double quotes".to_owned(),
                    });
                }
                StyleValue::Text(serde_json::from_str(raw).map_err(|error| {
                    AdapterError::InvalidValue {
                        pointer: format!("/style/{name}"),
                        reason: error.to_string(),
                    }
                })?)
            }
            _ => {
                return Err(AdapterError::InvalidValue {
                    pointer: format!("/style/{name}"),
                    reason: "style values must be literal numbers or strings".to_owned(),
                });
            }
        };
        if output
            .insert(
                name.clone(),
                StyleEntry {
                    value: parsed,
                    span: SourceSpan {
                        start: value.start_byte(),
                        end: value.end_byte(),
                    },
                },
            )
            .is_some()
        {
            return Err(AdapterError::InvalidValue {
                pointer: format!("/style/{name}"),
                reason: "style property is duplicated".to_owned(),
            });
        }
    }
    Ok(output)
}

fn tag_name(node: Node<'_>, source: &str) -> Result<String, AdapterError> {
    node.child_by_field_name("name")
        .filter(|name| name.kind() == "identifier")
        .map(|name| source[name.byte_range()].to_owned())
        .ok_or_else(|| AdapterError::InvalidValue {
            pointer: "/entities/*/kind".to_owned(),
            reason:
                "mapped tags must be lowercase intrinsic identifiers, not components or namespaces"
                    .to_owned(),
        })
        .and_then(|name| {
            if name.bytes().all(|byte| byte.is_ascii_lowercase()) {
                Ok(name)
            } else {
                Err(AdapterError::InvalidValue {
                    pointer: "/entities/*/kind".to_owned(),
                    reason: "mapped tags must be lowercase intrinsic identifiers".to_owned(),
                })
            }
        })
}

fn require_root_attributes(
    attributes: &BTreeMap<String, Attribute<'_>>,
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
    attributes: &BTreeMap<String, Attribute<'_>>,
    name: &str,
    source: &str,
) -> Result<(String, SourceSpan), AdapterError> {
    let attribute = attributes
        .get(name)
        .ok_or_else(|| AdapterError::InvalidValue {
            pointer: format!("/attributes/{name}"),
            reason: "required literal attribute is missing".to_owned(),
        })?;
    text_value(attribute, name, source)
}

fn text_value(
    attribute: &Attribute<'_>,
    name: &str,
    source: &str,
) -> Result<(String, SourceSpan), AdapterError> {
    let AttributeValue::Text { span } = attribute.value else {
        return Err(AdapterError::InvalidValue {
            pointer: format!("/attributes/{name}"),
            reason: "attribute must be a quoted literal".to_owned(),
        });
    };
    Ok((
        unescape_jsx(
            &source[span.start..span.end],
            &format!("/attributes/{name}"),
        )?,
        span,
    ))
}

fn expression_attribute<'tree>(
    attributes: &BTreeMap<String, Attribute<'tree>>,
    name: &str,
) -> Result<Node<'tree>, AdapterError> {
    match attributes.get(name).map(|attribute| attribute.value) {
        Some(AttributeValue::Expression(node)) => Ok(node),
        _ => Err(AdapterError::InvalidValue {
            pointer: format!("/attributes/{name}"),
            reason: "attribute must be a literal JSX expression".to_owned(),
        }),
    }
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

fn number_style(
    style: &BTreeMap<String, StyleEntry>,
    key: &str,
    id: EntityId,
) -> Result<f64, AdapterError> {
    match style.get(key).map(|entry| &entry.value) {
        Some(StyleValue::Number(value)) => Ok(*value),
        _ => invalid(
            id,
            "/authored",
            &format!("style.{key} must be a finite number"),
        ),
    }
}

fn text_style(
    style: &BTreeMap<String, StyleEntry>,
    key: &str,
    id: EntityId,
) -> Result<String, AdapterError> {
    match style.get(key).map(|entry| &entry.value) {
        Some(StyleValue::Text(value)) => Ok(value.clone()),
        _ => invalid(
            id,
            "/authored",
            &format!("style.{key} must be a literal string"),
        ),
    }
}

fn require_text(
    style: &BTreeMap<String, StyleEntry>,
    key: &str,
    expected: &str,
    id: EntityId,
) -> Result<(), AdapterError> {
    if text_style(style, key, id)? == expected {
        Ok(())
    } else {
        invalid(
            id,
            "/authored",
            &format!("style.{key} must equal {expected:?}"),
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
